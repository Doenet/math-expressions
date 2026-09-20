//! The certified zero-equivalence service: `is_zero(e, a) -> MaybeBool`
//! (`Some(true)` = certified zero, `Some(false)` = certified nonzero,
//! `None` = undecided). Soundness is the invariant: it never answers `Some(_)`
//! unless the answer is certain.

use num_rational::BigRational;

use super::algebraic::rootof_is_zero;
use super::eval::exact_eval;
use crate::assumptions::{Assumptions, MaybeBool};
use crate::expr::Expr;
use crate::num::Number;

/// Certified test for `e ≡ 0`: `Some(true)` = provably zero, `Some(false)` =
/// provably nonzero, `None` = undecided.
///
/// The `_a` assumptions are accepted for forward compatibility (sign/realness
/// reasoning is not yet implemented) but not yet consulted.
pub fn is_zero(e: &Expr, _a: &Assumptions) -> MaybeBool {
    let c = crate::normalize::canonicalize(&crate::normalize::expand(e));
    let vars = free_vars(&c);
    if let Some(v) = certify_canonical(&c, &vars) {
        return Some(v);
    }
    if vars.is_empty() {
        None
    } else {
        // (c) certified refuter: a single certified-nonzero sample proves the
        // expression is not identically zero. Sampling can never *confirm*
        // zero, so this stage only ever yields `Some(false)` or `None`.
        refute_by_sampling(&c, &vars)
    }
}

/// Accept-only fast path of [`is_zero`]: `true` iff `e` is *certified*
/// identically zero by the exact stages alone — no numeric sampling. `false`
/// means "not certified", **not** "nonzero". The right gate for callers with
/// their own cheaper rejection test (e.g. the integration gate), since the
/// sampling refuter burns its full arbitrary-precision budget precisely when
/// the expression *is* zero.
pub(crate) fn certified_zero(e: &Expr, _a: &Assumptions) -> bool {
    let c = crate::normalize::canonicalize(&crate::normalize::expand(e));
    let vars = free_vars(&c);
    certify_canonical(&c, &vars) == Some(true)
}

/// The non-sampling certification pipeline on a canonical, expanded input:
/// (a) structural cancellation, (d) rational normal form, (b) exact constant
/// evaluation (variable-free only — the one stage that can also certify
/// *nonzero*), then the RootOf reducer. `None` = undecided.
fn certify_canonical(c: &Expr, vars: &[String]) -> MaybeBool {
    // (a) structural: expand + canonicalize caught polynomial identities.
    if matches!(c, Expr::Num(n) if n.is_zero()) {
        return Some(true);
    }
    // (d) rational-function normalization (S2): a rational identity whose
    // combined numerator cancels to zero is certified zero — this decides
    // `1/(x+1) + 1/(x-1) - 2x/(x²-1)` and the like, treating opaque kernels as
    // independent indeterminates (sound: never `true` for a nonzero value).
    if crate::polynomials::ratform::is_identically_zero(c) {
        return Some(true);
    }
    if vars.is_empty() {
        // (b) exact constant evaluation. Failure falls through to the RootOf
        // decider, then to Unknown (adversarial almost-zeros land there —
        // never a wrong `Some(true)`).
        if let Some(v) = exact_eval(c) {
            return Some(v.is_zero());
        }
        return rootof_is_zero(c);
    }
    None
}

/// The non-constant free variable names of `c`.
fn free_vars(c: &Expr) -> Vec<String> {
    crate::ops::variables(c)
        .into_iter()
        .filter(|v| !crate::expr::sym::is_constant_symbol(v))
        .collect()
}

/// Deterministic "random" rational sample points, chosen to dodge common
/// removable structure (small integers, halves) and singularities at 0.
const SAMPLE_POINTS: &[(i64, i64)] = &[(7, 3), (-11, 5), (13, 4), (2, 7), (-17, 6), (23, 8)];

fn refute_by_sampling(e: &Expr, vars: &[String]) -> MaybeBool {
    use std::collections::HashMap;
    for round in 0..SAMPLE_POINTS.len() {
        let subs: HashMap<String, Expr> = vars
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let (n, d) = SAMPLE_POINTS[(round + i) % SAMPLE_POINTS.len()];
                (
                    v.clone(),
                    Expr::Num(Number::from_bigrational(BigRational::new(
                        n.into(),
                        d.into(),
                    ))),
                )
            })
            .collect();
        let at = crate::ops::substitute(e, &subs);
        match crate::eval_numeric::certified_digits::evaluate_to_precision(&at, 15) {
            crate::eval_numeric::certified_digits::Precise::Exact(n) if !n.is_zero() => {
                return Some(false)
            }
            crate::eval_numeric::certified_digits::Precise::Bounded(m) if certified_nonzero(&m) => {
                return Some(false)
            }
            _ => {}
        }
    }
    None
}

/// The ±1-ulp arbitrary-precision contract, via the single shared test on
/// `MpFix` (see `MpFix::excludes_zero`).
fn certified_nonzero(m: &crate::eval_numeric::certified_digits::fix::MpFix) -> bool {
    m.excludes_zero()
}
