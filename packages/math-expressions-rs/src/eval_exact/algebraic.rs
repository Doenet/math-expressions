//! RootOf algebraic identities: a certified zero test for expressions built
//! from rationals and a *single* `RootOf` leaf by `+`, `*`, and nonnegative
//! integer powers, via reduction in `ℚ[t]/(p)`.

use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};

use super::value::spend;
use crate::assumptions::MaybeBool;
use crate::expr::Expr;

/// Certified zero test for an expression built from rationals and a *single*
/// `RootOf` leaf `α` by `+`, `*`, and nonnegative integer powers.
///
/// The value is folded into `ℚ[t]/(p)`, where `p` is `α`'s defining
/// polynomial, by reducing modulo `p` at every step. If the result is the zero
/// polynomial then the value is `q(t)·p(t)` for some `q`, so it vanishes at
/// `α` (`p(α) = 0`) — sound **regardless of whether `p` is irreducible**. The
/// converse needs irreducibility, so a nonzero remainder yields `None`, never
/// `Some(false)`.
pub(super) fn rootof_is_zero(e: &Expr) -> MaybeBool {
    let root = unique_rootof(e)?;
    let Expr::RootOf { poly, .. } = &root else {
        return None;
    };
    let p = crate::polynomials::rootof::coeffs_to_upoly(poly)?;
    if crate::polynomials::univariate::degree(&p) == 0 {
        return None;
    }
    let mut budget = crate::resource_limits::current().max_exact_eval_ops;
    let r = fold_rootof(e, &p, &mut budget)?;
    crate::polynomials::univariate::is_zero(&r).then_some(true)
}

/// The one distinct `RootOf` leaf in `e`, or `None` if there are none or more
/// than one (a compositum of distinct algebraics is not supported).
fn unique_rootof(e: &Expr) -> Option<Expr> {
    fn walk(e: &Expr, found: &mut Option<Expr>, multiple: &mut bool) {
        if let Expr::RootOf { .. } = e {
            match found {
                None => *found = Some(e.clone()),
                Some(prev) if prev == e => {}
                Some(_) => *multiple = true,
            }
        }
        for c in e.children() {
            walk(c, found, multiple);
        }
    }
    let (mut found, mut multiple) = (None, false);
    walk(e, &mut found, &mut multiple);
    (!multiple).then_some(found).flatten()
}

/// Fold `e` into its coefficient vector in `ℚ[t]/(p)` (low → high), reducing
/// modulo `p` after each operation.
fn fold_rootof(e: &Expr, p: &[BigRational], budget: &mut i64) -> Option<Vec<BigRational>> {
    spend(budget)?;
    let reduce = |a: Vec<BigRational>| crate::polynomials::univariate::divrem(&a, p).1;
    Some(match e {
        Expr::Num(n) => vec![n.to_bigrational()?],
        Expr::RootOf { .. } => reduce(vec![BigRational::zero(), BigRational::one()]),
        Expr::Add(ts) => {
            let mut acc = Vec::new();
            for t in ts {
                acc = crate::polynomials::univariate::add_p(&acc, &fold_rootof(t, p, budget)?);
            }
            reduce(acc)
        }
        Expr::Mul(fs) => {
            let mut acc = vec![BigRational::one()];
            for f in fs {
                acc = reduce(crate::polynomials::univariate::mul(
                    &acc,
                    &fold_rootof(f, p, budget)?,
                ));
            }
            acc
        }
        Expr::Pow(b, k) => {
            let Expr::Num(n) = &**k else { return None };
            let ki = n
                .to_bigrational()
                .and_then(|q| q.is_integer().then(|| q.to_integer()))?
                .to_i64()?;
            if ki < 0 {
                return None;
            }
            let base = fold_rootof(b, p, budget)?;
            let mut acc = vec![BigRational::one()];
            for _ in 0..ki {
                spend(budget)?;
                acc = reduce(crate::polynomials::univariate::mul(&acc, &base));
            }
            acc
        }
        _ => return None,
    })
}
