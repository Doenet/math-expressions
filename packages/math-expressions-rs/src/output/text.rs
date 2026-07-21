//! Text output — a precedence-based pretty-printer walking `Expr` directly.
//!
//! Unlike the JS `ast-to-text.js` (which decides parenthesisation by regex-
//! matching its own rendered output), this tracks numeric precedence: every
//! node renders knowing its precedence, and a parent parenthesises a child
//! only when the child's precedence is below what the position requires. The
//! precedence ladder matches the text grammar, so output round-trips through
//! the parser with minimal parentheses. Correctness is enforced by
//! `tests/roundtrip.rs` (`parse(to_text(e))` is structurally equal to `e`).

use super::{
    deriv_var, f64_positional_string, greek_unicode, number_is_negative as is_negative, pow_suffix,
    prec, split_sign,
};
use crate::expr::{Expr, MathConst, RelOp, SeqKind};
use crate::num::Number;

#[derive(Debug, Clone)]
pub struct TextOpts {
    /// Emit unicode operators/greek letters (`≤`, `θ`) vs ASCII words.
    pub unicode: bool,
}

impl Default for TextOpts {
    fn default() -> Self {
        TextOpts { unicode: true }
    }
}

pub fn convert(expr: &Expr, opts: &TextOpts) -> String {
    Writer { opts }.emit(expr, 0)
}

struct Writer<'a> {
    opts: &'a TextOpts,
}

impl Writer<'_> {
    /// Render `e`, wrapping in parens if its precedence is below `ctx`.
    fn emit(&self, e: &Expr, ctx: u8) -> String {
        let (s, p) = self.render(e);
        if p < ctx {
            format!("({})", s)
        } else {
            s
        }
    }

    fn render(&self, e: &Expr) -> (String, u8) {
        use prec::{ADD, AND, ATOM, INDEX, MUL, NEG, NOT, OR, POW, REL, SIGN};
        match e {
            Expr::Num(n) => self.render_number(n),
            // Prints as its function-application spelling, which reparses to
            // the same leaf.
            Expr::RootOf { poly, index } => {
                self.render(&crate::rootof::as_apply(poly, *index))
            }
            Expr::Sym(s) => {
                let name = s.name();
                // Sign-string symbols (name contains + or -) re-lex as
                // operators, so they need parens in any operator context.
                let p = if name.contains(['+', '-']) {
                    SIGN
                } else {
                    ATOM
                };
                (self.render_symbol(&name), p)
            }
            // NegInf renders with a leading sign, so it binds like a negation.
            Expr::Const(c) => (
                self.render_const(*c),
                if *c == MathConst::NegInf { NEG } else { ATOM },
            ),
            Expr::Blank => ("\u{ff3f}".to_string(), ATOM),
            Expr::Ldots => ("...".to_string(), ATOM),

            Expr::Add(terms) => (self.render_add(terms), ADD),
            Expr::Mul(factors) => (self.render_mul(factors), MUL),
            Expr::Div(a, b) => (
                format!("{}/{}", self.emit(a, MUL), self.emit(b, MUL + 1)),
                MUL,
            ),
            Expr::Neg(x) => (format!("-{}", self.emit(x, MUL)), NEG),
            // `^` is left-associative; its superscript slot in the grammar is a
            // single tight atom, so anything but a simple atom/subscript needs
            // parens to round-trip.
            Expr::Pow(b, e) => {
                // `(x^y)^z` must parenthesize the inner power; only a power base
                // needs it — other same-precedence bases (`f'` in `f'^a(x)`) stay
                // unwrapped to round-trip.
                let base_ctx = if matches!(&**b, Expr::Pow(..)) { POW + 1 } else { POW };
                (
                    format!("{}^{}", self.emit(b, base_ctx), self.superscript(e)),
                    POW,
                )
            }

            Expr::And(xs) => (self.join_logical(xs, " and "), AND),
            Expr::Or(xs) => (self.join_logical(xs, " or "), OR),
            Expr::Not(x) => (
                format!(
                    "{}{}",
                    if self.opts.unicode { "¬" } else { "not " },
                    self.paren_if_spaced(x)
                ),
                NOT,
            ),
            Expr::Union(xs) => (
                self.join(
                    xs,
                    if self.opts.unicode {
                        " ∪ "
                    } else {
                        " union "
                    },
                    ADD + 1,
                ),
                ADD,
            ),
            Expr::Intersect(xs) => (
                self.join(
                    xs,
                    if self.opts.unicode {
                        " ∩ "
                    } else {
                        " intersect "
                    },
                    ADD + 1,
                ),
                ADD,
            ),

            Expr::Prime(x) => (format!("{}'", self.emit(x, POW)), POW),
            // `_` is right-associative: `x_y_z` parses as `x_(y_z)`, so the base
            // (left) is the tighter side.
            Expr::Index(a, b) => (
                format!("{}_{}", self.emit(a, INDEX + 1), self.superscript(b)),
                INDEX,
            ),

            Expr::Apply(head, args) => self.render_apply(head, args),
            Expr::Seq(kind, xs) => self.render_seq(*kind, xs),
            Expr::Interval { endpoints, closed } => {
                (self.render_interval(endpoints, *closed), ATOM)
            }
            Expr::Relation { operands, ops } => (self.render_relation(operands, ops), REL),
            Expr::Matrix {
                rows,
                cols,
                entries,
            } => (self.render_matrix(*rows, *cols, entries), ATOM),
            Expr::OtherOp(name, args) => self.render_other(&name.name(), args),
        }
    }

    fn join(&self, xs: &[Expr], sep: &str, ctx: u8) -> String {
        xs.iter()
            .map(|x| self.emit(x, ctx))
            .collect::<Vec<_>>()
            .join(sep)
    }

    /// Parenthesize a logical operand that renders as a compound expression (its
    /// string has a space and is not already fully parenthesized) — port of the
    /// JS ast-to-text `and`/`or`/`not` rule.
    fn paren_if_spaced(&self, e: &Expr) -> String {
        let s = self.emit(e, 0);
        if s.contains(' ') && !(s.starts_with('(') && s.ends_with(')')) {
            format!("({})", s)
        } else {
            s
        }
    }

    fn join_logical(&self, xs: &[Expr], sep: &str) -> String {
        xs.iter()
            .map(|x| self.paren_if_spaced(x))
            .collect::<Vec<_>>()
            .join(sep)
    }

    fn render_number(&self, n: &Number) -> (String, u8) {
        // Terminating decimals (all integers, and every parse-produced
        // rational — denominator 2^a·5^b) render positionally, as atoms.
        if let Some(dec) = n.terminating_decimal() {
            let p = if dec.starts_with('-') {
                prec::NEG
            } else {
                prec::ATOM
            };
            return (dec, p);
        }
        // A non-terminating fraction (only from later normalization) renders
        // as `a/b`, binding like the division it re-parses to.
        if let Some((num, den)) = n.rational_parts() {
            let p = if num.starts_with('-') {
                prec::NEG
            } else {
                prec::MUL
            };
            return (format!("{}/{}", num, den), p);
        }
        // Float: numerical-evaluation result, positional (never exponential).
        let s = f64_positional_string(n.to_f64());
        let p = if s.starts_with('-') {
            prec::NEG
        } else {
            prec::ATOM
        };
        (s, p)
    }

    fn render_symbol(&self, name: &str) -> String {
        if self.opts.unicode {
            if let Some(u) = greek_unicode(name) {
                return u.to_string();
            }
        }
        name.to_string()
    }

    fn render_const(&self, c: MathConst) -> String {
        match c {
            MathConst::Pi => if self.opts.unicode { "π" } else { "pi" }.to_string(),
            MathConst::E => "e".to_string(),
            MathConst::I => "i".to_string(),
            MathConst::Inf => if self.opts.unicode { "∞" } else { "infinity" }.to_string(),
            MathConst::NegInf => if self.opts.unicode {
                "-∞"
            } else {
                "-infinity"
            }
            .to_string(),
            MathConst::NaN => "NaN".to_string(),
        }
    }

    /// A sum: first term rendered with its own sign, later terms joined with
    /// ` + `/` - ` by inspecting the term structurally (Neg or negative Num) —
    /// never by string-matching, and never pulling a sign out of a Mul (which
    /// would not round-trip).
    fn render_add(&self, terms: &[Expr]) -> String {
        // A single-element Add is the parser's unary-plus form (`+x`).
        if terms.len() == 1 {
            return format!("+{}", self.emit(&terms[0], prec::ADD + 1));
        }
        let mut out = String::new();
        for (i, t) in terms.iter().enumerate() {
            // A `±` term carries its own operator (`± …`), so it is joined with a
            // plain space rather than ` + ` — `5 + ±3` would be wrong.
            if i > 0 && crate::pm::is_pm(t) {
                out.push(' ');
                out.push_str(&self.emit(t, prec::ADD + 1));
                continue;
            }
            let (neg, body) = split_sign(t);
            if i == 0 {
                if neg {
                    out.push('-');
                }
            } else if neg {
                out.push_str(" - ");
            } else {
                out.push_str(" + ");
            }
            out.push_str(&self.emit(&body, prec::ADD + 1));
        }
        out
    }

    fn render_mul(&self, factors: &[Expr]) -> String {
        let mut out = String::new();
        for (i, f) in factors.iter().enumerate() {
            let s = self.emit(f, if i == 0 { prec::MUL } else { prec::MUL + 1 });
            if i > 0 {
                // A space disambiguates tokens; use ` * ` when the right factor
                // begins with a digit (so two numbers don't merge) or the left
                // factor is a shorthand `∠A` (which would otherwise absorb it).
                if s.starts_with(|c: char| c.is_ascii_digit())
                    || is_shorthand_angle(&factors[i - 1])
                {
                    out.push_str(" * ");
                } else {
                    out.push(' ');
                }
            }
            out.push_str(&s);
        }
        out
    }

    fn render_apply(&self, head: &Expr, args: &[Expr]) -> (String, u8) {
        // Special notations for particular function heads.
        if let Expr::Sym(s) = head {
            match s.name().as_str() {
                "abs" if args.len() == 1 => {
                    return (format!("|{}|", self.emit(&args[0], 0)), prec::ATOM)
                }
                // factorial is postfix `!`, so it prints at POW precedence.
                "factorial" if args.len() == 1 => {
                    return (format!("{}!", self.emit(&args[0], prec::POW)), prec::POW)
                }
                _ => {}
            }
        }
        let args_str = args
            .iter()
            .map(|a| self.emit(a, prec::LIST + 1))
            .collect::<Vec<_>>()
            .join(", ");
        // The head is a "modified function" (symbol with primes/subscripts/
        // superscripts) — render at POW so `f'(x)`, `sin^2(x)` don't get the
        // head parenthesised (which would re-parse as multiplication).
        (
            format!("{}({})", self.emit(head, prec::POW), args_str),
            prec::ATOM,
        )
    }

    /// A subscript or superscript slot: the grammar accepts only a single
    /// tight atom there, so wrap anything else in parens.
    fn superscript(&self, e: &Expr) -> String {
        if is_simple_superscript(e) {
            self.emit(e, 0)
        } else {
            format!("({})", self.emit(e, 0))
        }
    }

    fn render_seq(&self, kind: SeqKind, xs: &[Expr]) -> (String, u8) {
        let inner = xs
            .iter()
            .map(|x| self.emit(x, prec::LIST + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let (s, p) = match kind {
            SeqKind::List => (inner, prec::LIST),
            SeqKind::Tuple | SeqKind::Vector => (format!("({})", inner), prec::ATOM),
            SeqKind::Array => (format!("[{}]", inner), prec::ATOM),
            SeqKind::Set => (format!("{{{}}}", inner), prec::ATOM),
            SeqKind::AltVector => (
                if self.opts.unicode {
                    format!("⟨{}⟩", inner)
                } else {
                    format!("({})", inner)
                },
                prec::ATOM,
            ),
        };
        (s, p)
    }

    fn render_interval(&self, endpoints: &(Expr, Expr), closed: (bool, bool)) -> String {
        let lo = self.emit(&endpoints.0, prec::LIST + 1);
        let hi = self.emit(&endpoints.1, prec::LIST + 1);
        let left = if closed.0 { '[' } else { '(' };
        let right = if closed.1 { ']' } else { ')' };
        format!("{}{}, {}{}", left, lo, hi, right)
    }

    fn render_relation(&self, operands: &[Expr], ops: &[RelOp]) -> String {
        let mut out = self.emit(&operands[0], prec::REL + 1);
        for (i, op) in ops.iter().enumerate() {
            out.push_str(&format!(" {} ", self.rel_symbol(*op)));
            out.push_str(&self.emit(&operands[i + 1], prec::REL + 1));
        }
        out
    }

    fn rel_symbol(&self, op: RelOp) -> &'static str {
        let u = self.opts.unicode;
        match op {
            RelOp::Eq => "=",
            RelOp::Ne => {
                if u {
                    "≠"
                } else {
                    "!="
                }
            }
            RelOp::Lt => "<",
            RelOp::Le => {
                if u {
                    "≤"
                } else {
                    "<="
                }
            }
            RelOp::Gt => ">",
            RelOp::Ge => {
                if u {
                    "≥"
                } else {
                    ">="
                }
            }
            RelOp::In => {
                if u {
                    "∈"
                } else {
                    "elementof"
                }
            }
            RelOp::NotIn => {
                if u {
                    "∉"
                } else {
                    "notelementof"
                }
            }
            RelOp::Ni => {
                if u {
                    "∋"
                } else {
                    "containselement"
                }
            }
            RelOp::NotNi => {
                if u {
                    "∌"
                } else {
                    "notcontainselement"
                }
            }
            RelOp::Subset => {
                if u {
                    "⊂"
                } else {
                    "subset"
                }
            }
            RelOp::NotSubset => {
                if u {
                    "⊄"
                } else {
                    "notsubset"
                }
            }
            RelOp::SubsetEq => {
                if u {
                    "⊆"
                } else {
                    "subseteq"
                }
            }
            RelOp::NotSubsetEq => {
                if u {
                    "⊈"
                } else {
                    "notsubseteq"
                }
            }
            RelOp::Superset => {
                if u {
                    "⊃"
                } else {
                    "superset"
                }
            }
            RelOp::NotSuperset => {
                if u {
                    "⊅"
                } else {
                    "notsuperset"
                }
            }
            RelOp::SupersetEq => {
                if u {
                    "⊇"
                } else {
                    "superseteq"
                }
            }
            RelOp::NotSupersetEq => {
                if u {
                    "⊉"
                } else {
                    "notsuperseteq"
                }
            }
        }
    }

    fn render_matrix(&self, rows: u32, cols: u32, entries: &[Expr]) -> String {
        // Text has no matrix input syntax; this is display-only.
        let mut out = String::from("[");
        for r in 0..rows as usize {
            let row: Vec<String> = (0..cols as usize)
                .map(|c| self.emit(&entries[r * cols as usize + c], prec::LIST + 1))
                .collect();
            out.push_str(&format!("[{}]", row.join(", ")));
            if r < rows as usize - 1 {
                out.push_str(", ");
            }
        }
        out.push(']');
        out
    }

    /// The long tail of notation operators carried as `OtherOp`.
    fn render_other(&self, name: &str, args: &[Expr]) -> (String, u8) {
        use prec::*;
        let one = |w: &Self, ctx| w.emit(&args[0], ctx);
        match name {
            "pm" => (
                format!(
                    "{} {}",
                    if self.opts.unicode { "±" } else { "+-" },
                    one(self, MUL)
                ),
                NEG,
            ),
            "forall" => (
                format!(
                    "{} {}",
                    if self.opts.unicode { "∀" } else { "forall" },
                    one(self, REL)
                ),
                REL,
            ),
            "exists" => (
                format!(
                    "{} {}",
                    if self.opts.unicode { "∃" } else { "exists" },
                    one(self, REL)
                ),
                REL,
            ),
            "implies" => (
                self.join(
                    args,
                    if self.opts.unicode {
                        " ⟹ "
                    } else {
                        " implies "
                    },
                    ARROW + 1,
                ),
                ARROW,
            ),
            "impliedby" => (
                self.join(
                    args,
                    if self.opts.unicode {
                        " ⟸ "
                    } else {
                        " impliedby "
                    },
                    ARROW + 1,
                ),
                ARROW,
            ),
            "iff" => (
                self.join(
                    args,
                    if self.opts.unicode { " ⟺ " } else { " iff " },
                    ARROW + 1,
                ),
                ARROW,
            ),
            "rightarrow" => (
                self.join(
                    args,
                    if self.opts.unicode {
                        " → "
                    } else {
                        " rightarrow "
                    },
                    ARROW + 1,
                ),
                ARROW,
            ),
            "leftarrow" => (
                self.join(
                    args,
                    if self.opts.unicode {
                        " ← "
                    } else {
                        " leftarrow "
                    },
                    ARROW + 1,
                ),
                ARROW,
            ),
            "leftrightarrow" => (
                self.join(
                    args,
                    if self.opts.unicode {
                        " ↔ "
                    } else {
                        " leftrightarrow "
                    },
                    ARROW + 1,
                ),
                ARROW,
            ),
            "perp" => (
                self.join(
                    args,
                    if self.opts.unicode { " ⟂ " } else { " perp " },
                    ADD + 1,
                ),
                ADD,
            ),
            "parallel" => (
                self.join(
                    args,
                    if self.opts.unicode {
                        " ∥ "
                    } else {
                        " parallel "
                    },
                    ADD + 1,
                ),
                ADD,
            ),
            ":" => (self.join(args, " : ", COLONBAR + 1), COLONBAR),
            "|" => (self.join(args, " | ", COLONBAR + 1), COLONBAR),
            "binom" => (
                format!(
                    "binom({}, {})",
                    one(self, LIST + 1),
                    self.emit(&args[1], LIST + 1)
                ),
                ATOM,
            ),
            "vec" => (format!("vec({})", one(self, LIST + 1)), ATOM),
            "linesegment" => (
                format!("linesegment({})", self.join(args, ", ", LIST + 1)),
                ATOM,
            ),
            "angle" => (self.render_angle(args), ATOM),
            "unit" => (self.render_unit(args), UNIT),
            "d" => (format!("d{}", one(self, ATOM)), POW),
            "derivative_leibniz" => (self.render_leibniz("d", args), MUL),
            "partial_derivative_leibniz" => (self.render_leibniz("∂", args), MUL),
            _ => (
                format!("{}({})", name, self.join(args, ", ", LIST + 1)),
                ATOM,
            ),
        }
    }

    fn render_angle(&self, args: &[Expr]) -> String {
        let a = if self.opts.unicode { "∠" } else { "angle" };
        // The parser's parenthesised angle only accepts a list (≥2 args) or a
        // product, so a single-argument angle must use the shorthand `∠A`
        // (which `render_mul` guards against greedily absorbing a neighbour).
        if args.len() == 1 {
            format!("{}{}", a, self.emit(&args[0], prec::POW))
        } else {
            format!("{}({})", a, self.join(args, ", ", prec::LIST + 1))
        }
    }

    fn render_unit(&self, args: &[Expr]) -> String {
        // Prefix units ($) render before the value; postfix (%, deg) after.
        if let Expr::Sym(s) = &args[0] {
            if s.name() == "$" {
                return format!("$ {}", self.emit(&args[1], prec::MUL));
            }
        }
        format!(
            "{} {}",
            self.emit(&args[0], prec::MUL),
            self.emit(&args[1], prec::ATOM)
        )
    }

    fn render_leibniz(&self, sym: &str, args: &[Expr]) -> String {
        // args: [ var1 | (var1, n) ,  tuple-of-denominator-vars ].
        // Spaces after each differential symbol let multi-character variables
        // re-lex as their own tokens (`d hello`, not the single symbol `dhello`).
        let (var1, n_deriv) = deriv_var(&args[0]);
        let num = format!(
            "{}{} {}",
            sym,
            pow_suffix(n_deriv),
            self.render_symbol(&var1)
        );

        let den = if let Expr::Seq(SeqKind::Tuple, parts) = &args[1] {
            parts
                .iter()
                .map(|part| {
                    let (v, e) = deriv_var(part);
                    format!("{} {}{}", sym, self.render_symbol(&v), pow_suffix(e))
                })
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            String::new()
        };
        format!("{}/{}", num, den)
    }
}

/// A single-argument `angle` renders as the greedy shorthand `∠A`.
fn is_shorthand_angle(e: &Expr) -> bool {
    matches!(e, Expr::OtherOp(name, args) if name.name() == "angle" && args.len() == 1)
}

/// Can this expression appear bare in a sub/superscript slot (a single tight
/// atom or a subscript chain of them)?
fn is_simple_superscript(e: &Expr) -> bool {
    match e {
        Expr::Num(n) => !is_negative(n),
        // Sign-string symbols (name like "++" or "2--") re-lex as operators,
        // so they can't appear bare in a super/subscript slot.
        Expr::Sym(s) => !s.name().contains(['+', '-']),
        // NegInf renders with a leading "-", which the slot can't hold bare.
        Expr::Const(c) => *c != MathConst::NegInf,
        Expr::Blank => true,
        Expr::Index(a, b) => is_simple_superscript(a) && is_simple_superscript(b),
        _ => false,
    }
}
