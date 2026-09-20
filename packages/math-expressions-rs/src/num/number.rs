//! Tiered number type and its exact/float arithmetic.
//!
//! User-typed decimals parse to *exact* rationals (`Int`/`Rat`/`Big`), never
//! `Float` — see [`Number::from_decimal_str`](super::Number::from_decimal_str).
//! `Float` is reserved for numerical evaluation results. Decimal parsing and
//! rendering live in [`decimal`](super::decimal); GCD in [`gcd`](super::gcd).

use super::gcd::gcd_i64;
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

/// How a non-integer exact rational should be *written back out*.
///
/// The two are the same value and compare equal; this only decides spelling.
/// It has to be carried rather than derived because the value alone cannot
/// answer the question: decimals parse to exact rationals by design, so `0.5`
/// and `1/2` are both `Rat(1, 2)` and the distinction is gone by the time
/// anything reaches the serializer or a printer.
///
/// `Decimal` is contagious through arithmetic, the same way `Float` is: once a
/// decimal quantity is involved the result is a decimal quantity. That makes
/// `Fraction` the identity for [`join`](Spelling::join) and hence the right
/// default for integers, floats, and every exact value the engine *computes*.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Spelling {
    /// `n/d` — a fraction of integers, or anything derived from one.
    #[default]
    Fraction,
    /// A positional decimal, when the expansion terminates — a decimal literal
    /// the user typed, or a value rounded to a number of decimal places.
    Decimal,
}

impl Spelling {
    /// The spelling of a result computed from two operands: `Decimal` wins.
    pub fn join(self, other: Spelling) -> Spelling {
        if self == Spelling::Decimal || other == Spelling::Decimal {
            Spelling::Decimal
        } else {
            Spelling::Fraction
        }
    }
}

/// Note the hand-written `PartialEq`/`Hash` below: [`Spelling`] is *not* part
/// of a number's identity. `0.5 == 1/2` structurally, so canonical trees stay
/// comparable by `==` and hashable as keys, exactly as before this field
/// existed.
#[derive(Debug, Clone)]
pub enum Number {
    /// Integers that fit in i64. No allocation.
    Int(i64),
    /// Reduced fractions. Invariant: den > 0, gcd(|num|, den) == 1, den != 1.
    Rat(i64, i64, Spelling),
    /// Arbitrary precision fallback. Boxed to keep Number small.
    Big(Box<BigNumber>),
    /// Floating-point value — produced by numerical evaluation only. User
    /// input never parses to `Float` (decimals are exact rationals).
    Float(F64),
    /// Exact **negative zero**. Its whole reason to exist is that `1/(−0)` is
    /// `−∞` while `1/0` is `+∞`, and the sign must survive a product
    /// (`(−1)·0 → −0`). It is *value-equal to `Int(0)`* — it compares equal,
    /// hashes identically, prints and serializes as `0`, and satisfies
    /// [`is_zero`](Number::is_zero) — so every consumer treats it as plain zero
    /// **except** the ones that explicitly ask [`is_neg_zero`](Number::is_neg_zero)
    /// (the division-by-zero pole fold). Positive zero stays `Int(0)`; this
    /// variant is only ever minted by the sign-aware arithmetic below
    /// (`neg`/`mul`/`add`/`sub`/`checked_div`) and by `annihilate`.
    NegZero,
}

#[derive(Debug, Clone)]
pub enum BigNumber {
    Int(BigInt),
    Rat(BigRational, Spelling),
}

/// Value equality: the spelling is deliberately excluded (see [`Number`]).
impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Number::Int(a), Number::Int(b)) => a == b,
            (Number::Rat(a, b, _), Number::Rat(c, d, _)) => a == c && b == d,
            (Number::Float(a), Number::Float(b)) => a == b,
            (Number::Big(a), Number::Big(b)) => a == b,
            // −0 is value-equal to +0 (the only other exact zero is `Int(0)`:
            // `Rat`/`Big` never reduce to zero).
            (Number::NegZero, Number::NegZero) => true,
            (Number::NegZero, Number::Int(0)) | (Number::Int(0), Number::NegZero) => true,
            _ => false,
        }
    }
}
impl Eq for Number {}

impl std::hash::Hash for Number {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // −0 compares equal to `Int(0)`, so it must hash identically.
        if matches!(self, Number::NegZero) {
            return Number::Int(0).hash(state);
        }
        std::mem::discriminant(self).hash(state);
        match self {
            Number::Int(i) => i.hash(state),
            Number::Rat(n, d, _) => (n, d).hash(state),
            Number::Float(f) => f.hash(state),
            Number::Big(b) => b.hash(state),
            Number::NegZero => unreachable!("handled above"),
        }
    }
}

impl PartialEq for BigNumber {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (BigNumber::Int(a), BigNumber::Int(b)) => a == b,
            (BigNumber::Rat(a, _), BigNumber::Rat(b, _)) => a == b,
            _ => false,
        }
    }
}
impl Eq for BigNumber {}

impl std::hash::Hash for BigNumber {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            BigNumber::Int(i) => i.hash(state),
            BigNumber::Rat(r, _) => r.hash(state),
        }
    }
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

    /// Reduced rational from an i64 numerator/denominator, spelled as a
    /// fraction. Enforces the `Rat` invariants (den > 0, gcd == 1, den != 1)
    /// and demotes to `Int` when the denominator reduces to 1. Panics on a zero
    /// denominator.
    pub fn rat(num: i64, den: i64) -> Number {
        Number::rat_spelled(num, den, Spelling::Fraction)
    }

    /// [`rat`](Number::rat) with an explicit [`Spelling`].
    pub fn rat_spelled(mut num: i64, mut den: i64, spelling: Spelling) -> Number {
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
            Number::Rat(num, den, spelling)
        }
    }

    /// How this value should be written back out. Integers and floats have only
    /// one spelling, and answer `Fraction` — the identity for
    /// [`Spelling::join`], so they never drag a sum or product either way.
    pub fn spelling(&self) -> Spelling {
        match self {
            Number::Rat(_, _, s) => *s,
            Number::Big(b) => match &**b {
                BigNumber::Rat(_, s) => *s,
                BigNumber::Int(_) => Spelling::Fraction,
            },
            Number::Int(_) | Number::Float(_) | Number::NegZero => Spelling::Fraction,
        }
    }

    /// This value with its spelling replaced. A no-op on integers and floats.
    pub fn with_spelling(&self, spelling: Spelling) -> Number {
        match self {
            Number::Rat(n, d, _) => Number::Rat(*n, *d, spelling),
            Number::Big(b) => match &**b {
                BigNumber::Rat(r, _) => Number::Big(Box::new(BigNumber::Rat(r.clone(), spelling))),
                BigNumber::Int(_) => self.clone(),
            },
            _ => self.clone(),
        }
    }

    /// Reduce and demote an arbitrary-precision integer to the smallest tier.
    pub fn from_bigint(v: BigInt) -> Number {
        match v.to_i64() {
            Some(i) => Number::Int(i),
            None => Number::Big(Box::new(BigNumber::Int(v))),
        }
    }

    /// Reduce and demote an arbitrary-precision rational, spelled as a
    /// fraction: to `Int` when integral and small, to `Rat` when numerator and
    /// denominator both fit i64, otherwise `Big`. `BigRational` keeps itself in
    /// lowest terms.
    pub fn from_bigrational(v: BigRational) -> Number {
        Number::from_bigrational_spelled(v, Spelling::Fraction)
    }

    /// [`from_bigrational`](Number::from_bigrational) with an explicit
    /// [`Spelling`].
    pub fn from_bigrational_spelled(v: BigRational, spelling: Spelling) -> Number {
        if v.is_integer() {
            return Number::from_bigint(v.to_integer());
        }
        if let (Some(n), Some(d)) = (v.numer().to_i64(), v.denom().to_i64()) {
            // Already reduced and non-integral, so den != 1 and den > 0.
            Number::Rat(n, d, spelling)
        } else {
            Number::Big(Box::new(BigNumber::Rat(v, spelling)))
        }
    }

    /// Round to `d` decimal places, ties away from zero. Negative `d` rounds to
    /// tens / hundreds / … Exact for rational values (so `2.345` → `2.35`, no
    /// float ambiguity); f64-based for `Float`.
    ///
    /// Extreme `d` is resolved *semantically* rather than computed: the scale
    /// 10^|d| is materialized as a `BigInt`, so a hostile `d` (e.g. from the
    /// wasm boundary) must neither allocate gigabytes nor grind debug-mode
    /// bignum arithmetic. Beyond ±4000 decimal places: a very positive `d`
    /// returns the value unchanged (no classroom value has finer structure);
    /// a very negative `d` compares the rounding unit to the value's magnitude
    /// (smaller → 0, larger → unchanged, with tie behaviour at those
    /// astronomical scales deliberately approximate).
    pub fn round_to_decimals(&self, d: i32) -> Number {
        let max_scale = crate::resource_limits::current().max_round_decimals;
        let d64 = i64::from(d);
        if d64 > max_scale {
            return self.clone();
        }
        if d64 < -max_scale {
            return match self.magnitude_log10() {
                // |value| far below the rounding unit → rounds to zero.
                Some(k) if k < -d64 => Number::Int(0),
                // Coarse rounding of an even more astronomical value: leading
                // digits dominate; unchanged is the bounded approximation.
                Some(_) => self.clone(),
                None => self.clone(), // zero / NaN
            };
        }
        // Fast path: an integer value is unchanged by rounding to ≥ 0 decimals.
        let is_int_value = matches!(self, Number::Int(_))
            || matches!(self, Number::Big(b) if matches!(&**b, BigNumber::Int(_)));
        if d >= 0 && is_int_value {
            return self.clone();
        }
        // A `Float` rounds through its *exact* binary value, like every other
        // variant — `BigRational::from_float` is lossless. The obvious
        // `(v * 10^d).round() / 10^d` is not: both 10^d and the product are
        // rounded, so `round_to_decimals(2e21, 2)` — a no-op on a value that
        // is already an integer — came back as `1.9999999999999997e21`,
        // because 2e23 is not representable. Doenet's display path asks for
        // exactly that combination (`displayDigits=3, displayDecimals=2`), so
        // every large number a student saw went through it. Legacy avoided the
        // trap by routing through a decimal string (`parseFloat(toFixed(v, n))`),
        // which is what the exact rational plus [`float_from_scaled`] does here.
        let exact = match self.to_bigrational() {
            Some(r) => r,
            // A `Float` rounds through its **shortest decimal spelling**, not
            // its exact binary expansion — `0.5555` is stored as
            // `0.55549999999999999…`, and rounding that to three places gives
            // `0.555` where the value the author wrote gives `0.556`.
            //
            // This is legacy's behaviour, and the earlier note here (that
            // legacy went through `parseFloat(toFixed(v, n))`) had it wrong:
            // mathjs's `format(v, {notation:"fixed"})` generates digits from the
            // shortest representation, so it gave `0.5555 → 0.556`,
            // `2.675 → 2.68` and `1.005 → 1.01` where `toFixed` gives
            // `0.555`, `2.67` and `1.00`. Measured against mathjs directly.
            //
            // The large-magnitude fix this branch was written for is unaffected:
            // `2e21` spells as its exact integer either way.
            None => {
                let f = self.to_f64();
                if !f.is_finite() {
                    return self.clone(); // NaN/±∞: no rounding can change them
                }
                match Number::from_decimal_str(&format!("{f}")).to_bigrational() {
                    Some(r) => r,
                    None => match BigRational::from_float(f) {
                        Some(r) => r,
                        None => return self.clone(),
                    },
                }
            }
        };
        let pow10 = BigInt::from(10).pow(d.unsigned_abs());
        let scale = if d >= 0 {
            BigRational::from_integer(pow10)
        } else {
            BigRational::new(BigInt::one(), pow10)
        };
        let rounded = (&exact * &scale).round(); // half away from zero
        if self.is_float() {
            // Rounding a small negative value to zero keeps the sign: legacy's
            // `parseFloat((-0.001).toFixed(2))` is `-0`, and the sign is not
            // decoration — `1/(-0)` is `-∞`. The scaled integer is `0` with no
            // sign to carry, so it is read off the value that went in.
            if rounded.is_zero() && exact.is_negative() {
                return Number::NegZero;
            }
            // Stays inexact: rounding a computed value does not make it exact.
            return Number::from_f64(float_from_scaled(&rounded.to_integer(), d));
        }
        // Rounding *to decimal places* produces a decimal, whatever went in:
        // `round_numbers_to_decimals(1/3, 2)` is `0.33`, not `33/100`.
        //
        // But only when rounding actually *changes* the value. `5/2` is `2.5`
        // exactly, so three significant figures leave it untouched — and a
        // no-op must not restyle a fraction as a decimal (`displayDigits`
        // defaults to 3, so every exact rational a student sees passes through
        // here; in a fractions lesson the fraction is the point). The value
        // keeps whatever spelling it already had — `1/2` stays a fraction, a
        // typed `0.5` stays a decimal. Reported by DoenetML (open item 8).
        let result = rounded / scale;
        if result == exact {
            return self.clone();
        }
        Number::from_bigrational_spelled(result, Spelling::Decimal)
    }

    /// `⌊log10 |self|⌋` — the decimal place of the leading significant digit —
    /// or `None` for zero/NaN. Uses f64 when the magnitude is in f64 range;
    /// for `Big` values beyond it (where `to_f64()` is ±∞ or underflows to 0),
    /// falls back to bit lengths (accuracy ±1, which only shifts a
    /// significant-figures boundary by one digit at ≳10³⁰⁸ magnitudes — the
    /// point is a sane finite result, not an unbounded/overflowing one).
    pub fn magnitude_log10(&self) -> Option<i64> {
        if self.is_zero() {
            return None;
        }
        let f = self.to_f64().abs();
        if f.is_finite() && f > 0.0 {
            return Some(f.log10().floor() as i64);
        }
        if f.is_nan() {
            return None;
        }
        // Exact value outside f64 range: approximate from bit lengths.
        let (num_bits, den_bits) = match self {
            Number::Big(b) => match &**b {
                BigNumber::Int(i) => (i.bits() as i64, 0i64),
                BigNumber::Rat(r, _) => (r.numer().bits() as i64, r.denom().bits() as i64),
            },
            // Small variants always fit f64; unreachable in practice.
            _ => return None,
        };
        Some(((num_bits - den_bits) as f64 * std::f64::consts::LOG10_2).floor() as i64)
    }

    pub fn to_f64(&self) -> f64 {
        match self {
            Number::Int(i) => *i as f64,
            Number::Rat(n, d, _) => rat_to_f64(*n, *d),
            Number::Float(f) => f.get(),
            Number::Big(b) => match &**b {
                BigNumber::Int(i) => i.to_f64().unwrap_or(f64::NAN),
                BigNumber::Rat(r, _) => r.to_f64().unwrap_or(f64::NAN),
            },
            Number::NegZero => -0.0,
        }
    }

    /// Whether this is the exact negative zero. The *only* predicate that
    /// distinguishes `−0` from `+0`; every other observation treats them alike.
    pub fn is_neg_zero(&self) -> bool {
        matches!(self, Number::NegZero)
    }

    /// Sign for the purpose of a *zero result*: `true` for negative values and
    /// for `−0`. (`is_negative` is `false` on `−0` — it is zero, not a negative
    /// number — so a dedicated helper is needed for XOR-of-signs logic.)
    fn zero_sign_neg(&self) -> bool {
        self.is_negative() || self.is_neg_zero()
    }

    pub fn is_positive(&self) -> bool {
        match self {
            Number::Int(i) => *i > 0,
            Number::Rat(n, ..) => *n > 0,
            Number::Float(f) => f.get() > 0.0,
            Number::Big(b) => match &**b {
                BigNumber::Int(i) => i.is_positive(),
                BigNumber::Rat(r, _) => r.is_positive(),
            },
            Number::NegZero => false,
        }
    }

    pub fn is_negative(&self) -> bool {
        match self {
            Number::Int(i) => *i < 0,
            Number::Rat(n, ..) => *n < 0,
            Number::Float(f) => f.get() < 0.0,
            Number::Big(b) => match &**b {
                BigNumber::Int(i) => i.is_negative(),
                BigNumber::Rat(r, _) => r.is_negative(),
            },
            // −0 is zero, not a negative number (matches IEEE `-0.0 < 0.0`).
            Number::NegZero => false,
        }
    }

    /// Numerator and denominator as strings, for a non-integral rational
    /// (`Rat` or big rational); `None` for integers, floats, and big
    /// integers. Used by formatters for the `a/b` / `\frac` fallback when the
    /// fraction does not terminate as a decimal.
    pub fn rational_parts(&self) -> Option<(String, String)> {
        match self {
            Number::Rat(n, d, _) => Some((n.to_string(), d.to_string())),
            Number::Big(b) => match &**b {
                BigNumber::Rat(r, _) => Some((r.numer().to_string(), r.denom().to_string())),
                BigNumber::Int(_) => None,
            },
            _ => None,
        }
    }

    /// Negation. `i64::MIN` has no positive counterpart in `i64`, so both
    /// integer tiers widen to `Big` rather than negating in place: a plain
    /// `-i` traps under `overflow-checks` (an *abort*, in this crate) and
    /// wraps back to `i64::MIN` without them, which is a wrong number on a
    /// grading path. `-9223372036854775808(1-x)` and
    /// `2^(-9223372036854775808/1)` are both typeable and both reached it.
    pub fn neg(&self) -> Number {
        match self {
            // Exact zero flips sign: −(+0) = −0, −(−0) = +0.
            Number::Int(0) => Number::NegZero,
            Number::NegZero => Number::Int(0),
            Number::Int(i) => match i.checked_neg() {
                Some(v) => Number::Int(v),
                None => Number::Big(Box::new(BigNumber::Int(-BigInt::from(*i)))),
            },
            Number::Rat(n, d, s) => match n.checked_neg() {
                Some(v) => Number::Rat(v, *d, *s),
                None => Number::from_bigrational_spelled(
                    -BigRational::new(BigInt::from(*n), BigInt::from(*d)),
                    *s,
                ),
            },
            Number::Float(f) => Number::Float(F64::new(-f.get())),
            Number::Big(b) => match &**b {
                BigNumber::Int(i) => Number::from_bigint(-i),
                BigNumber::Rat(r, s) => Number::from_bigrational_spelled(-r, *s),
            },
        }
    }

    pub const fn zero() -> Number {
        Number::Int(0)
    }
    pub const fn one() -> Number {
        Number::Int(1)
    }

    pub fn is_zero(&self) -> bool {
        match self {
            Number::Int(i) => *i == 0,
            Number::Rat(n, ..) => *n == 0,
            Number::Float(f) => f.get() == 0.0,
            Number::Big(b) => match &**b {
                BigNumber::Int(i) => i.is_zero(),
                BigNumber::Rat(r, _) => r.is_zero(),
            },
            Number::NegZero => true,
        }
    }

    /// Exactly the value one, in whatever representation.
    ///
    /// `Float(1.0)` counts, symmetrically with [`is_zero`](Number::is_zero),
    /// which has always accepted `Float(0.0)`. Only `Int(1)` used to, and the
    /// asymmetry was observable: this predicate is what drops the identity
    /// factor in [`mul`](crate::normalize::mul) and the identity exponent in
    /// [`pow`](crate::normalize::pow), so a coefficient that folded to one
    /// *through a float* stayed written down. `fromAst(["*", 0.5, 2, "x"])`
    /// simplified to `1·x` while `fromText("0.5*2*x")` gave `x` — the same
    /// mathematics, and a structural-equality answer that depended on which
    /// door the expression came in through.
    ///
    /// (Only `Float` can hold a non-`Int` one: `Rat` maintains `den != 1` and
    /// `Big` demotes to `Int` when it fits, so neither can represent it.)
    pub fn is_one(&self) -> bool {
        match self {
            Number::Int(1) => true,
            Number::Float(f) => f.get() == 1.0,
            _ => false,
        }
    }

    pub fn abs(&self) -> Number {
        if self.is_neg_zero() {
            Number::Int(0) // |−0| = +0
        } else if self.is_negative() {
            self.neg()
        } else {
            self.clone()
        }
    }

    /// A finite exact rational as a `BigRational`; `None` for `Float`. The
    /// common currency for exact arithmetic across the tiers.
    pub(crate) fn to_bigrational(&self) -> Option<BigRational> {
        match self {
            Number::Int(i) => Some(BigRational::from_integer(BigInt::from(*i))),
            Number::Rat(n, d, _) => Some(BigRational::new(BigInt::from(*n), BigInt::from(*d))),
            Number::Big(b) => Some(match &**b {
                BigNumber::Int(i) => BigRational::from_integer(i.clone()),
                BigNumber::Rat(r, _) => r.clone(),
            }),
            Number::NegZero => Some(BigRational::zero()),
            Number::Float(_) => None,
        }
    }

    /// The value as a small `(numerator, denominator)` pair, or `None` when it
    /// is not one — for consumers that can only work in exact small-integer
    /// arithmetic, such as equality's finite-field stage.
    ///
    /// `max_den` is the honesty bound, and it is doing real work: a decimal
    /// whose reduced denominator is enormous is usually an *approximation of
    /// something else* (`3.141592653589793` is π, not 3141592653589793/10¹⁵),
    /// and a consumer that took it at face value would separate two spellings
    /// of the same quantity. Small fractions stay exact, which is what lets the
    /// field reject `0.33 ≠ 1/3`.
    ///
    /// A `Float` is read through its **shortest decimal spelling**, so `1e-7`
    /// answers `(1, 10_000_000)` — the same as the literal `0.0000001` — while
    /// `0.30000000000000004` needs 17 digits, blows `max_den`, and answers
    /// `None`. That is the intended split: a float that spells as a simple
    /// fraction *is* that fraction, and one that does not is round-off, which
    /// must stay comparable only within tolerance.
    pub(crate) fn simple_rational(&self, max_den: u64) -> Option<(i64, i64)> {
        match self {
            Number::Int(v) => Some((*v, 1)),
            Number::NegZero => Some((0, 1)),
            Number::Rat(num, den, _) => (den.unsigned_abs() <= max_den).then_some((*num, *den)),
            // Big integers and high-precision decimals are past the bound by
            // construction.
            Number::Big(_) => None,
            Number::Float(f) => {
                let v = f.get();
                if !v.is_finite() {
                    return None;
                }
                if v == 0.0 {
                    return Some((0, 1));
                }
                let (digits, n) = super::shortest_digits(v.abs());
                let sign = if v < 0.0 { -1i64 } else { 1 };
                let mantissa: i64 = digits.parse().ok()?;
                let scale = n - digits.len() as i64;
                if scale >= 0 {
                    // An integral value: 10^scale must still fit.
                    let factor = 10i64.checked_pow(u32::try_from(scale).ok()?)?;
                    Some((sign * mantissa.checked_mul(factor)?, 1))
                } else {
                    let den = 10i64.checked_pow(u32::try_from(-scale).ok()?)?;
                    (den.unsigned_abs() <= max_den).then_some((sign * mantissa, den))
                }
            }
        }
    }

    fn is_float(&self) -> bool {
        matches!(self, Number::Float(_))
    }

    /// Whether this value is the *result of* inexact arithmetic rather than an
    /// exact quantity. Only [`Number::Float`] is — every other variant carries
    /// its value exactly, including decimals, which parse to rationals. Read by
    /// consumers that must not treat f64 low digits as meaningful, such as
    /// equality's bare-number stage.
    pub fn is_inexact(&self) -> bool {
        self.is_float()
    }

    /// Exact binary op on two exact operands, or f64 arithmetic if either is a
    /// `Float` (float-ness is contagious — a `Float` operand marks an inexact
    /// evaluation result). The `Int op Int` fast path stays allocation-free.
    fn binop(
        &self,
        other: &Number,
        int_checked: impl Fn(i64, i64) -> Option<i64>,
        exact: impl Fn(BigRational, BigRational) -> BigRational,
        float: impl Fn(f64, f64) -> f64,
    ) -> Number {
        if let (Number::Int(a), Number::Int(b)) = (self, other) {
            if let Some(v) = int_checked(*a, *b) {
                return Number::Int(v);
            }
        }
        if self.is_float() || other.is_float() {
            return Number::Float(F64::new(float(self.to_f64(), other.to_f64())));
        }
        Number::from_bigrational_spelled(
            exact(
                self.to_bigrational().unwrap(),
                other.to_bigrational().unwrap(),
            ),
            // Decimal is contagious, so `0.5 + 1/4` reads back as `0.75` while
            // `1/2 + 1/4` reads back as `3/4`. See `Spelling`.
            self.spelling().join(other.spelling()),
        )
    }

    pub fn add(&self, other: &Number) -> Number {
        // −0 + −0 = −0 (IEEE). Every other zero-sum is +0, which the exact fold
        // already yields (`−0` reads as `0` through `to_bigrational`).
        if self.is_neg_zero() && other.is_neg_zero() {
            return Number::NegZero;
        }
        self.binop(other, i64::checked_add, |a, b| a + b, |a, b| a + b)
    }
    pub fn sub(&self, other: &Number) -> Number {
        // a − b with both exact zeros is −0 only for `−0 − (+0)`.
        if self.is_zero() && other.is_zero() && !self.is_float() && !other.is_float() {
            return if self.zero_sign_neg() && !other.zero_sign_neg() {
                Number::NegZero
            } else {
                Number::Int(0)
            };
        }
        self.binop(other, i64::checked_sub, |a, b| a - b, |a, b| a - b)
    }
    pub fn mul(&self, other: &Number) -> Number {
        // Exact signed zero: the sign of a zero product is the XOR of the
        // operand signs, which the plain (bigrational) fold discards —
        // `(−3)·0` is −0, not +0. Float operands keep IEEE's own signed zero,
        // so only the all-exact case is intercepted here.
        if !self.is_float() && !other.is_float() && (self.is_zero() || other.is_zero()) {
            return if self.zero_sign_neg() ^ other.zero_sign_neg() {
                Number::NegZero
            } else {
                Number::Int(0)
            };
        }
        self.binop(other, i64::checked_mul, |a, b| a * b, |a, b| a * b)
    }

    /// Division, or `None` when dividing by (exact) zero — the caller leaves
    /// the expression unfolded rather than fabricating an infinity. Float ÷ 0.0
    /// follows IEEE (±∞/NaN), matching JS.
    pub fn checked_div(&self, other: &Number) -> Option<Number> {
        if other.is_zero() && !self.is_float() && !other.is_float() {
            return None;
        }
        // Exact `0 / nonzero` carries a sign: `0/(−5)` is −0. (`other` is
        // nonzero here — a zero divisor returned `None` above.) Float division
        // follows IEEE through `binop`.
        if self.is_zero() && !self.is_float() && !other.is_float() {
            return Some(if self.zero_sign_neg() ^ other.zero_sign_neg() {
                Number::NegZero
            } else {
                Number::Int(0)
            });
        }
        Some(self.binop(
            other,
            |_, _| None, // never take the i64 path: division is not closed on i64
            |a, b| a / b,
            |a, b| a / b,
        ))
    }

    /// Raise to an integer power. `None` for `0` to a negative power (an
    /// exact division by zero), and for exponents so large the exact result
    /// would be astronomically big — the caller leaves the node unfolded
    /// either way. `0^0 == 1`, matching JS `Math.pow`.
    pub fn checked_pow_int(&self, exp: i64) -> Option<Number> {
        if let Number::Float(f) = self {
            // powi takes i32; saturate rather than wrap for absurd exponents.
            let e = exp.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
            return Some(Number::Float(F64::new(f.get().powi(e))));
        }
        if exp == 0 {
            return Some(Number::one());
        }
        if self.is_zero() {
            return if exp < 0 {
                // 0^negative: a pole. Left `None` so the caller keeps the
                // `Pow(0, negative)` node for the ∞/NaN fold, which reads the
                // sign of `−0` to choose `+∞` vs `−∞`.
                None
            } else if self.is_neg_zero() && exp % 2 != 0 {
                Some(Number::NegZero) // (−0)^odd = −0
            } else {
                Some(Number::zero())
            };
        }
        let base = self.to_bigrational().unwrap();
        // Refuse exact results beyond ~10^6 bits (canonicalization must stay
        // cheap on any input; `2^(10^12)` is not a number to materialize).
        // |±1| is exempt: its powers stay one digit.
        let base_bits = base.numer().bits().max(base.denom().bits());
        if base_bits > 1
            && exp.unsigned_abs().saturating_mul(base_bits)
                > crate::resource_limits::current().max_pow_bits
        {
            return None;
        }
        let mag = bigrat_powu(base, exp.unsigned_abs());
        let result = if exp < 0 { mag.recip() } else { mag };
        // A power of a decimal is a decimal (`0.5^2` is `0.25`); a power of an
        // integer or a fraction is a fraction (`2^(-2)` is `1/4`). That second
        // case is the one origin of a fraction spelling that is not a literal
        // `a/b`: canonicalization turns every division into a negative power.
        Some(Number::from_bigrational_spelled(result, self.spelling()))
    }
}

/// The nearest f64 to `m × 10^(−d)`, where `m` is an exact scaled integer.
///
/// Via the decimal spelling rather than arithmetic: `m as f64 / 10^d` would
/// round twice and re-introduce the error the exact rounding just removed,
/// whereas Rust's float parser is correctly rounded, so the one rounding it
/// performs is the only one. (This is what legacy's `parseFloat(toFixed(…))`
/// was doing.) `m` can be thousands of digits — the exponent form keeps the
/// string proportional to `m`, not to `d`.
fn float_from_scaled(m: &BigInt, d: i32) -> f64 {
    format!("{m}e{}", -i64::from(d)).parse().unwrap_or(f64::NAN)
}

/// The nearest f64 to the exact ratio `n/d`.
///
/// `n as f64 / d as f64` rounds *twice* once a part exceeds 2^53 — once
/// converting the part, once dividing — and the two roundings compound into a
/// result that is not the nearest f64 to `n/d`. A user-typed
/// `35203423.02352343201` is `Rat(3520342302352343201, 10^11)`, whose numerator
/// is past 2^53, and the naive division landed one ulp low: `.tree` carried
/// `35203423.02352343` where JS reading the same literal gives
/// `35203423.023523435`.
///
/// Within 2^53 both conversions are exact, so the single division is correctly
/// rounded — that is the overwhelmingly common case and stays on the cheap
/// path. Beyond it, `BigRational` converts correctly. (`Rat`'s normal form
/// keeps `d > 0`; the zero guard is only so a violated invariant degrades to
/// an IEEE infinity rather than panicking inside wasm.)
fn rat_to_f64(n: i64, d: i64) -> f64 {
    const EXACT: u64 = 1 << 53;
    if d == 0 || (n.unsigned_abs() <= EXACT && d.unsigned_abs() <= EXACT) {
        return n as f64 / d as f64;
    }
    BigRational::new(BigInt::from(n), BigInt::from(d))
        .to_f64()
        .unwrap_or(f64::NAN)
}

/// `base^n` by exponentiation-by-squaring (n unsigned; caller handles sign).
fn bigrat_powu(base: BigRational, mut n: u64) -> BigRational {
    let mut result = BigRational::one();
    let mut b = base;
    while n > 0 {
        if n & 1 == 1 {
            result *= &b;
        }
        n >>= 1;
        if n > 0 {
            b = &b * &b;
        }
    }
    result
}
