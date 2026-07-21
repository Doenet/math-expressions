//! Output formatters (PORTING_PLAN.md §12): `Expr` → text / LaTeX.
//!
//! These are clean precedence-based pretty-printers that walk `Expr`
//! directly, rather than transcriptions of the JS formatters (which decide
//! parenthesisation by regex-matching their own output over the ad-hoc JS
//! tree shape). Correctness is enforced by round-tripping through the parsers
//! (`tests/roundtrip.rs`), not by matching JS output byte-for-byte.

pub mod latex;
pub mod text;

use crate::expr::{Expr, SeqKind};
use crate::num::Number;

pub use latex::LatexOpts;
pub use text::TextOpts;

/// Precedence ladder (tighter binds higher), aligned with the parser grammars,
/// shared by both formatters so output round-trips with minimal parentheses.
pub(crate) mod prec {
    /// Sign-string symbols — parenthesise everywhere but the top level.
    pub const SIGN: u8 = 1;
    pub const LIST: u8 = 10;
    pub const COLONBAR: u8 = 15;
    pub const ARROW: u8 = 20;
    pub const OR: u8 = 30;
    pub const AND: u8 = 40;
    pub const NOT: u8 = 45;
    pub const REL: u8 = 50;
    pub const ADD: u8 = 60;
    pub const NEG: u8 = 65;
    /// A unit-bearing quantity (`x %`, `$ x`, `x°`) binds looser than
    /// multiplication, so it parenthesizes as a factor in a product
    /// (`\left(x \%\right) y`) but not standalone or in a sum.
    pub const UNIT: u8 = 66;
    pub const MUL: u8 = 70;
    pub const POW: u8 = 90;
    pub const INDEX: u8 = 95;
    pub const ATOM: u8 = 100;
}

/// Split a leading sign out of a sum term, structurally (never from a Mul,
/// which would not round-trip). Borrows where possible; only a negative
/// number needs an owned negation.
pub(crate) fn split_sign(e: &Expr) -> (bool, std::borrow::Cow<'_, Expr>) {
    use std::borrow::Cow;
    match e {
        Expr::Neg(x) => (true, Cow::Borrowed(&**x)),
        Expr::Num(n) if number_is_negative(n) => (true, Cow::Owned(Expr::Num(n.neg()))),
        _ => (false, Cow::Borrowed(e)),
    }
}

pub(crate) fn number_is_negative(n: &Number) -> bool {
    n.is_negative()
}

/// A Leibniz-notation variable entry is either `x` or `(x, n)`.
pub(crate) fn deriv_var(e: &Expr) -> (String, i64) {
    match e {
        Expr::Seq(SeqKind::Tuple, parts) if parts.len() == 2 => {
            let v = match &parts[0] {
                Expr::Sym(s) => s.name(),
                other => format!("{:?}", other),
            };
            let n = match &parts[1] {
                Expr::Num(Number::Int(i)) => *i,
                _ => 1,
            };
            (v, n)
        }
        Expr::Sym(s) => (s.name(), 1),
        other => (format!("{:?}", other), 1),
    }
}

pub(crate) fn pow_suffix(n: i64) -> String {
    if n > 1 {
        format!("^{}", n)
    } else {
        String::new()
    }
}

/// Render an expression as plain text (educational-math notation).
pub fn to_text(expr: &Expr, opts: &TextOpts) -> String {
    text::convert(expr, opts)
}

/// Render an expression as LaTeX.
pub fn to_latex(expr: &Expr, opts: &LatexOpts) -> String {
    latex::convert(expr, opts)
}

/// Greek-letter (and a few symbol) name → unicode, shared by the formatters.
pub(crate) fn greek_unicode(name: &str) -> Option<&'static str> {
    Some(match name {
        "alpha" => "α",
        "beta" => "β",
        "Gamma" => "Γ",
        "gamma" => "γ",
        "Delta" => "Δ",
        "delta" => "δ",
        "epsilon" => "ε",
        "zeta" => "ζ",
        "eta" => "η",
        "Theta" => "ϴ",
        "theta" => "θ",
        "iota" => "ι",
        "kappa" => "κ",
        "Lambda" => "Λ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "Xi" => "Ξ",
        "xi" => "ξ",
        "Pi" => "Π",
        "pi" => "π",
        "rho" => "ρ",
        "Sigma" => "Σ",
        "sigma" => "σ",
        "tau" => "τ",
        "Upsilon" => "Υ",
        "upsilon" => "υ",
        "Phi" => "Φ",
        "phi" => "ϕ",
        "Psi" => "Ψ",
        "psi" => "ψ",
        "Omega" => "Ω",
        "omega" => "ω",
        "emptyset" => "∅",
        // Named glyphs the lexer accepts as single VARMULTICHAR tokens; their
        // ASCII names would re-split, so they must render as the glyph.
        "spade" => "♠",
        "heart" => "♡",
        "diamond" => "♢",
        "club" => "♣",
        "bigstar" => "★",
        "bigcirc" => "◯",
        "lozenge" => "◊",
        "bigtriangleup" => "△",
        "bigtriangledown" => "▽",
        "blacklozenge" => "⧫",
        "blacksquare" => "■",
        "blacktriangle" => "▲",
        "blacktriangledown" => "▼",
        "blacktriangleleft" => "◀",
        "blacktriangleright" => "▶",
        "Box" => "□",
        "circ" => "∘",
        "star" => "⋆",
        "perp" => "⟂",
        _ => return None,
    })
}

/// Render a float in positional decimal notation, never exponential, using
/// the shortest digit string that round-trips. Exponential forms cannot be
/// re-parsed reliably: the parsers' scientific literals are context-sensitive
/// (the exponent is spelled `E` and folds only before a delimiter) and a
/// lowercase `e` means Euler's number. Positional form parses unambiguously
/// anywhere, at worst verbosely (3e-12 → "0.000000000003").
pub(crate) fn f64_positional_string(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v == 0.0 {
        return "0".to_string();
    }
    if v < 0.0 {
        return format!("-{}", f64_positional_string(-v));
    }
    if v.is_infinite() {
        return "Infinity".to_string();
    }
    let (s, n) = crate::num::shortest_digits(v);
    let k = s.len() as i64;
    if k <= n {
        format!("{}{}", s, "0".repeat((n - k) as usize))
    } else if n > 0 {
        format!("{}.{}", &s[..n as usize], &s[n as usize..])
    } else {
        format!("0.{}{}", "0".repeat((-n) as usize), s)
    }
}
