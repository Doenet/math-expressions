//! Exponential and logarithms. Canonically the natural log is `log` (the JS
//! convention); `ln` is an alias — but `ln` appears in
//! `move_exponent_spellings` because `canon_apply` moves exponents *before*
//! renaming, so `ln^2(x)` must match under its original spelling.

use super::{apply, int, FnDef, DEFAULTS};
use crate::precise::kernels::{FixId, FnKernel};
use crate::norm::{add, mul, pow};

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
            mul(vec![u.clone(), apply("ln", u.clone())]),
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
                mul(vec![u.clone(), apply("ln", u.clone())]),
                mul(vec![int(-1), u]),
            ]),
            pow(apply("ln", int(10)), int(-1)),
        ])
    }),
    eval1: Some(|z| Some(z.log10())),
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
