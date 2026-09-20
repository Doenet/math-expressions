//! Exponential and logarithms. Canonically the natural log is `log` (the JS
//! convention); `ln` is an alias — but `ln` appears in
//! `move_exponent_spellings` because `canon_apply` moves exponents *before*
//! renaming, so `ln^2(x)` must match under its original spelling.

use super::{apply, int, FnDef, DEFAULTS};
use crate::eval_numeric::certified_digits::kernels::{FixId, FnKernel};
use crate::normalize::{add, mul, pow};
use num_bigint::BigInt;
use num_complex::Complex64;
use num_rational::BigRational;
use num_traits::{One, Zero};

pub const EXP: FnDef = FnDef {
    name: "exp",
    parse_text: &["exp"],
    parse_latex: &["exp"],
    derivative: Some("exp(x)"),
    antiderivative: Some(|u| apply("exp", u)),
    eval1: Some(|z| Some(z.exp())),
    latex_commands: &[("exp", "exp")],
    kernel: Some(&EXP_KERNEL),
    ..DEFAULTS
};

pub(crate) const EXP_KERNEL: FnKernel = FnKernel {
    f: f64::exp,
    df: f64::exp,
    domain: |_| true,
    fix: Some(FixId::Exp),
    cf: |z| z.exp(),
    cdfm: |z| z.exp().norm(),
};

pub const LOG: FnDef = FnDef {
    name: "log",
    aliases: &["ln"],
    parse_text: &["log", "ln"],
    parse_latex: &["log", "ln"],
    move_exponent_spellings: &["log", "ln"],
    derivative: Some("1/x"),
    antiderivative: Some(|u| {
        add(vec![
            mul(vec![u.clone(), apply("log", u.clone())]),
            mul(vec![int(-1), u]),
        ])
    }),
    eval1: Some(|z| Some(z.ln())),
    latex_commands: &[("log", "log"), ("ln", "ln")],
    kernel: Some(&LN_KERNEL),
    ..DEFAULTS
};

pub(crate) const LN_KERNEL: FnKernel = FnKernel {
    f: f64::ln,
    df: |x| 1.0 / x,
    domain: |x| x > 0.0,
    fix: Some(FixId::Ln),
    cf: |z| z.ln(),
    cdfm: |z| 1.0 / z.norm().max(f64::MIN_POSITIVE),
};

pub const LOG10: FnDef = FnDef {
    name: "log10",
    parse_text: &["log10"],
    parse_latex: &["log10"],
    // No derivative template: matches the historical table, where log10 fell
    // back to prime notation (mathjs parity).
    antiderivative: Some(|u| {
        mul(vec![
            add(vec![
                mul(vec![u.clone(), apply("log", u.clone())]),
                mul(vec![int(-1), u]),
            ]),
            pow(apply("log", int(10)), int(-1)),
        ])
    }),
    eval1: Some(|z| Some(real_log(z, f64::log10, Complex64::log10))),
    fold_exact: Some(|xs| match xs {
        [v] => exact_log(v, &BigRational::from(BigInt::from(10))),
        _ => None,
    }),
    latex_commands: &[("log10", "log10")],
    latex_head: Some("\\log_{10}"),
    kernel: Some(&LOG10_KERNEL),
    ..DEFAULTS
};

pub(crate) const LOG10_KERNEL: FnKernel = FnKernel {
    f: f64::log10,
    df: |x| 1.0 / (x * std::f64::consts::LN_10),
    domain: |x| x > 0.0,
    fix: Some(FixId::Log10),
    cf: |z| z.ln() / std::f64::consts::LN_10,
    cdfm: |z| 1.0 / (z.norm() * std::f64::consts::LN_10).max(f64::MIN_POSITIVE),
};

pub const LOG2: FnDef = FnDef {
    name: "log2",
    // No parser spelling, matching legacy: `log2` is absent from both parsers'
    // applied-function defaults there too, so `"log2(8)"` reads as `log2 · 8`
    // in either library. It is reachable from a directly built tree, from a
    // caller-supplied `appliedFunctionSymbols`, and from `log_2(8)`.
    eval1: Some(|z| Some(real_log(z, f64::log2, Complex64::log2))),
    fold_exact: Some(|xs| match xs {
        [v] => exact_log(v, &BigRational::from(BigInt::from(2))),
        _ => None,
    }),
    ..DEFAULTS
};

/// A real positive argument goes through the dedicated `f64` routine rather
/// than the complex `ln z / ln b`: libm's `log10`/`log2` are exact on powers
/// of their base, so `log10(1000)` reads back as `3` and not
/// `2.9999999999999996`. `simplify` folds that case exactly regardless (see
/// `exact_log`); this keeps the *numeric* path honest too, which is what
/// `evaluate_to_constant` reports.
fn real_log(z: Complex64, real: fn(f64) -> f64, complex: fn(Complex64) -> Complex64) -> Complex64 {
    if z.im == 0.0 && z.re > 0.0 {
        return Complex64::new(real(z.re), 0.0);
    }
    complex(z)
}

/// The exact logarithm of `value` in `base` — `log₁₀(1000) = 3`,
/// `log₂(1/8) = −3` — or `None` when `value` is not an exact integer power of
/// `base`, which leaves the application symbolic rather than emitting a float.
///
/// Decided by repeated exact division, never as `ln value / ln base`. That
/// quotient is what makes the legacy library answer `2.9999999999999996` for
/// `log(1000, 10)`; it gets `log10(1000) = 3` only because V8's dedicated
/// `Math.log10` happens to be exact on powers of ten.
pub(crate) fn exact_log(value: &BigRational, base: &BigRational) -> Option<BigRational> {
    if !base.is_integer() || base <= &BigRational::one() || value <= &BigRational::zero() {
        return None;
    }
    // A power of an integer base is either an integer (non-negative exponent)
    // or the reciprocal of one; anything else — `2/3`, `12/5` — cannot be one.
    if value.is_integer() {
        return integer_log(value.numer(), base.numer())
            .map(|k| BigRational::from(BigInt::from(k)));
    }
    if value.numer().is_one() {
        return integer_log(value.denom(), base.numer())
            .map(|k| BigRational::from(BigInt::from(-k)));
    }
    None
}

/// `k` such that `base^k == value`, for positive integers with `base ≥ 2`.
///
/// Binary search on `k`, not repeated division: stripping one factor per
/// iteration costs `k` big divisions on operands that stay nearly full size, so
/// `log₂(2^200000)` — one short expression — took seconds. `base^k` has more
/// than `k · (bits(base) − 1)` bits, so `k ≤ bits(value) / (bits(base) − 1)`
/// bounds the search, and that bound also keeps every `pow` inside it to about
/// the size of `value` itself. `O(log k)` bignum powers rather than `O(k)`
/// divisions.
fn integer_log(value: &BigInt, base: &BigInt) -> Option<i64> {
    let base_bits = base.bits();
    if base_bits < 2 {
        return None; // `base ≥ 2`; the caller guarantees it, but the bound below needs it
    }
    let (mut lo, mut hi) = (0u32, (value.bits() / (base_bits - 1)) as u32);
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2); // upper mid: `lo` only ever grows
        if &base.pow(mid) <= value {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    (base.pow(lo) == *value).then_some(lo as i64)
}
