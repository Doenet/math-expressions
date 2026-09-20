//! The accessor layer: thin, mostly alias-aware projections of the registry
//! that the rest of the crate calls instead of the old per-subsystem tables in
//! `parse/`, `normalize/`, `calculus/diff.rs`, and `calculus/integrate/`.

use super::def::{EvalN, FoldExact};
use super::registry::{lookup, ALL};
use crate::expr::Expr;
use num_complex::Complex64;
use std::collections::HashMap;
use std::sync::OnceLock;

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
/// match — see [`FnDef::eval1`](super::FnDef::eval1)).
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
    let mut names: Vec<String> = ALL
        .iter()
        .flat_map(|d| d.parse_latex)
        .map(|s| s.to_string())
        .collect();
    // `rootof` is notation for the RootOf leaf, not a math function (no
    // FnDef): registering it here lets `\operatorname{rootof}\left(p,
    // k\right)` — the LaTeX printer's RootOf form — re-parse as the
    // application that canonicalize folds back into the leaf, closing the
    // LaTeX round-trip. The text parser needs no entry (a VAR followed by a
    // parenthesized tuple already applies).
    names.push("rootof".to_string());
    names
}

/// N-ary complex evaluation for the canonical spelling `name` — the
/// aggregates. Tried before [`eval1`]/[`eval2`], since a variadic function
/// must handle its one- and two-argument cases itself.
pub fn evaln(name: &str) -> Option<EvalN> {
    let def = lookup(name)?;
    (def.name == name).then_some(def.evaln).flatten()
}

/// The exact folder for the canonical spelling `name`
/// (see [`FnDef::fold_exact`](super::FnDef::fold_exact)).
pub fn fold_exact(name: &str) -> Option<FoldExact> {
    let def = lookup(name)?;
    (def.name == name).then_some(def.fold_exact).flatten()
}
