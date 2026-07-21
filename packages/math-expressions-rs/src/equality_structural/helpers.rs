//! Shared primitives the predicates lean on: integer/fraction extraction, gcd
//! and power-free tests, numeric content, sign peeling, and symbol counting.
//!
//! Every check that reasons about a leading sign goes through [`split_sign`], so
//! the spellings of "negative" — a `Neg` wrapper, a negative numeric literal, a
//! fraction with a negative part (`Div(-1, 2)`) — are handled in one place.

use crate::expr::Expr;
use crate::num::Number;

pub(super) fn as_int(e: &Expr) -> Option<i64> {
    match e {
        Expr::Num(Number::Int(i)) => Some(*i),
        _ => None,
    }
}

/// The (numerator, denominator) of a **written** fraction — a `Div` of two
/// integers. A decimal literal (`Num(Rat)`, e.g. `2.5`) is *not* a written
/// fraction `a/b`, so it returns `None` (keeping the decimal-vs-fraction
/// distinction the whole module rests on).
pub(super) fn int_div(e: &Expr) -> Option<(i64, i64)> {
    match e {
        Expr::Div(a, b) => Some((as_int(a)?, as_int(b)?)),
        _ => None,
    }
}

/// A signed integer term, folding a leading `Neg` (`-2` and `Neg(2)` → `-2`).
pub(super) fn signed_int(e: &Expr) -> Option<i64> {
    let (neg, rest) = split_sign(e);
    let v = as_int(rest)?;
    Some(if neg { -v } else { v })
}

/// A signed written fraction, folding a leading `Neg` into the numerator
/// (`-(1/3)` → `(-1, 3)`). Decimals still return `None` (see [`int_div`]).
pub(super) fn signed_div(e: &Expr) -> Option<(i64, i64)> {
    let (neg, rest) = split_sign(e);
    let (n, d) = int_div(rest)?;
    Some(if neg { (-n, d) } else { (n, d) })
}

pub(super) fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.unsigned_abs(), b.unsigned_abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a as i64
}

/// Is `n` free of any perfect `k`-th-power factor > 1? (`k = 2` is square-free,
/// for `sqrt`; `k = 3` cube-free, for `cbrt`; etc.) A `k`-th root of such an
/// integer cannot be simplified by pulling a factor out.
pub(super) fn is_power_free_int(n: i64, k: u32) -> bool {
    if k < 2 {
        return true;
    }
    let mut n = n.unsigned_abs();
    if n == 0 {
        return false;
    }
    let mut d: u64 = 2;
    while let Some(dk) = d.checked_pow(k) {
        if dk > n {
            break;
        }
        if n.is_multiple_of(dk) {
            return false;
        }
        while n.is_multiple_of(d) {
            n /= d;
        }
        d += 1;
    }
    true
}

/// The integer numeric content of a written expression — the gcd of its
/// coefficients — used to spot a common numeric factor between a fraction's
/// numerator and denominator (`2x/2`, `4x/6`). Symbols/powers/functions are 1.
pub(super) fn content(e: &Expr) -> i64 {
    match split_sign(e).1 {
        Expr::Num(Number::Int(n)) => *n,
        Expr::Mul(fs) => fs.iter().map(content).fold(1i64, |a, b| a.saturating_mul(b)),
        Expr::Add(ts) => ts.iter().map(content).fold(0i64, gcd),
        _ => 1,
    }
}

/// Number of times the symbol `name` appears anywhere in `e`.
pub(super) fn symbol_occurrences(e: &Expr, name: &str) -> usize {
    let here = matches!(e, Expr::Sym(s) if s.name() == name) as usize;
    here + e
        .children()
        .iter()
        .map(|c| symbol_occurrences(c, name))
        .sum::<usize>()
}

/// Peel leading `Neg` wrappers: returns whether the overall sign is flipped (an
/// odd number of `Neg`s) and the un-wrapped expression.
fn split_sign(e: &Expr) -> (bool, &Expr) {
    match e {
        Expr::Neg(x) => {
            let (neg, inner) = split_sign(x);
            (!neg, inner)
        }
        other => (false, other),
    }
}

/// The expression with any leading `Neg` wrappers removed (magnitude only).
pub(super) fn strip_neg(e: &Expr) -> &Expr {
    split_sign(e).1
}

/// Is `e` written with an overall negative sign? Folds a `Neg` wrapper, a
/// negative numeric literal, and a fraction with an odd number of negative
/// parts, so every spelling agrees (used on exponents and fraction parts).
pub(super) fn is_negative_sign(e: &Expr) -> bool {
    let (neg, rest) = split_sign(e);
    neg ^ match rest {
        Expr::Num(n) => n.is_negative(),
        Expr::Div(a, b) => is_negative_sign(a) ^ is_negative_sign(b),
        _ => false,
    }
}
