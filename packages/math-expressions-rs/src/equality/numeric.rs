//! The numerical stage: prove equality by finding one small neighborhood where
//! both functions agree at several clustered points (JS `find_equality_region`).

use super::fuzzy::{build_fuzzy_tol, FuzzyTol};
use super::seedrandom::SeedRandom;
use super::EqOptions;
use crate::eval_numeric::complex::{eval_complex, free_symbols, Env};
use crate::expr::Expr;
use num_complex::Complex64;

// JS numerical-equality constants (lib/expression/equality/numerical.js).
/// Clustered agreeing points needed to accept a region.
pub(super) const MINIMUM_MATCHES: usize = 10;
/// Disagreeing base points tolerated before rejecting — branch-cut identities
/// disagree at many points, so this must be generous.
pub(super) const NUMBER_TRIES: usize = 100;
/// Base-point sampling radii, largest-first so a non-identity reveals its
/// global disagreement before small scales probe near the origin.
///
/// [`equals_numerical`] reaches only the *first*, deliberately — see the note
/// on its sampling loop. The ± stage
/// ([`super::plus_minus::pm_multiset_equals`]) still cycles the whole list,
/// which it flags as a known divergence from the JS.
pub(super) const BINDING_SCALES: [f64; 6] = [10.0, 1.0, 100.0, 0.1, 1000.0, 0.01];
/// Radius of the cluster probed around an agreeing base point — **fixed**, not
/// a fraction of the base-point scale.
///
/// This is `noninteger_binding_scale / 100` in the JS, where
/// `noninteger_binding_scale` is initialised to 1 and never assigned again, so
/// the neighbourhood is always 0.01 however far out the base point was drawn.
/// It reads like it was meant to track the base scale, but it does not, and the
/// difference is not cosmetic: at the default scale of 10 a `scale / 100`
/// neighbourhood is **ten times wider**, so a base point sitting inside a
/// narrow agreeing region will usually have a neighbour outside it and the
/// whole region is thrown away. That cost real grading agreement — see the
/// `errorInNumbers` note in the DoenetML tree.
pub(super) const NEIGHBORHOOD_RADIUS: f64 = 0.01;
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

    // Seeded exactly as the JS stage this mirrors: `equalsViaComplex` uses
    // `seedrandom("complex_seed")` and `equalsViaReal` uses `"real_seed"`.
    // Sharing the JS generator is what makes borderline grading decisions
    // reproducible rather than a coin flip — see `seedrandom`.
    let mut rng = SeedRandom::new(if opts.real_only {
        "real_seed"
    } else {
        "complex_seed"
    });
    let mut num_unequal = 0;
    // One flat budget of `10 · NUMBER_TRIES` attempts, all at the *first*
    // binding scale, ending early once `NUMBER_TRIES` base points have
    // positively disagreed. Points that are merely unusable (out of bounds,
    // underflowed, non-evaluable) do not count against the budget, which is why
    // the attempt count is ten times the disagreement count.
    //
    // The scale only advances in the JS for the all-zero case, which this
    // implementation excludes at `usable` instead — so in practice the search
    // never leaves radius 10. Iterating the remaining scales here was not a
    // harmless generalisation: at radius 1 a response whose error is twice the
    // allowed amount agrees over most of the sampled range, so a wrong answer
    // that radius 10 correctly rejects gets accepted a scale later. The budget
    // ran out first, so nothing depended on it, but it was a live hazard.
    // Variables the assumptions prove integer are sampled over the integers,
    // not the complex disk (JS `integer_variables`): `(-1)^n·(-1)^n` equals `1`
    // only because `n ∈ Z`. Empty unless a caller supplied an assumption store.
    let integer_vars: Vec<bool> = vars
        .iter()
        .map(|v| crate::is_integer(&Expr::sym(v), &opts.assumptions) == Some(true))
        .collect();

    let scale = BINDING_SCALES[0];
    for _ in 0..(10 * NUMBER_TRIES) {
        match find_region(
            a,
            b,
            &vars,
            scale,
            &mut rng,
            opts,
            fuzzy.as_ref(),
            &integer_vars,
        ) {
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
    false
}

enum Region {
    Equal,
    Unequal,
    Skip,
}

/// Sample a base point at radius `scale`; if both sides agree there, confirm
/// across a tight cluster of radius [`NEIGHBORHOOD_RADIUS`]. `Equal` iff ≥
/// `MINIMUM_MATCHES` neighborhood points are usable and agree; `Unequal` if the
/// base or any neighborhood point disagrees; `Skip` if too few points are
/// usable.
// A private sampler helper; the arguments are the point parameters, not
// distinct concerns worth grouping into a struct.
#[allow(clippy::too_many_arguments)]
fn find_region(
    a: &Expr,
    b: &Expr,
    vars: &[String],
    scale: f64,
    rng: &mut SeedRandom,
    opts: &EqOptions,
    fuzzy: Option<&FuzzyTol>,
    integer: &[bool],
) -> Region {
    // Extra tolerance from the allowed number error at a given point; a
    // non-evaluable tolerance makes the point disagree (JS parity).
    let extra = |env: &Env| -> Option<f64> {
        match fuzzy {
            None => Some(0.0),
            Some(f) => f.at(env),
        }
    };

    let base = sample_point(vars, scale, None, rng, opts.real_only, integer);
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
        let near = sample_point(
            vars,
            NEIGHBORHOOD_RADIUS,
            Some(&base),
            rng,
            opts.real_only,
            integer,
        );
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
    rng: &mut SeedRandom,
    real_only: bool,
    integer: &[bool],
) -> Env {
    // `rng() * 2 * radius - radius`, one draw per real coordinate, taken in
    // variable order — the JS `randomRealBindings` / `randomComplexBindings`.
    // The arithmetic is spelled their way rather than as a range sample so the
    // same stream yields the same points.
    let mut binding: Vec<(String, Complex64)> = {
        let mut draw = |c: f64| c + rng.next_f64() * 2.0 * scale - scale;
        vars.iter()
            .map(|v| {
                let c = center
                    .and_then(|c| c.get(v).copied())
                    .unwrap_or(Complex64::new(0.0, 0.0));
                let re = draw(c.re);
                let im = if real_only { 0.0 } else { draw(c.im) };
                (v.clone(), Complex64::new(re, im))
            })
            .collect()
    };
    // Then overwrite each integer-assumed variable with a fresh random integer
    // in [−10, 10], ignoring the center — JS `generate_random_integer`, applied
    // after all complex coordinates are drawn (so the untouched draws keep the
    // stream aligned) and in both the base and neighborhood passes.
    for (i, slot) in binding.iter_mut().enumerate() {
        if integer.get(i).copied().unwrap_or(false) {
            let k = (rng.next_f64() * 21.0).floor() - 10.0;
            slot.1 = Complex64::new(k, 0.0);
        }
    }
    binding.into_iter().collect()
}

/// Tolerance test matching JS `find_equality_region`, plus the
/// allowed number error. JS ordering: `tol = extra + min_mag·rel`, capped at
/// 10% of the smaller magnitude, then the zero/absolute adjustment.
pub(super) fn close_numeric_fuzzy(
    va: Complex64,
    vb: Complex64,
    opts: &EqOptions,
    extra: f64,
) -> bool {
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
