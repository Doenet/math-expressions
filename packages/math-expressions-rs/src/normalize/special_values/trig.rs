//! The forward direction: sin/cos/tan/cot/sec/csc at a lattice angle, and the
//! parity and π-periodicity that reduce an argument onto the lattice first.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, Zero};

use super::inverse_trig::compose_with_inverse;
use super::util::{apply, canon, is_pi, neg_leading, negate};
use crate::expr::Expr;

pub(super) fn fold_trig(name: &str, arg: &Expr) -> Option<Expr> {
    let (sign, new_arg, dropped_pi) = normalize_trig_arg(name, arg);
    let value = crate::eval_exact::trig_special_value(name, &new_arg)
        .or_else(|| compose_with_inverse(name, &new_arg));
    let arg_changed = new_arg != canon(arg);
    if value.is_none() && sign > 0 && !arg_changed {
        return None; // nothing folded
    }
    // Odd parity on its own is a rearrangement, not a simplification:
    // `sin(-x)` and `-sin(x)` are the same tree in two spellings, and moving
    // the sign outward loses the shape the caller wrote. It earns its place
    // only when it unlocks something else — a lattice value (`sin(-π/6)` →
    // `-1/2`) or a periodicity drop, where the sign is what the drop leaves
    // behind (`sin(π - x)` → `-sin(-x)` → `sin(x)`). With neither in hand,
    // leave the expression as it came: `sin(-x + 2π)` still sheds its `2π`,
    // and lands on `sin(-x)` rather than `-sin(x)`.
    //
    // Even parity is not in the same position and still applies unconditionally:
    // `cos(-x)` → `cos(x)` deletes a node rather than moving one, and `sign`
    // stays positive there, so this guard does not reach it.
    if value.is_none() && sign < 0 && !dropped_pi {
        return None;
    }
    let core = value.unwrap_or_else(|| apply(name, new_arg));
    Some(if sign < 0 { negate(&core) } else { core })
}

/// Pull the sign out of a negative-leading argument (parity) and drop integer
/// multiples of π (periodicity). Returns `(sign, reduced_arg, dropped_pi)`,
/// where `dropped_pi` says whether the periodicity step actually removed a
/// multiple of π — the caller uses it to tell a reduction from a mere
/// rearrangement.
fn normalize_trig_arg(name: &str, arg: &Expr) -> (i32, Expr, bool) {
    let mut sign = 1;
    let mut a = canon(arg);
    if neg_leading(&a) {
        a = negate_terms(&a);
        if is_odd_fn(name) {
            sign = -sign;
        }
    }
    let mut dropped_pi = false;
    let (pi_coeff, rest) = split_pi(&a);
    if pi_coeff.is_integer() && !pi_coeff.is_zero() {
        let k = pi_coeff.to_integer();
        let k_is_odd = (&k % 2i32).abs() == BigInt::from(1);
        // sin/cos/sec/csc have period 2π (odd k flips sign); tan/cot have
        // period π (any integer k drops out with no sign change).
        if matches!(name, "sin" | "cos" | "sec" | "csc") && k_is_odd {
            sign = -sign;
        }
        a = rest;
        dropped_pi = true;
        // The drop can leave a negative-leading remainder — `sin(π − x)` comes
        // out of it as `−sin(−x)` — so parity runs once more on the way out.
        // Only here: a fold is already happening, so moving the sign is what
        // cancels the one the drop just introduced, rather than the free-
        // standing rearrangement the caller is protected from below.
        if neg_leading(&a) && is_odd_fn(name) {
            a = negate_terms(&a);
            sign = -sign;
        }
    }
    (sign, a, dropped_pi)
}

/// `−e`, distributed over a sum.
///
/// [`negate`] alone leaves `−(−x + 2π)` as the product `−1·(−x + 2π)`, and
/// canonicalization does not distribute it — that is `expand`'s job. But
/// [`split_pi`] recognizes a π multiple only as a *term*, so an undistributed
/// negation hides the `2π` and the periodicity drop misses. It used to be
/// caught on the next turn of the fixpoint loop, which worked only because the
/// sign was pulled out unconditionally; now that a fold with nothing to show
/// for itself declines, there is no next turn.
fn negate_terms(e: &Expr) -> Expr {
    match e {
        Expr::Add(ts) => canon(&crate::normalize::add(ts.iter().map(negate).collect())),
        other => negate(other),
    }
}

/// sin, tan, csc, cot are odd; cos, sec are even.
fn is_odd_fn(name: &str) -> bool {
    matches!(name, "sin" | "tan" | "csc" | "cot")
}

/// Split `e` into `(q, rest)` with `e = q·π + rest`, gathering every summand
/// that is a rational multiple of π into `q`.
fn split_pi(e: &Expr) -> (BigRational, Expr) {
    let terms: Vec<Expr> = match e {
        Expr::Add(ts) => ts.clone(),
        other => vec![other.clone()],
    };
    let mut coeff = BigRational::zero();
    let mut rest = Vec::new();
    for t in terms {
        match pi_multiple(&t) {
            Some(q) => coeff += q,
            None => rest.push(t),
        }
    }
    (coeff, canon(&crate::normalize::add(rest)))
}

/// The rational `q` when `e = q·π` (`π`, `3π`, `-π/2`, …), else `None`.
fn pi_multiple(e: &Expr) -> Option<BigRational> {
    if is_pi(e) {
        return Some(BigRational::from_integer(BigInt::from(1)));
    }
    let Expr::Mul(fs) = e else { return None };
    let mut coeff = BigRational::from_integer(BigInt::from(1));
    let mut saw_pi = false;
    for f in fs {
        if is_pi(f) {
            if saw_pi {
                return None; // π² is not a rational multiple of π
            }
            saw_pi = true;
        } else if let Expr::Num(n) = f {
            coeff *= n.to_bigrational()?;
        } else {
            return None;
        }
    }
    saw_pi.then_some(coeff)
}
