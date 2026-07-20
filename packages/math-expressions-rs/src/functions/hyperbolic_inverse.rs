//! Inverse hyperbolic functions (canonical `a…` spellings; `arc…` aliases).

use super::{FnDef, DEFAULTS};

pub const ASINH: FnDef = FnDef {
    name: "asinh",
    aliases: &["arcsinh"],
    parse_text: &["asinh", "arcsinh"],
    parse_latex: &["asinh", "arcsinh"],
    derivative: Some("1/sqrt(x^2 + 1)"),
    eval1: Some(|z| Some(z.asinh())),
    ..DEFAULTS
};

pub const ACOSH: FnDef = FnDef {
    name: "acosh",
    aliases: &["arccosh"],
    parse_text: &["acosh", "arccosh"],
    parse_latex: &["acosh", "arccosh"],
    derivative: Some("1/sqrt(x^2 - 1)"),
    eval1: Some(|z| Some(z.acosh())),
    ..DEFAULTS
};

pub const ATANH: FnDef = FnDef {
    name: "atanh",
    aliases: &["arctanh"],
    parse_text: &["atanh", "arctanh"],
    parse_latex: &["atanh", "arctanh"],
    derivative: Some("1/(1 - x^2)"),
    eval1: Some(|z| Some(z.atanh())),
    ..DEFAULTS
};

pub const ASECH: FnDef = FnDef {
    name: "asech",
    aliases: &["arcsech"],
    parse_text: &["asech", "arcsech"],
    parse_latex: &["asech", "arcsech"],
    derivative: Some("-(1/sqrt(1 - x^2))/x"),
    eval1: Some(|z| Some(z.inv().acosh())),
    ..DEFAULTS
};

pub const ACSCH: FnDef = FnDef {
    name: "acsch",
    aliases: &["arccsch"],
    parse_text: &["acsch", "arccsch"],
    parse_latex: &["acsch", "arccsch"],
    derivative: Some("-(1/sqrt(x^2 + 1))/abs(x)"),
    eval1: Some(|z| Some(z.inv().asinh())),
    ..DEFAULTS
};

pub const ACOTH: FnDef = FnDef {
    name: "acoth",
    aliases: &["arccoth"],
    parse_text: &["acoth", "arccoth"],
    parse_latex: &["acoth", "arccoth"],
    derivative: Some("-1/(1 - x^2)"),
    eval1: Some(|z| Some(z.inv().atanh())),
    ..DEFAULTS
};
