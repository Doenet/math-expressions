//! The six trigonometric functions.

use super::{apply, int, FnDef, DEFAULTS};
use crate::precise::kernels::{FixId, FnKernel};
use crate::norm::mul;

pub const SIN: FnDef = FnDef {
    name: "sin",
    parse_text: &["sin"],
    parse_latex: &["sin"],
    inverse: Some("asin"),
    move_exponent_spellings: &["sin"],
    derivative: Some("cos(x)"),
    antiderivative: Some(|u| mul(vec![int(-1), apply("cos", u)])),
    eval1: Some(|z| Some(z.sin())),
    latex_commands: &[("sin", "sin")],
    kernel: Some(&SIN_KERNEL),
    ..DEFAULTS
};

pub(crate) const SIN_KERNEL: FnKernel = FnKernel {
    f: f64::sin,
    df: f64::cos,
    domain: |_| true,
    fix: Some(FixId::Sin),
    cf: |z| z.sin(),
    cdfm: |z| z.cos().norm(),
};

pub const COS: FnDef = FnDef {
    name: "cos",
    parse_text: &["cos"],
    parse_latex: &["cos"],
    inverse: Some("acos"),
    move_exponent_spellings: &["cos"],
    derivative: Some("-sin(x)"),
    antiderivative: Some(|u| apply("sin", u)),
    eval1: Some(|z| Some(z.cos())),
    latex_commands: &[("cos", "cos")],
    kernel: Some(&COS_KERNEL),
    ..DEFAULTS
};

pub(crate) const COS_KERNEL: FnKernel = FnKernel {
    f: f64::cos,
    df: |x| -x.sin(),
    domain: |_| true,
    fix: Some(FixId::Cos),
    cf: |z| z.cos(),
    cdfm: |z| z.sin().norm(),
};

pub const TAN: FnDef = FnDef {
    name: "tan",
    parse_text: &["tan"],
    parse_latex: &["tan"],
    inverse: Some("atan"),
    move_exponent_spellings: &["tan"],
    derivative: Some("sec(x)^2"),
    antiderivative: Some(|u| mul(vec![int(-1), apply("ln", apply("cos", u))])),
    eval1: Some(|z| Some(z.tan())),
    latex_commands: &[("tan", "tan")],
    kernel: Some(&TAN_KERNEL),
    ..DEFAULTS
};

pub(crate) const TAN_KERNEL: FnKernel = FnKernel {
    f: f64::tan,
    df: |x| {
        let c = x.cos();
        1.0 / (c * c)
    },
    domain: |_| true,
    fix: Some(FixId::Tan),
    cf: |z| z.tan(),
    cdfm: |z| {
        let c = z.cos().norm();
        1.0 / (c * c).max(f64::MIN_POSITIVE)
    },
};

pub const SEC: FnDef = FnDef {
    name: "sec",
    parse_text: &["sec"],
    parse_latex: &["sec"],
    inverse: Some("asec"),
    move_exponent_spellings: &["sec"],
    derivative: Some("sec(x)*tan(x)"),
    eval1: Some(|z| Some(z.cos().inv())),
    latex_commands: &[("sec", "sec")],
    ..DEFAULTS
};

pub const CSC: FnDef = FnDef {
    name: "csc",
    aliases: &["cosec"],
    parse_text: &["csc"],
    parse_latex: &["csc"],
    inverse: Some("acsc"),
    move_exponent_spellings: &["csc"],
    derivative: Some("-csc(x)*cot(x)"),
    eval1: Some(|z| Some(z.sin().inv())),
    latex_commands: &[("csc", "csc")],
    ..DEFAULTS
};

pub const COT: FnDef = FnDef {
    name: "cot",
    parse_text: &["cot"],
    parse_latex: &["cot"],
    inverse: Some("acot"),
    move_exponent_spellings: &["cot"],
    derivative: Some("-csc(x)^2"),
    antiderivative: Some(|u| apply("ln", apply("sin", u))),
    eval1: Some(|z| Some(z.tan().inv())),
    latex_commands: &[("cot", "cot")],
    ..DEFAULTS
};
