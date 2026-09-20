//! Normalization: a pure faithful-layer → canonical-layer transform. Canonical
//! form eliminates the display-only variants (`Div`, `Neg`), flattens and sorts
//! commutative operators, folds constants *exactly*, and combines like terms
//! and like powers — so two equal canonical expressions are identical trees and
//! structural equality is tree comparison.
//!
//! `canonicalize` is confluent, cheap, and assumption-free. Heuristic
//! simplification (root pulling, trig/log identities) is a separate, deferred
//! layer that needs the assumptions system.
//!
//! Barrel module. The canonical layer's core is factored into:
//!
//! - [`canonicalize`] — the bottom-up dispatch and application/relation canon
//! - [`constructors`] — the `add`/`mul`/`pow` smart constructors
//! - [`matrix_ops`]   — literal-matrix helpers used by the constructors
//! - [`units`]        — scaling-unit desugaring (`%`, `deg`, `$`)
//! - [`full`]         — the aggressive `full_simplify` fixpoint driver that the
//!   public `simplify` / `simplify_with` delegate to
//!
//! plus the `expand`, `order`, `present`, `simplify` (the base rewrite
//! clusters), `special_values`, `fold_apply`, and `syntactic` passes.

mod canonicalize;
mod constructors;
mod default_order;
mod full;
mod matrix_ops;
mod units;

pub(crate) mod expand;
pub(crate) mod fold_apply;
pub(crate) mod order;
pub(crate) mod present;
pub(crate) mod simplify;
pub(crate) mod special_values;
pub(crate) mod syntactic;

pub use expand::expand;
pub(crate) use expand::expand_core;
pub(crate) use fold_apply::spread_list_argument;
pub use fold_apply::{fold_numeric_applications, fold_numeric_applications_approx};
pub(crate) use order::cmp;
pub(crate) use present::{present, split_number};
pub use simplify::{flatten_logical, push_not, simplify, simplify_logical, simplify_with};
pub(crate) use simplify::{rule_infnan, simplify_base_with, simplify_canonical, simplify_core};
pub(crate) use special_values::fold_imaginary_power;
pub use special_values::fold_special_values;
pub use syntactic::{
    normalize_syntactic, pass_applied_functions as normalize_applied_functions,
    pass_negative_numbers as normalize_negative_numbers,
};

pub use canonicalize::canonicalize;
pub(crate) use constructors::{
    add, exp_call, exp_call_arg, mul, pow, split_coeff, without_like_term_collection,
};
pub use default_order::{cmp_default_order, default_order};
pub(crate) use default_order::{js_operands, legacy_operator};
pub(crate) use full::full_simplify;
pub(crate) use matrix_ops::{
    contract_pair, identity_matrix, is_matrix_valued, is_vector_valued, matmul_literal,
};
pub use units::desugar_units;
pub(crate) use units::{fold_units, scaling_unit_and_value, unit_body};
