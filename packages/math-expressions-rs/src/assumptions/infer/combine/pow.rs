//! The `^` rule: what `base^exp` inherits from its parts.
//!
//! A port of the `^` branch that every JS predicate in `element_of_sets.js`
//! carries. They share a shape: two cases that settle the answer outright
//! (`0^positive` is `0`, `nonzero^0` is `1`), then a common tail that asks
//! about realness, sign and integrality of the two operands. The literal-
//! exponent rules above the tail are ours, and are the only place parity is
//! available directly.

use super::super::super::facts::Facts;
use super::super::super::Assumptions;
use super::super::facts;
use crate::expr::Expr;
use crate::num::Number;

pub(in super::super) fn pow(base: &Facts, ef: &Facts, exp: &Expr, a: &Assumptions) -> Facts {
    // The two cases JS settles outright, ahead of any structural reasoning.
    if base.nonzero == Some(false) && ef.positive == Some(true) {
        return Facts::of_f64(0.0); // 0^positive
    }
    if base.nonzero == Some(true) && ef.nonzero == Some(false) {
        return Facts::of_f64(1.0); // nonzero^0
    }

    let mut out = Facts::unknown();

    // A nonzero base cannot be raised to zero (JS: `is_nonzero` of a power is
    // simply the base's, once `0^positive` is out of the way).
    if base.nonzero == Some(true) {
        out.nonzero = Some(true);
    }

    // An integer to a nonnegative integer power is an integer. JS insists the
    // exponent be an *integer* rather than merely nonnegative, so that it
    // never has to decide `9^(1/2)`.
    if base.integer == Some(true) && ef.integer == Some(true) && ef.nonneg == Some(true) {
        out.integer = Some(true);
    }

    // Integer exponent as a literal (the only case with parity information).
    let lit = match exp {
        Expr::Num(Number::Int(k)) => Some(*k),
        _ => None,
    };

    // b^k for integer k: real base stays real (nonzero if k may be negative).
    if let Some(k) = lit {
        if base.real == Some(true) && (k >= 0 || base.nonzero == Some(true)) {
            out.real = Some(true);
            out.complex = Some(true);
        }
        // Reciprocal of a definite-sign real: sign carries through a negative
        // odd exponent (JS reaches this via its division rule; positive bases
        // are covered below). Positive odd exponents deliberately stay
        // unknown for negative bases, matching the JS Pow path.
        if k < 0 && k % 2 != 0 && base.negative == Some(true) {
            out.real = Some(true);
            out.complex = Some(true);
            out.nonneg = Some(false);
            out.positive = Some(false);
            out.nonzero = Some(true);
        }
        // Odd positive power of a nonnegative real stays nonnegative (JS
        // infers this, though not the negative-base analogue).
        if k > 0 && k % 2 != 0 && base.nonneg == Some(true) && base.real == Some(true) {
            out.nonneg = Some(true);
        }
        // Even power of a real: nonnegative; positive iff base nonzero.
        // (Odd powers of negatives deliberately stay unknown, like JS.)
        if k != 0 && k % 2 == 0 && base.real == Some(true) {
            out.nonneg = Some(true);
            if base.nonzero == Some(true) {
                out.positive = Some(true);
                out.nonzero = Some(true);
            }
        }
    }

    // From here on, the port of the shared tail of the JS `^` branches. It is
    // reached only once `0^possibly-nonpositive` has been excluded: either the
    // base is known nonzero, or the exponent is known positive.
    let reach = base.nonzero == Some(true) || ef.positive == Some(true);
    if reach {
        if base.real == Some(true) && ef.real == Some(true) {
            let real = if base.nonneg == Some(true) {
                // A nonnegative base needs the exponent to avoid `0^0`, which
                // either a positive base or a positive exponent guarantees.
                base.positive == Some(true) || ef.positive == Some(true)
            } else {
                // A possibly-negative base needs an integer exponent.
                ef.integer == Some(true)
            };
            if real {
                out.real = Some(true);
                out.complex = Some(true);
            }
        }
        if base.complex == Some(true) && ef.complex == Some(true) {
            out.complex = Some(true);
        }
        if pow_is_signed(base, ef, exp, a, false) {
            out.nonneg = Some(true);
        }
        if pow_is_signed(base, ef, exp, a, true) {
            out.positive = Some(true);
        }
    }

    // Positive real base: positive for any real exponent (covers 1/x, sqrt
    // as x^(1/2), and symbolic real exponents).
    if base.positive == Some(true)
        && (ef.real == Some(true) || lit.is_some() || is_real_exponent_shape(exp))
    {
        out.real = Some(true);
        out.complex = Some(true);
        out.positive = Some(true);
        out.nonneg = Some(true);
        out.nonzero = Some(true);
    }
    out
}

/// Is `base^exp` positive (`strict`) / nonnegative (`!strict`)? The port of
/// the tail of JS's `is_positive_ast` `^` branch, which only ever answers
/// "yes" or "don't know" once the collapsing cases have been split off.
fn pow_is_signed(base: &Facts, ef: &Facts, exp: &Expr, a: &Assumptions, strict: bool) -> bool {
    if base.nonzero != Some(true) {
        // A possibly-zero base can only be *non-strictly* signed, and only
        // when the exponent is definitely positive.
        if strict || ef.positive != Some(true) {
            return false;
        }
    }
    if base.real != Some(true) {
        return false;
    }
    let base_signed = if strict { base.positive } else { base.nonneg };
    if base_signed != Some(true) {
        // A base that may have the wrong sign is rescued only by an even
        // exponent.
        return is_even_exponent(exp, a);
    }
    ef.real == Some(true)
}

/// Is the exponent an even integer? There is no `is_even` in the lattice, so —
/// like JS — the question is put back to the engine as "is `exp/2` an
/// integer?", which catches `2x` for integer `x` and not just a literal.
fn is_even_exponent(exp: &Expr, a: &Assumptions) -> bool {
    if let Expr::Num(Number::Int(k)) = exp {
        return k % 2 == 0;
    }
    let halved = Expr::Div(Box::new(exp.clone()), Box::new(Expr::Num(Number::Int(2))));
    facts(&crate::normalize::canonicalize(&halved), a).integer == Some(true)
}

/// A numeric (rational) exponent — real by construction even though it is not
/// an integer literal (e.g. the `1/2` in `sqrt` written as a power).
fn is_real_exponent_shape(e: &Expr) -> bool {
    matches!(e, Expr::Num(n) if !n.to_f64().is_nan())
}
