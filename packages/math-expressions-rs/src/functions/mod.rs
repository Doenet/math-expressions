//! Per-function registry (tmp/IMPROVEMENT_PLAN.md Phase 1).
//!
//! ONE place describes everything the crate knows about a named math
//! function: its spellings, parser-default membership, normalization,
//! notated inverse, and calculus rules. Definitions live in family files
//! under `functions/`; [`ALL`] registers them; the accessors at the bottom
//! replace the per-subsystem tables that used to be scattered across
//! `parse/`, `norm/`, `diff.rs`, and `integrate/`.
//!
//! # Adding a function
//!
//! 1. Write a `FnDef` const in the right family file (or give it its own
//!    file when the definition grows past ~150 lines).
//! 2. Add it to [`ALL`].
//! 3. Add parse/round-trip/behavior tests.
//!
//! If the new function needs an edit anywhere *else*, that facet has not
//! been migrated into `FnDef` yet — prefer migrating it over extending an
//! old table (the registry is only done when this list is the whole job).
//!
//! What deliberately stays OUTSIDE the registry:
//! - Notation-shape rendering (`\sqrt{…}`, `\left|…\right|`, `n!`,
//!   `\lfloor…\rfloor`) — printer concerns, like `\frac`.
//! - The MpFix numerical core in `precise/kernels.rs` (series, argument
//!   reduction, shared π/ln2/e caches): a tightly-coupled unit —
//!   `const_pi` feeds sin/cos, `const_ln2` feeds exp *and* ln, tan
//!   composes sin/cos — so it stays together; `FnDef::kernel` is the
//!   per-function pointer into it.
//! - Non-function notation (greek letters, relations, units) — improvement
//!   plan Phase 2.

use crate::expr::Expr;
use num_complex::Complex64;
use std::collections::HashMap;
use std::sync::OnceLock;

pub mod exp_log;
pub mod hyperbolic;
pub mod hyperbolic_inverse;
pub mod misc;
pub mod powers;
pub mod trig;
pub mod trig_inverse;

/// Everything the crate knows about one named math function.
///
/// Definitions spell out only the facets that apply and default the rest
/// with `..DEFAULTS`.
pub struct FnDef {
    /// Canonical spelling — what normalization rewrites *to* (`asin`, `log`).
    pub name: &'static str,
    /// Alternate spellings folded to `name` by [`canonical_name`] (the JS
    /// `function_normalizations` table: `arcsin`, `ln`, `cosec`, …).
    pub aliases: &'static [&'static str],
    /// Spellings this function contributes to the TEXT parser's default
    /// `applied_function_symbols` list.
    pub parse_text: &'static [&'static str],
    /// Spellings contributed to the LATEX parser's default list. May differ
    /// from `parse_text` (e.g. `re` in text vs `Re` in LaTeX).
    pub parse_latex: &'static [&'static str],
    /// Canonical name of the notated inverse (`sin` → `asin`) for the
    /// `f^(-1)(x)` → `af(x)` rewrite. Matched on `name` only — the rewrite
    /// sites run before alias renaming, and the historical tables never
    /// listed aliases here.
    pub inverse: Option<&'static str>,
    /// Spellings for which `f^n(x)` (n ≠ −1) rewrites to `(f(x))^n`. Listed
    /// explicitly (not derived from `aliases`) because the rewrite runs
    /// *before* name normalization in `canon_apply`, and the historical
    /// MOVE_EXPONENT_OUTSIDE set covered `ln` but not `cosec`.
    pub move_exponent_spellings: &'static [&'static str],
    /// The mathjs derivative-table entry, as a text template in the
    /// placeholder `x` (`None`: `diff` falls back to prime notation).
    pub derivative: Option<&'static str>,
    /// One antiderivative in the argument `u`, as an expression builder
    /// using the `norm` smart constructors — exactly the shapes the
    /// integrator's elementary table historically produced. The caller
    /// handles the linear-inner-argument division.
    pub antiderivative: Option<fn(Expr) -> Expr>,
    /// Complex evaluation with one argument. `None` result: undefined on
    /// that input (e.g. `floor` of a non-real value). Matched on the
    /// canonical spelling only — evaluation runs on canonicalized trees,
    /// and the historical `known_function` list never held aliases.
    pub eval1: Option<fn(Complex64) -> Option<Complex64>>,
    /// Complex evaluation with two arguments (`atan2`, `mod`, …).
    pub eval2: Option<fn(Complex64, Complex64) -> Option<Complex64>>,
    /// LaTeX control-word rendering, per spelling: `("asin", "arcsin")`
    /// renders the symbol `asin` as `\arcsin`. Spellings not listed fall
    /// back to `\operatorname{…}`. Per-spelling (like
    /// `move_exponent_spellings`) because faithful trees carry unnormalized
    /// names — `ln` renders `\ln` while `cosec` never had a control word.
    pub latex_commands: &'static [(&'static str, &'static str)],
    /// Override for the head of a rendered LaTeX application, where it
    /// differs from the symbol form (`log10` → `\log_{10}`, `re` → `\Re`).
    pub latex_head: Option<&'static str>,
    /// Precise-evaluation kernel (`precise/` tier-0 obligations + optional
    /// MpFix tier-2 kernel). The tape's `Op::Call` id space is the order of
    /// kernel-bearing defs in [`ALL`] (`precise::kernels::registry`).
    pub kernel: Option<&'static crate::precise::kernels::FnKernel>,
}

/// The all-defaults definition, for `..DEFAULTS` in family files.
pub const DEFAULTS: FnDef = FnDef {
    name: "",
    aliases: &[],
    parse_text: &[],
    parse_latex: &[],
    inverse: None,
    move_exponent_spellings: &[],
    derivative: None,
    antiderivative: None,
    eval1: None,
    eval2: None,
    latex_commands: &[],
    latex_head: None,
    kernel: None,
};

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
                    panic!("functions::ALL registers {key:?} twice");
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

/// `Some(canonical)` iff `name` is a non-canonical alias — the contract of
/// the old `function_normalizations` tables (`None` for canonical spellings
/// and unknown names).
pub fn canonical_name(name: &str) -> Option<&'static str> {
    let def = lookup(name)?;
    (def.name != name).then_some(def.name)
}

/// The notated inverse for the *canonical* spelling `name` (`sin` → `asin`).
pub fn inverse_of(name: &str) -> Option<&'static str> {
    let def = lookup(name)?;
    (def.name == name).then_some(def.inverse).flatten()
}

/// Does `f^n(x)` → `(f(x))^n` apply to this (pre-normalization) spelling?
pub fn moves_exponent_outside(name: &str) -> bool {
    lookup(name).is_some_and(|d| d.move_exponent_spellings.contains(&name))
}

/// The derivative template for `name` (alias-aware: `arcsin` finds the
/// `asin` entry, matching the old table's `"asin" | "arcsin"` arms).
pub fn derivative_template(name: &str) -> Option<&'static str> {
    lookup(name)?.derivative
}

/// The antiderivative builder for `name` (alias-aware, like the old
/// `"atan" | "arctan"` arms in the integrator's elementary table).
pub fn antiderivative_builder(name: &str) -> Option<fn(Expr) -> Expr> {
    lookup(name)?.antiderivative
}

/// Unary complex evaluation for the *canonical* spelling `name` (exact
/// match — see [`FnDef::eval1`]).
pub fn eval1(name: &str) -> Option<fn(Complex64) -> Option<Complex64>> {
    let def = lookup(name)?;
    (def.name == name).then_some(def.eval1).flatten()
}

/// Binary complex evaluation for the canonical spelling `name`.
pub fn eval2(name: &str) -> Option<fn(Complex64, Complex64) -> Option<Complex64>> {
    let def = lookup(name)?;
    (def.name == name).then_some(def.eval2).flatten()
}

/// The LaTeX control word for this exact spelling (`asin` → `arcsin`,
/// `ln` → `ln`), or `None` for `\operatorname{…}` fallback. Uses its own
/// spelling index, NOT [`lookup`]: `Re`/`Im` have control words but are
/// neither names nor aliases (aliasing them would wrongly opt them into
/// name normalization).
pub fn latex_command(name: &str) -> Option<&'static str> {
    static INDEX: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    INDEX
        .get_or_init(|| {
            let mut m = HashMap::new();
            for def in ALL {
                for (spelling, cmd) in def.latex_commands {
                    if m.insert(*spelling, *cmd).is_some() {
                        panic!("latex spelling {spelling:?} registered twice");
                    }
                }
            }
            m
        })
        .get(name)
        .copied()
}

/// The LaTeX application-head override for `name`, if any.
pub fn latex_apply_head(name: &str) -> Option<&'static str> {
    lookup(name)?.latex_head
}

/// The text parser's default `applied_function_symbols`.
pub fn applied_text_names() -> Vec<String> {
    ALL.iter()
        .flat_map(|d| d.parse_text)
        .map(|s| s.to_string())
        .collect()
}

/// The LaTeX parser's default `applied_function_symbols`.
pub fn applied_latex_names() -> Vec<String> {
    ALL.iter()
        .flat_map(|d| d.parse_latex)
        .map(|s| s.to_string())
        .collect()
}

// Builder shorthand shared by the antiderivative closures in family files.
// Same helpers the integrator used, so the built shapes are identical.
pub(crate) fn int(i: i64) -> Expr {
    Expr::Num(crate::num::Number::Int(i))
}

pub(crate) fn apply(name: &str, arg: Expr) -> Expr {
    Expr::Apply(Box::new(Expr::sym(name)), vec![arg])
}

/// Apply a real function to a (near-)real complex value, else `None`.
/// Shared by the `eval1` closures of real-only functions (floor/ceil/…).
pub(crate) fn real_only(z: Complex64, f: fn(f64) -> f64) -> Option<Complex64> {
    if z.im.abs() < 1e-9 {
        Some(Complex64::new(f(z.re), 0.0))
    } else {
        None
    }
}
