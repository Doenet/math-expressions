//! Symbolic (indefinite) integration — INTEGRATION_PLAN.md phases I1+I2.
//!
//! Engine order (§2): linearity / constant-slide → the complete rational
//! engine (§3, `rational.rs`) → the elementary table with linear inner
//! arguments → derivative-divides u-substitution (which re-enters the whole
//! pipeline on the substituted integrand). All recursion shares one fuel
//! budget (`max_integration_steps`), and every top-level success must pass
//! the gate `equals(derivative(F, x), f)` — a wrong answer is discarded,
//! never returned.

pub(crate) mod rational;

use crate::assumptions::Assumptions;
use crate::expr::Expr;
use crate::norm::{add, canonicalize, mul, pow};
use crate::num::Number;

fn int(i: i64) -> Expr {
    Expr::Num(Number::Int(i))
}

fn apply(name: &str, arg: Expr) -> Expr {
    Expr::Apply(Box::new(Expr::sym(name)), vec![arg])
}

fn depends_on(e: &Expr, x: &str) -> bool {
    crate::ops::variables(e).iter().any(|v| v == x)
}

/// One antiderivative of `f` with respect to `x` (no `+ C` — the caller's
/// concern, as in Rubi/mathjs). `None` is the honest "no elementary form
/// found within budget" — the caller can still integrate numerically via
/// `integrate_to_precision`.
pub fn integrate(f: &Expr, x: &str, _assumptions: &Assumptions) -> Option<Expr> {
    let fc = canonicalize(f);
    let mut fuel = crate::resource_limits::current().max_integration_steps;
    let result = integ(&fc, x, &mut fuel)?;
    // The gate (plan §2c): verify by differentiation. Accept iff the sampled
    // `equals` OR the certified exact stages (FULL_SIMPLIFY S1: structural
    // cancellation, exact constants, rational normal form) confirm
    // `F' - f ≡ 0`. `equals` runs first because it is cheap and accepts almost
    // every correct candidate; the certified pass then *rescues* sound
    // antiderivatives that sampling wrongly rejects (tolerance/domain
    // artifacts). The disjunction is order-independent, so this costs the old
    // gate's time on the accept path. `is_zero`'s sampling-refuter stage is
    // deliberately not used here: its certified reject duplicates the `equals`
    // reject, and on true zeros it burns its full arbitrary-precision budget
    // before returning Unknown (measured ~35× suite slowdown).
    let df = crate::diff::derivative(&result, x);
    if !crate::equality::equals(&df, &fc, &crate::equality::EqOptions::default()) {
        let residual = Expr::Add(vec![df, Expr::Neg(Box::new(fc.clone()))]);
        if !crate::exact::certified_zero(&residual, _assumptions) {
            return None;
        }
    }
    Some(crate::norm::simplify(&result))
}

fn integ(e: &Expr, x: &str, fuel: &mut i64) -> Option<Expr> {
    *fuel -= 1;
    if *fuel < 0 {
        return None;
    }
    let xs = Expr::sym(x);
    // ∫ c dx = c·x.
    if !depends_on(e, x) {
        return Some(mul(vec![e.clone(), xs]));
    }
    // Linearity.
    if let Expr::Add(ts) = e {
        let parts: Option<Vec<Expr>> = ts.iter().map(|t| integ(t, x, fuel)).collect();
        return parts.map(add);
    }
    // Slide the x-free coefficient out of a product.
    if let Expr::Mul(fs) = e {
        let (coeff, core): (Vec<Expr>, Vec<Expr>) =
            fs.iter().cloned().partition(|f| !depends_on(f, x));
        if !coeff.is_empty() && !core.is_empty() {
            let inner = integ(&mul(core), x, fuel)?;
            return Some(mul(vec![mul(coeff), inner]));
        }
    }
    // The complete rational engine (I1).
    if let Some((n, d)) = rational::expr_to_ratfun(e, x) {
        if let Some(res) = rational::integrate_rational(&n, &d, x) {
            return Some(res);
        }
    }
    // Elementary table with linear inner arguments (I2).
    if let Some(res) = table_match(e, x) {
        return Some(res);
    }
    // Derivative-divides u-substitution (I2), re-entering the pipeline.
    usub(e, x, fuel)
}

/// `u = a + b·x` with x-free `b` (returned). `None` if `u` is not linear
/// in `x`. The constant part is never needed by the rules — only `b`.
fn linear_coeff(u: &Expr, x: &str) -> Option<Expr> {
    fn term_coeff(t: &Expr, x: &str) -> Option<Expr> {
        // A canonical term that is exactly c·x (or x).
        match t {
            Expr::Sym(s) if s.name() == x => Some(int(1)),
            Expr::Mul(fs) => {
                let mut coeff = Vec::new();
                let mut seen_x = false;
                for f in fs {
                    match f {
                        Expr::Sym(s) if s.name() == x => {
                            if seen_x {
                                return None;
                            }
                            seen_x = true;
                        }
                        f if !depends_on(f, x) => coeff.push(f.clone()),
                        _ => return None,
                    }
                }
                seen_x.then(|| mul(coeff))
            }
            _ => None,
        }
    }
    match u {
        _ if !depends_on(u, x) => None,
        Expr::Add(ts) => {
            let mut b: Option<Expr> = None;
            for t in ts {
                if !depends_on(t, x) {
                    continue;
                }
                let c = term_coeff(t, x)?;
                b = Some(match b {
                    None => c,
                    Some(prev) => add(vec![prev, c]),
                });
            }
            b.filter(|b| !matches!(b, Expr::Num(n) if n.is_zero()))
        }
        other => term_coeff(other, x),
    }
}

fn over(e: Expr, b: &Expr) -> Expr {
    if matches!(b, Expr::Num(n) if n.is_one()) {
        e
    } else {
        mul(vec![e, pow(b.clone(), int(-1))])
    }
}

/// The elementary table (Rubi cluster 1 + pervasive `a + b·x` linear
/// substitution): every row is `∫ g(u) dx = G(u)/b` for linear `u`.
fn table_match(e: &Expr, x: &str) -> Option<Expr> {
    match e {
        // (a+bx)^n and c^(a+bx).
        Expr::Pow(base0, exp0) => {
            // `sqrt(w)^k` is `w^(k/2)`: unify so the power rows see one
            // spelling (canonical form keeps sqrt as an application).
            let (base, exp): (Expr, Expr) = match (&**base0, &**exp0) {
                (Expr::Apply(h, args), Expr::Num(n))
                    if matches!(&**h, Expr::Sym(s) if s.name() == "sqrt")
                        && args.len() == 1 =>
                {
                    (args[0].clone(), Expr::Num(n.mul(&Number::rat(1, 2))))
                }
                _ => ((**base0).clone(), (**exp0).clone()),
            };
            let (base, exp) = (&base, &exp);
            // Power of a linear argument with an x-free exponent.
            if let Some(b) = linear_coeff(base, x) {
                if !depends_on(exp, x) {
                    if matches!(exp, Expr::Num(n) if n.to_f64() == -1.0) {
                        return Some(over(apply("ln", base.clone()), &b));
                    }
                    // u^n → u^(n+1)/(n+1): exponent must be a number ≠ −1.
                    if let Expr::Num(n) = exp {
                        let n1 = n.add(&Number::Int(1));
                        if !n1.is_zero() {
                            let f = mul(vec![
                                pow(base.clone(), Expr::Num(n1.clone())),
                                pow(Expr::Num(n1), int(-1)),
                            ]);
                            return Some(over(f, &b));
                        }
                    }
                }
            }
            // Exponential: c^u, x-free base.
            if !depends_on(base, x) {
                if let Some(b) = linear_coeff(exp, x) {
                    let is_e = matches!(base, Expr::Const(crate::expr::MathConst::E))
                        || matches!(base, Expr::Sym(s) if s.name() == "e");
                    if is_e {
                        return Some(over(e.clone(), &b));
                    }
                    if matches!(base, Expr::Num(n) if n.is_positive() && !n.is_one()) {
                        let f = mul(vec![
                            e.clone(),
                            pow(apply("ln", base.clone()), int(-1)),
                        ]);
                        return Some(over(f, &b));
                    }
                }
            }
            // 1/√(c − b·u²) → asin(u·√(b/c))/√b (the inverse-trig table row
            // in its canonical Pow clothing).
            if matches!(exp, Expr::Num(n) if n.to_f64() == -0.5) {
                if let Some((c, b_coef, u, ub)) = concave_quadratic(base, x) {
                    let ratio = &b_coef / &c;
                    let s = rational::sqrt_expr(&ratio);
                    let inv_sqrt_b = pow(rational::sqrt_expr(&b_coef), int(-1));
                    let f = mul(vec![
                        inv_sqrt_b,
                        apply("asin", mul(vec![u, s])),
                    ]);
                    return Some(over(f, &ub));
                }
            }
            // sec²/csc² in canonical clothing: cos(u)^(−2), sin(u)^(−2).
            if let (Expr::Apply(h, args), Expr::Num(Number::Int(-2))) = (base, exp) {
                if let (Expr::Sym(f), [u]) = (&**h, args.as_slice()) {
                    if let Some(b) = linear_coeff(u, x) {
                        match f.name().as_str() {
                            "cos" => return Some(over(apply("tan", u.clone()), &b)),
                            "sin" => {
                                let cot = mul(vec![
                                    int(-1),
                                    apply("cos", u.clone()),
                                    pow(apply("sin", u.clone()), int(-1)),
                                ]);
                                return Some(over(cot, &b));
                            }
                            _ => {}
                        }
                    }
                }
            }
            None
        }
        Expr::Apply(head, args) => {
            // The elementary antiderivative table is `FnDef::antiderivative`
            // in `crate::functions` (alias-aware: `arctan` finds `atan`).
            let (Expr::Sym(f), [u]) = (&**head, args.as_slice()) else {
                return None;
            };
            let builder = crate::functions::antiderivative_builder(&f.name())?;
            let b = linear_coeff(u, x)?;
            Some(over(builder(u.clone()), &b))
        }
        _ => None,
    }
}

/// Match `c − b·w²` with rational `c, b > 0` and `w` linear in `x`.
/// Returns `(c, b, w, linear coefficient of w)`.
fn concave_quadratic(
    base: &Expr,
    x: &str,
) -> Option<(
    num_rational::BigRational,
    num_rational::BigRational,
    Expr,
    Expr,
)> {
    use num_rational::BigRational;
    use num_traits::Signed;
    let Expr::Add(ts) = base else { return None };
    let mut c: Option<BigRational> = None;
    let mut quad: Option<(BigRational, Expr)> = None;
    for t in ts {
        match t {
            Expr::Num(n) => {
                if c.is_some() {
                    return None;
                }
                c = n.to_bigrational();
            }
            Expr::Pow(w, k) if matches!(&**k, Expr::Num(Number::Int(2))) => {
                if quad.is_some() {
                    return None;
                }
                quad = Some((-BigRational::from_integer((-1).into()), (**w).clone()));
            }
            Expr::Mul(fs) => {
                let mut coeff: Option<BigRational> = None;
                let mut w: Option<Expr> = None;
                for f in fs {
                    match f {
                        Expr::Num(n) => coeff = n.to_bigrational(),
                        Expr::Pow(b, k) if matches!(&**k, Expr::Num(Number::Int(2))) => {
                            w = Some((**b).clone())
                        }
                        _ => return None,
                    }
                }
                if quad.is_some() {
                    return None;
                }
                quad = Some((coeff?, w?));
            }
            _ => return None,
        }
    }
    let (a, w) = quad?;
    let c = c?;
    // c − b·w²: need c > 0, a < 0.
    if !c.is_positive() || !a.is_negative() {
        return None;
    }
    let ub = linear_coeff(&w, x)?;
    Some((c, -a, w, ub))
}

/// Derivative-divides: for each composite candidate `u`, test whether
/// `f / u′` rewrites as a function of `u` alone; if so, integrate that in a
/// fresh variable and substitute back (Rubi's substitution meta-rule; the
/// SymPy `manualintegrate` workhorse).
fn usub(e: &Expr, x: &str, fuel: &mut i64) -> Option<Expr> {
    const U: &str = "_usub";
    let mut candidates: Vec<Expr> = Vec::new();
    collect_candidates(e, &mut candidates);
    candidates.retain(|u| depends_on(u, x) && !matches!(u, Expr::Sym(_)));
    candidates.dedup();
    candidates.truncate(crate::resource_limits::current().max_integration_candidates);
    for u in candidates {
        let du = canonicalize(&crate::diff::derivative(&u, x));
        if matches!(&du, Expr::Num(n) if n.is_zero()) {
            continue;
        }
        // f/u′, aggressively cancelled.
        let q = crate::norm::simplify_core(&crate::ops::reduce_rational(&Expr::Div(
            Box::new(e.clone()),
            Box::new(du.clone()),
        )));
        let replaced = replace_subtree(&q, &u, &Expr::sym(U));
        if depends_on(&replaced, x) {
            continue;
        }
        let inner = integ(&canonicalize(&replaced), U, fuel)?;
        let subs = std::collections::HashMap::from([(U.to_string(), u.clone())]);
        return Some(canonicalize(&crate::ops::substitute(&inner, &subs)));
    }
    None
}

/// Composite subtrees worth trying as `u`: application arguments, the
/// applications themselves, and non-atomic power bases.
fn collect_candidates(e: &Expr, out: &mut Vec<Expr>) {
    match e {
        Expr::Apply(_, args) => {
            for a in args {
                out.push(a.clone());
                collect_candidates(a, out);
            }
            out.push(e.clone());
        }
        Expr::Pow(b, ex) => {
            out.push((**b).clone());
            // x⁴ hides x² (and x⁶ hides x³): propose divisor powers so
            // u = x² can match inside 1 − x⁴ (replace_subtree understands
            // the power-multiple rewrite).
            if let Expr::Num(Number::Int(k)) = &**ex {
                for d in 2..*k {
                    if *k % d == 0 {
                        out.push(pow((**b).clone(), int(d)));
                    }
                }
            }
            collect_candidates(b, out);
            collect_candidates(ex, out);
        }
        _ => {
            for c in e.children() {
                collect_candidates(c, out);
            }
        }
    }
}

/// Structural replacement of every occurrence of `target` by `to`, plus the
/// power-multiple rewrite: with target `b^k`, an occurrence `b^(k·j)`
/// becomes `to^j` (this is what lets u = x² act inside x⁴).
fn replace_subtree(e: &Expr, target: &Expr, to: &Expr) -> Expr {
    if e == target {
        return to.clone();
    }
    if let (Expr::Pow(tb, tk), Expr::Pow(eb, ek)) = (target, e) {
        if tb == eb {
            if let (Expr::Num(Number::Int(tk)), Expr::Num(Number::Int(ek))) = (&**tk, &**ek) {
                if *tk >= 2 && *ek % *tk == 0 && *ek != *tk {
                    return Expr::Pow(Box::new(to.clone()), Box::new(int(*ek / *tk)));
                }
            }
        }
    }
    crate::norm::syntactic::map_children(e, |c| replace_subtree(c, target, to))
}
