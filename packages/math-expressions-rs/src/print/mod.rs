//! Output formatters: `Expr` → text / LaTeX.
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

/// How many operands a specially-rendered `OtherOp` head needs before either
/// printer may index into its argument list; `0` for every head whose renderer
/// only ever `join`s, and for every head with no special rendering at all.
///
/// `expr/serde.rs`'s catch-all builds `OtherOp(name, args)` for any unknown
/// head with no arity check whatsoever, so `me.fromAst(["pm"])`,
/// `["binom","x"]`, `["unit","x"]` and `["d"]` are all constructible from JS.
/// The transform layers cope — a sweep of 22 heads × 4 arities × 4 wrappers ×
/// 6 operations found no panic in `simplify`, `canonicalize`, `expand` or
/// `default_order` — but the printers indexed `args[0]`/`args[1]` directly,
/// which was 104 panics. This crate is built `panic = "abort"`, so each one
/// killed the worker, and printing is the one thing that happens to *every*
/// expression on its way to the screen.
///
/// A head that does not meet its arity falls through to the generic
/// `name(args…)` form, which is what `render_angle` already does by hand and
/// what an unrecognized head has always got.
pub(crate) fn other_op_min_arity(name: &str) -> usize {
    match name {
        "binom" | "unit" | "derivative_leibniz" | "partial_derivative_leibniz" => 2,
        "pm" | "forall" | "exists" | "vec" | "d" => 1,
        _ => 0,
    }
}

/// The shared body of both printers' `Add` rendering: `a + b - c`, with the
/// leading term's sign attached rather than spelled with an operator, and a
/// `pm` term joined by a plain space because it carries its own operator
/// (`5 + ±3` would be wrong).
///
/// The two printers differ only in how a term renders, so that is the one
/// thing passed in; the joining logic was byte-identical in both.
///
/// A single-element `Add` is the parsers' unary-plus form (`+x`) and keeps its
/// sign.
pub(crate) fn render_add_terms(terms: &[Expr], emit: impl Fn(&Expr, u8) -> String) -> String {
    if terms.len() == 1 {
        return format!("+{}", emit(&terms[0], prec::ADD + 1));
    }
    let mut out = String::new();
    for (i, t) in terms.iter().enumerate() {
        if i > 0 && crate::ops::pm::is_pm(t) {
            out.push(' ');
            out.push_str(&emit(t, prec::ADD + 1));
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
        out.push_str(&emit(&body, prec::ADD + 1));
    }
    out
}

/// A one-argument `angle`, which both printers render with the prefix
/// shorthand (`∠A` / `\angle A`) rather than the parenthesized form.
pub(crate) fn is_shorthand_angle(e: &Expr) -> bool {
    matches!(e, Expr::OtherOp(name, args) if name.name() == "angle" && args.len() == 1)
}

/// Split a leading sign out of a term, structurally. Borrows where possible;
/// an owned node is returned only when the sign has to be pushed inward
/// (negating a number, or a product's first factor).
///
/// A product with a negative leading factor carries that factor's sign
/// (`["*",-3,"b"]` is `-(3 b)`), so it splits too — which is what lets a sum
/// render `a - 3 b` rather than `a + (-3) b`, and a bare product `-3 b` rather
/// than `(-3) b` (DoenetML open item 10). This is display-only: the positive
/// product re-parses to `Neg(Mul(...))`, an equal but distinct tree. That is
/// acceptable because the form is one the parsers only reach through an
/// explicitly parenthesized negative literal — `parse_text("(-3)b")` is
/// `Mul([Num(-3), Sym("b")])`, while `-3b` is a `Neg` — and that case is
/// exactly the one this rule exists to render readably.
pub(crate) fn split_sign(e: &Expr) -> (bool, std::borrow::Cow<'_, Expr>) {
    use std::borrow::Cow;
    match e {
        Expr::Neg(x) => (true, Cow::Borrowed(&**x)),
        Expr::Num(n) if number_is_negative(n) => (true, Cow::Owned(Expr::Num(n.neg()))),
        Expr::Mul(fs) => match strip_mul_leading_sign(fs) {
            Some(pos) => (true, Cow::Owned(Expr::Mul(pos))),
            None => (false, Cow::Borrowed(e)),
        },
        _ => (false, Cow::Borrowed(e)),
    }
}

/// If a product's first factor carries a pullable sign, return the factor list
/// with that factor made positive; otherwise `None`.
fn strip_mul_leading_sign(factors: &[Expr]) -> Option<Vec<Expr>> {
    let (neg, body) = split_sign(factors.first()?);
    if !neg {
        return None;
    }
    let mut out = Vec::with_capacity(factors.len());
    out.push(body.into_owned());
    out.extend(factors[1..].iter().cloned());
    Some(out)
}

/// Display-only pass (port of the legacy `normalize_display_negative_fractions`)
/// that pulls a leading negative factor out of a fraction's numerator into a
/// unary minus: `["/",-2,3]` shows as `-2/3`, and `z + (-2)/3` as `z - 2/3`.
/// Purely presentational — run at the printer entry, never on a stored tree.
///
/// The `inside_unary_minus` guard skips a fraction that is already the operand
/// of a unary minus (`-(-2/3)` stays as written, not `--2/3`); every other node
/// recurses with the guard cleared, so only a fraction directly under a `Neg`
/// is left alone.
pub(crate) fn normalize_display_negative_fractions(e: &Expr) -> Expr {
    ndf(e, false)
}

fn ndf(e: &Expr, inside_unary_minus: bool) -> Expr {
    match e {
        // A unary minus sets the guard for its immediate operand only.
        Expr::Neg(x) => Expr::Neg(Box::new(ndf(x, true))),
        Expr::Div(num, den) => {
            if !inside_unary_minus {
                if let (true, pos) = split_sign(num) {
                    return Expr::Neg(Box::new(Expr::Div(
                        Box::new(ndf(pos.as_ref(), false)),
                        Box::new(ndf(den, false)),
                    )));
                }
            }
            Expr::Div(Box::new(ndf(num, false)), Box::new(ndf(den, false)))
        }
        _ => crate::expr::map_children(e, |c| ndf(c, false)),
    }
}

pub(crate) fn number_is_negative(n: &Number) -> bool {
    n.is_negative()
}

/// Whether an `Apply` head is the integral sign — a bare `int` symbol, possibly
/// wrapped in the sub/superscript limits (`∫_a^b`). Mirrors the legacy check
/// that strips a `^` then a `_` from the head before comparing to `int`.
pub(crate) fn is_integral_head(head: &Expr) -> bool {
    match head {
        Expr::Sym(s) => s.name() == "int",
        Expr::Pow(b, _) | Expr::Index(b, _) => is_integral_head(b),
        _ => false,
    }
}

/// A Leibniz-notation variable entry is either `x` or `(x, n)`. A malformed
/// entry renders through the text printer — never `Debug`, whose Rust syntax
/// (`Num(Int(2))`) must not leak into user-facing output.
pub(crate) fn deriv_var(e: &Expr) -> (String, i64) {
    let render = |e: &Expr| text::convert(e, &Default::default());
    match e {
        Expr::Seq(SeqKind::Tuple, parts) if parts.len() == 2 => {
            let v = match &parts[0] {
                Expr::Sym(s) => s.name(),
                other => render(other),
            };
            let n = match &parts[1] {
                Expr::Num(Number::Int(i)) => *i,
                _ => 1,
            };
            (v, n)
        }
        Expr::Sym(s) => (s.name(), 1),
        other => (render(other), 1),
    }
}

pub(crate) fn pow_suffix(n: i64) -> String {
    if n > 1 {
        format!("^{}", n)
    } else {
        String::new()
    }
}

/// Render an expression as plain text (educational-math notation). Flattens
/// first, since parsing is now faithful (keeps raw grouping) but the formatters
/// assume flat n-ary operators; idempotent on already-canonical trees.
pub fn to_text(expr: &Expr, opts: &TextOpts) -> String {
    text::convert(&crate::expr::flatten(expr.clone()), opts)
}

/// Render an expression as LaTeX. Flattens first (see [`to_text`]).
pub fn to_latex(expr: &Expr, opts: &LatexOpts) -> String {
    latex::convert(&crate::expr::flatten(expr.clone()), opts)
}

/// Greek-letter (and a few symbol) name → unicode, shared by the formatters.
/// Mirrors JS `ast-to-text.js` `symbolConversions` exactly, including its
/// omissions: `chi` is deliberately absent (JS has no `chi` entry either), so
/// the `chi` symbol renders as ASCII `"chi"` in both engines. Do not add it —
/// that would emit `χ` where JS emits `chi`, a text-output parity break.
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
        "int" => "∫",
        _ => return None,
    })
}

/// Render a float in positional decimal notation, never exponential, using
/// the shortest digit string that round-trips. This is what
/// `avoidScientificNotation` selects, and the legacy `expandScientificNotation`
/// helper it is a port of: positional form parses unambiguously anywhere, at
/// worst verbosely (3e-12 → "0.000000000003"), whereas the parsers' scientific
/// literals are context-sensitive (the exponent is spelled `E` and folds only
/// before a delimiter) and a lowercase `e` means Euler's number.
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
    crate::num::positional_from_digits(&s, n)
}

/// Whether a power is `integer ^ integer`, which neither printer pads.
///
/// Legacy carved this out explicitly ("have integer^integer, as in scientific
/// notation … don't want to pad these numbers with zeros"): a product like
/// `123 * 10^28` is scientific notation written by hand, and padding it to five
/// digits gives `123.00 * 10.000^28.000`. Padding is a decimal-display option,
/// and an integer power is not a decimal.
pub(crate) fn is_integer_power(base: &Expr, exp: &Expr) -> bool {
    // An integer is exactly a number with no `a/b` spelling to report.
    let integer = |e: &Expr| {
        matches!(e, Expr::Num(n) if n.rational_parts().is_none()
        && !matches!(n, Number::Float(_)))
    };
    integer(base) && integer(exp)
}

/// How a float should display: as one positional string, or split into the
/// `mantissa × 10^exponent` pair the two printers spell differently
/// (`1.23 * 10^(-11)` vs `1.23 \cdot 10^{-11}`).
pub(crate) enum FloatRender {
    Positional(String),
    Scientific { mantissa: String, exponent: i64 },
}

/// Decide how a float displays, resolving the notation threshold and the
/// padding options *together* — they interact, and legacy resolved the
/// interaction in one place (`ast-to-text.js`, the `eIndex` branch), so this
/// does too rather than leaving each printer to rediscover it.
///
/// Three rules come from there:
/// - `avoid_scientific` forces positional at any magnitude.
/// - Padding decimals onto a *positive* exponent saves no zeros, so the whole
///   number reverts to positional (legacy's `toLocaleString("fullwide")`).
/// - Otherwise the padding applies to the mantissa, with the requested decimal
///   places shifted by the exponent — asking for 5 decimals of `1.23e-12` asks
///   nothing of the mantissa.
///
/// Non-finite values are never padded: `NaN` padded to five digits would read
/// `NaN.00`.
pub(crate) fn render_float(
    v: f64,
    avoid_scientific: bool,
    pad_to_digits: Option<u32>,
    pad_to_decimals: Option<u32>,
) -> FloatRender {
    let positional = || {
        FloatRender::Positional(pad_number(
            &f64_positional_string(v),
            pad_to_digits,
            pad_to_decimals,
        ))
    };
    if !v.is_finite() {
        return FloatRender::Positional(f64_positional_string(v));
    }
    if avoid_scientific || v == 0.0 {
        return positional();
    }
    let Some((mantissa, exponent)) = crate::num::js_exponential_parts(v.abs()) else {
        return positional();
    };
    if exponent > 0 && pad_to_decimals.is_some() {
        return positional();
    }
    let decimals = pad_to_decimals
        .map(|d| i64::from(d) + exponent)
        .and_then(|d| u32::try_from(d).ok());
    let signed = if v < 0.0 {
        format!("-{mantissa}")
    } else {
        mantissa
    };
    FloatRender::Scientific {
        mantissa: pad_number(&signed, pad_to_digits, decimals),
        exponent,
    }
}

/// [`render_float`] for a value whose exact decimal expansion is known — an
/// integer or a decimal-spelled rational.
///
/// These render from their own digits rather than an f64's shortest
/// round-trip, which is the whole point of holding a typed decimal exactly.
/// Notation is a separate question from digits, though, and below `0.000001`
/// it gets the float's answer: the reader shown `5.252 * 10^(-13)` for a
/// computed value should not be shown `0.0000000000005252` for a typed one,
/// where the leading zeros are unreadable and carry no information. Legacy
/// never had to decide this — every number it held was a float — so the
/// threshold is [`js_exponential_parts`]'s, applied to the exact digits.
///
/// DoenetML's `avoidScientificNotation` attribute depends on this: it exists
/// to *turn the threshold off*, and if exact values never reached it the
/// attribute would do nothing. Its test pins both sides symmetrically —
/// `2000000000000000000000 x^2` renders `2 \cdot 10^{21} x^{2}` by default and
/// positionally under the attribute — so the threshold applies at both ends,
/// not just where the leading zeros are.
///
/// Exactness is untouched either way: `2 * 10^21` *is* the exact value, so
/// upstream request 08 (a large value must not be perturbed by rounding) is
/// unaffected — only its spelling changes.
///
/// The padding rules are [`render_float`]'s, for the same reasons.
pub(crate) fn render_exact_decimal(
    s: &str,
    avoid_scientific: bool,
    pad_to_digits: Option<u32>,
    pad_to_decimals: Option<u32>,
) -> FloatRender {
    let positional = || FloatRender::Positional(pad_number(s, pad_to_digits, pad_to_decimals));
    if avoid_scientific {
        return positional();
    }
    // Zero has no exponent, and a string this does not understand is better
    // shown as it stands than guessed at.
    let Some((digits, n, negative)) = exact_decimal_parts(s) else {
        return positional();
    };
    // Positional over `0.000001 ..< 1e21`, exponential outside it.
    if -6 < n && n <= 21 {
        return positional();
    }
    // Padding decimals onto a positive exponent saves no zeros, so the whole
    // number reverts to positional — [`render_float`]'s rule, for parity.
    if n > 0 && pad_to_decimals.is_some() {
        return positional();
    }
    let mantissa = if digits.len() == 1 {
        digits
    } else {
        format!("{}.{}", &digits[..1], &digits[1..])
    };
    let exponent = n - 1;
    let decimals = pad_to_decimals
        .map(|d| i64::from(d) + exponent)
        .and_then(|d| u32::try_from(d).ok());
    let signed = if negative {
        format!("-{mantissa}")
    } else {
        mantissa
    };
    FloatRender::Scientific {
        mantissa: pad_number(&signed, pad_to_digits, decimals),
        exponent,
    }
}

/// Split an exact decimal string into `(digits, n, negative)` with
/// value = `±0.digits × 10^n` and `digits` free of leading and trailing zeros
/// — the same normalisation [`crate::num::shortest_digits`] produces for an
/// f64, so the two feed the same threshold.
///
/// `None` for zero (no exponent) and for anything that is not a plain decimal
/// numeral.
fn exact_decimal_parts(s: &str) -> Option<(String, i64, bool)> {
    let (negative, body) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let (int_part, frac_part) = body.split_once('.').unwrap_or((body, ""));
    if int_part.is_empty()
        || !int_part
            .bytes()
            .chain(frac_part.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let all = format!("{int_part}{frac_part}");
    let without_leading = all.trim_start_matches('0');
    let digits = without_leading.trim_end_matches('0');
    if digits.is_empty() {
        return None;
    }
    let leading = all.len() - without_leading.len();
    Some((
        digits.to_string(),
        int_part.len() as i64 - leading as i64,
        negative,
    ))
}

/// Append trailing zeros to a rendered positional number so it shows at least
/// `pad_to_digits` significant characters and/or `pad_to_decimals` fractional
/// places — port of the legacy `padNumberStringToDigitsAndDecimals`
/// (`converters/pad-numbers.js`). Pads only: never rounds, shortens, or moves
/// the point. A `None` or `0` bound is inactive. `s` is a positional magnitude,
/// possibly signed (`-1.5`), never exponential — matching the strings the
/// number printers produce.
///
/// Both bounds are clamped to [`MAX_PAD`]: padding is pure `"0".repeat(n)`, so
/// an unclamped bound turns a render option into an out-of-memory abort, which
/// on wasm (`panic = "abort"`) takes the whole worker with it. Legacy raised a
/// `RangeError` at the same wall; a clamp keeps the render alive instead.
pub(crate) fn pad_number(
    s: &str,
    pad_to_digits: Option<u32>,
    pad_to_decimals: Option<u32>,
) -> String {
    let bound = |d: Option<u32>| d.filter(|&d| d > 0).map(|d| (d as usize).min(MAX_PAD));
    let (digits, decimals) = (bound(pad_to_digits), bound(pad_to_decimals));
    match (digits, decimals) {
        (None, None) => s.to_string(),
        (None, Some(dec)) => pad_to_decimals_str(s, dec),
        (Some(dig), None) => pad_to_digits_str(s, dig),
        (Some(dig), Some(dec)) => pad_to_digits_and_decimals(s, dig, dec),
    }
}

/// The largest padding [`pad_number`] will honor. Well past any real display
/// use (a rendered number nobody reads is still a number nobody reads), and
/// small enough that the worst case is a few KB rather than an allocation
/// failure.
const MAX_PAD: usize = 1024;

/// Chars in the leading `0.0*` run (the JS `/^0\.0*/` match). Only called when
/// `s` begins `0.`.
fn leading_zero_run(s: &str) -> usize {
    2 + s[2..].chars().take_while(|&c| c == '0').count()
}

/// Fractional-digit count — chars after the `.` (0 if none).
fn decimal_count(s: &str) -> usize {
    s.split_once('.').map_or(0, |(_, frac)| frac.len())
}

fn pad_to_digits_str(s: &str, n_digits: usize) -> String {
    let mut s = s.to_string();
    let mut n_chars = n_digits;
    if s.contains('.') {
        // A non-leading-zero head (including a `-` sign) costs one char for the
        // point; a `0.00…` head costs the whole run — mirrors the JS branch.
        n_chars += if s.starts_with('0') {
            leading_zero_run(&s)
        } else {
            1
        };
        if s.len() < n_chars {
            s.push_str(&"0".repeat(n_chars - s.len()));
        }
    } else if s.len() < n_chars {
        let n_pad = n_chars - s.len();
        s.push('.');
        s.push_str(&"0".repeat(n_pad));
    }
    s
}

fn pad_to_decimals_str(s: &str, n_decimals: usize) -> String {
    let mut s = s.to_string();
    if s.contains('.') {
        let current = decimal_count(&s);
        if current < n_decimals {
            s.push_str(&"0".repeat(n_decimals - current));
        }
    } else {
        s.push('.');
        s.push_str(&"0".repeat(n_decimals));
    }
    s
}

fn pad_to_digits_and_decimals(s: &str, n_digits: usize, n_decimals: usize) -> String {
    let mut s = s.to_string();
    if s.contains('.') {
        let mut n_chars = n_digits;
        n_chars += if s.starts_with('0') {
            leading_zero_run(&s)
        } else {
            1
        };
        let mut n_pad = n_chars.saturating_sub(s.len());
        let current = decimal_count(&s);
        if current < n_decimals {
            n_pad = n_pad.max(n_decimals - current);
        }
        if n_pad > 0 {
            s.push_str(&"0".repeat(n_pad));
        }
    } else {
        let n_pad = n_digits.saturating_sub(s.len()).max(n_decimals);
        s.push('.');
        s.push_str(&"0".repeat(n_pad));
    }
    s
}

#[cfg(test)]
mod pad_tests {
    use super::pad_number;
    fn d(s: &str, dec: u32) -> String {
        pad_number(s, None, Some(dec))
    }
    fn g(s: &str, dig: u32) -> String {
        pad_number(s, Some(dig), None)
    }

    #[test]
    fn pads_decimals() {
        assert_eq!(d("1.5", 4), "1.5000");
        assert_eq!(d("2", 3), "2.000");
        assert_eq!(d("1.5", 1), "1.5"); // already enough
        assert_eq!(d("-0.75", 4), "-0.7500");
    }

    #[test]
    fn pads_digits() {
        assert_eq!(g("5", 4), "5.000");
        assert_eq!(g("1.5", 4), "1.500"); // 1,5,0,0 significant chars + point
        assert_eq!(g("0.005", 2), "0.0050"); // leading-zero run counted
    }

    #[test]
    fn pads_to_the_larger_of_both() {
        assert_eq!(pad_number("1.5", Some(6), Some(2)), "1.50000"); // 6 sig chars win
        assert_eq!(pad_number("1.5", Some(2), Some(4)), "1.5000"); // decimals win
        assert_eq!(pad_number("3", Some(4), Some(2)), "3.000"); // digits win, integer
    }

    #[test]
    fn zero_and_none_bounds_are_inactive() {
        assert_eq!(pad_number("1.5", None, None), "1.5");
        assert_eq!(pad_number("1.5", Some(0), Some(0)), "1.5");
    }

    /// An absurd bound clamps instead of trying to allocate 4 GB of zeros.
    #[test]
    fn an_absurd_bound_clamps_instead_of_exhausting_memory() {
        // `1.` plus MAX_PAD fractional zeros.
        assert_eq!(d("1.5", u32::MAX).len(), 2 + super::MAX_PAD);
        // MAX_PAD significant characters plus the decimal point.
        assert_eq!(g("5", u32::MAX).len(), super::MAX_PAD + 1);
    }
}
