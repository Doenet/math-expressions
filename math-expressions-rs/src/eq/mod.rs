//! Equality testing (PORTING_PLAN.md §10, redesign note §3.5). The staged
//! algorithm: blank guard → canonical structural compare (exact, the §3a
//! payoff) → numerical sampling at random complex points. Finite-field
//! rejection (stage 2) and discrete-infinite-set (stage 4) are deferred; the
//! remaining stages already decide the large majority of cases.

use crate::eval::{eval_complex, free_symbols, Env};
use crate::expr::{Expr, SeqKind};
use crate::norm::canonicalize;
use num_complex::Complex64;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// Options mirroring the JS `equals` parameters (PORTING_PLAN.md §10). Only the
/// tolerances and coercion flags affect this first-cut implementation;
/// `allowed_error_in_numbers` fuzzy matching is a documented follow-up.
#[derive(Debug, Clone)]
pub struct EqOptions {
    pub relative_tolerance: f64,
    pub absolute_tolerance: f64,
    pub tolerance_for_zero: f64,
    pub allow_blanks: bool,
    pub coerce_tuples_arrays: bool,
    pub coerce_vectors: bool,
    /// Number of random complex sample points for the numerical stage.
    pub num_samples: usize,
}

impl Default for EqOptions {
    fn default() -> Self {
        EqOptions {
            relative_tolerance: 1e-12,
            absolute_tolerance: 0.0,
            tolerance_for_zero: 1e-15,
            allow_blanks: false,
            coerce_tuples_arrays: true,
            coerce_vectors: true,
            num_samples: 20,
        }
    }
}

/// Are `a` and `b` mathematically equal?
pub fn equals(a: &Expr, b: &Expr, opts: &EqOptions) -> bool {
    // Stage 0: a blank (missing operand) makes equality undefined.
    if !opts.allow_blanks && (contains_blank(a) || contains_blank(b)) {
        return false;
    }

    // Stage 1: exact structural equality of canonical forms.
    let ca = coerce_seqs(canonicalize(a), opts);
    let cb = coerce_seqs(canonicalize(b), opts);
    if ca == cb {
        return true;
    }

    // When both sides fold to a bare exact number, stage 1 is *definitive*:
    // they are unequal, and the numerical stage must not override with f64
    // slop (this is the §3a exactness win — `10^20+1` ≠ `10^20+2`). Structure
    // that did not fully evaluate (roots, functions) still needs sampling.
    if matches!(ca, Expr::Num(_)) && matches!(cb, Expr::Num(_)) {
        return false;
    }

    // Stage 3: numerical agreement at random complex points.
    equals_numerical(&ca, &cb, opts)
}

/// Does the tree contain a `Blank` (missing operand)? A variant check, not a
/// magic-symbol scan.
fn contains_blank(e: &Expr) -> bool {
    match e {
        Expr::Blank => true,
        Expr::Neg(x) | Expr::Not(x) | Expr::Prime(x) => contains_blank(x),
        Expr::Pow(a, b) | Expr::Div(a, b) | Expr::Index(a, b) => {
            contains_blank(a) || contains_blank(b)
        }
        Expr::Add(xs)
        | Expr::Mul(xs)
        | Expr::And(xs)
        | Expr::Or(xs)
        | Expr::Union(xs)
        | Expr::Intersect(xs)
        | Expr::Seq(_, xs)
        | Expr::OtherOp(_, xs) => xs.iter().any(contains_blank),
        Expr::Apply(h, xs) => contains_blank(h) || xs.iter().any(contains_blank),
        Expr::Interval { endpoints, .. } => {
            contains_blank(&endpoints.0) || contains_blank(&endpoints.1)
        }
        Expr::Relation { operands, .. } => operands.iter().any(contains_blank),
        Expr::Matrix { entries, .. } => entries.iter().any(contains_blank),
        _ => false,
    }
}

/// Map coerced sequence kinds to a common kind so `(1,2)`, `[1,2]`, and vector
/// forms compare equal when the corresponding flag is set.
fn coerce_seqs(e: Expr, opts: &EqOptions) -> Expr {
    fn recur(e: Expr, opts: &EqOptions) -> Expr {
        let map_kind = |k: SeqKind| match k {
            SeqKind::Array if opts.coerce_tuples_arrays => SeqKind::Tuple,
            SeqKind::Vector | SeqKind::AltVector if opts.coerce_vectors => SeqKind::Tuple,
            other => other,
        };
        match e {
            Expr::Seq(k, xs) => Expr::Seq(
                map_kind(k),
                xs.into_iter().map(|x| recur(x, opts)).collect(),
            ),
            Expr::Add(xs) => Expr::Add(xs.into_iter().map(|x| recur(x, opts)).collect()),
            Expr::Mul(xs) => Expr::Mul(xs.into_iter().map(|x| recur(x, opts)).collect()),
            Expr::Pow(a, b) => Expr::Pow(Box::new(recur(*a, opts)), Box::new(recur(*b, opts))),
            Expr::Apply(h, xs) => Expr::Apply(
                Box::new(recur(*h, opts)),
                xs.into_iter().map(|x| recur(x, opts)).collect(),
            ),
            other => other,
        }
    }
    recur(e, opts)
}

/// Sample both expressions at random complex points; accept if they agree
/// within tolerance everywhere they can both be evaluated. A fixed seed keeps
/// results reproducible (and avoids OS entropy, which is unavailable on wasm).
fn equals_numerical(a: &Expr, b: &Expr, opts: &EqOptions) -> bool {
    let mut vars = std::collections::BTreeSet::new();
    free_symbols(a, &mut vars);
    free_symbols(b, &mut vars);
    let vars: Vec<String> = vars.into_iter().collect();

    let mut rng = SmallRng::seed_from_u64(0x5EED_1234_ABCD_0001);
    let mut agreements = 0;
    let mut attempts = 0;
    // Keep sampling until we have enough successful (pole-free) comparisons or
    // run out of attempts; a single disagreement is a definitive reject.
    while agreements < opts.num_samples && attempts < opts.num_samples * 4 {
        attempts += 1;
        let env: Env = vars
            .iter()
            .map(|v| {
                // Sample in a modest box, offset from the origin to dodge the
                // most common poles/branch points.
                let re = rng.random_range(-2.0..2.0) + 0.3;
                let im = rng.random_range(-2.0..2.0) + 0.2;
                (v.clone(), Complex64::new(re, im))
            })
            .collect();

        let (Some(va), Some(vb)) = (eval_complex(a, &env), eval_complex(b, &env)) else {
            // Not numerically evaluable here: can't confirm equality.
            return false;
        };
        if !va.re.is_finite() || !va.im.is_finite() || !vb.re.is_finite() || !vb.im.is_finite() {
            continue; // hit a pole; resample
        }
        if close(va, vb, opts) {
            agreements += 1;
        } else {
            return false;
        }
    }
    agreements > 0
}

fn close(a: Complex64, b: Complex64, opts: &EqOptions) -> bool {
    let diff = (a - b).norm();
    let scale = a.norm().max(b.norm());
    if scale <= opts.tolerance_for_zero {
        return diff <= opts.tolerance_for_zero;
    }
    diff <= opts.absolute_tolerance + opts.relative_tolerance * scale
}
