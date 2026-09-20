//! Root/magnitude functions. `sqrt`/`cbrt`/`nthroot` normalize to explicit
//! powers during canonicalization; they exist here for parsing and for the
//! derivative table (which runs on the faithful layer). `cbrt`/`nthroot`
//! are text-only: LaTeX `\sqrt[n]{…}` is grammar, not an applied symbol.

use super::{FnDef, DEFAULTS};
use crate::eval_numeric::certified_digits::kernels::{FixId, FnKernel};
use crate::expr::Expr;
use crate::normalize::{mul, pow};
use crate::num::Number;
use num_bigint::BigInt;
use num_complex::Complex64;
use num_rational::BigRational;
use num_traits::Signed;

pub const SQRT: FnDef = FnDef {
    name: "sqrt",
    parse_text: &["sqrt"],
    parse_latex: &["sqrt"],
    derivative: Some("1/(2*sqrt(x))"),
    antiderivative: Some(|u| {
        mul(vec![
            Expr::Num(Number::rat(2, 3)),
            pow(u, Expr::Num(Number::rat(3, 2))),
        ])
    }),
    eval1: Some(|z| Some(z.sqrt())),
    latex_commands: &[("sqrt", "sqrt")],
    kernel: Some(&SQRT_KERNEL),
    ..DEFAULTS
};

pub(crate) const SQRT_KERNEL: FnKernel = FnKernel {
    f: f64::sqrt,
    df: |x| 0.5 / x.sqrt(),
    domain: |x| x >= 0.0,
    fix: Some(FixId::Sqrt),
    cf: |z| z.sqrt(),
    cdfm: |z| 0.5 / z.sqrt().norm().max(f64::MIN_POSITIVE),
};

// `cbrt`/`nthroot` evaluate a real argument on the **real** branch — an odd
// root of a negative real is the real root (`cbrt(-8)` is `-2`, not the
// principal `1 + i√3`) — matching the branch `simplify`'s radical cluster
// commits to and the `Pow(negative real, 1/odd)` rule in
// `eval_numeric::complex`. For constant arguments `simplify` usually folds
// first and masks these, but they still decide *sampling* (`equals` on
// `cbrt(x)` where `x` takes negative values), so leaving them principal while
// `x^(1/n)` reads real would split the two spellings of the same root.
// Arguments off the real axis stay principal.

pub const CBRT: FnDef = FnDef {
    name: "cbrt",
    parse_text: &["cbrt"],
    derivative: Some("1/(3*cbrt(x)^2)"),
    eval1: Some(|z| {
        Some(if z.im == 0.0 {
            Complex64::new(z.re.cbrt(), 0.0)
        } else {
            z.powf(1.0 / 3.0)
        })
    }),
    ..DEFAULTS
};

pub const NTHROOT: FnDef = FnDef {
    name: "nthroot",
    parse_text: &["nthroot"],
    eval2: Some(|a, b| {
        // Odd integer degree of a negative real: the real root, signed. Any
        // other shape — even or non-integer degree, base off the real axis —
        // is the principal value, as before.
        if a.im == 0.0
            && a.re < 0.0
            && b.im == 0.0
            && b.re.fract() == 0.0
            && b.re.abs() <= i32::MAX as f64
            && (b.re as i64) % 2 != 0
        {
            return Some(Complex64::new(-(-a.re).powf(b.re.recip()), 0.0));
        }
        Some(a.powc(b.inv()))
    }),
    ..DEFAULTS
};

pub const ABS: FnDef = FnDef {
    name: "abs",
    parse_text: &["abs"],
    parse_latex: &["abs"],
    derivative: Some("abs(x)/x"),
    eval1: Some(|z| Some(Complex64::new(z.norm(), 0.0))),
    fold_exact: Some(|xs| match xs {
        [v] => Some(v.abs()),
        _ => None,
    }),
    latex_commands: &[("abs", "abs")],
    kernel: Some(&ABS_KERNEL),
    ..DEFAULTS
};

pub(crate) const ABS_KERNEL: FnKernel = FnKernel {
    f: f64::abs,
    df: |x| x.signum(),
    domain: |_| true,
    fix: Some(FixId::Abs),
    cf: |z| Complex64::new(z.norm(), 0.0),
    cdfm: |_| 1.0,
};

pub const SIGN: FnDef = FnDef {
    name: "sign",
    parse_text: &["sign"],
    parse_latex: &["sign"],
    eval1: Some(|z| {
        Some(if z.norm() == 0.0 {
            Complex64::ZERO
        } else {
            z / z.norm()
        })
    }),
    fold_exact: Some(|xs| match xs {
        [v] => Some(BigRational::from(BigInt::from(match v.numer().sign() {
            num_bigint::Sign::Minus => -1,
            num_bigint::Sign::NoSign => 0,
            num_bigint::Sign::Plus => 1,
        }))),
        _ => None,
    }),
    latex_commands: &[("sign", "sign")],
    ..DEFAULTS
};
