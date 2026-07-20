//! Resource limits (PORTING_PLAN.md §7f).
//!
//! Note: these are *computational safety* bounds, unrelated to calculus
//! limits (`lim`). All expression input is untrusted (student answers), so
//! every unpredictable pass is bounded. This module is the single source of
//! truth for those bounds — previously seven ad-hoc constants scattered
//! across the crate. Every bound counts **operations/sizes, never
//! wall-clock**, so verdicts are identical on every machine (grading engine;
//! reproducible tests).
//!
//! The current limits live in a thread-local (WASM is single-threaded, and the
//! crate already uses thread-local symbol interning), so no signatures change:
//! deep call sites read [`current`]. Embedders can tighten or relax a scope
//! with [`with`]:
//!
//! ```
//! use math_expressions::resource_limits::{self, ResourceLimits};
//! let strict = ResourceLimits { max_expand_terms: 100, ..ResourceLimits::default() };
//! let result = resource_limits::with(strict, || {
//!     // expand()/simplify()/equals() here run under the tighter cap
//! });
//! ```

use std::cell::Cell;

/// Deterministic resource bounds for the unpredictable passes. Defaults are
/// generous: far above anything classroom input reaches, low enough that
/// adversarial input cannot exhaust memory or stall the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceLimits {
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
    /// Largest polynomial degree `factor` will process; beyond it the input
    /// is returned unfactored. Bounds the dense-coefficient allocation an
    /// adversarial exponent (`x^10^9`) would otherwise force.
    pub max_factor_degree: usize,
    /// Node/operation budget for the exact-constant evaluator (`exact.rs`,
    /// FULL_SIMPLIFY §9). Bounds the S1 `is_zero` tower evaluation.
    pub max_exact_eval_ops: i64,
    /// Cap on the number of summands `ratform` (FULL_SIMPLIFY §9, S2) will put
    /// over a common denominator before bailing to the unchanged form —
    /// bounds `together`/`cancel` coefficient swell.
    pub max_ratform_terms: usize,
    /// Accepted+rejected step cap for the adaptive ODE solver.
    pub max_ode_steps: usize,
    /// Candidate singular cells per divergence-classification call.
    pub max_singularity_candidates: usize,
    /// Certified-sign bisection budget across a classification call.
    pub max_certificate_bisections: usize,
    /// Cell-shrink iterations for tail-bounded improper evaluation.
    pub max_improper_refinements: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        ResourceLimits {
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
            max_factor_degree: 64,
            max_exact_eval_ops: 10_000,
            max_ratform_terms: 512,
            max_ode_steps: 10_000,
            max_singularity_candidates: 32,
            max_certificate_bisections: 4_096,
            max_improper_refinements: 40,
        }
    }
}

thread_local! {
    static CURRENT: Cell<ResourceLimits> = Cell::new(ResourceLimits::default());
}

/// The limits in effect on this thread.
pub fn current() -> ResourceLimits {
    CURRENT.with(Cell::get)
}

/// Run `f` with `limits` in effect, restoring the previous limits afterwards
/// (including on panic/unwind).
pub fn with<R>(limits: ResourceLimits, f: impl FnOnce() -> R) -> R {
    struct Restore(ResourceLimits);
    impl Drop for Restore {
        fn drop(&mut self) {
            CURRENT.with(|c| c.set(self.0));
        }
    }
    let _restore = Restore(CURRENT.with(|c| c.replace(limits)));
    f()
}
