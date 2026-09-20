//! Exact-constant evaluation and the certified zero-equivalence service.
//!
//! Two public entry points:
//!
//! * [`Exact`] / [`exact_eval`] — a rigorous evaluator for real constants over
//!   the field ℚ adjoined with surds (√ of nonnegative rationals), π and e as
//!   transcendental generators, and the trig/exp/log special values that land
//!   in that field. It only ever returns a value it can *prove* correct;
//!   anything outside the tower yields `None`. The ring itself lives in
//!   [`value`], the evaluator in [`eval`].
//! * [`is_zero`] — `is_zero(e, a) -> MaybeBool` (`Some(true)` = certified zero,
//!   `Some(false)` = certified nonzero, `None` = undecided). Soundness is the
//!   invariant: it never answers `Some(_)` unless the answer is certain. The
//!   service lives in [`zero_testing`]; the single-`RootOf` decider in
//!   [`algebraic`].

mod algebraic;
mod eval;
mod value;
mod zero_testing;

pub use eval::exact_eval;
pub use value::Exact;
pub use zero_testing::is_zero;

// Crate-internal entry points (used qualified as `crate::eval_exact::…`).
pub(crate) use eval::{inverse_trig_special_value, trig_special_value};
pub(crate) use zero_testing::certified_zero;
