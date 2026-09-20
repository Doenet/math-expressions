//! The vocabulary the four folding modules share: which names count as trig,
//! and the small predicates and constructors they all reach for.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Signed;

use crate::expr::Expr;

pub(super) const TRIG: &[&str] = &["sin", "cos", "tan", "cot", "sec", "csc"];
pub(super) const INVERSE_TRIG: &[&str] = &["asin", "acos", "atan", "asec", "acsc", "acot"];

pub(super) fn canon(e: &Expr) -> Expr {
    crate::normalize::canonicalize(e)
}

pub(super) fn apply(name: &str, arg: Expr) -> Expr {
    Expr::Apply(Box::new(Expr::sym(name)), vec![arg])
}

pub(super) fn negate(e: &Expr) -> Expr {
    canon(&crate::normalize::mul(vec![Expr::int(-1), e.clone()]))
}

/// `−e`, when `e` is *syntactically* negated: a negative number, or a `Mul`
/// with a negative leading coefficient.
///
/// Stricter than [`neg_leading`] on purpose. Its result is guaranteed not to be
/// negated in turn (`Mul(−1, x) → x`, `Mul(−2, x) → Mul(2, x)`), so a rewrite
/// keyed on it cannot ping-pong inside the surrounding fixpoint — which a sum
/// like `−x−1` would do under the looser test, since negating it just parks a
/// `−1` in front of the sum.
pub(super) fn strip_negation(e: &Expr) -> Option<Expr> {
    let leading_is_negative = match e {
        Expr::Num(n) => n.to_bigrational().is_some_and(|q| q.is_negative()),
        Expr::Mul(fs) => matches!(fs.first(), Some(Expr::Num(n))
            if n.to_bigrational().is_some_and(|q| q.is_negative())),
        _ => false,
    };
    leading_is_negative.then(|| negate(e))
}

/// Heuristic "is this expression negative-leading" test for parity extraction.
pub(super) fn neg_leading(e: &Expr) -> bool {
    match e {
        Expr::Num(n) => n.to_bigrational().is_some_and(|q| q.is_negative()),
        Expr::Neg(_) => true,
        Expr::Mul(fs) => fs.first().is_some_and(neg_leading),
        Expr::Add(ts) => ts.first().is_some_and(neg_leading),
        _ => false,
    }
}

pub(super) use crate::constant_policy::{is_e, is_i, is_pi};

pub(super) fn is_zero_expr(e: &Expr) -> bool {
    matches!(canon(e), Expr::Num(n) if n.is_zero())
}

pub(super) fn is_one_expr(e: &Expr) -> bool {
    matches!(canon(e), Expr::Num(n) if n.to_bigrational().is_some_and(|q| q == BigRational::from_integer(BigInt::from(1))))
}
