//! The numerical stage: prove equality by finding one small neighborhood where
//! both functions agree at several clustered points (JS `find_equality_region`).

use super::fuzzy::{build_fuzzy_tol, FuzzyTol};
use super::EqOptions;
use crate::eval::{eval_complex, free_symbols, Env};
use crate::expr::Expr;
use num_complex::Complex64;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

// JS numerical-equality constants (lib/expression/equality/numerical.js).
/// Clustered agreeing points needed to accept a region.
pub(super) const MINIMUM_MATCHES: usize = 10;
/// Disagreeing base points tolerated before rejecting — branch-cut identities
/// disagree at many points, so this must be generous.
pub(super) const NUMBER_TRIES: usize = 100;
/// Base-point sampling radii, tried in order. Large scales first so a non-identity
/// reveals its global disagreement before small scales probe near the origin;
/// neighborhoods use `scale / 100`.
pub(super) const BINDING_SCALES: [f64; 6] = [10.0, 1.0, 100.0, 0.1, 1000.0, 0.01];
/// `Number.MAX_VALUE * 1e-20` — larger magnitudes are out of bounds.
pub(super) const MAX_VALUE: f64 = f64::MAX * 1e-20;

/// Numerical equality by the JS `find_equality_region` strategy: prove equality
/// by finding **one** small neighborhood where both functions agree at several
/// clustered points (agreement on an open set ⟹ identical, by analyticity),
/// while *tolerating* base points that disagree — which happens for identities
/// that hold only off a branch cut, e.g. `log(a^2 b) = 2 log a + log b`. This
/// leniency is safe only because the finite-field filter (stage 2) has already
/// rejected the near-misses it would otherwise accept (`e^(10x)` vs `e^(10x)+C`).
pub(super) fn equals_numerical(a: &Expr, b: &Expr, opts: &EqOptions) -> bool {
    let mut vars = std::collections::BTreeSet::new();
    free_symbols(a, &mut vars);
    free_symbols(b, &mut vars);
    let vars: Vec<String> = vars.into_iter().collect();

    // Constant expressions (no free symbols) are a single value each — compare
    // directly, including a genuine zero (`sin(pi) = 0`), which the region
    // search below deliberately excludes as underflow.
    if vars.is_empty() {
        let env = Env::new();
        // Constant expressions still honour the allowed number error: the
        // sensitivity tolerance is itself a constant here.
        let extra = if opts.allowed_error_in_numbers > 0.0 {
            match build_fuzzy_tol(a, &vars, opts).map(|f| f.at(&env)) {
                Some(Some(t)) => t,
                Some(None) => return false,
                None => 0.0,
            }
        } else {
            0.0
        };
        return match (eval_complex(a, &env), eval_complex(b, &env)) {
            (Some(va), Some(vb))
                if va.re.is_finite()
                    && va.im.is_finite()
                    && vb.re.is_finite()
                    && vb.im.is_finite() =>
            {
                close_numeric_fuzzy(va, vb, opts, extra)
            }
            _ => false,
        };
    }

    // Sensitivity-based extra tolerance for the allowed number error (built
    // from the first argument's numbers, like the JS).
    let fuzzy = if opts.allowed_error_in_numbers > 0.0 {
        build_fuzzy_tol(a, &vars, opts)
    } else {
        None
    };

    let mut rng = SmallRng::seed_from_u64(0x5EED_1234_ABCD_0001);
    let mut num_unequal = 0;
    for scale in BINDING_SCALES {
        for _ in 0..NUMBER_TRIES {
            match find_region(a, b, &vars, scale, &mut rng, opts, fuzzy.as_ref()) {
                Region::Equal => return true,
                Region::Unequal => {
                    num_unequal += 1;
                    if num_unequal > NUMBER_TRIES {
                        return false;
                    }
                }
                Region::Skip => {}
            }
        }
    }
    false
}

enum Region {
    Equal,
    Unequal,
    Skip,
}

/// Sample a base point at radius `scale`; if both sides agree there, confirm
/// across a tight neighborhood (`scale / 100`). `Equal` iff ≥ `MINIMUM_MATCHES`
/// neighborhood points are usable and agree; `Unequal` if the base or any
/// neighborhood point disagrees; `Skip` if too few points are usable.
fn find_region(
    a: &Expr,
    b: &Expr,
    vars: &[String],
    scale: f64,
    rng: &mut SmallRng,
    opts: &EqOptions,
    fuzzy: Option<&FuzzyTol>,
) -> Region {
    // Extra tolerance from the allowed number error at a given point; a
    // non-evaluable tolerance makes the point disagree (JS parity).
    let extra = |env: &Env| -> Option<f64> {
        match fuzzy {
            None => Some(0.0),
            Some(f) => f.at(env),
        }
    };

    let base = sample_point(vars, scale, None, rng, opts.real_only);
    let (Some(va), Some(vb)) = (eval_complex(a, &base), eval_complex(b, &base)) else {
        return Region::Skip;
    };
    if !usable(va, vb) {
        return Region::Skip;
    }
    let Some(tol_extra) = extra(&base) else {
        return Region::Unequal;
    };
    if !close_numeric_fuzzy(va, vb, opts, tol_extra) {
        return Region::Unequal;
    }

    let mut finite_tries = 0;
    for _ in 0..100 {
        let near = sample_point(vars, scale / 100.0, Some(&base), rng, opts.real_only);
        let (Some(va2), Some(vb2)) = (eval_complex(a, &near), eval_complex(b, &near)) else {
            continue;
        };
        if !usable(va2, vb2) {
            continue;
        }
        finite_tries += 1;
        let Some(tol_extra2) = extra(&near) else {
            return Region::Unequal;
        };
        if !close_numeric_fuzzy(va2, vb2, opts, tol_extra2) {
            return Region::Unequal;
        }
        if finite_tries >= MINIMUM_MATCHES {
            return Region::Equal;
        }
    }
    Region::Skip
}

/// A sample point is usable if both values are finite, in bounds, and nonzero.
/// An exact `0.0` from a *variable* expression is underflow (canonicalization
/// folds genuine zero functions before this stage), and letting it count —
/// whether as a both-zero "agreement" or a one-sided `tolerance_for_zero`
/// match — accepts distinct functions that underflow across a region
/// (`x^sin(x)` vs `x^cos(x)`, or vs a literal `0`). Note this makes an
/// unsimplified identically-zero expression (e.g. `sin²x+cos²x−1`) unprovable
/// against `0` at this stage; JS decides that pair in its *simplify* stage
/// (Pythagorean rewrite — not yet ported), not numerically.
fn usable(va: Complex64, vb: Complex64) -> bool {
    va.re.is_finite()
        && va.im.is_finite()
        && vb.re.is_finite()
        && vb.im.is_finite()
        && va.norm() < MAX_VALUE
        && vb.norm() < MAX_VALUE
        && va.norm() > 0.0
        && vb.norm() > 0.0
}

/// Sample each variable uniformly in a `scale`-radius complex box, optionally
/// centered on a prior point (for neighborhood probing). Mirrors JS
/// `randomComplexBindings`.
pub(super) fn sample_point(
    vars: &[String],
    scale: f64,
    center: Option<&Env>,
    rng: &mut SmallRng,
    real_only: bool,
) -> Env {
    vars.iter()
        .map(|v| {
            let c = center
                .and_then(|c| c.get(v).copied())
                .unwrap_or(Complex64::new(0.0, 0.0));
            let re = c.re + rng.random_range(-scale..scale);
            let im = if real_only {
                0.0
            } else {
                c.im + rng.random_range(-scale..scale)
            };
            (v.clone(), Complex64::new(re, im))
        })
        .collect()
}

/// Tolerance test matching JS `find_equality_region`, plus the
/// allowed number error. JS ordering: `tol = extra + min_mag·rel`, capped at
/// 10% of the smaller magnitude, then the zero/absolute adjustment.
pub(super) fn close_numeric_fuzzy(va: Complex64, vb: Complex64, opts: &EqOptions, extra: f64) -> bool {
    let min_mag = va.norm().min(vb.norm());
    let max_mag = va.norm().max(vb.norm());
    if max_mag == 0.0 {
        return true;
    }
    let mut tol = (extra + min_mag * opts.relative_tolerance).min(0.1 * min_mag);
    if tol == 0.0 && (va.norm() == 0.0 || vb.norm() == 0.0) {
        tol += opts.tolerance_for_zero;
    } else {
        tol += opts.absolute_tolerance;
    }
    (va - vb).norm() < tol
}
