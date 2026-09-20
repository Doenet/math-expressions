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
    deriv_var, greek_unicode, number_is_negative as is_negative, pow_suffix, prec, split_sign,
};
use crate::expr::{Expr, MathConst, RelOp, SeqKind};
use crate::num::Number;

#[derive(Debug, Clone)]
pub struct TextOpts {
    /// Emit unicode operators/greek letters (`≤`, `θ`) vs ASCII words.
    pub unicode: bool,
    /// Decimal / argument-separator notation.
    pub notation: crate::notation::NumberNotation,
    /// Pad every rendered number to at least this many significant characters
    /// (`padToDigits`). `None`/`0` = no padding.
    pub pad_to_digits: Option<u32>,
    /// Pad every rendered number to at least this many decimal places
    /// (`padToDecimals`). `None`/`0` = no padding.
    pub pad_to_decimals: Option<u32>,
    /// Render blank leaves (`＿`) visibly (`showBlanks`); when false they emit
    /// as the empty string.
    pub show_blanks: bool,
    /// Put an explicit `*` between every pair of factors instead of the usual
    /// juxtaposition (`explicitMultiplicationSymbols`).
    pub explicit_multiplication_symbols: bool,
    /// Render every float positionally, however large or small
    /// (`avoidScientificNotation`). Off by default, matching legacy: a float
    /// outside `0.000001 ..< 1e21` renders as `1.23 * 10^22`.
    pub avoid_scientific_notation: bool,
}

impl Default for TextOpts {
    fn default() -> Self {
        TextOpts {
            unicode: true,
            notation: crate::notation::NumberNotation::default(),
            pad_to_digits: None,
            pad_to_decimals: None,
            show_blanks: true,
            explicit_multiplication_symbols: false,
            avoid_scientific_notation: false,
        }
    }
}

pub fn convert(expr: &Expr, opts: &TextOpts) -> String {
    let expr = super::normalize_display_negative_fractions(expr);
    Writer { opts, pad: true }.emit(&expr, 0)
}

struct Writer<'a> {
    opts: &'a TextOpts,
    /// Whether the `padToDigits`/`padToDecimals` options apply here. Cleared
    /// inside an `integer^integer` — see [`super::is_integer_power`].
    pad: bool,
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

    /// What the bracket notations (`|…|`, `…!`) wrap: the whole argument of
    /// the application. The text twin of
    /// [`latex::Writer::sole_argument`](super::latex) — see the long note
    /// there. In the JS AST an application has exactly one operand and a
    /// multi-argument application is an application *to a tuple*, so at any
    /// arity but one the tuple is what belongs inside the brackets. Guarding
    /// these arms on `args.len() == 1` instead dropped the notation entirely
    /// and printed `abs(x, y)` where legacy wrote `|(x, y)|`.
    fn sole_argument(&self, args: &[Expr], ctx: u8) -> String {
        match args {
            [only] => self.emit(only, ctx),
            _ => self.render_seq(SeqKind::Tuple, args).0,
        }
    }

    fn render(&self, e: &Expr) -> (String, u8) {
        use prec::{ADD, AND, ATOM, INDEX, MUL, NEG, NOT, OR, POW, REL, SIGN};
        match e {
            Expr::Num(n) => self.render_number(n),
            // Prints as its function-application spelling, which reparses to
            // the same leaf.
            Expr::RootOf { poly, index } => {
                self.render(&crate::polynomials::rootof::as_apply(poly, *index))
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
            // Display only. The parsers cannot produce `Expr::Bool` at all, and
            // `true` is not even a symbol to them — implicit multiplication
            // lexes it as `t*r*u*e`. So this does not round-trip through text
            // in any form; the AST round-trip is the faithful one.
            Expr::Bool(b) => (b.to_string(), ATOM),
            Expr::Blank => (
                if self.opts.show_blanks {
                    "\u{ff3f}".to_string()
                } else {
                    String::new()
                },
                ATOM,
            ),
            Expr::Ldots => ("...".to_string(), ATOM),

            Expr::Add(terms) => (self.render_add(terms), ADD),
            Expr::Mul(factors) => self.render_mul(factors),
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
                let base_ctx = if matches!(&**b, Expr::Pow(..)) {
                    POW + 1
                } else {
                    POW
                };
                let w = self.without_padding_in_integer_power(b, e);
                (format!("{}^{}", w.emit(b, base_ctx), w.superscript(e)), POW)
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
            Expr::Matrix(m) => (self.render_matrix(m.rows(), m.cols(), m.entries()), ATOM),
            Expr::OtherOp(name, args) => self.render_other(&name.name(), args),
        }
    }

    fn join(&self, xs: &[Expr], sep: &str, ctx: u8) -> String {
        xs.iter()
            .map(|x| self.emit(x, ctx))
            .collect::<Vec<_>>()
            .join(sep)
    }

    /// The argument/tuple/list separator for the active notation, with a
    /// trailing space (`", "` by default; `"; "` under comma notation).
    fn arg_sep(&self) -> String {
        format!("{} ", self.opts.notation.argument_separator)
    }

    /// Retarget the `.` decimal point of an already-rendered number to the
    /// active decimal separator. A number string's only `.` is its decimal
    /// point, so this is unambiguous; a no-op under default notation.
    fn decimal(&self, s: String) -> String {
        let d = self.opts.notation.decimal_separator;
        if d == '.' {
            s
        } else {
            s.replace('.', &d.to_string())
        }
    }

    /// Parenthesize a logical operand that renders as a compound expression (its
    /// string has a space and is not already fully parenthesized) — port of the
    /// JS ast-to-text `and`/`or`/`not` rule.
    fn paren_if_spaced(&self, e: &Expr) -> String {
        let s = self.emit(e, 0);
        if s.contains(' ') && !is_single_paren_group(&s) {
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
        let decimal = n.decimal_spelling();
        // A *fraction*-spelled rational renders as `a/b`, binding like the
        // division it re-parses to. It is checked first so `3/6` prints `1/2`
        // rather than `0.5` — `decimal_spelling` declines it for exactly that
        // reason. Padding is a decimal-display option and does not apply here.
        if decimal.is_none() {
            if let Some((num, den)) = n.rational_parts() {
                let p = if num.starts_with('-') {
                    prec::NEG
                } else {
                    prec::MUL
                };
                return (format!("{}/{}", num, den), p);
            }
        }
        // Everything else is positional-or-scientific. Integers and
        // decimal-spelled rationals supply their own exact digits; a float
        // supplies its shortest round-trip. Both then face the same ECMAScript
        // magnitude threshold, so a typed `5.252E-13` reads `5.252 * 10^(-13)`
        // just as a computed one does — unless `avoid_scientific_notation` is
        // set.
        let (pad_digits, pad_decimals) = self.pad_bounds();
        let rendered = match decimal {
            Some(dec) => super::render_exact_decimal(
                &dec,
                self.opts.avoid_scientific_notation,
                pad_digits,
                pad_decimals,
            ),
            None => self.render_float(n.to_f64()),
        };
        match rendered {
            super::FloatRender::Positional(s) => {
                let p = if s.starts_with('-') {
                    prec::NEG
                } else {
                    prec::ATOM
                };
                (self.decimal(s), p)
            }
            // `* 10^…` re-parses as the product it is spelled as, so it binds
            // like one: parenthesised as a power's base (`(1.23 * 10^(-11))^5`)
            // but not inside a sum. A negative exponent needs its own parens —
            // `10^-11` is not text-grammar.
            super::FloatRender::Scientific { mantissa, exponent } => {
                let e = if exponent < 0 {
                    format!("({exponent})")
                } else {
                    exponent.to_string()
                };
                let p = if mantissa.starts_with('-') {
                    prec::NEG
                } else {
                    prec::MUL
                };
                (format!("{} * 10^{}", self.decimal(mantissa), e), p)
            }
        }
    }

    /// The padding bounds in force, which an `integer^integer` suppresses.
    fn pad_bounds(&self) -> (Option<u32>, Option<u32>) {
        if self.pad {
            (self.opts.pad_to_digits, self.opts.pad_to_decimals)
        } else {
            (None, None)
        }
    }

    /// This writer, or a non-padding one for the operands of a power that
    /// [`super::is_integer_power`] says legacy left alone.
    fn without_padding_in_integer_power(&self, base: &Expr, exp: &Expr) -> Writer<'_> {
        Writer {
            opts: self.opts,
            pad: self.pad && !super::is_integer_power(base, exp),
        }
    }

    /// Apply the notation threshold and the `padToDigits`/`padToDecimals`
    /// render options to a float (before the decimal separator is localized).
    fn render_float(&self, v: f64) -> super::FloatRender {
        let (digits, decimals) = self.pad_bounds();
        super::render_float(v, self.opts.avoid_scientific_notation, digits, decimals)
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
            // Display only — no text spelling parses back (see `Expr::Bool`).
            MathConst::None => "None".to_string(),
        }
    }

    /// A sum: first term rendered with its own sign, later terms joined with
    /// ` + `/` - ` by inspecting the term structurally (Neg or negative Num) —
    /// never by string-matching, and never pulling a sign out of a Mul (which
    /// would not round-trip).
    fn render_add(&self, terms: &[Expr]) -> String {
        super::render_add_terms(terms, |e, ctx| self.emit(e, ctx))
    }

    /// Returns the product's precedence too: a negative leading factor makes
    /// the whole product bind like a negation (`NEG`), so it parenthesises as a
    /// fraction numerator / power base but reads `-3 b`, not `(-3) b`.
    fn render_mul(&self, factors: &[Expr]) -> (String, u8) {
        let mut out = String::new();
        let mut p = prec::MUL;
        for (i, f) in factors.iter().enumerate() {
            let s = if i == 0 {
                // The sign of a negative leading factor renders inline, without
                // parentheses (port of the JS `factor()` at term level); a sum
                // pulls it into the connective via `split_sign` before reaching
                // here, so this branch is only hit standalone / as a factor.
                match split_sign(f) {
                    (true, body) => {
                        p = prec::NEG;
                        format!("-{}", self.emit(&body, prec::MUL))
                    }
                    (false, _) => self.emit(f, prec::MUL),
                }
            } else {
                self.emit(f, prec::MUL + 1)
            };
            if i > 0 {
                // `explicitMultiplicationSymbols`: a bare `*` between every pair
                // (port of the legacy `termFactors.join("*")`).
                if self.opts.explicit_multiplication_symbols {
                    out.push('*');
                }
                // Otherwise a space disambiguates tokens; use ` * ` when the
                // right factor begins with a digit (so two numbers don't merge)
                // or the left factor is a shorthand `∠A` (which would absorb it).
                else if s.starts_with(|c: char| c.is_ascii_digit())
                    || super::is_shorthand_angle(&factors[i - 1])
                {
                    out.push_str(" * ");
                } else {
                    out.push(' ');
                }
            }
            out.push_str(&s);
        }
        (out, p)
    }

    fn render_apply(&self, head: &Expr, args: &[Expr]) -> (String, u8) {
        // An integral `∫_a^b <integrand>`: keep the `∫` glyph (a `d x`
        // differential was already split into a `["d", x]` factor at parse
        // time, so it renders as `dx`) and drop the parentheses a generic
        // application would put round the integrand. Port of the legacy `apply`
        // integral branch; DoenetML open item 10.
        if args.len() == 1 && super::is_integral_head(head) {
            return (
                format!(
                    "{} {}",
                    self.emit(head, prec::POW),
                    self.emit(&args[0], prec::MUL)
                ),
                prec::MUL,
            );
        }
        // Special notations for particular function heads.
        if let Expr::Sym(s) = head {
            match s.name().as_str() {
                "abs" => return (format!("|{}|", self.sole_argument(args, 0)), prec::ATOM),
                // factorial is postfix `!`, so it prints at POW precedence.
                "factorial" => {
                    return (
                        format!("{}!", self.sole_argument(args, prec::POW)),
                        prec::POW,
                    )
                }
                _ => {}
            }
        }
        let args_str = args
            .iter()
            .map(|a| self.emit(a, prec::LIST + 1))
            .collect::<Vec<_>>()
            .join(&self.arg_sep());
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
            .join(&self.arg_sep());
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
        format!("{}{}{}{}{}", left, lo, self.arg_sep(), hi, right)
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
            out.push_str(&format!("[{}]", row.join(&self.arg_sep())));
            if r < rows as usize - 1 {
                out.push_str(&self.arg_sep());
            }
        }
        out.push(']');
        out
    }

    /// The long tail of notation operators carried as `OtherOp`.
    fn render_other(&self, name: &str, args: &[Expr]) -> (String, u8) {
        use prec::*;
        // A head with too few operands to render in its own notation drops to
        // the generic form; see [`super::other_op_min_arity`].
        if args.len() < super::other_op_min_arity(name) {
            return self.render_other_generic(name, args);
        }
        let one = |w: &Self, ctx| w.emit(&args[0], ctx);
        match name {
            // ASCII spells it as the word the lexer keyword-matches. `+-` would
            // re-lex as two sign operators (`+- 3` parses as `+(-3)`), so it is
            // not a spelling this printer may emit.
            "pm" => (
                format!(
                    "{} {}",
                    if self.opts.unicode { "±" } else { "plusminus" },
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
                format!(
                    "linesegment({})",
                    self.join(args, &self.arg_sep(), LIST + 1)
                ),
                ATOM,
            ),
            "angle" => (self.render_angle(args), ATOM),
            "unit" => (self.render_unit(args), UNIT),
            "d" => (format!("d{}", one(self, ATOM)), POW),
            "derivative_leibniz" => (self.render_leibniz("d", args), MUL),
            "partial_derivative_leibniz" => (self.render_leibniz("∂", args), MUL),
            _ => self.render_other_generic(name, args),
        }
    }

    /// `name(args…)` — the form every unrecognized head takes, and the fallback
    /// for a recognized head whose operand count is too low for its notation.
    fn render_other_generic(&self, name: &str, args: &[Expr]) -> (String, u8) {
        (
            format!(
                "{}({})",
                name,
                self.join(args, &self.arg_sep(), prec::LIST + 1)
            ),
            prec::ATOM,
        )
    }

    fn render_angle(&self, args: &[Expr]) -> String {
        let a = if self.opts.unicode { "∠" } else { "angle" };
        // The parser's parenthesised angle only accepts a list (≥2 args) or a
        // product, so a single-argument angle must use the shorthand `∠A`
        // (which `render_mul` guards against greedily absorbing a neighbour).
        if args.len() == 1 {
            format!("{}{}", a, self.emit(&args[0], prec::POW))
        } else {
            format!(
                "{}({})",
                a,
                self.join(args, &self.arg_sep(), prec::LIST + 1)
            )
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
        let (var1, n_deriv) = deriv_var(&args[0]);
        let den_parts: Vec<(String, i64)> = match &args[1] {
            Expr::Seq(SeqKind::Tuple, parts) => parts
                .iter()
                .map(|part| {
                    let (v, e) = deriv_var(part);
                    (self.render_symbol(&v), e)
                })
                .collect(),
            _ => Vec::new(),
        };
        let var1 = self.render_symbol(&var1);

        // `dx/dt` is how the notation is written, and it re-parses: the lexer
        // reads `dx` as the differential `d` plus the one-character variable.
        // A *multi*-character variable would be swallowed whole (`dhello` is one
        // symbol), so those — and only those — need a separating space.
        let sep = if std::iter::once(&var1)
            .chain(den_parts.iter().map(|(v, _)| v))
            .all(|v| v.chars().count() == 1)
        {
            ""
        } else {
            " "
        };

        let num = format!("{}{}{}{}", sym, pow_suffix(n_deriv), sep, var1);
        let den = den_parts
            .iter()
            .map(|(v, e)| format!("{}{}{}{}", sym, sep, v, pow_suffix(*e)))
            .collect::<Vec<_>>()
            .join(sep);
        format!("{}/{}", num, den)
    }
}

/// Is `s` a *single* parenthesized group — an opening `(` whose matching `)`
/// is the final character — rather than several adjacent groups such as
/// `(a) or (b)`?
///
/// The old test was `starts_with('(') && ends_with(')')`, which reads `true`
/// for both and so wrongly suppressed the wrap around a spaced compound like
/// `(a) or (b)`. Once un-wrapped it re-parses with the wrong binding — e.g. a
/// nested `or` under `not` escapes its scope. A balance scan tells one group
/// from many: the first return to depth 0 must be the last char.
fn is_single_paren_group(s: &str) -> bool {
    if !s.starts_with('(') {
        return false;
    }
    let mut depth: i32 = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return i + c.len_utf8() == s.len();
                }
            }
            _ => {}
        }
    }
    false
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
