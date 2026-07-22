//! Polynomial layer (PORTING_PLAN.md §8), scoped to what the public API needs:
//! multivariate GCD for `reduce_rational` (cancel common polynomial factors in
//! a fraction). Follows the plan's §8c recursive dense model (SymPy's dmp):
//! a polynomial in `vars[0..n]` is a coefficient list in `vars[0]` whose
//! entries are polynomials in the remaining variables; the innermost level is
//! an exact `BigRational`.
//!
//! GCD is the classic primitive PRS: pseudo-remainders in the main variable,
//! contents/primitive parts via recursive GCD of the coefficients. Over the
//! ground field ℚ the gcd of nonzero constants is 1 (units), which makes the
//! univariate case plain Euclid and the multivariate case terminate at
//! primitive parts. Everything is bounded (degree, PRS steps, dense size) via
//! `limits` — untrusted input must not trigger coefficient swell.

use crate::expr::Expr;
use crate::num::Number;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

/// Degree cap per variable and PRS iteration cap (deterministic, §7f).
const MAX_DEGREE: usize = 64;
const MAX_PRS_STEPS: usize = 128;

/// Recursive dense representation. `Ground` only at the innermost level;
/// `Nested` coefficient lists are ascending-degree with no trailing zeros
/// (the zero polynomial is an empty list).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Rep {
    Ground(BigRational),
    Nested(Vec<Rep>),
}

impl Rep {
    fn zero(depth: usize) -> Rep {
        if depth == 0 {
            Rep::Ground(BigRational::zero())
        } else {
            Rep::Nested(vec![])
        }
    }

    fn one(depth: usize) -> Rep {
        if depth == 0 {
            Rep::Ground(BigRational::one())
        } else {
            Rep::Nested(vec![Rep::one(depth - 1)])
        }
    }

    fn is_zero(&self) -> bool {
        match self {
            Rep::Ground(g) => g.is_zero(),
            Rep::Nested(cs) => cs.is_empty(),
        }
    }

    /// Degree in the main variable (0 for ground / constants; 0 for zero too —
    /// callers check `is_zero` first where it matters).
    fn degree(&self) -> usize {
        match self {
            Rep::Ground(_) => 0,
            Rep::Nested(cs) => cs.len().saturating_sub(1),
        }
    }

    fn coeffs(&self) -> &[Rep] {
        match self {
            Rep::Nested(cs) => cs,
            Rep::Ground(_) => &[],
        }
    }

    /// Leading coefficient (one level deeper). Zero poly → zero coefficient.
    fn lc(&self, depth: usize) -> Rep {
        match self {
            Rep::Nested(cs) => cs.last().cloned().unwrap_or_else(|| Rep::zero(depth - 1)),
            Rep::Ground(_) => self.clone(),
        }
    }

    fn normalize(mut self) -> Rep {
        if let Rep::Nested(cs) = &mut self {
            while matches!(cs.last(), Some(c) if c.is_zero()) {
                cs.pop();
            }
        }
        self
    }

    /// Is this (at any nesting depth) the constant 1?
    fn is_one(&self) -> bool {
        match self {
            Rep::Ground(g) => g.is_one(),
            Rep::Nested(cs) => cs.len() == 1 && cs[0].is_one(),
        }
    }

    /// Is this free of the outer `levels` variables (a constant at this level)?
    fn total_degree_zero(&self) -> bool {
        match self {
            Rep::Ground(_) => true,
            Rep::Nested(cs) => cs.len() <= 1 && cs.first().is_none_or(Rep::total_degree_zero),
        }
    }
}

fn add(a: &Rep, b: &Rep, depth: usize) -> Rep {
    match (a, b) {
        (Rep::Ground(x), Rep::Ground(y)) => Rep::Ground(x + y),
        (Rep::Nested(xs), Rep::Nested(ys)) => {
            let n = xs.len().max(ys.len());
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let zero = Rep::zero(depth - 1);
                let x = xs.get(i).unwrap_or(&zero);
                let y = ys.get(i).unwrap_or(&zero);
                out.push(add(x, y, depth - 1));
            }
            Rep::Nested(out).normalize()
        }
        _ => unreachable!("depth mismatch"),
    }
}

fn neg(a: &Rep) -> Rep {
    match a {
        Rep::Ground(x) => Rep::Ground(-x),
        Rep::Nested(xs) => Rep::Nested(xs.iter().map(neg).collect()),
    }
}

fn sub(a: &Rep, b: &Rep, depth: usize) -> Rep {
    add(a, &neg(b), depth)
}

fn mul(a: &Rep, b: &Rep, depth: usize) -> Option<Rep> {
    match (a, b) {
        (Rep::Ground(x), Rep::Ground(y)) => Some(Rep::Ground(x * y)),
        (Rep::Nested(xs), Rep::Nested(ys)) => {
            if xs.is_empty() || ys.is_empty() {
                return Some(Rep::zero(depth));
            }
            let n = xs.len() + ys.len() - 1;
            if n > MAX_DEGREE + 1 {
                return None; // degree blow-up guard
            }
            let mut out = vec![Rep::zero(depth - 1); n];
            for (i, x) in xs.iter().enumerate() {
                for (j, y) in ys.iter().enumerate() {
                    let p = mul(x, y, depth - 1)?;
                    out[i + j] = add(&out[i + j], &p, depth - 1);
                }
            }
            Some(Rep::Nested(out).normalize())
        }
        _ => unreachable!("depth mismatch"),
    }
}

/// Multiply by `x^k` (shift coefficients up).
fn shift(a: &Rep, k: usize, depth: usize) -> Rep {
    match a {
        Rep::Nested(xs) if !xs.is_empty() => {
            let mut out = vec![Rep::zero(depth - 1); k];
            out.extend(xs.iter().cloned());
            Rep::Nested(out)
        }
        _ => a.clone(),
    }
}

/// Exact division `a / b` in the polynomial ring, or `None` when `b` does not
/// divide `a` exactly (classical long division with recursive exact division
/// of leading coefficients).
fn exact_div(a: &Rep, b: &Rep, depth: usize) -> Option<Rep> {
    if b.is_zero() {
        return None;
    }
    if depth == 0 {
        let (Rep::Ground(x), Rep::Ground(y)) = (a, b) else {
            unreachable!()
        };
        return Some(Rep::Ground(x / y));
    }
    // A constant (degree-0) divisor divides coefficient-wise.
    if b.degree() == 0 && !matches!(b, Rep::Ground(_)) {
        let bc = &b.coeffs()[0];
        let cs = a
            .coeffs()
            .iter()
            .map(|c| exact_div(c, bc, depth - 1))
            .collect::<Option<Vec<_>>>()?;
        return Some(Rep::Nested(cs).normalize());
    }
    let mut r = a.clone().normalize();
    let mut q = Rep::zero(depth);
    let db = b.degree();
    let lb = b.lc(depth);
    let mut fuel = MAX_PRS_STEPS;
    while !r.is_zero() && r.degree() >= db {
        fuel = fuel.checked_sub(1)?;
        let c = exact_div(&r.lc(depth), &lb, depth - 1)?;
        let k = r.degree() - db;
        // term = c · x^k
        let term = shift(&Rep::Nested(vec![c]), k, depth);
        q = add(&q, &term, depth);
        let tb = mul(&term, b, depth)?;
        r = sub(&r, &tb, depth);
    }
    r.is_zero().then_some(q)
}

/// Pseudo-remainder of `a` by `b` in the main variable: the remainder of
/// `lc(b)^(da-db+1) · a` divided by `b` (always exact steps).
fn pseudo_rem(a: &Rep, b: &Rep, depth: usize) -> Option<Rep> {
    let db = b.degree();
    let lb = b.lc(depth);
    let mut r = a.clone().normalize();
    let mut fuel = MAX_PRS_STEPS;
    while !r.is_zero() && r.degree() >= db {
        fuel = fuel.checked_sub(1)?;
        let k = r.degree() - db;
        let lr = r.lc(depth);
        // r ← lc(b)·r − lc(r)·x^k·b
        let lbr = mul_coeff(&r, &lb, depth)?;
        let t = shift(&Rep::Nested(vec![lr]), k, depth);
        let tb = mul(&t, b, depth)?;
        r = sub(&lbr, &tb, depth).normalize();
    }
    Some(r)
}

/// Multiply every coefficient by a one-level-deeper value.
fn mul_coeff(a: &Rep, c: &Rep, depth: usize) -> Option<Rep> {
    match a {
        Rep::Nested(xs) => {
            let cs = xs
                .iter()
                .map(|x| mul(x, c, depth - 1))
                .collect::<Option<Vec<_>>>()?;
            Some(Rep::Nested(cs).normalize())
        }
        Rep::Ground(_) => mul(a, c, depth),
    }
}

/// Content: the GCD of the coefficients (one level deeper); primitive part is
/// the poly divided by it.
fn content(a: &Rep, depth: usize) -> Option<Rep> {
    let mut c = Rep::zero(depth - 1);
    for coeff in a.coeffs() {
        c = gcd(&c, coeff, depth - 1)?;
    }
    Some(c)
}

/// GCD of two rationals in the integer-primitive sense:
/// `gcd(p1/q1, p2/q2) = gcd(p1·q2, p2·q1)/(q1·q2)` (positive). This makes
/// primitive parts strip rational content (`−5x+10` → `−x+2`), which both
/// normalizes the result and bounds PRS coefficient swell.
fn ground_gcd(x: &BigRational, y: &BigRational) -> BigRational {
    fn int_gcd(mut a: BigInt, mut b: BigInt) -> BigInt {
        while !b.is_zero() {
            let r = &a % &b;
            a = b;
            b = r;
        }
        if a.is_negative() {
            -a
        } else {
            a
        }
    }
    if x.is_zero() {
        return y.abs();
    }
    if y.is_zero() {
        return x.abs();
    }
    let num = int_gcd(x.numer() * y.denom(), y.numer() * x.denom());
    BigRational::new(num, x.denom() * y.denom())
}

/// GCD (up to sign), primitive PRS with integer-primitive ground convention.
pub(crate) fn gcd(a: &Rep, b: &Rep, depth: usize) -> Option<Rep> {
    if depth == 0 {
        let (Rep::Ground(x), Rep::Ground(y)) = (a, b) else {
            unreachable!()
        };
        return Some(Rep::Ground(ground_gcd(x, y)));
    }
    let a = a.clone().normalize();
    let b = b.clone().normalize();
    if a.is_zero() {
        return Some(b);
    }
    if b.is_zero() {
        return Some(a);
    }

    let ca = content(&a, depth)?;
    let cb = content(&b, depth)?;
    let cg = gcd(&ca, &cb, depth - 1)?;
    let mut p = exact_div_coeffs(&a, &ca, depth)?;
    let mut q = exact_div_coeffs(&b, &cb, depth)?;
    if p.degree() < q.degree() {
        std::mem::swap(&mut p, &mut q);
    }

    let mut fuel = MAX_PRS_STEPS;
    loop {
        fuel = fuel.checked_sub(1)?;
        let r = pseudo_rem(&p, &q, depth)?;
        if r.is_zero() {
            break;
        }
        let cr = content(&r, depth)?;
        p = q;
        q = exact_div_coeffs(&r, &cr, depth)?;
    }
    // gcd = content-gcd · primitive(q), normalized so the leading ground
    // coefficient is positive (a canonical unit choice).
    let g = mul_coeff(&q, &cg, depth)?;
    Some(make_lc_positive(g))
}

/// Divide every coefficient by `c` exactly.
fn exact_div_coeffs(a: &Rep, c: &Rep, depth: usize) -> Option<Rep> {
    if c.is_zero() {
        return Some(a.clone());
    }
    match a {
        Rep::Nested(xs) => {
            let cs = xs
                .iter()
                .map(|x| exact_div(x, c, depth - 1))
                .collect::<Option<Vec<_>>>()?;
            Some(Rep::Nested(cs).normalize())
        }
        Rep::Ground(_) => exact_div(a, c, 0),
    }
}

/// Flip the overall sign so the innermost leading ground coefficient is
/// positive (fixes the gcd's unit ambiguity deterministically).
fn make_lc_positive(a: Rep) -> Rep {
    fn leading_ground(a: &Rep) -> Option<&BigRational> {
        match a {
            Rep::Ground(g) => Some(g),
            Rep::Nested(cs) => cs.last().and_then(leading_ground),
        }
    }
    if matches!(leading_ground(&a), Some(g) if g.is_negative()) {
        neg(&a)
    } else {
        a
    }
}

// ---- Expr ↔ poly conversion ----

/// Convert a *canonical* expression into a polynomial over `vars` (all free
/// variables must be listed). `None` if the expression is not a polynomial in
/// those variables (function applications, non-integer powers, …) or exceeds
/// the degree cap.
pub(crate) fn expr_to_poly(e: &Expr, vars: &[String]) -> Option<Rep> {
    let depth = vars.len();
    match e {
        Expr::Num(n) => {
            let g = number_to_rational(n)?;
            Some(ground_lift(Rep::Ground(g), depth))
        }
        Expr::Sym(s) => {
            let name = s.name();
            if crate::sym::is_constant_symbol(&name) {
                return None; // pi/e are not rational polynomial coefficients
            }
            let idx = vars.iter().position(|v| *v == name)?;
            Some(var_monomial(idx, depth))
        }
        Expr::Add(ts) => {
            let mut acc = Rep::zero(depth);
            for t in ts {
                acc = add(&acc, &expr_to_poly(t, vars)?, depth);
            }
            Some(acc)
        }
        Expr::Mul(fs) => {
            let mut acc = Rep::one(depth);
            for f in fs {
                acc = mul(&acc, &expr_to_poly(f, vars)?, depth)?;
            }
            Some(acc)
        }
        Expr::Pow(b, x) => {
            let Expr::Num(Number::Int(k)) = &**x else {
                return None;
            };
            if *k < 0 || *k as usize > MAX_DEGREE {
                return None;
            }
            let base = expr_to_poly(b, vars)?;
            let mut acc = Rep::one(depth);
            for _ in 0..*k {
                acc = mul(&acc, &base, depth)?;
            }
            Some(acc)
        }
        _ => None,
    }
}

/// `x_idx` as a polynomial at the given depth.
fn var_monomial(idx: usize, depth: usize) -> Rep {
    if idx == 0 {
        Rep::Nested(vec![Rep::zero(depth - 1), Rep::one(depth - 1)])
    } else {
        Rep::Nested(vec![var_monomial(idx - 1, depth - 1)])
    }
}

/// A ground constant lifted to the given depth.
fn ground_lift(g: Rep, depth: usize) -> Rep {
    if depth == 0 {
        g
    } else if g.is_zero() {
        Rep::zero(depth)
    } else {
        Rep::Nested(vec![ground_lift(g, depth - 1)])
    }
}

fn number_to_rational(n: &Number) -> Option<BigRational> {
    // Single source of truth in num.rs — this wrapper only keeps the local
    // call-site name; do not re-implement the conversion here.
    n.to_bigrational()
}

/// Convert back to a canonical expression.
pub(crate) fn poly_to_expr(rep: &Rep, vars: &[String]) -> Expr {
    match rep {
        Rep::Ground(g) => Expr::Num(Number::from_bigrational(g.clone())),
        Rep::Nested(cs) => {
            let terms = cs
                .iter()
                .enumerate()
                .filter(|(_, c)| !c.is_zero())
                .map(|(i, c)| {
                    let coeff = poly_to_expr(c, &vars[1..]);
                    let power = crate::norm::pow(
                        Expr::sym(&vars[0]),
                        Expr::Num(Number::Int(i as i64)),
                    );
                    crate::norm::mul(vec![coeff, power])
                })
                .collect();
            crate::norm::add(terms)
        }
    }
}

/// Exact polynomial division at the top level (used by `reduce_rational`
/// after the gcd is known to divide both sides).
pub(crate) fn exact_div_top(a: &Rep, b: &Rep, depth: usize) -> Option<Rep> {
    exact_div(a, b, depth)
}

/// The rational content (gcd of every ground coefficient, positive), and the
/// poly divided by it. Zero polys return (1, self) so callers never divide
/// by zero.
pub(crate) fn strip_rational_content(a: &Rep) -> (BigRational, Rep) {
    fn walk(a: &Rep, acc: &mut BigRational) {
        match a {
            Rep::Ground(g) => *acc = ground_gcd(acc, g),
            Rep::Nested(cs) => cs.iter().for_each(|c| walk(c, acc)),
        }
    }
    let mut c = BigRational::zero();
    walk(a, &mut c);
    if c.is_zero() || c.is_one() {
        return (BigRational::one(), a.clone());
    }
    fn scale(a: &Rep, inv: &BigRational) -> Rep {
        match a {
            Rep::Ground(g) => Rep::Ground(g * inv),
            Rep::Nested(cs) => Rep::Nested(cs.iter().map(|x| scale(x, inv)).collect()),
        }
    }
    let inv = c.recip();
    (c, scale(a, &inv))
}

/// Is the gcd trivial (a constant in every variable)?
pub(crate) fn is_trivial(g: &Rep) -> bool {
    g.total_degree_zero() || g.is_one()
}
