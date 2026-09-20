//! Rational-function normal form.
//!
//! [`together`] / [`cancel`] put an expression over a single common
//! denominator and reduce numerator and denominator to lowest terms with the
//! multivariate polynomial GCD ([`super::multivariate`]). Non-rational subtrees —
//! `sin x`, `√x`, `π`, `RootOf` leaves, … — are held fixed as opaque
//! [`kernel`](super::kernel)s, which is what lets rational normalization apply
//! *underneath* any function: `1/sin(x) + 1/sin(x)` becomes `2/sin(x)` without
//! the simplifier ever needing to understand `sin`.
//!
//! Kernelization is sound but incomplete (see [`kernel`](super::kernel)), which
//! is the right direction for [`is_identically_zero`]: it can only ever fail to
//! recognize a zero, never claim one.

use num_traits::One;

use crate::expr::Expr;
use crate::num::Number;
use crate::polynomials;
use crate::polynomials::kernel::{indeterminates, kernelize, Kernels};

/// Maximum distinct indeterminates (real variables + kernels) the dense
/// recursive polynomial model will accept before we bail to the unchanged
/// form. Guards the 2ᵏ blow-up of `k` independent linear denominators (e.g.
/// `∑ 1/(xᵢ+1)`), which the per-variable degree cap alone does not catch.
const MAX_INDETERMINATES: usize = 6;

/// Combine `e` into a single reduced fraction `num/den` in lowest terms, with
/// opaque kernels held fixed. Returns `e` canonicalized unchanged when the
/// rational structure is too large to normalize within the caps.
pub fn together(e: &Expr) -> Expr {
    match rational_normal(e) {
        Some((num, den)) if is_one_expr(&den) => num,
        Some((num, den)) => {
            crate::normalize::canonicalize(&Expr::Div(Box::new(num), Box::new(den)))
        }
        None => crate::normalize::canonicalize(e),
    }
}

/// Cancel the common factors of a ratio, reducing it to lowest terms. For this
/// normal form it coincides with [`together`] (both return the coprime single
/// fraction); kept as a separate name to mirror the CAS vocabulary and to give
/// callers that only want cancellation a clear entry point.
pub fn cancel(e: &Expr) -> Expr {
    together(e)
}

/// Certified test used by `eval_exact::is_zero` stage (d): `true` iff `e` normalizes
/// to a zero numerator over a nonzero denominator, i.e. `e ≡ 0` as a rational
/// function in its variables and kernels. Never `true` for a non-zero `e`.
pub(crate) fn is_identically_zero(e: &Expr) -> bool {
    matches!(rational_normal(e), Some((num, _)) if is_zero_expr(&num))
}

/// The reduced `(numerator, denominator)` pair as canonical, kernel-restored
/// expressions, or `None` when the input is outside the caps.
pub(crate) fn rational_normal(e: &Expr) -> Option<(Expr, Expr)> {
    let canon = crate::normalize::canonicalize(e);

    // Replace opaque (non-rational) subtrees with fresh kernel symbols.
    let mut kernels = Kernels::default();
    let ke = kernelize(&canon, &mut kernels);

    let vars = indeterminates(&ke);
    if vars.is_empty() || vars.len() > MAX_INDETERMINATES {
        return None;
    }

    // (num, den) as polynomial expressions over `vars` (+ kernels).
    let (num_e, den_e) = rational_parts(&ke)?;
    if is_zero_expr(&den_e) {
        return None; // 0 denominator — undefined, refuse
    }
    let pn = polynomials::expr_to_poly(&crate::normalize::canonicalize(&num_e), &vars)?;
    let pd = polynomials::expr_to_poly(&crate::normalize::canonicalize(&den_e), &vars)?;

    // Cancel gcd, then normalize rational content onto the numerator (so
    // `(2x+4)/2` → `x+2`, matching `reduce_rational`).
    let g = polynomials::gcd(&pn, &pd, vars.len())?;
    let (pn, pd) = if polynomials::is_trivial(&g) {
        (pn, pd)
    } else {
        (
            polynomials::exact_div_top(&pn, &g, vars.len())?,
            polynomials::exact_div_top(&pd, &g, vars.len())?,
        )
    };
    let (cn, pn) = polynomials::strip_rational_content(&pn);
    let (cd, pd) = polynomials::strip_rational_content(&pd);
    let scalar = Expr::Num(Number::from_bigrational(cn / cd));

    let num = crate::normalize::mul(vec![scalar, polynomials::poly_to_expr(&pn, &vars)]);
    let den = polynomials::poly_to_expr(&pd, &vars);

    // Restore the kernels and canonicalize both halves.
    let num = crate::normalize::canonicalize(&kernels.restore(&num));
    let den = crate::normalize::canonicalize(&kernels.restore(&den));
    Some((num, den))
}

/// `(numerator, denominator)` of a *kernelized* expression as polynomial trees
/// (no `Div`, no negative powers). Builds them with the canonical constructors
/// so the result feeds straight into `expr_to_poly`. `None` on a non-rational
/// node (should not occur post-kernelization) or a term-count breach.
fn rational_parts(e: &Expr) -> Option<(Expr, Expr)> {
    let one = || Expr::int(1);
    Some(match e {
        Expr::Num(_) | Expr::Sym(_) => (e.clone(), one()),
        Expr::Neg(a) => {
            let (n, d) = rational_parts(a)?;
            (neg(n), d)
        }
        Expr::Add(ts) => {
            if ts.len() > crate::resource_limits::current().max_ratform_terms {
                return None;
            }
            let mut acc = (Expr::int(0), one());
            for t in ts {
                let (n, d) = rational_parts(t)?;
                // acc = (acc.n·d + n·acc.d) / (acc.d·d)
                let num = crate::normalize::add(vec![
                    crate::normalize::mul(vec![acc.0, d.clone()]),
                    crate::normalize::mul(vec![n, acc.1.clone()]),
                ]);
                let den = crate::normalize::mul(vec![acc.1, d]);
                acc = (num, den);
            }
            acc
        }
        Expr::Mul(fs) => {
            let (mut num, mut den) = (one(), one());
            for f in fs {
                let (n, d) = rational_parts(f)?;
                num = crate::normalize::mul(vec![num, n]);
                den = crate::normalize::mul(vec![den, d]);
            }
            (num, den)
        }
        Expr::Div(a, b) => {
            let (na, da) = rational_parts(a)?;
            let (nb, db) = rational_parts(b)?;
            (
                crate::normalize::mul(vec![na, db]),
                crate::normalize::mul(vec![da, nb]),
            )
        }
        Expr::Pow(b, k) => {
            let Expr::Num(Number::Int(k)) = &**k else {
                return None;
            };
            let (n, d) = rational_parts(b)?;
            if *k >= 0 {
                (pow_int(n, *k), pow_int(d, *k))
            } else {
                (pow_int(d, -*k), pow_int(n, -*k))
            }
        }
        _ => return None,
    })
}

fn neg(x: Expr) -> Expr {
    crate::normalize::mul(vec![Expr::int(-1), x])
}

fn pow_int(base: Expr, k: i64) -> Expr {
    crate::normalize::pow(base, Expr::int(k))
}

fn is_zero_expr(e: &Expr) -> bool {
    matches!(crate::normalize::canonicalize(e), Expr::Num(n) if n.is_zero())
}

fn is_one_expr(e: &Expr) -> bool {
    matches!(crate::normalize::canonicalize(e), Expr::Num(n) if n.to_bigrational().is_some_and(|q| q.is_one()))
}
