//! The single registration point: [`ALL`] lists every [`FnDef`], and [`lookup`]
//! resolves a spelling (name or alias) to its definition through the index
//! built from that list.

use super::def::FnDef;
use super::{aggregate, exp_log, hyperbolic, hyperbolic_inverse, misc, powers, trig, trig_inverse};
use std::collections::HashMap;
use std::sync::OnceLock;

/// The single registration point. A definition not listed here does not
/// exist as far as the crate is concerned. (`static`, not `const`: the
/// registry must have one identity — kernel ids are positions in it.)
pub static ALL: &[&FnDef] = &[
    &trig::SIN,
    &trig::COS,
    &trig::TAN,
    &trig::SEC,
    &trig::CSC,
    &trig::COT,
    &trig_inverse::ASIN,
    &trig_inverse::ACOS,
    &trig_inverse::ATAN,
    &trig_inverse::ASEC,
    &trig_inverse::ACSC,
    &trig_inverse::ACOT,
    &trig_inverse::ATAN2,
    &hyperbolic::SINH,
    &hyperbolic::COSH,
    &hyperbolic::TANH,
    &hyperbolic::SECH,
    &hyperbolic::CSCH,
    &hyperbolic::COTH,
    &hyperbolic_inverse::ASINH,
    &hyperbolic_inverse::ACOSH,
    &hyperbolic_inverse::ATANH,
    &hyperbolic_inverse::ASECH,
    &hyperbolic_inverse::ACSCH,
    &hyperbolic_inverse::ACOTH,
    &exp_log::EXP,
    &exp_log::LOG,
    &exp_log::LOG10,
    &exp_log::LOG2,
    &powers::SQRT,
    &powers::CBRT,
    &powers::NTHROOT,
    &powers::ABS,
    &powers::SIGN,
    &misc::MOD,
    &misc::ERF,
    &misc::ARG,
    &misc::CONJ,
    &misc::RE,
    &misc::IM,
    &misc::DET,
    &misc::TRACE,
    &misc::NPR,
    &misc::NCR,
    &misc::FLOOR,
    &misc::CEIL,
    &misc::ROUND,
    &misc::ROOTOF,
    &misc::FACTORIAL,
    &aggregate::SUM,
    &aggregate::PROD,
    &aggregate::COUNT,
    &aggregate::MEAN,
    &aggregate::MEDIAN,
    &aggregate::MAX,
    &aggregate::MIN,
    &aggregate::VARIANCE,
    &aggregate::STD,
];

/// Name/alias → definition, built once. Duplicate names or aliases are a
/// registration bug; the registry unit test checks this on every run (the
/// panic here backstops non-test use).
fn index() -> &'static HashMap<&'static str, &'static FnDef> {
    static INDEX: OnceLock<HashMap<&'static str, &'static FnDef>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut m = HashMap::new();
        for def in ALL {
            for key in std::iter::once(&def.name).chain(def.aliases) {
                if m.insert(*key, *def).is_some() {
                    panic!("special_functions::ALL registers {key:?} twice");
                }
            }
        }
        m
    })
}

/// The definition for `name`, resolving aliases (`arcsin` → the `asin` def).
pub fn lookup(name: &str) -> Option<&'static FnDef> {
    index().get(name).copied()
}
