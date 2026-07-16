//! Tiered number type (PORTING_PLAN.md §3).
//!
//! The parser only constructs `Int` and `Float`; `Rat`/`Big` arithmetic
//! arrives with the normalisation phase.

use num_bigint::BigInt;
use num_rational::BigRational;

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
    /// Floating-point value (scientific notation at parse time, numerical
    /// evaluation results).
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

    pub fn to_f64(&self) -> f64 {
        match self {
            Number::Int(i) => *i as f64,
            Number::Rat(n, d) => *n as f64 / *d as f64,
            Number::Float(f) => f.get(),
            Number::Big(_) => unimplemented!("Big arithmetic arrives in phase 4"),
        }
    }

    pub fn is_positive(&self) -> bool {
        match self {
            Number::Int(i) => *i > 0,
            Number::Rat(n, _) => *n > 0,
            Number::Float(f) => f.get() > 0.0,
            Number::Big(_) => unimplemented!("Big arithmetic arrives in phase 4"),
        }
    }

    pub fn neg(&self) -> Number {
        match self {
            Number::Int(i) => Number::Int(-i),
            Number::Rat(n, d) => Number::Rat(-n, *d),
            Number::Float(f) => Number::Float(F64::new(-f.get())),
            Number::Big(_) => unimplemented!("Big arithmetic arrives in phase 4"),
        }
    }

    /// Format the way JavaScript stringifies a number (integral values have
    /// no decimal point). Used by the parser's sign-string concatenation.
    pub fn js_string(&self) -> String {
        match self {
            Number::Int(i) => i.to_string(),
            Number::Float(f) => js_f64_to_string(f.get()),
            Number::Rat(n, d) => format!("{}/{}", n, d),
            Number::Big(_) => unimplemented!("Big arithmetic arrives in phase 4"),
        }
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
