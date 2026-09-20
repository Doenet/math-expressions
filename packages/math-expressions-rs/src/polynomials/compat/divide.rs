//! Multivariate division with remainder, and inter-reduction of a list of
//! polynomials.
//!
//! Division is by a *list* of divisors: at each step the largest term of the
//! running remainder that any divisor's leading term divides is cancelled, and
//! the quotient is recorded as `(which divisor, by what monomial)`. That list is
//! the standard expression `f = Σ mᵢ·g_{sᵢ} + f′`, and reading the monomials
//! back out is how exact division recovers a quotient.

use super::arith::{polynomial_mul, polynomial_sub};
use super::mono::{initial_term, max_div_init, mono_div, mono_to_poly};
use super::rep::{c_recip, is_literal_one, Mono, Poly};
use crate::expr::Expr;

/// `f = Σ mᵢ·divs[sᵢ] + f′`, returned as `(the (sᵢ, mᵢ) list, f′)`.
pub fn poly_div(f: &Poly, divs: &[Poly]) -> (Vec<(usize, Mono)>, Poly) {
    let inits: Vec<Mono> = divs.iter().map(initial_term).collect();
    divide_by(f.clone(), divs, &inits)
}

/// Reduce `polys[i]` against every *other* polynomial in the list.
///
/// # Panics
///
/// If `i` is out of range. Callers reachable from JS must bounds-check first —
/// the wasm crate is built `panic = "abort"`, where a panic traps the module.
pub fn reduce_ith(i: usize, polys: &[Poly]) -> Poly {
    // A zero initial term is never divisible, so index `i` can never be chosen
    // — that is how the polynomial is kept from cancelling against itself.
    let inits: Vec<Mono> = polys
        .iter()
        .enumerate()
        .map(|(j, p)| {
            if j == i {
                Mono::Coeff(Expr::int(0))
            } else {
                initial_term(p)
            }
        })
        .collect();
    divide_by(polys[i].clone(), polys, &inits).1
}

fn divide_by(mut f: Poly, divs: &[Poly], inits: &[Mono]) -> (Vec<(usize, Mono)>, Poly) {
    let mut quotient = Vec::new();
    while let Some((term, sp)) = max_div_init(&f, inits) {
        // `max_div_init` only reports a term its divisor divides, so this
        // cannot fail; stopping rather than panicking keeps a hypothetical
        // disagreement between the two from taking down the caller.
        let Some(mp) = mono_div(&term, &inits[sp]) else {
            break;
        };
        f = polynomial_sub(&f, &polynomial_mul(&mono_to_poly(&mp), &divs[sp]));
        quotient.push((sp, mp));
    }
    (quotient, f)
}

/// Drop zero polynomials; collapse to `[1]` if any nonzero constant is present
/// (the ideal is everything) and to `[0]` if nothing is left.
pub fn prereduce(polys: &[Poly]) -> Vec<Poly> {
    let mut out = Vec::new();
    for p in polys {
        if p.is_literal_zero() {
            continue;
        }
        if !p.is_rec() {
            return vec![Poly::one()];
        }
        out.push(p.clone());
    }
    if out.is_empty() {
        vec![Poly::zero()]
    } else {
        out
    }
}

/// Reduce a list of polynomials against each other to a fixpoint, then scale
/// each to a leading coefficient of 1.
pub fn reduce(polys: &[Poly]) -> Vec<Poly> {
    let mut polys = prereduce(polys);
    // Nothing to reduce against — and, as in the JS engine, no rescaling
    // either, so a single generator comes back exactly as it went in.
    if polys.len() == 1 {
        return polys;
    }

    let mut changed = true;
    while changed {
        changed = false;
        polys = prereduce(&polys);
        for i in 0..polys.len() {
            let h = reduce_ith(i, &polys);
            if h != polys[i] {
                polys[i] = h;
                changed = true;
            }
        }
    }

    for p in polys.iter_mut() {
        *p = make_monic(p);
    }
    polys
}

/// Scale so the leading coefficient is 1. A polynomial whose leading term is a
/// bare constant *is* a unit, and becomes `1`.
pub fn make_monic(p: &Poly) -> Poly {
    let init = initial_term(p);
    if !init.is_term() {
        return Poly::one();
    }
    if is_literal_one(init.coeff()) {
        return p.clone();
    }
    polynomial_mul(p, &Poly::Coeff(c_recip(init.coeff())))
}
