//! **Value** equality: do two
//! expressions denote the same mathematical object? This is the value axis —
//! contrast [`equality_structural`](crate::equality_structural), which asks
//! whether an answer is in a required *form* (factored, reduced, …), and
//! [`precise`](crate::precise), which is certified arbitrary-precision numeric
//! evaluation.
//!
//! ## It is a staged decision procedure, not a numerical test
//!
//! `equals` is **mostly exact**. It tries cheap, exact, and type-directed
//! stages first and only falls back to numerical sampling as a last resort.
//! Crucially, that final numerical stage is **not certified** — it is a lenient
//! heuristic made *sound* by the exact stages that run ahead of it (see stage
//! 7). If you want a guaranteed-accurate-or-`Unknown` number, that is
//! [`precise`](crate::precise), not this module.
//!
//! The stages, in order ([`api::equals`]). Before stage 0, `equals` desugars
//! scaling units on both sides (`50% → 50/100`, `deg`, `$`) so the stages below
//! see unit-free trees:
//!
//! 0. **Blank guard** — a missing operand makes equality undefined ⇒ `false`
//!    (unless `allow_blanks`).
//! 1. **Exact canonical compare** — [`canonicalize`](crate::norm::canonicalize)
//!    both sides and compare trees. Most equal pairs agree here with no
//!    numerics (the exactness payoff: `10^20+1 ≠ 10^20+2` is decided, not
//!    sampled).
//! 2. **Exact simplified compare** — `simplify_canonical` adds the heuristic
//!    rewrite clusters (radical, trig, ∞/NaN) to a fixpoint, then compare trees
//!    again, so identities like `sin²x+cos²x = 1` resolve *structurally*.
//! 3. **Fuzzy structural compare** — only under a grading number-error
//!    allowance: number leaves compared within tolerance ([`fuzzy`]).
//! 4. **Definitive exact-number rejection** — if both sides fold to bare
//!    numbers and differ, they are unequal and the numerical stage is forbidden
//!    from overriding with f64 slop.
//! 5. **Type-directed dispatch** (each *before* any sampling):
//!    - `±` value-set equality ([`plus_minus`]),
//!    - comparison relations equal up to proportional standard forms
//!      ([`relations`]),
//!    - discrete infinite sets (periodic solution sets like `π/4 + nπ`) by
//!      residue-class covering ([`discrete_infinite`]).
//! 6. **Finite-field rejection** — **exact** evaluation in ℤ/pℤ
//!    ([`finite_field`]). It never *confirms* equality, only rejects; it is the
//!    filter that catches additive/structural near-misses (`e^(10x)` vs
//!    `e^(10x)+C`) that floating-point sampling would mask.
//! 7. **Numerical sampling** ([`numeric`]) — the last resort: agreement on a
//!    small neighborhood of random points ⇒ identical by analyticity, while
//!    *tolerating* base-point disagreement (branch-cut identities). This
//!    leniency is safe **only because stage 6 already rejected** the near-misses
//!    it would otherwise wrongly accept — the soundness comes from the exact
//!    pre-filter, not from an error bound.
//!
//! [`equals_via_real`](api::equals_via_real) is the same procedure restricted
//! to real sample points (JS `equalsViaReal`), and
//! [`equals_syntactic`](api::equals_syntactic) is the form-level whole-tree
//! compare shared with [`equality_structural`](crate::equality_structural).
//!
//! ## Layout
//!
//! Barrel module. Public entry points live in [`api`]; each stage is a focused
//! submodule:
//!
//! - [`options`]     — the [`EqOptions`] knobs (tolerances, coercion, sampling mode)
//! - [`api`]         — `equals`, `equals_via_real`, `equals_syntactic`, `contains_blank`
//! - [`numeric`]     — random-point sampling (the `find_equality_region` strategy)
//! - [`fuzzy`]        — number-error-tolerant structural comparison
//! - [`relations`]   — comparison relations compared up to proportionality
//! - [`plus_minus`]  — `±` value-set equality
//! - [`finite_field`]     — the exact ℤ/pℤ rejection filter
//! - [`discrete_infinite`] — periodic solution sets

pub mod discrete_infinite;
mod api;
mod finite_field;
mod fuzzy;
mod numeric;
mod options;
mod plus_minus;
mod relations;

pub use api::{contains_blank, equals, equals_syntactic, equals_via_real};
pub use finite_field::finite_field_evaluate;
pub use options::EqOptions;
