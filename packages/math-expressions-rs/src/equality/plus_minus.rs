//! Plus-minus (±) set equality — port of JS `equality/numerical.js` pm branch
//! and `equality/pm-numerical.js`.
//!
//! `a ± b` denotes the value set `{a+b, a-b}`. Two expressions are pm-equal when
//! their sign-expansions produce matching value collections at every random
//! binding. Relations dispatch by operator: an equation `lhs = rhs` compares the
//! PRODUCT of its (deduplicated) sign-branches proportionally — recovering the
//! scaling invariance of the non-pm `=` path (`x = 5±3` ≡ `2x = 10±6`) — while
//! an inequality (and any non-relation) compares the standard-form value
//! multisets directly.

use super::fuzzy::{build_fuzzy_tol, FuzzyTol};
use super::numeric::{
    close_numeric_fuzzy, sample_point, BINDING_SCALES, MAX_VALUE, MINIMUM_MATCHES, NUMBER_TRIES,
};
use super::relations::{as_comparison, proportional, Comparison};
use super::{equals, EqOptions};
use crate::eval::{eval_complex, free_symbols};
use crate::expr::{Expr, RelOp};
use crate::norm::{canonicalize, simplify_canonical};
use num_complex::Complex64;
use rand::rngs::SmallRng;
use rand::SeedableRng;

pub(super) fn pm_equals(a: &Expr, b: &Expr, opts: &EqOptions) -> bool {
    // Sequences (tuples/vectors/…): compare componentwise, re-entering `equals`
    // on each component so a `±` inside one component is handled numerically
    // there. The whole-sequence multiset path below cannot evaluate a `Seq`
    // (`eval_complex` returns `None`), so without this a pm-bearing tuple would
    // only ever match a structurally identical one. (`ca`/`cb` are already
    // sequence-kind–coerced by `equals`, so equal kinds compare directly.)
    match (a, b) {
        (Expr::Seq(ka, xs), Expr::Seq(kb, ys)) => {
            return ka == kb
                && xs.len() == ys.len()
                && xs.iter().zip(ys).all(|(x, y)| equals(x, y, opts));
        }
        (Expr::Seq(..), _) | (_, Expr::Seq(..)) => return false,
        // `Matrix` and `Interval` are deliberately NOT compared componentwise:
        // `±` is unsupported inside a matrix entry or an interval endpoint (see
        // the `pm` module docs). They fall through to the numeric path below,
        // which cannot evaluate them, so such an expression is equal only to a
        // structurally identical one. Callers should not put `±` there.
        _ => {}
    }

    if let (Some(ca), Some(cb)) = (as_comparison(a), as_comparison(b)) {
        // `as_comparison` already folds `>`/`≥` into `<`/`≤` by swapping sides,
        // so the operators are directly comparable.
        if ca.op != cb.op {
            return false;
        }
        let std_form = |c: Comparison| {
            canonicalize(&Expr::Add(vec![c.lhs, Expr::Neg(Box::new(c.rhs))]))
        };
        let sa = std_form(ca);
        let sb = std_form(cb);
        if a_is_equation(a) {
            // Solution set = zero set of the product of the distinct sign
            // branches; two equations are equal iff those products are
            // proportional (any nonzero factor). Dedup so a vacuous `±0` does
            // not inflate a factor's multiplicity (`x = 5±0` ≡ `x = 5`).
            let (Some(pa), Some(pb)) = (pm_branch_product(&sa), pm_branch_product(&sb)) else {
                return false;
            };
            return proportional(&pa, &pb, false, opts);
        }
        // Inequality: compare the standard-form value multisets.
        return pm_multiset_equals(&sa, &sb, opts);
    }
    // Non-relation: compare the value multisets of the two expressions.
    pm_multiset_equals(a, b, opts)
}

fn a_is_equation(e: &Expr) -> bool {
    matches!(e, Expr::Relation { ops, .. } if ops.as_slice() == [RelOp::Eq])
}

/// Product of the *distinct* sign-expansion branches of `e` (each simplified for
/// dedup). `None` if there are too many `pm` operators to enumerate.
fn pm_branch_product(e: &Expr) -> Option<Expr> {
    let branches = crate::pm::expand_pm_signs(e).ok()?;
    let mut kept: Vec<Expr> = Vec::new();
    for br in branches {
        let s = simplify_canonical(canonicalize(&br));
        if !kept.contains(&s) {
            kept.push(s);
        }
    }
    Some(if kept.len() == 1 {
        kept.pop().unwrap()
    } else {
        Expr::Mul(kept)
    })
}

/// Sample random real bindings and require the sign-expansion value multisets of
/// the two sides to match at every usable point (≥ [`MINIMUM_MATCHES`] matches to
/// accept; any mismatch rejects). Per-variant numeric-error tolerance is built
/// from the `a` (LHS) side only, mirroring `component_equals`.
fn pm_multiset_equals(a: &Expr, b: &Expr, opts: &EqOptions) -> bool {
    let (Ok(a_variants), Ok(b_variants)) =
        (crate::pm::expand_pm_signs(a), crate::pm::expand_pm_signs(b))
    else {
        return false;
    };

    let mut vars = std::collections::BTreeSet::new();
    for e in a_variants.iter().chain(b_variants.iter()) {
        free_symbols(e, &mut vars);
    }
    let vars: Vec<String> = vars.into_iter().collect();

    // Per-variant sensitivity tolerance for the LHS variants (numbers → params).
    let a_tol: Vec<Option<FuzzyTol>> = if opts.allowed_error_in_numbers > 0.0 {
        a_variants
            .iter()
            .map(|v| build_fuzzy_tol(v, &vars, opts))
            .collect()
    } else {
        Vec::new()
    };

    let mut rng = SmallRng::seed_from_u64(0x5EED_1234_ABCD_0003);
    let minimum_matches = if vars.is_empty() { 1 } else { MINIMUM_MATCHES };
    let max_iter = 10 * NUMBER_TRIES;
    let mut matches = 0;

    for i in 0..max_iter {
        let scale = BINDING_SCALES[(i / 20) % BINDING_SCALES.len()];
        // JS `randomBindings` uses real bindings for the pm path.
        let env = sample_point(&vars, scale, None, &mut rng, true);

        let (Some(av), Some(bv)) = (
            a_variants.iter().map(|e| eval_complex(e, &env)).collect::<Option<Vec<_>>>(),
            b_variants.iter().map(|e| eval_complex(e, &env)).collect::<Option<Vec<_>>>(),
        ) else {
            continue;
        };
        if !all_finite_bounded(&av) || !all_finite_bounded(&bv) {
            continue;
        }

        // Per-variant tolerances for the LHS; a non-evaluable one skips the point.
        let tols: Vec<f64> = if a_tol.is_empty() {
            Vec::new()
        } else {
            let mut t = Vec::with_capacity(av.len());
            let mut skip = false;
            for ft in &a_tol {
                match ft {
                    None => t.push(0.0),
                    Some(f) => match f.at(&env) {
                        Some(x) => t.push(x),
                        None => {
                            skip = true;
                            break;
                        }
                    },
                }
            }
            if skip {
                continue;
            }
            t
        };

        if !value_multisets_match(&av, &bv, opts, &tols) {
            return false;
        }
        matches += 1;
        if matches >= minimum_matches {
            return true;
        }
    }
    false
}

fn all_finite_bounded(vs: &[Complex64]) -> bool {
    vs.iter()
        .all(|v| v.re.is_finite() && v.im.is_finite() && v.norm() < MAX_VALUE)
}

/// Do the two value collections coincide within tolerance? Equal cardinality
/// requires a perfect one-to-one pairing (maximum bipartite matching, so a loose
/// early variant can't consume the only value a tighter later one needs);
/// unequal cardinality (different `pm` counts, e.g. a vacuous `±0`) falls back to
/// mutual set cover. Tolerances are keyed on the LHS (`a`) index only.
fn value_multisets_match(
    a: &[Complex64],
    b: &[Complex64],
    opts: &EqOptions,
    tol_a: &[f64],
) -> bool {
    let can_match = |i: usize, j: usize| {
        close_numeric_fuzzy(a[i], b[j], opts, tol_a.get(i).copied().unwrap_or(0.0))
    };

    if a.len() != b.len() {
        return (0..a.len()).all(|i| (0..b.len()).any(|j| can_match(i, j)))
            && (0..b.len()).all(|j| (0..a.len()).any(|i| can_match(i, j)));
    }

    // Maximum bipartite matching (Kuhn's). The `close` relation is recomputed on
    // demand rather than materialized into an n×n matrix — with up to `2^10`
    // variants per side that matrix would be a megabyte allocated on *every*
    // sample point. For the usual case (random samples are well separated, so
    // the graph is already a near-perfect matching) each `augment` succeeds on
    // its first edge, so this stays near-linear; the dominant cost of the pm
    // path is evaluating the variants, not the matching.
    let n = a.len();
    let mut match_b = vec![usize::MAX; n];
    for i in 0..n {
        let mut visited = vec![false; n];
        if !augment(i, n, &can_match, &mut match_b, &mut visited) {
            return false;
        }
    }
    true
}

/// Kuhn's augmenting-path step, with the edge relation supplied lazily.
fn augment(
    i: usize,
    n: usize,
    can_match: &impl Fn(usize, usize) -> bool,
    match_b: &mut [usize],
    visited: &mut [bool],
) -> bool {
    for j in 0..n {
        if can_match(i, j) && !visited[j] {
            visited[j] = true;
            if match_b[j] == usize::MAX || augment(match_b[j], n, can_match, match_b, visited) {
                match_b[j] = i;
                return true;
            }
        }
    }
    false
}
