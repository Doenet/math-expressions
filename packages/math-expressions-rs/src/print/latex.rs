//! LaTeX output — a precedence-based pretty-printer walking `Expr` directly
//! (sibling of `text.rs`). LaTeX's braces make `\frac{}{}`, `x^{}`, `x_{}`
//! self-delimiting, so their contents never need parentheses; parenthesisation
//! is otherwise the same precedence comparison as the text formatter.
//! Correctness is enforced by round-tripping through the LaTeX parser.

use super::{deriv_var, pow_suffix, prec, split_sign};
use crate::expr::{Expr, MathConst, RelOp, SeqKind};
use crate::num::Number;

#[derive(Debug, Clone)]
pub struct LatexOpts {
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
    /// Put an explicit `\cdot` between every pair of factors
    /// (`explicitMultiplicationSymbols`). Legacy LaTeX had no such option;
    /// honored here so the flag is not silently dropped.
    pub explicit_multiplication_symbols: bool,
    /// Render every float positionally, however large or small
    /// (`avoidScientificNotation`). Off by default, matching legacy: a float
    /// outside `0.000001 ..< 1e21` renders as `1.23 \cdot 10^{22}`.
    pub avoid_scientific_notation: bool,
    /// The `amsmath` environment a matrix renders into (`matrixEnvironment`) —
    /// `bmatrix` (square brackets) by default, `pmatrix` for round ones.
    pub matrix_environment: String,
}

impl Default for LatexOpts {
    fn default() -> Self {
        LatexOpts {
            notation: crate::notation::NumberNotation::default(),
            pad_to_digits: None,
            pad_to_decimals: None,
            show_blanks: true,
            explicit_multiplication_symbols: false,
            avoid_scientific_notation: false,
            matrix_environment: "bmatrix".to_string(),
        }
    }
}

pub fn convert(expr: &Expr, opts: &LatexOpts) -> String {
    let expr = super::normalize_display_negative_fractions(expr);
    Writer { opts, pad: true }.emit(&expr, 0)
}

struct Writer<'a> {
    opts: &'a LatexOpts,
    /// Whether the `padToDigits`/`padToDecimals` options apply here. Cleared
    /// inside an `integer^integer` — see [`super::is_integer_power`].
    pad: bool,
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

    /// What the bracket notations (`|…|`, `⌊…⌋`, `√…`, `…!`) wrap: the whole
    /// argument of the application.
    ///
    /// In the JS AST an application has exactly one operand, and a
    /// multi-argument application is an application *to a tuple*
    /// (`["apply", "abs", ["tuple", "x", "y"]]`), so the tuple is what belongs
    /// inside the brackets — which is what the legacy library rendered. These
    /// arms used to be guarded on `args.len() == 1` and fall through to the
    /// generic `head\left(…\right)` form otherwise, which spelled the head as
    /// a LaTeX command that does not exist: `abs(x, y)` came out as
    /// `\abs\left( x, y \right)` and `sqrt(x, y)` as `\sqrt\left( x, y
    /// \right)`, neither of which MathJax can render.
    fn sole_argument(&self, args: &[Expr], ctx: u8) -> String {
        match args {
            [only] => self.emit(only, ctx),
            _ => self.render_seq(SeqKind::Tuple, args).0,
        }
    }

    /// [`sole_argument`](Self::sole_argument) inside braces — `\sqrt{…}`.
    fn braced_argument(&self, args: &[Expr]) -> String {
        format!("{{{}}}", self.sole_argument(args, 0))
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
                    self.emit(&crate::polynomials::rootof::poly_display(poly, "t"), 0),
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
            // Display only — see the text printer; no LaTeX spelling parses
            // back to a boolean.
            Expr::Bool(b) => (format!("\\operatorname{{{b}}}"), ATOM),
            Expr::Blank => (
                if self.opts.show_blanks {
                    "\u{ff3f}".to_string()
                } else {
                    String::new()
                },
                ATOM,
            ),
            Expr::Ldots => ("\\ldots".to_string(), ATOM),

            Expr::Add(terms) => (self.render_add(terms), ADD),
            Expr::Mul(factors) => self.render_mul(factors),
            // \frac is self-delimiting: an atom whose arguments need no parens.
            Expr::Div(a, b) => (format!("\\frac{}{}", self.braced(a), self.braced(b)), ATOM),
            Expr::Neg(x) => (format!("-{}", self.emit(x, MUL)), NEG),
            // `(x^y)^z` must parenthesize the inner power — bare `x^{y}^{z}` is
            // invalid LaTeX (double superscript) — and a radical raised to a
            // power reads clearer parenthesized (`\left(\sqrt{2}\right)^{3}`).
            // Other same-precedence bases (`f'` in `f'^a(x)`) stay unwrapped so
            // they round-trip.
            Expr::Pow(b, e) => {
                let w = self.without_padding_in_integer_power(b, e);
                let base = if matches!(&**b, Expr::Pow(..)) || is_radical(b) {
                    format!("\\left({}\\right)", w.emit(b, 0))
                } else {
                    w.emit(b, POW)
                };
                (format!("{}^{}", base, w.braced(e)), POW)
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
            Expr::Matrix(m) => (self.render_matrix(m.rows(), m.cols(), m.entries()), ATOM),
            Expr::OtherOp(name, args) => self.render_other(&name.name(), args),
        }
    }

    fn render_number(&self, n: &Number) -> (String, u8) {
        use prec::{ATOM, NEG};
        // A non-finite *float* has no positional spelling. The shared helper
        // answers the text word (`Infinity`), which in LaTeX re-parses as a
        // product of eight letters; spell it the way `Const(Inf)` is spelled
        // instead. The test is on the variant rather than on `to_f64`, which
        // overflows to infinity for a perfectly finite big integer.
        if let Number::Float(f) = n {
            let v = f.get();
            if v.is_infinite() {
                return if v > 0.0 {
                    ("\\infty".to_string(), ATOM)
                } else {
                    ("-\\infty".to_string(), NEG)
                };
            }
        }
        let decimal = n.decimal_spelling();
        // A fraction renders as `\frac` (self-delimiting, so an atom). It is
        // checked first so `3/6` prints `\frac{1}{2}` rather than `0.5` —
        // `decimal_spelling` declines it for exactly that reason. Padding is a
        // decimal-display option and does not apply here.
        if decimal.is_none() {
            if let Some((num, den)) = n.rational_parts() {
                return match num.strip_prefix('-') {
                    Some(pos) => (format!("-\\frac{{{}}}{{{}}}", pos, den), NEG),
                    None => (format!("\\frac{{{}}}{{{}}}", num, den), ATOM),
                };
            }
        }
        // Everything else is positional-or-scientific. Integers and
        // decimal-spelled rationals supply their own exact digits (so a typed
        // `0.5` round-trips as `0.5`); a float supplies its shortest
        // round-trip. Both then face the same ECMAScript magnitude threshold,
        // rendering as `mantissa \cdot 10^{exponent}` past it unless
        // `avoid_scientific_notation` is set.
        let (digits, decimals) = self.pad_bounds();
        let rendered = match decimal {
            Some(dec) => super::render_exact_decimal(
                &dec,
                self.opts.avoid_scientific_notation,
                digits,
                decimals,
            ),
            None => super::render_float(
                n.to_f64(),
                self.opts.avoid_scientific_notation,
                digits,
                decimals,
            ),
        };
        match rendered {
            super::FloatRender::Positional(s) => {
                let p = if s.starts_with('-') { NEG } else { ATOM };
                (self.decimal(s), p)
            }
            // The braces delimit the exponent, so a negative one needs no
            // parens; the product itself still binds like a product, and so
            // parenthesises as a power's base.
            super::FloatRender::Scientific { mantissa, exponent } => {
                let p = if mantissa.starts_with('-') {
                    NEG
                } else {
                    prec::MUL
                };
                (
                    format!("{} \\cdot 10^{{{}}}", self.decimal(mantissa), exponent),
                    p,
                )
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

    /// The argument/tuple/list separator for the active notation, with a
    /// trailing space (`", "` by default; `"; "` under comma notation).
    fn arg_sep(&self) -> String {
        format!("{} ", self.opts.notation.argument_separator)
    }

    /// Retarget the `.` decimal point of a rendered number to the active
    /// decimal separator. A decimal comma is emitted as `{,}` so MathJax /
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
        if s.contains(' ') && !is_single_delimited_group(&s) {
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
            // Display only — no LaTeX spelling parses back (see `Expr::Bool`).
            MathConst::None => "\\operatorname{None}".to_string(),
        }
    }

    fn render_add(&self, terms: &[Expr]) -> String {
        super::render_add_terms(terms, |e, ctx| self.emit(e, ctx))
    }

    /// Returns the product's precedence too: a negative leading factor makes
    /// the whole product bind like a negation (`NEG`), so it parenthesises as a
    /// power base but reads `-3 b`, not `\left(-3\right) b`.
    fn render_mul(&self, factors: &[Expr]) -> (String, u8) {
        let mut out = String::new();
        let mut p = prec::MUL;
        for (i, f) in factors.iter().enumerate() {
            let s = if i == 0 {
                // The sign of a negative leading factor renders inline, without
                // parentheses (port of the JS `factor()` at term level); a sum
                // pulls it into the connective via `split_sign` before here.
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
                // `\cdot` when forced (`explicitMultiplicationSymbols`), between
                // adjacent numerals, or after a shorthand `\angle A` (which would
                // otherwise absorb the next factor); a space otherwise.
                if self.opts.explicit_multiplication_symbols
                    || s.starts_with(|c: char| c.is_ascii_digit())
                    || super::is_shorthand_angle(&factors[i - 1])
                {
                    out.push_str(" \\cdot ");
                } else {
                    out.push(' ');
                }
            }
            out.push_str(&s);
        }
        (out, p)
    }

    fn render_apply(&self, head: &Expr, args: &[Expr]) -> (String, u8) {
        // An integral `\int_a^b <integrand>`: keep the `\int` and drop the
        // parentheses a generic application would put round the integrand (the
        // `d x` differential is already a `["d", x]` factor). Port of the legacy
        // `apply` integral branch; DoenetML open item 10.
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
        if let Expr::Sym(s) = head {
            match s.name().as_str() {
                "abs" => {
                    return (
                        format!("\\left|{}\\right|", self.sole_argument(args, 0)),
                        prec::ATOM,
                    )
                }
                "floor" => {
                    return (
                        format!(
                            "\\left\\lfloor {} \\right\\rfloor",
                            self.sole_argument(args, 0)
                        ),
                        prec::ATOM,
                    )
                }
                "ceil" => {
                    return (
                        format!(
                            "\\left\\lceil {} \\right\\rceil",
                            self.sole_argument(args, 0)
                        ),
                        prec::ATOM,
                    )
                }
                "sqrt" => return (format!("\\sqrt{}", self.braced_argument(args)), prec::ATOM),
                "cbrt" => {
                    return (
                        format!("\\sqrt[3]{}", self.braced_argument(args)),
                        prec::ATOM,
                    )
                }
                // The one genuinely two-argument notation here: the second
                // argument is the index, not part of what the radical wraps.
                // At any other arity there is no index to raise, so what is
                // left is a plain radical over the whole argument — which is
                // what legacy rendered, and what `normalize::canonicalize`
                // already assumes when it rewrites `nthroot(x)` to `sqrt(x)`.
                // Falling through instead would print `\operatorname{nthroot}`
                // for a tree the rest of the engine treats as a square root.
                "nthroot" => {
                    return (
                        match args {
                            [radicand, index] => {
                                format!("\\sqrt[{}]{}", self.emit(index, 0), self.braced(radicand))
                            }
                            _ => format!("\\sqrt{}", self.braced_argument(args)),
                        },
                        prec::ATOM,
                    )
                }
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
        // Function heads with dedicated LaTeX spellings (`FnDef::latex_head`).
        let head_str = match head {
            Expr::Sym(s) => match crate::special_functions::latex_apply_head(&s.name()) {
                Some(h) => h.to_string(),
                None => self.emit(head, prec::POW),
            },
            _ => self.emit(head, prec::POW),
        };
        // A multi-argument application *is* an application to a tuple, so its
        // parentheses are the tuple's and are padded the way `render_seq` pads
        // every other delimiter pair. A single argument is not a tuple and
        // stays tight (`\sin\left(x\right)`).
        if args.len() > 1 {
            return (
                format!("{}\\left( {} \\right)", head_str, args_str),
                prec::ATOM,
            );
        }
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
        let env = &self.opts.matrix_environment;
        let mut out = format!("\\begin{{{}}} ", env);
        for r in 0..rows as usize {
            let row: Vec<String> = (0..cols as usize)
                .map(|c| self.emit(&entries[r * cols as usize + c], prec::LIST + 1))
                .collect();
            out.push_str(&row.join(" & "));
            if r < rows as usize - 1 {
                out.push_str(" \\\\ ");
            }
        }
        out.push_str(&format!(" \\end{{{}}}", env));
        out
    }

    fn render_other(&self, name: &str, args: &[Expr]) -> (String, u8) {
        use prec::*;
        // A head with too few operands to render in its own notation drops to
        // the generic form; see [`super::other_op_min_arity`].
        if args.len() < super::other_op_min_arity(name) {
            return self.render_other_generic(name, args);
        }
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
            "partial_derivative_leibniz" => (self.render_leibniz("\\partial", args), ATOM),
            _ => self.render_other_generic(name, args),
        }
    }

    /// `\operatorname{name}(args…)` — the form every unrecognized head takes,
    /// and the fallback for a recognized head whose operand count is too low
    /// for its notation.
    fn render_other_generic(&self, name: &str, args: &[Expr]) -> (String, u8) {
        (
            format!(
                "\\operatorname{{{}}}\\left({}\\right)",
                name,
                self.join(args, &self.arg_sep(), prec::LIST + 1)
            ),
            prec::ATOM,
        )
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
        // The separator belongs *after* the order, not after the symbol: a
        // hard-coded `\partial ` produced `\partial ^{2}x`. `cat` puts a space
        // in only where a control word would otherwise swallow what follows.
        let num = cat(
            &format!("{}{}", sym, pow_suffix(n_deriv)),
            &self.render_symbol(&var1),
        );
        let den = if let Expr::Seq(SeqKind::Tuple, parts) = &args[1] {
            parts
                .iter()
                .map(|part| {
                    let (v, e) = deriv_var(part);
                    format!("{}{}", cat(sym, &self.render_symbol(&v)), pow_suffix(e))
                })
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            String::new()
        };
        format!("\\frac{{{}}}{{{}}}", num, den)
    }
}

/// Concatenate two LaTeX fragments, inserting a space only where one is needed.
///
/// A control word (`\partial`) ends at the first non-letter, so `\partial x`
/// must keep its space or TeX reads the command `\partialx`. Nothing else does:
/// `dx`, `\partial^{2}`, `d\tau` are all unambiguous. The space also goes in
/// before another control word, where it is optional but far more readable.
fn cat(left: &str, right: &str) -> String {
    let trailing_letters = left.len()
        - left
            .trim_end_matches(|c: char| c.is_ascii_alphabetic())
            .len();
    let ends_in_control_word =
        trailing_letters > 0 && left[..left.len() - trailing_letters].ends_with('\\');
    let starts_a_name = right.starts_with(|c: char| c.is_ascii_alphabetic() || c == '\\');
    if ends_in_control_word && starts_a_name {
        format!("{} {}", left, right)
    } else {
        format!("{}{}", left, right)
    }
}

/// A radical (`\sqrt`, `\sqrt[3]`, `\sqrt[n]`): self-delimiting, but reads
/// clearer parenthesized when raised to a power (`\left(\sqrt{2}\right)^{3}`).
///
/// This must agree with the `sqrt`/`cbrt`/`nthroot` arms of
/// [`Writer::render_apply`], which emit a radical at *every* arity — the arity
/// only chooses whether there is an index. It used to be guarded on
/// `args.len() == 1`, which stopped matching those arms once they learned to
/// wrap a multi-argument list, and a disagreement here is silent: the radical
/// still renders, it just loses the parentheses, so `sqrt(x, y)^3` came out as
/// `\sqrt{\left( x, y \right)}^{3}` where legacy wrote
/// `\left(\sqrt{\left( x, y \right)}\right)^{3}`.
fn is_radical(e: &Expr) -> bool {
    matches!(e, Expr::Apply(head, _) if matches!(&**head, Expr::Sym(s)
        if matches!(s.name().as_str(), "sqrt" | "cbrt" | "nthroot")))
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
    if let Some(cmd) = crate::special_functions::latex_command(name) {
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

/// Is `s` a *single* `\left(…\right)` group — the opening `\left(` whose
/// matching `\right)` is the end of the string — rather than several adjacent
/// groups such as `\left(a\right) or \left(b\right)`?
///
/// The old test was `starts_with("\\left(") && ends_with("\\right)")`, which
/// reads `true` for both and so wrongly suppressed the wrap around a spaced
/// compound, letting the re-parse bind it differently (see the text-printer
/// twin `is_single_paren_group`). This counts `\left`/`\right` tokens — bare
/// parens inside would be wrong, since `\left[`/`\left\{` also nest.
fn is_single_delimited_group(s: &str) -> bool {
    if !s.starts_with("\\left(") {
        return false;
    }
    let mut depth: i32 = 0;
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        if rest.starts_with("\\left") {
            depth += 1;
            i += "\\left".len();
        } else if rest.starts_with("\\right") {
            depth -= 1;
            i += "\\right".len();
            if depth == 0 {
                // The outermost close matches the initial `\left(`, so it is a
                // `\right)`; the whole string is one group only if it ends here.
                return &s[i..] == ")";
            }
        } else {
            // A whole character, not a byte: `i` has to stay on a UTF-8
            // boundary or the next `&s[i..]` panics — and this crate is built
            // `panic = "abort"`, so that takes the module down. The blank glyph
            // `＿` (U+FF3F) is three bytes and is exactly what DoenetML leaves
            // in an unfilled slot, so `\left(＿, ＿\right) …` aborted. The text
            // twin `is_single_paren_group` walks `char_indices` for this reason.
            i += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    false
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
/// `crate::special_functions`.
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
