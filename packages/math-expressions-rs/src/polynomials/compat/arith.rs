//! Ring operations on [`Poly`]: add, negate, subtract, multiply, raise to a
//! power, and convert back to an expression.
//!
//! Every operation keeps the invariant the representation depends on: terms
//! sorted by increasing degree, no zero terms, and a node that has collapsed to
//! a constant returned as a [`Poly::Coeff`] rather than a one-term polynomial.

use super::rep::{c_add, c_mul, c_neg, cmp_var, same_var, Poly};
use crate::expr::Expr;
use std::cmp::Ordering;

/// Rewrite whichever of `p`, `q` is written in the later variable as a
/// polynomial that is constant in the other's variable, so the two can be added
/// or multiplied term by term.
fn in_same_leading_variable(p: Poly, q: Poly) -> (Poly, Poly) {
    if same_var(p.var(), q.var()) {
        return (p, q);
    }
    if cmp_var(p.var(), q.var()) == Ordering::Less {
        let var = p.var().clone();
        (p, q.constant_in(&var))
    } else {
        let var = q.var().clone();
        (p.constant_in(&var), q)
    }
}

/// Put both operands into the same leading variable, wrapping a coefficient as
/// a degree-0 polynomial in the other's variable.
fn align(p: &Poly, q: &Poly) -> Option<(Poly, Poly)> {
    match (p.is_rec(), q.is_rec()) {
        (false, false) => None,
        (false, true) => {
            let var = q.var().clone();
            Some((p.clone().constant_in(&var), q.clone()))
        }
        (true, false) => {
            let var = p.var().clone();
            Some((p.clone(), q.clone().constant_in(&var)))
        }
        (true, true) => Some(in_same_leading_variable(p.clone(), q.clone())),
    }
}

pub fn polynomial_add(p: &Poly, q: &Poly) -> Poly {
    let Some((p, q)) = align(p, q) else {
        return match (p, q) {
            (Poly::Coeff(a), Poly::Coeff(b)) => Poly::Coeff(c_add(a, b)),
            _ => unreachable!("align only declines two coefficients"),
        };
    };

    let var = p.var().clone();
    let (pt, qt) = (p.terms(), q.terms());
    let mut sum: Vec<(i64, Poly)> = Vec::with_capacity(pt.len() + qt.len());
    let (mut i, mut j) = (0, 0);

    while i < pt.len() || j < qt.len() {
        if i == pt.len() {
            if qt[j].1.truthy() {
                sum.push(qt[j].clone());
            }
            j += 1;
        } else if j == qt.len() {
            if pt[i].1.truthy() {
                sum.push(pt[i].clone());
            }
            i += 1;
        } else {
            match pt[i].0.cmp(&qt[j].0) {
                Ordering::Equal => {
                    let t = polynomial_add(&pt[i].1, &qt[j].1);
                    if t.truthy() {
                        sum.push((pt[i].0, t));
                    }
                    i += 1;
                    j += 1;
                }
                Ordering::Less => {
                    if pt[i].1.truthy() {
                        sum.push(pt[i].clone());
                    }
                    i += 1;
                }
                Ordering::Greater => {
                    if qt[j].1.truthy() {
                        sum.push(qt[j].clone());
                    }
                    j += 1;
                }
            }
        }
    }

    finish(var, sum)
}

/// Assemble a term list into a [`Poly`], restoring the invariant the module doc
/// states: everything cancelled is [`Poly::zero`], and a lone degree-0 term is
/// its own coefficient rather than a one-term polynomial. Callers compare the
/// returned AST with deep equality, so the two spellings are not
/// interchangeable — `["polynomial","x",[[0,12]]]` is not `12`.
fn finish(var: Expr, mut terms: Vec<(i64, Poly)>) -> Poly {
    if terms.is_empty() {
        return Poly::zero();
    }
    if terms.len() == 1 && terms[0].0 == 0 {
        return terms.remove(0).1;
    }
    Poly::Rec { var, terms }
}

/// Is `p` a single term? Such a polynomial stays one term under multiplication,
/// so raising it to a power costs nothing regardless of the exponent.
fn is_monomial(p: &Poly) -> bool {
    match p {
        Poly::Coeff(_) => true,
        Poly::Rec { terms, .. } => terms.len() <= 1 && terms.iter().all(|(_, c)| is_monomial(c)),
    }
}

pub fn polynomial_neg(p: &Poly) -> Poly {
    match p {
        Poly::Coeff(e) => Poly::Coeff(c_neg(e)),
        Poly::Rec { var, terms } => Poly::Rec {
            var: var.clone(),
            terms: terms.iter().map(|(d, c)| (*d, polynomial_neg(c))).collect(),
        },
    }
}

pub fn polynomial_sub(p: &Poly, q: &Poly) -> Poly {
    polynomial_add(p, &polynomial_neg(q))
}

pub fn polynomial_mul(p: &Poly, q: &Poly) -> Poly {
    match (p, q) {
        (Poly::Coeff(a), Poly::Coeff(b)) => return Poly::Coeff(c_mul(a, b)),
        // A zero (or NaN) coefficient annihilates the polynomial. The JS engine
        // fell through to its two-polynomial branch here and read `p[1]` off a
        // number; the product it would have built is this one.
        (Poly::Coeff(a), Poly::Rec { .. }) if !p.truthy() => return Poly::Coeff(a.clone()),
        (Poly::Rec { .. }, Poly::Coeff(b)) if !q.truthy() => return Poly::Coeff(b.clone()),
        (Poly::Coeff(_), Poly::Rec { var, terms }) => {
            return finish(
                var.clone(),
                terms
                    .iter()
                    .filter(|(_, c)| c.truthy())
                    .map(|(d, c)| (*d, polynomial_mul(p, c)))
                    .collect(),
            )
        }
        (Poly::Rec { var, terms }, Poly::Coeff(_)) => {
            return finish(
                var.clone(),
                terms
                    .iter()
                    .filter(|(_, c)| c.truthy())
                    .map(|(d, c)| (*d, polynomial_mul(c, q)))
                    .collect(),
            )
        }
        _ => {}
    }

    let (p, q) = in_same_leading_variable(p.clone(), q.clone());
    let var = p.var().clone();
    let (pt, qt) = (p.terms(), q.terms());

    // The degrees that occur in the product, in increasing order.
    let mut degrees: Vec<i64> = pt
        .iter()
        .flat_map(|(a, _)| qt.iter().map(move |(b, _)| a + b))
        .collect();
    degrees.sort_unstable();
    degrees.dedup();

    let mut terms = Vec::with_capacity(degrees.len());
    for deg in degrees {
        let mut sum = Poly::zero();
        for (dp, cp) in pt.iter().take_while(|(d, _)| *d <= deg) {
            if let Some((_, cq)) = qt
                .iter()
                .take_while(|(d, _)| *d <= deg)
                .find(|(dq, _)| dp + dq == deg)
            {
                sum = polynomial_add(&sum, &polynomial_mul(cp, cq));
            }
        }
        if sum.truthy() {
            terms.push((deg, sum));
        }
    }

    finish(var, terms)
}

/// `p^e` by binary exponentiation, or `None` when `e` is not a literal
/// non-negative integer, or when the expansion is refused as too large.
///
/// Raising a multi-term polynomial is a multinomial expansion — the same work
/// [`normalize::expand`](crate::normalize::expand) does — and
/// [`polynomial_mul`] is quadratic in the term count, so the cost grows about
/// cubically in the exponent: `(x+1)^400` takes seconds. This is reachable from
/// a Doenet answer box through `expression_to_polynomial`, so the exponent is
/// held to the same `max_expand_power` budget `expand` uses. A *monomial* base
/// is exempt: `x^1000` is one term however large the exponent, and refusing it
/// would reject ordinary input.
pub fn polynomial_pow(p: &Poly, e: &Poly) -> Option<Poly> {
    let Poly::Coeff(Expr::Num(n)) = e else {
        return None;
    };
    let v = n.to_f64();
    if !v.is_finite() || v < 0.0 || v.fract() != 0.0 || v > i64::MAX as f64 {
        return None;
    }
    if v > crate::resource_limits::current().max_expand_power as f64 && !is_monomial(p) {
        return None;
    }

    let mut e = v as i64;
    let mut base = p.clone();
    let mut res = Poly::one();
    while e > 0 {
        if e & 1 == 1 {
            res = polynomial_mul(&res, &base);
        }
        base = polynomial_mul(&base, &base);
        e >>= 1;
    }
    Some(res)
}

/// The polynomial written back out as an expression, simplified.
pub fn polynomial_to_expression(p: &Poly) -> Expr {
    let (var, terms) = match p {
        Poly::Coeff(e) => return e.clone(),
        Poly::Rec { var, terms } => (var, terms),
    };

    let mut parts: Vec<Expr> = Vec::new();
    for (deg, coeff) in terms {
        if !coeff.truthy() {
            continue;
        }
        let c = polynomial_to_expression(coeff);
        parts.push(match deg {
            0 => c,
            1 => Expr::Mul(vec![c, var.clone()]),
            d => Expr::Mul(vec![
                c,
                Expr::Pow(Box::new(var.clone()), Box::new(Expr::int(*d))),
            ]),
        });
    }

    let out = match parts.len() {
        0 => return Expr::int(0),
        1 => parts.pop().expect("checked"),
        _ => Expr::Add(parts),
    };
    crate::simplify(&out)
}
