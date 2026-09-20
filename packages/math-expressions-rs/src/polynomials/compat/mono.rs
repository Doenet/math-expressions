//! Monomials and the lexicographic order on them.
//!
//! The order is the one Buchberger's algorithm runs under: variables ranked by
//! the legacy default order, then by descending exponent. `mono_div` and
//! `mono_gcd` walk the two variable lists in that order, which is why every
//! monomial's variables are kept sorted by it.

use super::arith::{polynomial_add, polynomial_sub};
use super::rep::{c_div, cmp_var, is_literal_one, same_var, truthy, Mono, Poly};
use crate::expr::Expr;
use std::cmp::Ordering;

/// Follow the highest-degree term down through every variable, collecting the
/// variable powers on the way.
fn leading(p: &Poly) -> (Expr, Vec<(Expr, i64)>) {
    let mut vars = Vec::new();
    let mut focus = p.clone();
    while let Poly::Rec { var, terms } = focus {
        // A polynomial node with no terms has no leading term. Additions never
        // leave one behind (they collapse to `0`), but a product of empty
        // factors can, and reading past the end is not a way to find out.
        let Some((deg, coeff)) = terms.last().cloned() else {
            return (Expr::int(0), vars);
        };
        vars.push((var, deg));
        focus = coeff;
    }
    match focus {
        Poly::Coeff(e) => (e, vars),
        Poly::Rec { .. } => unreachable!("loop exits only on a coefficient"),
    }
}

/// The leading term of `p`. A coefficient is its own initial term, and is
/// returned as a bare coefficient rather than a variable-free monomial —
/// `poly_div` distinguishes the two when it decides what divides what.
pub fn initial_term(p: &Poly) -> Mono {
    match p {
        Poly::Coeff(e) => Mono::Coeff(e.clone()),
        Poly::Rec { .. } => {
            let (coeff, vars) = leading(p);
            Mono::Term { coeff, vars }
        }
    }
}

/// Is `left` strictly earlier than `right` in the lexicographic order?
///
/// A constant is below every monomial, and a tie on the shared prefix is broken
/// in favour of the monomial with more variables.
pub fn mono_less_than(left: &Mono, right: &Mono) -> bool {
    if !right.is_term() {
        return false; // nothing is below a constant
    }
    if !left.is_term() {
        return true;
    }

    let (lv, rv) = (left.vars(), right.vars());
    for i in 0..lv.len().min(rv.len()) {
        if !same_var(&lv[i].0, &rv[i].0) {
            return cmp_var(&lv[i].0, &rv[i].0) != Ordering::Less;
        }
        match lv[i].1.cmp(&rv[i].1) {
            Ordering::Less => return true,
            Ordering::Greater => return false,
            Ordering::Equal => {}
        }
    }
    lv.len() < rv.len()
}

/// The greatest common divisor of two monomials, with coefficient 1 — or `1`
/// when either side is a constant or they share no variable.
pub fn mono_gcd(left: &Mono, right: &Mono) -> Mono {
    if !left.is_term() || !right.is_term() {
        return Mono::Coeff(Expr::int(1));
    }

    let (lv, rv) = (left.vars(), right.vars());
    let mut vars = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < lv.len() && j < rv.len() {
        if same_var(&lv[i].0, &rv[j].0) {
            vars.push((lv[i].0.clone(), lv[i].1.min(rv[j].1)));
            i += 1;
            j += 1;
        } else if cmp_var(&lv[i].0, &rv[j].0) == Ordering::Less {
            i += 1;
        } else {
            // Also covers two distinct variables the default order cannot
            // separate: advancing one side keeps the walk finite, where the JS
            // engine advanced neither and looped forever.
            j += 1;
        }
    }

    if vars.is_empty() {
        Mono::Coeff(Expr::int(1))
    } else {
        Mono::Term {
            coeff: Expr::int(1),
            vars,
        }
    }
}

/// `top / bottom`, or `None` when `bottom` does not divide `top`.
pub fn mono_div(top: &Mono, bottom: &Mono) -> Option<Mono> {
    if !bottom.is_term() {
        if is_literal_one(bottom.coeff()) {
            return Some(top.clone());
        }
        return Some(match top {
            Mono::Coeff(t) => Mono::Coeff(c_div(t, bottom.coeff())),
            Mono::Term { coeff, vars } => Mono::Term {
                coeff: c_div(coeff, bottom.coeff()),
                vars: vars.clone(),
            },
        });
    }
    if !top.is_term() {
        return None; // a constant is not divisible by a monomial
    }

    let (tv, bv) = (top.vars(), bottom.vars());
    let mut vars = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < tv.len() && j < bv.len() {
        if same_var(&tv[i].0, &bv[j].0) {
            if tv[i].1 < bv[j].1 {
                return None;
            }
            if tv[i].1 != bv[j].1 {
                vars.push((tv[i].0.clone(), tv[i].1 - bv[j].1));
            }
            i += 1;
            j += 1;
        } else if cmp_var(&tv[i].0, &bv[j].0) == Ordering::Less {
            vars.push(tv[i].clone());
            i += 1;
        } else {
            return None; // `bottom` carries a variable `top` does not
        }
    }
    if j < bv.len() {
        return None;
    }
    vars.extend_from_slice(&tv[i..]);

    let coeff = if is_literal_one(bottom.coeff()) {
        top.coeff().clone()
    } else {
        c_div(top.coeff(), bottom.coeff())
    };
    Some(if vars.is_empty() {
        Mono::Coeff(coeff)
    } else {
        Mono::Term { coeff, vars }
    })
}

/// Does `bottom` divide `top`? Variables only — a nonzero constant divides
/// anything.
pub fn mono_is_div(top: &Mono, bottom: &Mono) -> bool {
    if !bottom.is_term() {
        return truthy(bottom.coeff());
    }
    if !top.is_term() {
        return false;
    }

    let (tv, bv) = (top.vars(), bottom.vars());
    let (mut i, mut j) = (0, 0);
    while i < tv.len() && j < bv.len() {
        if same_var(&tv[i].0, &bv[j].0) {
            if tv[i].1 < bv[j].1 {
                return false;
            }
            i += 1;
            j += 1;
        } else if cmp_var(&tv[i].0, &bv[j].0) == Ordering::Less {
            i += 1;
        } else {
            return false;
        }
    }
    j >= bv.len()
}

/// The monomial as a nested polynomial.
pub fn mono_to_poly(m: &Mono) -> Poly {
    let Mono::Term { coeff, vars } = m else {
        return Poly::Coeff(m.coeff().clone());
    };
    let mut out = Poly::Coeff(coeff.clone());
    for (var, deg) in vars.iter().rev() {
        out = Poly::Rec {
            var: var.clone(),
            terms: vec![(*deg, out)],
        };
    }
    out
}

/// The largest term of `f` divisible by one of `monos`, and the index of the
/// divisor — or `None` when no term is.
///
/// Terms are peeled off in decreasing order, so the first hit is the largest.
pub fn max_div_init(f: &Poly, monos: &[Mono]) -> Option<(Mono, usize)> {
    let mut f = f.clone();
    loop {
        if f.is_literal_zero() {
            return None;
        }
        // Always a `Term` here, even with no variables: a constant term of `f`
        // is still a term of `f`, and only a divisor list that is itself
        // constant can divide it.
        let (coeff, vars) = leading(&f);
        let term = Mono::Term { coeff, vars };
        if let Some(i) = monos.iter().position(|m| mono_is_div(&term, m)) {
            return Some((term, i));
        }
        f = polynomial_sub(&f, &mono_to_poly(&term));
    }
}

/// `Σ mᵢ` over the monomials a division recorded — the quotient, reassembled.
pub fn sum_monomials(monos: &[(usize, Mono)]) -> Poly {
    monos.iter().fold(Poly::zero(), |acc, (_, m)| {
        polynomial_add(&acc, &mono_to_poly(m))
    })
}
