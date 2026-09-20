//! Reading an expression as a polynomial.
//!
//! Two decisions here are load-bearing and neither is arbitrary:
//!
//! - `π`, `e` and `i` are **numbers**, not variables, so `9x^(2/3) − πx` has
//!   the coefficient `−π`. Getting this backwards would silently change every
//!   gcd downstream, because a symbol treated as a variable divides things a
//!   coefficient does not.
//! - Anything that is not a sum, product, power or quotient — `sin(x)`, `x_1`,
//!   `x/y` with a symbolic denominator — becomes an **opaque variable**, kept
//!   whole. A non-integer power is split first: `t^3.1` is `(t^(1/10))^31`, a
//!   genuine polynomial in the variable `t^(1/10)`, when the exponent's
//!   denominator is small enough to be worth naming.

use super::arith::{polynomial_add, polynomial_mul, polynomial_neg, polynomial_pow};
use super::rep::Poly;
use crate::expr::Expr;
use crate::num::Number;

/// Operators a polynomial may be built from. Anything else makes the whole
/// tree opaque — or, if it appears *anywhere* inside, makes it not a
/// polynomial at all.
const ALLOWED: [&str; 7] = ["+", "-", "*", "^", "/", "_", "prime"];

/// The largest denominator worth turning into a root variable: `t^3.1` becomes
/// a polynomial in `t^(1/10)`, but `t^3.1415` would need `t^(1/2000)` and stays
/// opaque instead.
const MAX_ROOT: i64 = 100;

/// `expr` as a polynomial, or `None` when it is not one.
pub fn expression_to_polynomial(expr: &Expr) -> Option<Poly> {
    match expr {
        // `π`, `e` and `i` are numbers, not variables — the distinction the
        // module docs open with. They reach here as symbols, since the JS AST
        // spells all three as bare strings. Only while declared, though: an
        // undeclared one is an indeterminate like any other name.
        Expr::Sym(s) => {
            return Some(if crate::expr::sym::is_constant_symbol(&s.name()) {
                Poly::Coeff(expr.clone())
            } else {
                opaque(expr)
            })
        }
        // `∞` and `NaN` are numbers too.
        Expr::Num(_) | Expr::Const(_) => return Some(Poly::Coeff(expr.clone())),
        // Not a tree node at all: no polynomial reading exists.
        Expr::Bool(_) | Expr::Blank => return None,
        _ => {}
    }

    // A closed subexpression is a coefficient, whatever it is made of.
    if eval_const(expr).is_finite() {
        return Some(Poly::Coeff(crate::simplify(expr)));
    }

    if !crate::operators(expr)
        .iter()
        .all(|o| ALLOWED.contains(&o.as_str()))
    {
        return None;
    }

    match expr {
        Expr::Add(operands) => {
            let mut acc = None;
            for o in operands {
                let p = expression_to_polynomial(o)?;
                acc = Some(match acc {
                    None => p,
                    Some(a) => polynomial_add(&a, &p),
                });
            }
            acc
        }
        Expr::Neg(a) => {
            let p = expression_to_polynomial(a)?;
            // The JS engine rejected a negation of zero here; a zero
            // polynomial negates to a zero polynomial.
            Some(polynomial_neg(&p))
        }
        Expr::Mul(operands) => {
            let mut acc = None;
            for o in operands {
                let p = expression_to_polynomial(o)?;
                acc = Some(match acc {
                    None => p,
                    Some(a) => polynomial_mul(&a, &p),
                });
            }
            acc
        }
        Expr::Pow(base, exponent) => power(expr, base, exponent),
        Expr::Div(numer, denom) => {
            let d = eval_const(denom);
            if !d.is_finite() {
                return Some(opaque(expr));
            }
            let n = expression_to_polynomial(numer)?;
            Some(polynomial_mul(
                &n,
                &Poly::Coeff(Expr::Div(Box::new(Expr::int(1)), Box::new(number(d)))),
            ))
        }
        _ => Some(opaque(expr)),
    }
}

fn power(whole: &Expr, base: &Expr, exponent: &Expr) -> Option<Poly> {
    let sub = expression_to_polynomial(base)?;
    let pow = crate::simplify(exponent);

    let literal = match &pow {
        Expr::Num(n) => {
            let v = n.to_f64();
            (v >= 0.0 && v.fract() == 0.0 && v.is_finite()).then_some(v)
        }
        _ => None,
    };

    let Some(e) = literal else {
        // A rational exponent p/q with a small q: the base's q-th root is the
        // variable, and the polynomial has the single term of degree p.
        let v = eval_const(&pow);
        if let Some((sign, n, d)) = as_fraction(v) {
            if d <= MAX_ROOT {
                let root = Expr::Pow(
                    Box::new(base.clone()),
                    Box::new(Expr::Div(Box::new(Expr::int(sign)), Box::new(Expr::int(d)))),
                );
                return Some(Poly::Rec {
                    var: crate::simplify(&root),
                    terms: vec![(n, Poly::one())],
                });
            }
        }
        return Some(opaque(whole));
    };

    if e == 0.0 {
        return Some(Poly::one());
    }
    if e == 1.0 {
        return Some(sub);
    }
    polynomial_pow(&sub, &Poly::Coeff(Expr::Num(Number::from_f64(e))))
}

/// The whole tree as a single polynomial variable of degree 1.
fn opaque(expr: &Expr) -> Poly {
    Poly::Rec {
        var: expr.clone(),
        terms: vec![(1, Poly::one())],
    }
}

/// The expression's value as a real number, or NaN when it has none — the
/// finiteness test the reader branches on.
fn eval_const(e: &Expr) -> f64 {
    let Some(v) = crate::evaluate_to_constant(e) else {
        return f64::NAN;
    };
    let real = if v.re.is_finite() {
        v.im.abs() <= 1e-10 * v.re.abs().max(1.0)
    } else {
        v.im == 0.0
    };
    if real {
        v.re
    } else {
        f64::NAN
    }
}

fn number(v: f64) -> Expr {
    if v.fract() == 0.0 && v.abs() < 9.007_199_254_740_992e15 {
        Expr::int(v as i64)
    } else {
        Expr::Num(Number::from_f64(v))
    }
}

/// `v` as a signed fraction `(±1, numerator, denominator)` in lowest terms.
///
/// Continued fractions, stopped as soon as a convergent reproduces `v` exactly
/// as a `f64` — so `3.1` comes back as `31/10` rather than as the ratio of the
/// binary approximation, which is the whole point of asking.
fn as_fraction(v: f64) -> Option<(i64, i64, i64)> {
    if !v.is_finite() {
        return None;
    }
    let sign = if v < 0.0 { -1 } else { 1 };
    let target = v.abs();

    let (mut h_prev, mut h) = (0i64, 1i64);
    let (mut k_prev, mut k) = (1i64, 0i64);
    let mut y = target;
    for _ in 0..64 {
        let a = y.floor();
        let ai = a as i64;
        let h_next = ai.checked_mul(h)?.checked_add(h_prev)?;
        let k_next = ai.checked_mul(k)?.checked_add(k_prev)?;
        (h_prev, h) = (h, h_next);
        (k_prev, k) = (k, k_next);
        if k != 0 && (h as f64) / (k as f64) == target {
            break;
        }
        let frac = y - a;
        if frac == 0.0 {
            break;
        }
        y = 1.0 / frac;
    }
    (k > 0).then_some((sign, h, k))
}
