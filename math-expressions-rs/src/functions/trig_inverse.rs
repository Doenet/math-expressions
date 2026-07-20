//! Inverse trigonometric functions (canonical `a…` spellings; the `arc…`
//! spellings are aliases), plus the two-argument `atan2`.

use super::{apply, int, FnDef, DEFAULTS};
use crate::precise::kernels::{FixId, FnKernel};
use crate::norm::{add, mul, pow};
use crate::num::Number;
use num_complex::Complex64;

pub const ASIN: FnDef = FnDef {
    name: "asin",
    aliases: &["arcsin"],
    parse_text: &["asin", "arcsin"],
    parse_latex: &["asin", "arcsin"],
    derivative: Some("1/sqrt(1 - x^2)"),
    antiderivative: Some(|u| {
        add(vec![
            mul(vec![u.clone(), apply("asin", u.clone())]),
            apply("sqrt", add(vec![int(1), mul(vec![int(-1), pow(u, int(2))])])),
        ])
    }),
    eval1: Some(|z| Some(z.asin())),
    latex_commands: &[("asin", "arcsin"), ("arcsin", "arcsin")],
    kernel: Some(&ASIN_KERNEL),
    ..DEFAULTS
};

pub(crate) const ASIN_KERNEL: FnKernel = FnKernel {
    f: f64::asin,
    df: |x| 1.0 / (1.0 - x * x).sqrt(),
    domain: |x| (-1.0..=1.0).contains(&x),
    fix: Some(FixId::Asin),
    cf: |z| z.asin(),
    cdfm: |z| {
        1.0 / (Complex64::new(1.0, 0.0) - z * z)
            .sqrt()
            .norm()
            .max(f64::MIN_POSITIVE)
    },
};

pub const ACOS: FnDef = FnDef {
    name: "acos",
    aliases: &["arccos"],
    parse_text: &["acos", "arccos"],
    parse_latex: &["acos", "arccos"],
    derivative: Some("-1/sqrt(1 - x^2)"),
    antiderivative: Some(|u| {
        add(vec![
            mul(vec![u.clone(), apply("acos", u.clone())]),
            mul(vec![
                int(-1),
                apply("sqrt", add(vec![int(1), mul(vec![int(-1), pow(u, int(2))])])),
            ]),
        ])
    }),
    eval1: Some(|z| Some(z.acos())),
    latex_commands: &[("acos", "arccos"), ("arccos", "arccos")],
    kernel: Some(&ACOS_KERNEL),
    ..DEFAULTS
};

pub(crate) const ACOS_KERNEL: FnKernel = FnKernel {
    f: f64::acos,
    df: |x| -1.0 / (1.0 - x * x).sqrt(),
    domain: |x| (-1.0..=1.0).contains(&x),
    fix: Some(FixId::Acos),
    cf: |z| z.acos(),
    cdfm: |z| {
        1.0 / (Complex64::new(1.0, 0.0) - z * z)
            .sqrt()
            .norm()
            .max(f64::MIN_POSITIVE)
    },
};

pub const ATAN: FnDef = FnDef {
    name: "atan",
    aliases: &["arctan"],
    parse_text: &["atan", "arctan"],
    parse_latex: &["atan", "arctan"],
    derivative: Some("1/(x^2 + 1)"),
    antiderivative: Some(|u| {
        add(vec![
            mul(vec![u.clone(), apply("atan", u.clone())]),
            mul(vec![
                crate::expr::Expr::Num(Number::rat(-1, 2)),
                apply("ln", add(vec![int(1), pow(u, int(2))])),
            ]),
        ])
    }),
    eval1: Some(|z| Some(z.atan())),
    latex_commands: &[("atan", "arctan"), ("arctan", "arctan")],
    kernel: Some(&ATAN_KERNEL),
    ..DEFAULTS
};

pub(crate) const ATAN_KERNEL: FnKernel = FnKernel {
    f: f64::atan,
    df: |x| 1.0 / (1.0 + x * x),
    domain: |_| true,
    fix: Some(FixId::Atan),
    cf: |z| z.atan(),
    cdfm: |z| 1.0 / (Complex64::new(1.0, 0.0) + z * z).norm().max(f64::MIN_POSITIVE),
};

pub const ASEC: FnDef = FnDef {
    name: "asec",
    aliases: &["arcsec"],
    parse_text: &["asec", "arcsec"],
    parse_latex: &["asec", "arcsec"],
    derivative: Some("(1/sqrt(x^2 - 1))/abs(x)"),
    eval1: Some(|z| Some(z.inv().acos())),
    latex_commands: &[("asec", "arcsec"), ("arcsec", "arcsec")],
    ..DEFAULTS
};

pub const ACSC: FnDef = FnDef {
    name: "acsc",
    aliases: &["arccsc"],
    parse_text: &["acsc", "arccsc"],
    parse_latex: &["acsc", "arccsc"],
    derivative: Some("-(1/sqrt(x^2 - 1))/abs(x)"),
    eval1: Some(|z| Some(z.inv().asin())),
    latex_commands: &[("acsc", "arccsc"), ("arccsc", "arccsc")],
    ..DEFAULTS
};

pub const ACOT: FnDef = FnDef {
    name: "acot",
    aliases: &["arccot"],
    parse_text: &["acot", "arccot"],
    parse_latex: &["acot", "arccot"],
    derivative: Some("-1/(x^2 + 1)"),
    eval1: Some(|z| Some(z.inv().atan())),
    latex_commands: &[("acot", "arccot"), ("arccot", "arccot")],
    ..DEFAULTS
};

pub const ATAN2: FnDef = FnDef {
    name: "atan2",
    parse_text: &["atan2"],
    parse_latex: &["atan2"],
    eval2: Some(|a, b| Some(Complex64::new(a.re.atan2(b.re), 0.0))),
    ..DEFAULTS
};
