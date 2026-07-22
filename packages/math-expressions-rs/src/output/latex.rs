//! LaTeX output — a precedence-based pretty-printer walking `Expr` directly
//! (sibling of `text.rs`). LaTeX's braces make `\frac{}{}`, `x^{}`, `x_{}`
//! self-delimiting, so their contents never need parentheses; parenthesisation
//! is otherwise the same precedence comparison as the text formatter.
//! Correctness is enforced by round-tripping through the LaTeX parser.

use super::{deriv_var, f64_positional_string, pow_suffix, prec, split_sign};
use crate::expr::{Expr, MathConst, RelOp, SeqKind};
use crate::num::Number;

#[derive(Debug, Clone, Default)]
pub struct LatexOpts {
    /// Decimal / argument-separator notation (I18N_MATH_NOTATION_PLAN).
    pub notation: crate::notation::NumberNotation,
}

pub fn convert(expr: &Expr, opts: &LatexOpts) -> String {
    Writer { opts }.emit(expr, 0)
}

struct Writer<'a> {
    opts: &'a LatexOpts,
}

impl Writer<'_> {
    fn emit(&self, e: &Expr, ctx: u8) -> String {
        let (s, p) = self.render(e);
        if p < ctx {
            format!("\\left({}\\right)", s)
        } else {
            s
        }
    }

    /// Render inside braces (superscript/subscript/frac argument): fully
    /// delimited, so no parentheses and any expression is allowed.
    fn braced(&self, e: &Expr) -> String {
        format!("{{{}}}", self.emit(e, 0))
    }

    fn render(&self, e: &Expr) -> (String, u8) {
        use prec::{ADD, AND, ATOM, INDEX, MUL, NEG, NOT, OR, POW, REL, SIGN};
        match e {
            Expr::Num(n) => self.render_number(n),
            // Same `rootof(p, k)` application form as the text printer —
            // `\operatorname{rootof}` is a registered applied name, so this
            // re-parses and canonicalizes back to the identical leaf (the
            // printers' round-trip contract; the old `\operatorname{Root}_k`
            // display form did not re-parse).
            Expr::RootOf { poly, index } => (
                format!(
                    "\\operatorname{{rootof}}\\left({}{}{}\\right)",
                    self.emit(&crate::rootof::poly_display(poly, "t"), 0),
                    self.arg_sep(),
                    index
                ),
                ATOM,
            ),
            Expr::Sym(s) => {
                let name = s.name();
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
            Expr::Ldots => ("\\ldots".to_string(), ATOM),

            Expr::Add(terms) => (self.render_add(terms), ADD),
            Expr::Mul(factors) => (self.render_mul(factors), MUL),
            // \frac is self-delimiting: an atom whose arguments need no parens.
            Expr::Div(a, b) => (format!("\\frac{}{}", self.braced(a), self.braced(b)), ATOM),
            Expr::Neg(x) => (format!("-{}", self.emit(x, MUL)), NEG),
            // `(x^y)^z` must parenthesize the inner power — bare `x^{y}^{z}` is
            // invalid LaTeX (double superscript) — and a radical raised to a
            // power reads clearer parenthesized (`\left(\sqrt{2}\right)^{3}`).
            // Other same-precedence bases (`f'` in `f'^a(x)`) stay unwrapped so
            // they round-trip.
            Expr::Pow(b, e) => {
                let base = if matches!(&**b, Expr::Pow(..)) || is_radical(b) {
                    format!("\\left({}\\right)", self.emit(b, 0))
                } else {
                    self.emit(b, POW)
                };
                (format!("{}^{}", base, self.braced(e)), POW)
            }

            Expr::And(xs) => (self.join_logical(xs, " \\land "), AND),
            Expr::Or(xs) => (self.join_logical(xs, " \\lor "), OR),
            Expr::Not(x) => (format!("\\lnot {}", self.paren_if_spaced(x)), NOT),
            Expr::Union(xs) => (self.join(xs, " \\cup ", ADD + 1), ADD),
            Expr::Intersect(xs) => (self.join(xs, " \\cap ", ADD + 1), ADD),

            Expr::Prime(x) => (format!("{}'", self.emit(x, POW)), POW),
            Expr::Index(a, b) => (
                format!("{}_{}", self.emit(a, INDEX + 1), self.braced(b)),
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

    fn render_number(&self, n: &Number) -> (String, u8) {
        use prec::{ATOM, NEG};
        // Terminating decimals (all integers, and every parse-produced
        // rational) render positionally, so `0.5` round-trips as `Rat(1,2)`
        // — rendering `\frac{1}{2}` would re-parse to a `Div`.
        if let Some(dec) = n.terminating_decimal() {
            let p = if dec.starts_with('-') { NEG } else { ATOM };
            return (self.decimal(dec), p);
        }
        // A non-terminating fraction renders as `\frac` (self-delimiting, so
        // an atom); only reachable from later normalization, not the parser.
        if let Some((num, den)) = n.rational_parts() {
            return match num.strip_prefix('-') {
                Some(pos) => (format!("-\\frac{{{}}}{{{}}}", pos, den), NEG),
                None => (format!("\\frac{{{}}}{{{}}}", num, den), ATOM),
            };
        }
        // Float: numerical-evaluation result, positional (never exponential).
        let s = f64_positional_string(n.to_f64());
        let p = if s.starts_with('-') { NEG } else { ATOM };
        (self.decimal(s), p)
    }

    /// The argument/tuple/list separator for the active notation, with a
    /// trailing space (`", "` by default; `"; "` under comma notation).
    fn arg_sep(&self) -> String {
        format!("{} ", self.opts.notation.argument_separator)
    }

    /// Retarget the `.` decimal point of a rendered number to the active
    /// decimal separator. A5: a decimal comma is emitted as `{,}` so MathJax /
    /// MathQuill don't add trailing-punctuation spacing. No-op under default.
    fn decimal(&self, s: String) -> String {
        let d = self.opts.notation.decimal_separator;
        if d == '.' {
            return s;
        }
        let rep = if d == ',' {
            "{,}".to_string()
        } else {
            d.to_string()
        };
        s.replace('.', &rep)
    }

    fn join(&self, xs: &[Expr], sep: &str, ctx: u8) -> String {
        xs.iter()
            .map(|x| self.emit(x, ctx))
            .collect::<Vec<_>>()
            .join(sep)
    }

    /// Wrap a logical operand in parentheses for clarity when it renders as a
    /// compound expression — i.e. its string contains a space and is not already
    /// fully parenthesized (port of the JS ast-to-latex `and`/`or`/`not` rule).
    fn paren_if_spaced(&self, e: &Expr) -> String {
        let s = self.emit(e, 0);
        if s.contains(' ') && !(s.starts_with("\\left(") && s.ends_with("\\right)")) {
            format!("\\left({}\\right)", s)
        } else {
            s
        }
    }

    /// Join logical operands (`and`/`or`) with `sep`, parenthesizing compound
    /// operands via [`paren_if_spaced`].
    fn join_logical(&self, xs: &[Expr], sep: &str) -> String {
        xs.iter()
            .map(|x| self.paren_if_spaced(x))
            .collect::<Vec<_>>()
            .join(sep)
    }

    fn render_symbol(&self, name: &str) -> String {
        string_convert(name)
    }

    fn render_const(&self, c: MathConst) -> String {
        match c {
            MathConst::Pi => "\\pi".to_string(),
            MathConst::E => "e".to_string(),
            MathConst::I => "i".to_string(),
            MathConst::Inf => "\\infty".to_string(),
            MathConst::NegInf => "-\\infty".to_string(),
            MathConst::NaN => "NaN".to_string(),
        }
    }

    fn render_add(&self, terms: &[Expr]) -> String {
        if terms.len() == 1 {
            return format!("+{}", self.emit(&terms[0], prec::ADD + 1));
        }
        let mut out = String::new();
        for (i, t) in terms.iter().enumerate() {
            // A `\pm` term carries its own operator, so it is joined with a plain
            // space rather than ` + ` — `5 + \pm 3` would be wrong.
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
                // `\cdot` between adjacent numerals or after a shorthand `\angle A`
                // (which would otherwise absorb the next factor); space otherwise.
                if s.starts_with(|c: char| c.is_ascii_digit())
                    || is_shorthand_angle(&factors[i - 1])
                {
                    out.push_str(" \\cdot ");
                } else {
                    out.push(' ');
                }
            }
            out.push_str(&s);
        }
        out
    }

    fn render_apply(&self, head: &Expr, args: &[Expr]) -> (String, u8) {
        if let Expr::Sym(s) = head {
            match s.name().as_str() {
                "abs" if args.len() == 1 => {
                    return (
                        format!("\\left|{}\\right|", self.emit(&args[0], 0)),
                        prec::ATOM,
                    )
                }
                "floor" if args.len() == 1 => {
                    return (
                        format!("\\left\\lfloor {} \\right\\rfloor", self.emit(&args[0], 0)),
                        prec::ATOM,
                    )
                }
                "ceil" if args.len() == 1 => {
                    return (
                        format!("\\left\\lceil {} \\right\\rceil", self.emit(&args[0], 0)),
                        prec::ATOM,
                    )
                }
                "sqrt" if args.len() == 1 => {
                    return (format!("\\sqrt{}", self.braced(&args[0])), prec::ATOM)
                }
                "cbrt" if args.len() == 1 => {
                    return (format!("\\sqrt[3]{}", self.braced(&args[0])), prec::ATOM)
                }
                "nthroot" if args.len() == 2 => {
                    return (
                        format!(
                            "\\sqrt[{}]{}",
                            self.emit(&args[1], 0),
                            self.braced(&args[0])
                        ),
                        prec::ATOM,
                    )
                }
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
            .join(&self.arg_sep());
        // Function heads with dedicated LaTeX spellings (`FnDef::latex_head`).
        let head_str = match head {
            Expr::Sym(s) => match crate::functions::latex_apply_head(&s.name()) {
                Some(h) => h.to_string(),
                None => self.emit(head, prec::POW),
            },
            _ => self.emit(head, prec::POW),
        };
        (
            format!("{}\\left({}\\right)", head_str, args_str),
            prec::ATOM,
        )
    }

    fn render_seq(&self, kind: SeqKind, xs: &[Expr]) -> (String, u8) {
        let inner = xs
            .iter()
            .map(|x| self.emit(x, prec::LIST + 1))
            .collect::<Vec<_>>()
            .join(&self.arg_sep());
        match kind {
            SeqKind::List => (inner, prec::LIST),
            SeqKind::Tuple | SeqKind::Vector => (format!("\\left( {} \\right)", inner), prec::ATOM),
            SeqKind::Array => (format!("\\left[ {} \\right]", inner), prec::ATOM),
            SeqKind::Set => (format!("\\left\\{{ {} \\right\\}}", inner), prec::ATOM),
            SeqKind::AltVector => (
                format!("\\left\\langle {} \\right\\rangle", inner),
                prec::ATOM,
            ),
        }
    }

    fn render_interval(&self, endpoints: &(Expr, Expr), closed: (bool, bool)) -> String {
        let lo = self.emit(&endpoints.0, prec::LIST + 1);
        let hi = self.emit(&endpoints.1, prec::LIST + 1);
        let left = if closed.0 { "\\left[" } else { "\\left(" };
        let right = if closed.1 { "\\right]" } else { "\\right)" };
        format!("{} {}{}{} {}", left, lo, self.arg_sep(), hi, right)
    }

    fn render_relation(&self, operands: &[Expr], ops: &[RelOp]) -> String {
        let mut out = self.emit(&operands[0], prec::REL + 1);
        for (i, op) in ops.iter().enumerate() {
            out.push_str(&format!(" {} ", rel_symbol(*op)));
            out.push_str(&self.emit(&operands[i + 1], prec::REL + 1));
        }
        out
    }

    fn render_matrix(&self, rows: u32, cols: u32, entries: &[Expr]) -> String {
        let mut out = String::from("\\begin{bmatrix} ");
        for r in 0..rows as usize {
            let row: Vec<String> = (0..cols as usize)
                .map(|c| self.emit(&entries[r * cols as usize + c], prec::LIST + 1))
                .collect();
            out.push_str(&row.join(" & "));
            if r < rows as usize - 1 {
                out.push_str(" \\\\ ");
            }
        }
        out.push_str(" \\end{bmatrix}");
        out
    }

    fn render_other(&self, name: &str, args: &[Expr]) -> (String, u8) {
        use prec::*;
        let one = |w: &Self, ctx| w.emit(&args[0], ctx);
        match name {
            "pm" => (format!("\\pm {}", one(self, MUL)), NEG),
            "forall" => (format!("\\forall {}", one(self, REL)), REL),
            "exists" => (format!("\\exists {}", one(self, REL)), REL),
            "implies" => (self.join(args, " \\implies ", ARROW + 1), ARROW),
            "impliedby" => (self.join(args, " \\impliedby ", ARROW + 1), ARROW),
            "iff" => (self.join(args, " \\iff ", ARROW + 1), ARROW),
            "rightarrow" => (self.join(args, " \\rightarrow ", ARROW + 1), ARROW),
            "leftarrow" => (self.join(args, " \\leftarrow ", ARROW + 1), ARROW),
            "leftrightarrow" => (self.join(args, " \\leftrightarrow ", ARROW + 1), ARROW),
            "perp" => (self.join(args, " \\perp ", ADD + 1), ADD),
            "parallel" => (self.join(args, " \\parallel ", ADD + 1), ADD),
            ":" => (self.join(args, " : ", COLONBAR + 1), COLONBAR),
            "|" => (self.join(args, " \\mid ", COLONBAR + 1), COLONBAR),
            "binom" => (
                format!("\\binom{}{}", self.braced(&args[0]), self.braced(&args[1])),
                ATOM,
            ),
            "vec" => (format!("\\vec{}", self.braced(&args[0])), ATOM),
            "linesegment" => (
                format!(
                    "\\overline{{{}}}",
                    args.iter()
                        .map(|a| self.emit(a, 0))
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
                ATOM,
            ),
            "angle" => (self.render_angle(args), ATOM),
            "unit" => (self.render_unit(args), UNIT),
            "d" => (format!("d{}", one(self, ATOM)), POW),
            "derivative_leibniz" => (self.render_leibniz("d", args), ATOM),
            "partial_derivative_leibniz" => (self.render_leibniz("\\partial ", args), ATOM),
            _ => (
                format!(
                    "\\operatorname{{{}}}\\left({}\\right)",
                    name,
                    self.join(args, &self.arg_sep(), LIST + 1)
                ),
                ATOM,
            ),
        }
    }

    fn render_angle(&self, args: &[Expr]) -> String {
        if args.len() == 1 {
            format!("\\angle {}", self.emit(&args[0], prec::POW))
        } else {
            format!(
                "\\angle\\left( {} \\right)",
                self.join(args, &self.arg_sep(), prec::LIST + 1)
            )
        }
    }

    fn render_unit(&self, args: &[Expr]) -> String {
        if let Expr::Sym(s) = &args[1] {
            if s.name() == "deg" {
                return format!("{}^{{\\circ}}", self.emit(&args[0], prec::POW));
            }
        }
        if let Expr::Sym(s) = &args[0] {
            if s.name() == "$" {
                return format!("\\$ {}", self.emit(&args[1], prec::MUL));
            }
        }
        format!(
            "{} {}",
            self.emit(&args[0], prec::MUL),
            self.emit(&args[1], prec::ATOM)
        )
    }

    fn render_leibniz(&self, sym: &str, args: &[Expr]) -> String {
        let (var1, n_deriv) = deriv_var(&args[0]);
        // `sym` carries its own trailing space where needed (`\partial `), so no
        // extra separator: `d` → `dx`, `\partial ` → `\partial x` (not the
        // double-spaced `\partial  x`).
        let num = format!(
            "{}{}{}",
            sym,
            pow_suffix(n_deriv),
            self.render_symbol(&var1)
        );
        let den = if let Expr::Seq(SeqKind::Tuple, parts) = &args[1] {
            parts
                .iter()
                .map(|part| {
                    let (v, e) = deriv_var(part);
                    format!("{}{}{}", sym, self.render_symbol(&v), pow_suffix(e))
                })
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            String::new()
        };
        format!("\\frac{{{}}}{{{}}}", num, den)
    }
}

fn is_shorthand_angle(e: &Expr) -> bool {
    matches!(e, Expr::OtherOp(name, args) if name.name() == "angle" && args.len() == 1)
}

/// A radical (`\sqrt`, `\sqrt[3]`, `\sqrt[n]`): self-delimiting, but reads
/// clearer parenthesized when raised to a power (`\left(\sqrt{2}\right)^{3}`).
fn is_radical(e: &Expr) -> bool {
    matches!(e, Expr::Apply(head, args) if matches!(&**head, Expr::Sym(s)
        if (matches!(s.name().as_str(), "sqrt" | "cbrt") && args.len() == 1)
            || (s.name() == "nthroot" && args.len() == 2)))
}

/// Symbol name → LaTeX. Multi-char names in the allowed set become control
/// words (`\theta`); functions likewise; anything else is `\operatorname{}`.
fn string_convert(name: &str) -> String {
    // LaTeX-special characters must be escaped, or they change the meaning of
    // the source: a bare `%` starts a comment (swallowing the rest of the line).
    match name {
        "%" => return "\\%".to_string(),
        "$" => return "\\$".to_string(),
        "&" => return "\\&".to_string(),
        "#" => return "\\#".to_string(),
        _ => {}
    }
    // Function spellings carry their control word on the registry
    // (`asin` → `\arcsin`, `ln` → `\ln`); unlisted spellings fall through
    // to the `\operatorname{…}` path below.
    if let Some(cmd) = crate::functions::latex_command(name) {
        return format!("\\{}", cmd);
    }
    let name = convert_latex_symbol(name).unwrap_or(name);
    if name.chars().count() > 1 {
        if is_allowed_latex_symbol(name) {
            format!("\\{}", name)
        } else {
            format!("\\operatorname{{{}}}", name)
        }
    } else if is_allowed_latex_symbol(name) {
        format!("\\{}", name)
    } else {
        name.to_string()
    }
}

fn rel_symbol(op: RelOp) -> &'static str {
    match op {
        RelOp::Eq => "=",
        RelOp::Ne => "\\ne",
        RelOp::Lt => "<",
        RelOp::Le => "\\le",
        RelOp::Gt => ">",
        RelOp::Ge => "\\ge",
        RelOp::In => "\\in",
        RelOp::NotIn => "\\notin",
        RelOp::Ni => "\\ni",
        RelOp::NotNi => "\\not\\ni",
        RelOp::Subset => "\\subset",
        RelOp::NotSubset => "\\not\\subset",
        RelOp::SubsetEq => "\\subseteq",
        RelOp::NotSubsetEq => "\\not\\subseteq",
        RelOp::Superset => "\\supset",
        RelOp::NotSuperset => "\\not\\supset",
        RelOp::SupersetEq => "\\supseteq",
        RelOp::NotSupersetEq => "\\not\\supseteq",
    }
}

/// Non-function symbols with LaTeX control words: greek letters and
/// notation. Function names live on `FnDef::latex_commands` in
/// `crate::functions` (Phase 2 of the improvement plan moves this list to a
/// shared notation table too).
const ALLOWED_LATEX_SYMBOLS: &[&str] = &[
    "alpha",
    "beta",
    "gamma",
    "Gamma",
    "delta",
    "Delta",
    "epsilon",
    "zeta",
    "eta",
    "theta",
    "Theta",
    "iota",
    "kappa",
    "lambda",
    "Lambda",
    "mu",
    "nu",
    "xi",
    "Xi",
    "pi",
    "Pi",
    "rho",
    "sigma",
    "Sigma",
    "tau",
    "Tau",
    "upsilon",
    "Upsilon",
    "phi",
    "Phi",
    "chi",
    "psi",
    "Psi",
    "omega",
    "Omega",
    "partial",
    "angle",
    "perp",
    "circ",
    "int",
    "varnothing",
];

fn is_allowed_latex_symbol(s: &str) -> bool {
    ALLOWED_LATEX_SYMBOLS.contains(&s)
}

/// Notation-symbol respellings (the function-name conversions `acos` →
/// `arccos` are `FnDef::latex_commands` now).
fn convert_latex_symbol(s: &str) -> Option<&str> {
    Some(match s {
        "deg" => "circ",
        "emptyset" => "varnothing",
        _ => return None,
    })
}
