//! Sign-error-tolerant comparison: does the answer match apart from `n`
//! misplaced minus signs?

use crate::equality::{equals, EqOptions};
use crate::expr::map_children;
use crate::expr::Expr;

/// Does `a` equal `b` after exactly `n` sign flips of subtrees of `a`?
/// `n = 0` is plain [`equals`]. Port of `equalSpecifiedSignErrors`: every
/// single-position negation is tried (root and every operand at every depth),
/// recursively for `n > 1`.
pub fn equal_specified_sign_errors(a: &Expr, b: &Expr, opts: &EqOptions, n: u32) -> bool {
    if n == 0 {
        return equals(a, b, opts);
    }
    single_negations(a)
        .iter()
        .any(|variant| equal_specified_sign_errors(variant, b, opts, n - 1))
}

/// Match with up to `max_sign_errors` sign errors; returns the smallest number
/// of errors that makes the pair equal (`Some(0)` = plainly equal), or `None`.
/// Port of `equalWithSignErrors`.
pub fn equal_with_sign_errors(
    a: &Expr,
    b: &Expr,
    opts: &EqOptions,
    max_sign_errors: u32,
) -> Option<u32> {
    (0..=max_sign_errors).find(|&n| equal_specified_sign_errors(a, b, opts, n))
}

/// Every tree obtained from `e` by negating exactly one subtree (including
/// the whole tree).
fn single_negations(e: &Expr) -> Vec<Expr> {
    let mut out = vec![Expr::Neg(Box::new(e.clone()))];
    let n_children = e.children().len();
    for i in 0..n_children {
        let child = e.children()[i].clone();
        for cv in single_negations(&child) {
            let mut idx = 0usize;
            out.push(map_children(e, |c| {
                let r = if idx == i { cv.clone() } else { c.clone() };
                idx += 1;
                r
            }));
        }
    }
    out
}
