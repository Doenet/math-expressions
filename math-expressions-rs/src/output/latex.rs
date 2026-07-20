//! LaTeX output — a precedence-based pretty-printer walking `Expr` directly
//! (sibling of `text.rs`). LaTeX's braces make `\frac{}{}`, `x^{}`, `x_{}`
//! self-delimiting, so their contents never need parentheses; parenthesisation
//! is otherwise the same precedence comparison as the text formatter.
//! Correctness is enforced by round-tripping through the LaTeX parser.

use super::{deriv_var, f64_positional_string, pow_suffix, prec, split_sign};
use crate::expr::{Expr, MathConst, RelOp, SeqKind};
use crate::num::Number;

#[derive(Debug, Clone, Default)]
pub struct LatexOpts {}

pub fn convert(expr: &Expr, _opts: &LatexOpts) -> String {
    Writer.emit(expr, 0)
}

struct Writer;

impl Writer {
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
            // MATRIX_PLAN §2a display form.
            Expr::RootOf { poly, index } => (
                format!(
                    "\\operatorname{{Root}}_{{{}}}\\!\\left({}\\right)",
                    index,
                    self.emit(&crate::rootof::poly_display(poly, "t"), 0)
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
            Expr::Pow(b, e) => (format!("{}^{}", self.emit(b, POW), self.braced(e)), POW),

            Expr::And(xs) => (self.join(xs, " \\land ", AND + 1), AND),
            Expr::Or(xs) => (self.join(xs, " \\lor ", OR + 1), OR),
            Expr::Not(x) => (format!("\\lnot {}", self.emit(x, NOT + 1)), NOT),
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
            return (dec, p);
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
        (s, p)
    }

    fn join(&self, xs: &[Expr], sep: &str, ctx: u8) -> String {
        xs.iter()
            .map(|x| self.emit(x, ctx))
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
            .join(", ");
        // Function heads with dedicated LaTeX spellings.
        let head_str = match head {
            Expr::Sym(s) if s.name() == "log10" => "\\log_{10}".to_string(),
            Expr::Sym(s) if s.name() == "re" => "\\Re".to_string(),
            Expr::Sym(s) if s.name() == "im" => "\\Im".to_string(),
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
            .join(", ");
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
        format!("{} {}, {} {}", left, lo, hi, right)
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
            "unit" => (self.render_unit(args), MUL),
            "d" => (format!("d{}", one(self, ATOM)), POW),
            "derivative_leibniz" => (self.render_leibniz("d", args), ATOM),
            "partial_derivative_leibniz" => (self.render_leibniz("\\partial ", args), ATOM),
            _ => (
                format!(
                    "\\operatorname{{{}}}\\left({}\\right)",
                    name,
                    self.join(args, ", ", LIST + 1)
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
                self.join(args, ", ", prec::LIST + 1)
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
                return format!("\\${}", self.emit(&args[1], prec::MUL));
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
        format!("\\frac{{{}}}{{{}}}", num, den)
    }
}

fn is_shorthand_angle(e: &Expr) -> bool {
    matches!(e, Expr::OtherOp(name, args) if name.name() == "angle" && args.len() == 1)
}

/// Symbol name → LaTeX. Multi-char names in the allowed set become control
/// words (`\theta`); functions likewise; anything else is `\operatorname{}`.
fn string_convert(name: &str) -> String {
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
    "abs",
    "exp",
    "log",
    "ln",
    "log10",
    "sign",
    "sqrt",
    "erf",
    "cos",
    "cosh",
    "cot",
    "coth",
    "csc",
    "csch",
    "sec",
    "sech",
    "sin",
    "sinh",
    "tan",
    "tanh",
    "arcsin",
    "arccos",
    "arctan",
    "arccsc",
    "arcsec",
    "arccot",
    "arg",
    "Re",
    "Im",
    "det",
    "angle",
    "perp",
    "circ",
    "int",
    "varnothing",
];

fn is_allowed_latex_symbol(s: &str) -> bool {
    ALLOWED_LATEX_SYMBOLS.contains(&s)
}

fn convert_latex_symbol(s: &str) -> Option<&str> {
    Some(match s {
        "acos" => "arccos",
        "acot" => "arccot",
        "acsc" => "arccsc",
        "asec" => "arcsec",
        "asin" => "arcsin",
        "atan" => "arctan",
        "deg" => "circ",
        "emptyset" => "varnothing",
        _ => return None,
    })
}
