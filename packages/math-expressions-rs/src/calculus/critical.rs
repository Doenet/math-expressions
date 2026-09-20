//! Exact critical points: the real solutions of `f'(x) = 0`.
//!
//! Numerical extremum hunting brackets a grid and refines each bracket, which
//! is where the classic failures live — a bracket that straddles two roots
//! reports one, a flat region reports a spurious one, and a pole next to a
//! genuine extremum can swallow it. Where the derivative is a *rational
//! function* none of that is necessary: the critical points are the roots of a
//! polynomial, and this module returns them exactly (a rational, or a `RootOf`
//! carrying its defining polynomial), in increasing order.
//!
//! The contract is deliberately three-valued, because a caller that still owns
//! a numerical fallback needs to tell "none exist" from "I could not decide":
//!
//! * `Some(points)` — the complete list, exactly. `Some(vec![])` means there
//!   are provably none.
//! * `None` — outside this method's reach; sample instead.
//!
//! Out of reach, specifically: a derivative that is not rational in `var`
//! (`cos(x)`, with infinitely many roots anyway), one carrying a free parameter
//! (`d/dx a·x² = 2ax`, whose roots depend on `a`), and a *constant zero*
//! derivative, where every point is critical and no finite list says so. Points
//! where `f'` fails to exist — the corner of `|x|` — are not reported either;
//! they are critical in the textbook sense, but finding them is not rational
//! root-finding and pretending otherwise would make the `Some` case a lie.

use crate::expr::{sym::is_constant_symbol, Expr};
use crate::num::Number;
use crate::ops::variables;
use crate::polynomials::{
    factor::extract_upoly,
    ratform::rational_normal,
    rootof::{make_rootof, numeric_root},
    univariate::{self, UPoly},
};

/// The critical points of `e` with respect to `var`, exactly, in increasing
/// order — or `None` where the method does not reach (see the module docs).
pub fn critical_points(e: &Expr, var: &str) -> Option<Vec<Expr>> {
    let d = crate::normalize::simplify(&crate::calculus::diff::derivative(e, var));

    // A derivative with no variable left in it is decided here, before the
    // rational machinery, which has no indeterminates to work with and would
    // decline: a non-zero constant slope never vanishes, and a zero one means
    // every point is critical.
    if !variables(&d).iter().any(|v| !is_constant_symbol(v)) {
        let value = crate::ops::evaluate_to_constant(&d)?;
        return (value != num_complex::Complex64::new(0.0, 0.0)).then(Vec::new);
    }

    // Over a common denominator and reduced, so a pole cannot masquerade as a
    // root: `rational_normal` cancels the gcd, so any factor shared by
    // numerator and denominator is already gone.
    let (num, _den) = rational_normal(&d)?;

    // Every remaining variable must be `var`: a free parameter would make the
    // answer symbolic (`d/dx a·x² = 2ax`), so decline. A non-rational subtree
    // (`sin(x)`) is not caught here — `rational_normal` restores it rather than
    // leaving a bare kernel symbol, so it reads as a function of `var` — but
    // `extract_upoly` below rejects it, which is where that case is decided.
    if variables(&num)
        .iter()
        .any(|v| v != var && !is_constant_symbol(v))
    {
        return None;
    }

    let poly = extract_upoly(
        &crate::normalize::canonicalize(&crate::normalize::expand(&num)),
        var,
    )?;
    match univariate::degree(&poly) {
        // A zero derivative is a constant function: every point is critical,
        // which no finite list can report.
        _ if univariate::is_zero(&poly) => None,
        // A non-zero constant derivative never vanishes.
        0 => Some(Vec::new()),
        _ => real_roots(&poly),
    }
}

/// The distinct real roots of `p`, exactly, in increasing order.
///
/// Squarefree decomposition first, so a repeated root is reported once — `x³`
/// has the single critical point `0`, not a double one. Within each squarefree
/// factor the rational roots come out as numbers and the rest stay as `RootOf`,
/// which is how the rest of the engine spells an algebraic number.
fn real_roots(p: &UPoly) -> Option<Vec<Expr>> {
    let mut out: Vec<(f64, Expr)> = Vec::new();
    for (sqfree, _mult) in univariate::squarefree_decomposition(p) {
        if univariate::degree(&sqfree) < 1 {
            continue;
        }
        let (rationals, cofactor) = univariate::rational_roots(&sqfree);
        for r in rationals {
            let n = Number::from_bigrational(r);
            out.push((n.to_f64(), Expr::Num(n)));
        }
        if univariate::degree(&cofactor) < 1 {
            continue;
        }
        // `make_rootof` normalizes the polynomial it stores, so the indices
        // below are indices into *its* root ordering, not the cofactor's.
        let Some(first) = make_rootof(&cofactor, 0) else {
            // Past the degree cap: the roots exist but cannot be named, and
            // silently dropping them would understate the answer.
            return None;
        };
        let Expr::RootOf { poly, .. } = &first else {
            unreachable!("make_rootof returns a RootOf")
        };
        for index in 0..(poly.len().saturating_sub(1)) {
            let z = numeric_root(poly, index as u32)?;
            // Complex roots are not critical points of a real function. The
            // tolerance is scaled, matching how the evaluator decides realness.
            if z.im.abs() > 1e-10 * z.re.abs().max(1.0) {
                continue;
            }
            out.push((
                z.re,
                Expr::RootOf {
                    poly: poly.clone(),
                    index: index as u32,
                },
            ));
        }
    }
    out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    Some(out.into_iter().map(|(_, e)| e).collect())
}
