//! Decimal-vs-exact and written-fraction shape predicates (reduced, mixed,
//! improper, single fraction).

use super::helpers::{as_int, content, gcd, int_div, signed_div, signed_int, strip_neg};
use super::radicals::denom_has_radical;
use crate::expr::Expr;
use crate::norm::canonicalize;
use crate::num::Number;

fn is_decimal_literal(n: &Number) -> bool {
    // In the faithful tree a decimal parses to Rat (fractional part) or Float
    // (huge). Integers are `Int`; typed fractions are `Div`, not `Rat`.
    matches!(n, Number::Rat(..) | Number::Float(_))
        || matches!(n, Number::Big(b) if matches!(**b, crate::num::BigNumber::Rat(_)))
}

pub(super) fn contains_decimal(e: &Expr) -> bool {
    e.any_subexpr(&|n| matches!(n, Expr::Num(m) if is_decimal_literal(m)))
}

/// Root is a bare number (optionally negated). Integers count — `3` is a valid
/// decimal answer; the F2 provenance tag is what distinguishes `3` from `3.0`.
pub(super) fn is_decimal_number(e: &Expr) -> bool {
    match e {
        Expr::Num(_) => true,
        Expr::Neg(x) => is_decimal_number(x),
        _ => false,
    }
}

pub(super) fn is_reduced_fraction(e: &Expr) -> bool {
    // No un-reduced rational and no surd denominator anywhere.
    fn every_fraction_reduced(e: &Expr) -> bool {
        let here = match e {
            Expr::Div(num, den) => match (as_int(num), as_int(den)) {
                // Pure integer fraction: lowest terms ⇔ coprime.
                (Some(n), Some(d)) => d != 0 && gcd(n, d) == 1,
                // Otherwise reduced ⇔ (a) numerator and denominator share no
                // common numeric factor — `canonicalize` folds `2x/2 → x`, so
                // this must be checked on the *written* form, not the canonical
                // one — and (b) no polynomial factor cancels.
                _ => {
                    gcd(content(num), content(den)) == 1
                        && canonicalize(e) == canonicalize(&crate::reduce_rational(e))
                }
            },
            // A decimal-origin rational is always stored in lowest terms.
            Expr::Num(Number::Rat(..)) => true,
            _ => true,
        };
        here && e.children().iter().all(|c| every_fraction_reduced(c))
    }
    !denom_has_radical(e) && every_fraction_reduced(e)
}

pub(super) fn is_mixed_number(e: &Expr) -> bool {
    // `±(a + b/c)`: an integer and a proper written fraction, both of the same
    // sign — so `2+1/3` and `-2-1/3` qualify, but `2-1/3` (= 5/3) does not.
    let Expr::Add(terms) = e else { return false };
    if terms.len() != 2 {
        return false;
    }
    let (int_part, frac) = match (signed_int(&terms[0]), signed_div(&terms[1])) {
        (Some(i), Some(f)) => (i, f),
        _ => match (signed_int(&terms[1]), signed_div(&terms[0])) {
            (Some(i), Some(f)) => (i, f),
            _ => return false,
        },
    };
    let (n, d) = frac;
    if int_part == 0 || n == 0 || d == 0 {
        return false;
    }
    // Same sign, proper (|n| < |d|), reduced.
    let int_positive = int_part > 0;
    let frac_positive = (n > 0) == (d > 0);
    int_positive == frac_positive
        && n.unsigned_abs() < d.unsigned_abs()
        && gcd(n, d) == 1
}

pub(super) fn is_improper_fraction(e: &Expr) -> bool {
    let strip = strip_neg(e);
    match int_div(strip) {
        Some((n, d)) => d != 0 && n.unsigned_abs() >= d.unsigned_abs(),
        None => false,
    }
}

pub(super) fn is_single_fraction(e: &Expr) -> bool {
    matches!(strip_neg(e), Expr::Div(..))
}
