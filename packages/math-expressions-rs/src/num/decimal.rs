//! Decimal ⇄ [`Number`] conversion: exact parsing of decimal literals, exact
//! terminating-decimal rendering, and JS-faithful f64 stringification.
//!
//! User-typed decimals parse to *exact* rationals (never `Float`); rationals
//! whose denominator is `2^a·5^b` render back as decimals, so decimal input
//! round-trips exactly. The f64 path reproduces JavaScript
//! `Number.prototype.toString()` for the parser's sign-string concatenation.

use super::number::{BigNumber, Number, Spelling, F64};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

impl Number {
    /// Parse a decimal NUMBER token to an *exact* rational. The value is
    /// `digits × 10^(exp − frac_len)`: a non-negative power of ten yields an
    /// integer (`Int`/`Big`), a negative one an exact fraction whose
    /// denominator is `2^a·5^b` (`Rat`/`Big`). Returns `Float` only for
    /// absurd exponents (|10^exp| beyond ~10^6 digits), where it approximates
    /// like JS `parseFloat`.
    ///
    /// Accepts the NUMBER token grammar (`12`, `1.`, `.3`, `1.2`, optional
    /// `E[+-]?digits`, surrounding whitespace the sci-notation lexer folds in)
    /// and, for overflow literals, `Const`-worthy infinities are the caller's
    /// concern — this returns the exact value, however large.
    pub fn from_decimal_str(text: &str) -> Number {
        let t = text.trim();
        let (mantissa, exp) = match t.split_once(['E', 'e']) {
            // An exponent overflowing i64 saturates (the magnitude cap below
            // then applies); `unwrap_or(0)` would silently misread the value.
            Some((m, e)) => (
                m,
                e.parse::<i64>().unwrap_or_else(|_| {
                    if e.starts_with('-') {
                        i64::MIN
                    } else {
                        i64::MAX
                    }
                }),
            ),
            None => (t, 0),
        };
        let (int_part, frac_part) = match mantissa.split_once('.') {
            Some((i, f)) => (i, f),
            None => (mantissa, ""),
        };

        let mut digits = String::with_capacity(int_part.len() + frac_part.len());
        digits.push_str(int_part);
        digits.push_str(frac_part);
        let digits = digits.trim_start_matches('0');
        let numer: BigInt = if digits.is_empty() {
            BigInt::zero()
        } else {
            match digits.parse() {
                Ok(n) => n,
                // The parsers only ever pass NUMBER tokens (all digits), so this
                // is unreachable in normal flow — but the method is `pub` and a
                // panic here would `abort` the whole wasm worker (item 9). Fall
                // back to the JS `parseFloat` approximation instead of trapping.
                Err(_) => {
                    let approx = t.replace(['E'], "e").parse().unwrap_or(f64::NAN);
                    return Number::Float(F64::new(approx));
                }
            }
        };

        let pow10 = exp.saturating_sub(frac_part.len() as i64);
        // Beyond ~10^6 decimal places the exact value is not worth
        // materializing (megabytes of BigInt, and `pow10 as u32` would wrap);
        // approximate through f64 like JS parseFloat (±inf / 0 / rounded).
        if pow10.unsigned_abs() > 1_000_000 {
            let approx: f64 = t.replace(['E'], "e").parse().unwrap_or(f64::NAN);
            return Number::Float(F64::new(approx));
        }
        let ten = BigInt::from(10u32);
        if pow10 >= 0 {
            Number::from_bigint(numer * ten.pow(pow10 as u32))
        } else {
            let den = ten.pow((-pow10) as u32);
            // Decimal-spelled: this is where the one and only decimal *origin*
            // is recorded, so `19.9` reads back as `19.9` rather than `199/10`.
            Number::from_bigrational_spelled(BigRational::new(numer, den), Spelling::Decimal)
        }
    }

    /// The positional decimal string this value should *display* as, or `None`
    /// when it should display as a fraction. That is
    /// [`terminating_decimal`](Number::terminating_decimal) plus the
    /// [`Spelling`] gate: a terminating rational that came from a fraction
    /// (`3/6`, `cos(pi/3)`) keeps its `n/d` spelling everywhere — the `.tree`,
    /// the text printer, and the LaTeX printer all read this one method, so
    /// they cannot disagree.
    pub fn decimal_spelling(&self) -> Option<String> {
        (self.spelling() == Spelling::Decimal || self.rational_parts().is_none())
            .then(|| self.terminating_decimal())
            .flatten()
    }

    /// Render an exact rational in positional decimal notation, iff its
    /// denominator divides a power of ten (`2^a·5^b`). Integers always
    /// succeed; a fraction like `1/3` returns `None`. **Asks whether the
    /// expansion terminates, not whether it should be used** — for display,
    /// call [`decimal_spelling`](Number::decimal_spelling). Computed with exact
    /// big-integer arithmetic so even long or `Big` literals reproduce
    /// digit-for-digit (an f64 projection would truncate).
    pub fn terminating_decimal(&self) -> Option<String> {
        let (numer, denom): (BigInt, BigInt) = match self {
            Number::Int(i) => return Some(i.to_string()),
            Number::NegZero => return Some("0".to_string()),
            Number::Rat(n, d, _) => (BigInt::from(*n), BigInt::from(*d)),
            Number::Big(b) => match &**b {
                BigNumber::Int(i) => return Some(i.to_string()),
                BigNumber::Rat(r, _) => (r.numer().clone(), r.denom().clone()),
            },
            Number::Float(_) => return None,
        };
        // Count the 2s and 5s in the denominator; anything left means the
        // decimal expansion does not terminate.
        let (twos, rest) = factor_out(denom, 2);
        let (fives, rest) = factor_out(rest, 5);
        if !rest.is_one() {
            return None;
        }
        // Scale numerator and denominator up to a common 10^k.
        let k = twos.max(fives);
        let scale = BigInt::from(2u32).pow(k - twos) * BigInt::from(5u32).pow(k - fives);
        let scaled = numer * scale; // value = scaled / 10^k
        Some(place_decimal_point(scaled, k as usize))
    }

    /// Format the way JavaScript stringifies a number (integral values have no
    /// decimal point). Used by the parser's sign-string concatenation, so
    /// rationals go through the f64 projection to stay JS-faithful — input
    /// `0.10` must yield the sign-string atom `0.1`, matching `parseFloat`.
    pub fn js_string(&self) -> String {
        match self {
            Number::Int(i) => i.to_string(),
            Number::NegZero => "0".to_string(),
            Number::Float(f) => js_f64_to_string(f.get()),
            Number::Rat(..) | Number::Big(_) => js_f64_to_string(self.to_f64()),
        }
    }
}

/// Divide `v` by `prime` as many times as it goes evenly; return the count and
/// the remaining cofactor.
fn factor_out(mut v: BigInt, prime: u32) -> (u32, BigInt) {
    let p = BigInt::from(prime);
    let mut count = 0;
    while !v.is_zero() && (&v % &p).is_zero() {
        v /= &p;
        count += 1;
    }
    (count, v)
}

/// Render `scaled / 10^k` as a positional decimal string, trimming trailing
/// fractional zeros.
fn place_decimal_point(scaled: BigInt, k: usize) -> String {
    if k == 0 {
        return scaled.to_string();
    }
    let negative = scaled.is_negative();
    let mut digits = scaled.abs().to_string();
    if digits.len() <= k {
        // Pad with leading zeros so there is at least one digit left of point.
        digits = "0".repeat(k - digits.len() + 1) + &digits;
    }
    let point = digits.len() - k;
    let int_part = &digits[..point];
    let frac_part = digits[point..].trim_end_matches('0');
    let sign = if negative { "-" } else { "" };
    if frac_part.is_empty() {
        format!("{}{}", sign, int_part)
    } else {
        format!("{}{}.{}", sign, int_part, frac_part)
    }
}

/// The shortest digit string for a positive finite f64 and its decimal
/// exponent: `(digits, n)` with value = 0.digits × 10^n. Derived from Rust's
/// `{:e}` formatting, which produces shortest round-trip digits like JS.
pub(crate) fn shortest_digits(v: f64) -> (String, i64) {
    debug_assert!(v > 0.0 && v.is_finite());
    let es = format!("{:e}", v);
    let (mantissa, exp) = es.split_once('e').expect("`{:e}` always has an e");
    let exp: i64 = exp.parse().unwrap();
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let s = digits.trim_end_matches('0');
    let s = if s.is_empty() { "0" } else { s };
    (s.to_string(), exp + 1)
}

/// Reproduce JavaScript `Number.prototype.toString()` for an f64, applying
/// the ECMAScript rendering rule (which switches to exponential notation
/// outside roughly 1e-6..1e21, where Rust's `{}` never would).
pub(crate) fn js_f64_to_string(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v == 0.0 {
        return "0".to_string();
    }
    if v < 0.0 {
        return format!("-{}", js_f64_to_string(-v));
    }
    if v.is_infinite() {
        return "Infinity".to_string();
    }
    match js_exponential_parts(v) {
        Some((mantissa, e)) => {
            let sign = if e >= 0 { "+" } else { "-" };
            format!("{}e{}{}", mantissa, sign, e.abs())
        }
        None => {
            let (s, n) = shortest_digits(v);
            positional_from_digits(&s, n)
        }
    }
}

/// The mantissa and decimal exponent JavaScript's `Number.prototype.toString()`
/// renders `v` with, or `None` when the ECMAScript rule keeps it positional.
/// `v` must be positive and finite; the sign belongs to the caller.
///
/// This *is* the threshold — the legacy printers had no constant of their own,
/// they called `toString()` and looked for an `e`. Factored out so the output
/// formatters can ask where the switch happens without restating it, and so
/// there is exactly one place to change if the rule ever moves.
pub(crate) fn js_exponential_parts(v: f64) -> Option<(String, i64)> {
    debug_assert!(v > 0.0 && v.is_finite());
    let (s, n) = shortest_digits(v);
    // Positional over `0.000001 ..< 1e21`, exponential outside it.
    if -6 < n && n <= 21 {
        return None;
    }
    let mantissa = if s.len() == 1 {
        s
    } else {
        format!("{}.{}", &s[..1], &s[1..])
    };
    Some((mantissa, n - 1))
}

/// Positional decimal for the digit string and exponent of a positive finite
/// f64 (value = `0.digits × 10^n`). Shared with the printers'
/// `f64_positional_string` so the two render the same digits.
pub(crate) fn positional_from_digits(s: &str, n: i64) -> String {
    let k = s.len() as i64;
    if k <= n {
        format!("{}{}", s, "0".repeat((n - k) as usize))
    } else if n > 0 {
        format!("{}.{}", &s[..n as usize], &s[n as usize..])
    } else {
        format!("0.{}{}", "0".repeat((-n) as usize), s)
    }
}
