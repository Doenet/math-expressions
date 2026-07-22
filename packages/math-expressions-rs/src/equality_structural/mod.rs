//! Structural comparison: predicates over
//! the **faithful** (pre-`canonicalize`) `Expr` that decide whether a student
//! wrote an answer with a required *structure* — factored, expanded, reduced,
//! radical-simplified, a decimal vs an exact value, etc. — as opposed to
//! whether it has the right *value* (that is `equals`). Each requirement is a
//! [`StructuralComparison`].
//!
//! Two rules make these checks meaningful:
//!
//! 1. **Never canonicalize the input first.** `canonicalize` folds `2/4 → 1/2`,
//!    rewrites `Div`→`Mul·Pow⁻¹`, sorts, and combines like terms — i.e. it
//!    erases the very structure under test. Predicates inspect the faithful tree
//!    (`TextToAst::convert` output) directly, using `canonicalize`/`expand`/
//!    `factor`/`reduce_rational` only as *oracles* applied to controlled
//!    sub-comparisons.
//! 2. **Structure ⊥ value.** [`check_structural_comparison`] answers only "is it in
//!    this structure?". The autograder primitive "…and equal to the key" is
//!    [`structural_equality`], a sibling to [`equals`](crate::equals) that
//!    follows the JS `equalsVia*` family (no batch "grade" step).
//!
//! Prior art: STACK answer tests (`FacForm`, `Expanded`, `LowestTerms`,
//! `SingleFrac`, `CompletedSquare`) and WeBWorK strict contexts. Standards
//! diverge on what to enforce, so every comparison is opt-in and independent.
//!
//! ## Vocabulary (value vs structural)
//!
//! - [`equals`](crate::equals) — **value** equality (do they mean the same
//!   number/expression?).
//! - [`structural_equality`] — **structural** comparison against a key, with a
//!   [`StructuralComparison`] method. Its base method
//!   [`SameStructure`](StructuralComparison::SameStructure) is order-sensitive whole-tree
//!   equality — the JS `equalsViaSyntax`, also exposed under its JS-parity name
//!   [`equals_syntactic`](crate::equals_syntactic). The other methods are
//!   specific-structure criteria (factored, reduced, …), each requiring value
//!   equality too.
//! - [`check_structural_comparison`] — a **unary** structural predicate (is *this*
//!   expression factored / reduced / …?), no key.
//!
//! So "syntactic equality" is not a separate concept: it is the `SameStructure`
//! structural comparison; `equals_syntactic` is its convenience name.
//!
//! Barrel module. The predicates are grouped by the structure they inspect:
//!
//! - [`types`]     — [`StructuralComparison`] and its result type
//! - [`compare`]   — the two public entry points and the criterion dispatch
//! - [`fractions`] — decimal/exact and written-fraction shapes
//! - [`radicals`]  — the one root decomposition and its simplification checks
//! - [`forms`]     — expanded / factored / like-terms / completed-square shapes
//! - [`helpers`]   — shared number, sign, and symbol primitives

mod compare;
mod forms;
mod fractions;
mod helpers;
mod radicals;
mod types;

pub use compare::{check_structural_comparison, structural_equality};
pub use types::{StructuralComparison, StructuralComparisonResult};
