//! Per-function registry.
//!
//! ONE place describes everything the crate knows about a named math
//! function: its spellings, parser-default membership, normalization,
//! notated inverse, and calculus rules. The [`FnDef`] schema lives in [`def`];
//! definitions live in the family files below; [`registry`] (`ALL` + `lookup`)
//! registers them; the accessors in [`query`] replace the per-subsystem tables
//! that used to be scattered across `parse/`, `normalize/`, `calculus/diff.rs`,
//! and `calculus/integrate/`.
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
//! - The MpFix numerical core in `eval_numeric/certified_digits/kernels.rs`
//!   (series, argument reduction, shared π/ln2/e caches): a tightly-coupled
//!   unit — `const_pi` feeds sin/cos, `const_ln2` feeds exp *and* ln, tan
//!   composes sin/cos — so it stays together; `FnDef::kernel` is the
//!   per-function pointer into it.
//! - Non-function notation (greek letters, relations, units).

pub mod aggregate;
pub mod exp_log;
pub mod hyperbolic;
pub mod hyperbolic_inverse;
pub mod misc;
pub mod powers;
pub mod trig;
pub mod trig_inverse;

mod builders;
mod def;
mod query;
mod registry;

pub use def::{EvalN, FnDef, FoldExact, DEFAULTS};
pub use query::{
    antiderivative_builder, applied_latex_names, applied_text_names, canonical_name,
    derivative_template, eval1, eval2, evaln, fold_exact, inverse_of, latex_apply_head,
    latex_command, moves_exponent_outside,
};
pub use registry::{lookup, ALL};

// Builder shorthand shared by the family files' antiderivative / eval1 closures
// (re-exported at the parent so `use super::{apply, int, real_only}` works).
pub(crate) use builders::{apply, int, real_only};
