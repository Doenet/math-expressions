//! Tiered number type (PORTING_PLAN.md §3, §3a).
//!
//! User-typed decimals parse to *exact* rationals (`Int`/`Rat`/`Big`), never
//! `Float` — see [`Number::from_decimal_str`]. `Float` is reserved for
//! numerical evaluation results. Rationals whose denominator is `2^a·5^b`
//! render back as decimals ([`Number::terminating_decimal`]), so decimal
//! input round-trips exactly.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

/// f64 wrapper providing Eq + Hash by bit pattern (f64 itself implements
/// neither). Policy: NaN == NaN, +0.0 != -0.0. Numeric comparisons in
/// equality testing go through tolerances, not this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct F64(u64);

impl F64 {
    pub fn new(v: f64) -> Self {
        F64(v.to_bits())
    }
    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Number {
    /// Integers that fit in i64. No allocation.
    Int(i64),
    /// Reduced fractions. Invariant: den > 0, gcd(|num|, den) == 1, den != 1.
    Rat(i64, i64),
    /// Arbitrary precision fallback. Boxed to keep Number small.
    Big(Box<BigNumber>),
    /// Floating-point value — produced by numerical evaluation only. User
    /// input never parses to `Float` (§3a: decimals are exact rationals).
    Float(F64),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BigNumber {
    Int(BigInt),
    Rat(BigRational),
}

impl Number {
    /// Number from an f64, demoting to Int when the value is integral —
    /// matches how JS number literals behave (JSON.stringify(3.0) == "3").
    /// The upper bound is exclusive: `i64::MAX as f64` rounds up to 2^63,
    /// which an `as` cast would silently saturate.
    pub fn from_f64(v: f64) -> Self {
        if v.fract() == 0.0 && v.is_finite() && v >= i64::MIN as f64 && v < i64::MAX as f64 {
            Number::Int(v as i64)
        } else {
            Number::Float(F64::new(v))
        }
    }

    /// Reduced rational from an i64 numerator/denominator. Enforces the `Rat`
    /// invariants (den > 0, gcd == 1, den != 1) and demotes to `Int` when the
    /// denominator reduces to 1. Panics on a zero denominator.
    pub fn rat(mut num: i64, mut den: i64) -> Number {
        assert!(den != 0, "rational with zero denominator");
        if den < 0 {
            num = -num;
            den = -den;
        }
        let g = gcd_i64(num, den);
        if g > 1 {
            num /= g;
            den /= g;
        }
        if den == 1 {
            Number::Int(num)
        } else {
            Number::Rat(num, den)
        }
    }

    /// Reduce and demote an arbitrary-precision integer to the smallest tier.
    pub fn from_bigint(v: BigInt) -> Number {
        match v.to_i64() {
            Some(i) => Number::Int(i),
            None => Number::Big(Box::new(BigNumber::Int(v))),
        }
    }

    /// Reduce and demote an arbitrary-precision rational: to `Int` when
    /// integral and small, to `Rat` when numerator and denominator both fit
    /// i64, otherwise `Big`. `BigRational` keeps itself in lowest terms.
    pub fn from_bigrational(v: BigRational) -> Number {
        if v.is_integer() {
            return Number::from_bigint(v.to_integer());
        }
        if let (Some(n), Some(d)) = (v.numer().to_i64(), v.denom().to_i64()) {
            // Already reduced and non-integral, so den != 1 and den > 0.
            Number::Rat(n, d)
        } else {
            Number::Big(Box::new(BigNumber::Rat(v)))
        }
    }

    pub fn to_f64(&self) -> f64 {
        match self {
            Number::Int(i) => *i as f64,
            Number::Rat(n, d) => *n as f64 / *d as f64,
            Number::Float(f) => f.get(),
            Number::Big(b) => match &**b {
                BigNumber::Int(i) => i.to_f64().unwrap_or(f64::NAN),
                BigNumber::Rat(r) => r.to_f64().unwrap_or(f64::NAN),
            },
        }
    }

    pub fn is_positive(&self) -> bool {
        match self {
            Number::Int(i) => *i > 0,
            Number::Rat(n, _) => *n > 0,
            Number::Float(f) => f.get() > 0.0,
            Number::Big(b) => match &**b {
                BigNumber::Int(i) => i.is_positive(),
                BigNumber::Rat(r) => r.is_positive(),
            },
        }
    }

    pub fn is_negative(&self) -> bool {
        match self {
            Number::Int(i) => *i < 0,
            Number::Rat(n, _) => *n < 0,
            Number::Float(f) => f.get() < 0.0,
            Number::Big(b) => match &**b {
                BigNumber::Int(i) => i.is_negative(),
                BigNumber::Rat(r) => r.is_negative(),
            },
        }
    }

    /// Numerator and denominator as strings, for a non-integral rational
    /// (`Rat` or big rational); `None` for integers, floats, and big
    /// integers. Used by formatters for the `a/b` / `\frac` fallback when the
    /// fraction does not terminate as a decimal.
    pub fn rational_parts(&self) -> Option<(String, String)> {
        match self {
            Number::Rat(n, d) => Some((n.to_string(), d.to_string())),
            Number::Big(b) => match &**b {
                BigNumber::Rat(r) => Some((r.numer().to_string(), r.denom().to_string())),
                BigNumber::Int(_) => None,
            },
            _ => None,
        }
    }

    pub fn neg(&self) -> Number {
        match self {
            Number::Int(i) => Number::Int(-i),
            Number::Rat(n, d) => Number::Rat(-n, *d),
            Number::Float(f) => Number::Float(F64::new(-f.get())),
            Number::Big(b) => match &**b {
                BigNumber::Int(i) => Number::from_bigint(-i),
                BigNumber::Rat(r) => Number::from_bigrational(-r),
            },
        }
    }

    /// Parse a decimal NUMBER token to an *exact* rational (§3a). The value is
    /// `digits × 10^(exp − frac_len)`: a non-negative power of ten yields an
    /// integer (`Int`/`Big`), a negative one an exact fraction whose
    /// denominator is `2^a·5^b` (`Rat`/`Big`). Never returns `Float`.
    ///
    /// Accepts the NUMBER token grammar (`12`, `1.`, `.3`, `1.2`, optional
    /// `E[+-]?digits`, surrounding whitespace the sci-notation lexer folds in)
    /// and, for overflow literals, `Const`-worthy infinities are the caller's
    /// concern — this returns the exact value, however large.
    pub fn from_decimal_str(text: &str) -> Number {
        let t = text.trim();
        let (mantissa, exp) = match t.split_once(['E', 'e']) {
            Some((m, e)) => (m, e.parse::<i64>().unwrap_or(0)),
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
            digits.parse().expect("NUMBER token is all digits")
        };

        let pow10 = exp - frac_part.len() as i64;
        let ten = BigInt::from(10u32);
        if pow10 >= 0 {
            Number::from_bigint(numer * ten.pow(pow10 as u32))
        } else {
            let den = ten.pow((-pow10) as u32);
            Number::from_bigrational(BigRational::new(numer, den))
        }
    }

    /// Render an exact rational in positional decimal notation, iff its
    /// denominator divides a power of ten (`2^a·5^b`). Integers always
    /// succeed; a fraction like `1/3` returns `None`. Computed with exact
    /// big-integer arithmetic so even long or `Big` literals reproduce
    /// digit-for-digit (an f64 projection would truncate).
    pub fn terminating_decimal(&self) -> Option<String> {
        let (numer, denom): (BigInt, BigInt) = match self {
            Number::Int(i) => return Some(i.to_string()),
            Number::Rat(n, d) => (BigInt::from(*n), BigInt::from(*d)),
            Number::Big(b) => match &**b {
                BigNumber::Int(i) => return Some(i.to_string()),
                BigNumber::Rat(r) => (r.numer().clone(), r.denom().clone()),
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
            Number::Float(f) => js_f64_to_string(f.get()),
            Number::Rat(..) | Number::Big(_) => js_f64_to_string(self.to_f64()),
        }
    }
}

/// Greatest common divisor of two i64s (by magnitude). `gcd(x, 0) == |x|`.
fn gcd_i64(a: i64, b: i64) -> i64 {
    let mut a = a.unsigned_abs();
    let mut b = b.unsigned_abs();
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a.min(i64::MAX as u64) as i64
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
    let (s, n) = shortest_digits(v);
    let k = s.len() as i64;

    if k <= n && n <= 21 {
        format!("{}{}", s, "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &s[..n as usize], &s[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{}", "0".repeat((-n) as usize), s)
    } else {
        let e = n - 1;
        let mantissa = if k == 1 {
            s.to_string()
        } else {
            format!("{}.{}", &s[..1], &s[1..])
        };
        let sign = if e >= 0 { "+" } else { "-" };
        format!("{}e{}{}", mantissa, sign, e.abs())
    }
}
