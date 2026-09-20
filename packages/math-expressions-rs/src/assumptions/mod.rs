//! Assumptions: variable facts and three-valued sign/set inference.
//!
//! The store ([`store`]) mirrors the JS `initialize_assumptions` shape:
//! per-variable facts added via [`Assumptions::add`], retrieved with
//! [`Assumptions::get`], removed with [`Assumptions::remove`]. A fact is a
//! canonical relation `Expr` (`x > 0`, `n ∈ Z`, `x ≠ 0`, `x = 3`, optionally
//! wrapped in `not`), with chains split on `And` and on a negated `Or`.
//! Generic assumptions (JS `add_generic_assumption`) are ported.
//!
//! Queries ([`queries`]) are the eight three-valued predicates of
//! `lib/assumptions/element_of_sets.js` — `is_integer`, `is_real`,
//! `is_complex`, `is_nonzero`, `is_nonnegative`, `is_positive`, `is_negative`,
//! `is_nonpositive` — returning `Some(true)` / `Some(false)` / `None`
//! (unknown), the JS `true/false/undefined`. The inference ([`infer`], over the
//! [`Facts`](facts::Facts) lattice) is a clean-slate bottom-up pass over the
//! canonical tree; behaviour is validated against the JS oracle by the
//! assumptions corpus. Deliberately mirrored JS conservatisms: an unassumed
//! variable is fully unknown (no default-real), odd powers of negatives get no
//! sign (`x³ | x<0` → unknown), and sums do no interval arithmetic
//! (`x−3 | x>4` → unknown sign).
//!
//! The [`TreeStore`](tree_store::TreeStore) the store carries alongside answers
//! the other question the JS API asks of an assumption set — not "is this
//! real?" but "what is known about `x`?", as a tree with `x` on the left. It
//! needs its own filing ([`tree_store`]), its own canonical form ([`clean`],
//! [`expand`]) and a transitive closure over the stored facts ([`derive`]),
//! none of which the predicate engine has any use for.

mod clean;
mod derive;
mod expand;
mod facts;
mod infer;
mod queries;
mod store;
pub mod tree_store;

/// Three-valued logic: `Some(true)` / `Some(false)` / `None` = unknown.
pub type MaybeBool = Option<bool>;

pub use expand::expand_relations;
pub use queries::{
    is_complex, is_integer, is_negative, is_nonnegative, is_nonpositive, is_nonzero, is_positive,
    is_real,
};
pub use store::Assumptions;
pub use tree_store::TreeStore;
