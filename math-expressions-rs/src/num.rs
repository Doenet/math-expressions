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
    pub fn from_f64(v: f64) -> Self {
        if v.fract() == 0.0 && v.is_finite() && v >= i64::MIN as f64 && v <= i64::MAX as f64 {
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
            Number::Float(f) => {
                let v = f.get();
                if v.fract() == 0.0 && v.is_finite() && v.abs() < 1e21 {
                    format!("{}", v as i64)
                } else {
                    format!("{}", v)
                }
            }
            Number::Rat(n, d) => format!("{}/{}", n, d),
            Number::Big(_) => unimplemented!("Big arithmetic arrives in phase 4"),
        }
    }
}
