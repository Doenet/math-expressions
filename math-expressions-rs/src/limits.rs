//! Resource limits (PORTING_PLAN.md §7f).
//!
//! All expression input is untrusted (student answers), so every unpredictable
//! pass is bounded. This module is the single source of truth for those
//! bounds — previously seven ad-hoc constants scattered across the crate.
//! Every limit counts **operations/sizes, never wall-clock**, so verdicts are
//! identical on every machine (grading engine; reproducible tests).
//!
//! The current limits live in a thread-local (WASM is single-threaded, and the
//! crate already uses thread-local symbol interning), so no signatures change:
//! deep call sites read [`current`]. Embedders can tighten or relax a scope
//! with [`with`]:
//!
//! ```
//! use math_expressions::limits::{self, Limits};
//! let strict = Limits { max_expand_terms: 100, ..Limits::default() };
//! let result = limits::with(strict, || {
//!     // expand()/simplify()/equals() here run under the tighter cap
//! });
//! ```

use std::cell::Cell;

/// Deterministic resource bounds for the unpredictable passes. Defaults are
/// generous: far above anything classroom input reaches, low enough that
/// adversarial input cannot exhaust memory or stall the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Largest integer exponent `expand` will multinomial-expand.
    pub max_expand_power: i64,
    /// Raw term-count cap per distribution step in `expand`; beyond it the
    /// node is left unexpanded.
    pub max_expand_terms: usize,
    /// Maximum rewrite rounds in `simplify`'s fixpoint loop.
    pub max_simplify_rounds: u32,
    /// Trial-division bound for radical simplification (perfect powers of any
    /// size are still found via the O(log) integer nth-root).
    pub max_trial_divisor: u64,
    /// Largest `n` for which `n!` folds exactly.
    pub max_factorial: i64,
    /// Residue-class cap in discrete-infinite-set containment.
    pub max_residues: i64,
    /// Decimal places beyond which rounding is resolved semantically instead
    /// of materializing `10^|d|`.
    pub max_round_decimals: i64,
    /// Bit-size cap on exact integer powers (`2^(10^12)` is not a number to
    /// materialize).
    pub max_pow_bits: u64,
    /// Largest matrix dimension for elimination-based operations
    /// (det/inverse/rref — MATRIX_PLAN §1b).
    pub max_matrix_dim: usize,
    /// Largest dimension for cofactor expansion over general symbolic
    /// entries (n! terms).
    pub max_symbolic_det_dim: usize,
    /// Working-precision cap (bits) for arbitrary-precision evaluation
    /// (ARBITRARY_PERCISION_PLAN §7). ~17000 bits ≈ 5000 decimal digits.
    pub max_eval_precision_bits: u32,
    /// Escalation rounds in the Ziv loop before answering `Unknown`.
    pub max_ziv_rounds: u32,
    /// Iteration cap for any single series/Newton loop in a fix kernel.
    pub max_series_terms: u32,
    /// Compile-time cap on evaluation-tape length.
    pub max_tape_ops: usize,
    /// MSB cap on a trig/exp argument before reduction is refused
    /// (`sin(2^2^20)` answers Unknown, in the spirit of `max_pow_bits`).
    pub max_trig_arg_bits: i64,
    /// Largest polynomial degree accepted into a `RootOf` (matches the poly
    /// layer's degree scale; char polys can't exceed `max_matrix_dim`).
    pub max_rootof_degree: usize,
    /// Bisection budget for certified real-root isolation (an operation
    /// count, not a precision — Sturm bisections across the whole call).
    pub max_isolation_bits: u32,
    /// Live-segment cap for certified adaptive quadrature.
    pub max_quadrature_segments: usize,
    /// Total rule firings (fuel) in symbolic integration, including u-sub
    /// and by-parts recursion (INTEGRATION_PLAN §6).
    pub max_integration_steps: i64,
    /// u-substitution / split candidates tried per node.
    pub max_integration_candidates: usize,
    /// Degree cap for the rational integration engine (Hermite/LRT inputs).
    pub max_lrt_degree: usize,
    /// Accepted+rejected step cap for the adaptive ODE solver.
    pub max_ode_steps: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_expand_power: 64,
            max_expand_terms: 4_000,
            max_simplify_rounds: 32,
            max_trial_divisor: 1 << 10,
            max_factorial: 10_000,
            max_residues: 10_000,
            max_round_decimals: 4_000,
            max_pow_bits: 1_000_000,
            max_matrix_dim: 64,
            max_symbolic_det_dim: 6,
            max_eval_precision_bits: 17_000,
            max_ziv_rounds: 6,
            max_series_terms: 100_000,
            max_tape_ops: 100_000,
            max_trig_arg_bits: 4_096,
            max_rootof_degree: 64,
            max_isolation_bits: 65_536,
            max_quadrature_segments: 16_384,
            max_integration_steps: 256,
            max_integration_candidates: 64,
            max_lrt_degree: 64,
            max_ode_steps: 10_000,
        }
    }
}

thread_local! {
    static CURRENT: Cell<Limits> = Cell::new(Limits::default());
}

/// The limits in effect on this thread.
pub fn current() -> Limits {
    CURRENT.with(Cell::get)
}

/// Run `f` with `limits` in effect, restoring the previous limits afterwards
/// (including on panic/unwind).
pub fn with<R>(limits: Limits, f: impl FnOnce() -> R) -> R {
    struct Restore(Limits);
    impl Drop for Restore {
        fn drop(&mut self) {
            CURRENT.with(|c| c.set(self.0));
        }
    }
    let _restore = Restore(CURRENT.with(|c| c.replace(limits)));
    f()
}
