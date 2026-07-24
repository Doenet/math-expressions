//! The six hyperbolic functions.

use super::{apply, FnDef, DEFAULTS};
use crate::precise::kernels::{FixId, FnKernel};

pub const SINH: FnDef = FnDef {
    name: "sinh",
    parse_text: &["sinh"],
    parse_latex: &["sinh"],
    inverse: Some("asinh"),
    move_exponent_spellings: &["sinh"],
    derivative: Some("cosh(x)"),
    antiderivative: Some(|u| apply("cosh", u)),
    eval1: Some(|z| Some(z.sinh())),
    latex_commands: &[("sinh", "sinh")],
    kernel: Some(&SINH_KERNEL),
    ..DEFAULTS
};

pub(crate) const SINH_KERNEL: FnKernel = FnKernel {
    f: f64::sinh,
    df: f64::cosh,
    domain: |_| true,
    fix: Some(FixId::Sinh),
    cf: |z| z.sinh(),
    cdfm: |z| z.cosh().norm(),
};

pub const COSH: FnDef = FnDef {
    name: "cosh",
    parse_text: &["cosh"],
    parse_latex: &["cosh"],
    inverse: Some("acosh"),
    move_exponent_spellings: &["cosh"],
    derivative: Some("sinh(x)"),
    antiderivative: Some(|u| apply("sinh", u)),
    eval1: Some(|z| Some(z.cosh())),
    latex_commands: &[("cosh", "cosh")],
    kernel: Some(&COSH_KERNEL),
    ..DEFAULTS
};

pub(crate) const COSH_KERNEL: FnKernel = FnKernel {
    f: f64::cosh,
    df: f64::sinh,
    domain: |_| true,
    fix: Some(FixId::Cosh),
    cf: |z| z.cosh(),
    cdfm: |z| z.sinh().norm(),
};

pub const TANH: FnDef = FnDef {
    name: "tanh",
    parse_text: &["tanh"],
    parse_latex: &["tanh"],
    inverse: Some("atanh"),
    move_exponent_spellings: &["tanh"],
    derivative: Some("sech(x)^2"),
    antiderivative: Some(|u| apply("log", apply("cosh", u))),
    eval1: Some(|z| Some(z.tanh())),
    latex_commands: &[("tanh", "tanh")],
    kernel: Some(&TANH_KERNEL),
    ..DEFAULTS
};

pub(crate) const TANH_KERNEL: FnKernel = FnKernel {
    f: f64::tanh,
    df: |x| {
        let c = x.cosh();
        1.0 / (c * c)
    },
    domain: |_| true,
    fix: Some(FixId::Tanh),
    cf: |z| z.tanh(),
    cdfm: |z| {
        let c = z.cosh().norm();
        1.0 / (c * c).max(f64::MIN_POSITIVE)
    },
};

pub const SECH: FnDef = FnDef {
    name: "sech",
    parse_text: &["sech"],
    parse_latex: &["sech"],
    inverse: Some("asech"),
    move_exponent_spellings: &["sech"],
    derivative: Some("-sech(x)*tanh(x)"),
    eval1: Some(|z| Some(z.cosh().inv())),
    latex_commands: &[("sech", "sech")],
    ..DEFAULTS
};

pub const CSCH: FnDef = FnDef {
    name: "csch",
    parse_text: &["csch"],
    parse_latex: &["csch"],
    inverse: Some("acsch"),
    move_exponent_spellings: &["csch"],
    derivative: Some("-csch(x)*coth(x)"),
    eval1: Some(|z| Some(z.sinh().inv())),
    latex_commands: &[("csch", "csch")],
    ..DEFAULTS
};

pub const COTH: FnDef = FnDef {
    name: "coth",
    parse_text: &["coth"],
    parse_latex: &["coth"],
    inverse: Some("acoth"),
    move_exponent_spellings: &["coth"],
    derivative: Some("-csch(x)^2"),
    eval1: Some(|z| Some(z.tanh().inv())),
    latex_commands: &[("coth", "coth")],
    ..DEFAULTS
};
