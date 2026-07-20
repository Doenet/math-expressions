//! Univariate polynomial factorization over ℚ (WHATS_LEFT item 8).
//!
//! `factor` expands the input to a polynomial, then factors it exactly:
//! rational content / leading coefficient out front, Yun squarefree
//! decomposition, and rational-root deflation into monic linear factors. Any
//! squarefree part with no rational roots (an irreducible quadratic, or a
//! higher-degree piece we do not split further over ℚ) is kept as a monic
//! polynomial factor. The result is a raw product tree — deliberately *not*
//! re-canonicalized, since that would expand it straight back out.
//!
//! Non-polynomial, multivariate, degree-capped (§7f `max_factor_degree`), or
//! otherwise unfactorable inputs are returned canonicalized and unchanged. Every factorization is checked against the
//! original with `equals` before being returned, so a bookkeeping slip can
//! never yield a wrong answer — only a missed factoring.

use crate::expr::Expr;
use crate::num::Number;
use crate::{norm, ops, upoly};
use num_rational::BigRational;
use num_traits::{One, Zero};

/// Factor a univariate polynomial over ℚ. Returns the input canonicalized and
/// unchanged when it is not a single-variable polynomial of degree ≥ 2.
pub fn factor(e: &Expr) -> Expr {
    factor_univariate(e).unwrap_or_else(|| norm::canonicalize(e))
}

fn factor_univariate(e: &Expr) -> Option<Expr> {
    let vars = ops::variables(e);
    let [var] = vars.as_slice() else {
        return None; // constant or multivariate — nothing univariate to do
    };
    let var = var.clone();

    // Fully distribute, then read off dense rational coefficients.
    let expanded = norm::canonicalize(&norm::expand(e));
    let coeffs = extract_upoly(&expanded, &var)?;
    if upoly::degree(&coeffs) < 2 {
        return None; // degree ≤ 1 is already irreducible
    }

    let lc = coeffs.last()?.clone();
    let mut factors: Vec<Expr> = Vec::new();
    for (sqfree, mult) in upoly::squarefree_decomposition(&coeffs) {
        let (roots, cofactor) = upoly::rational_roots(&sqfree);
        for r in &roots {
            factors.push(with_exponent(linear_factor(r, &var), mult));
        }
        if upoly::degree(&cofactor) >= 1 {
            factors.push(with_exponent(upoly_to_expr(&cofactor, &var), mult));
        }
    }

    if factors.is_empty() {
        return None;
    }
    if !lc.is_one() {
        factors.insert(0, Expr::Num(Number::from_bigrational(lc)));
    }
    let factored = if factors.len() == 1 {
        factors.into_iter().next().unwrap()
    } else {
        Expr::Mul(factors)
    };

    // Safety gate: never hand back a factorization that isn't equal.
    crate::eq::equals(&factored, e, &crate::eq::EqOptions::default()).then_some(factored)
}

/// `x - r`, as a raw two-term sum (`x` alone when `r = 0`).
fn linear_factor(r: &BigRational, var: &str) -> Expr {
    if r.is_zero() {
        return Expr::sym(var);
    }
    Expr::Add(vec![
        Expr::sym(var),
        Expr::Num(Number::from_bigrational(-r.clone())),
    ])
}

/// Render dense coefficients (low → high) as a raw `c₀ + c₁x + c₂x² + …` sum.
fn upoly_to_expr(coeffs: &[BigRational], var: &str) -> Expr {
    let mono = |i: usize| match i {
        0 => None,
        1 => Some(Expr::sym(var)),
        _ => Some(Expr::Pow(
            Box::new(Expr::sym(var)),
            Box::new(Expr::int(i as i64)),
        )),
    };
    let terms: Vec<Expr> = coeffs
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.is_zero())
        .map(|(i, c)| {
            let num = Expr::Num(Number::from_bigrational(c.clone()));
            match (mono(i), c.is_one()) {
                (None, _) => num,        // constant term
                (Some(m), true) => m,    // coefficient 1
                (Some(m), false) => Expr::Mul(vec![num, m]),
            }
        })
        .collect();
    match terms.len() {
        1 => terms.into_iter().next().unwrap(),
        _ => Expr::Add(terms),
    }
}

/// Raise a factor to `n` (raw `Pow`; the factor itself when `n = 1`).
fn with_exponent(base: Expr, n: u32) -> Expr {
    if n == 1 {
        base
    } else {
        Expr::Pow(Box::new(base), Box::new(Expr::int(i64::from(n))))
    }
}

/// Dense rational coefficients of a *canonical, expanded* single-variable
/// polynomial tree (`Add` of monomials; each monomial `Num`, `x^k`, or a `Mul`
/// of those). `None` if any term is not a monomial in `var`, or if the degree
/// exceeds `max_factor_degree` (§7f: the dense vector below allocates one
/// entry per degree, so an adversarial `x^10^9` must be refused, not sized).
fn extract_upoly(e: &Expr, var: &str) -> Option<upoly::UPoly> {
    let cap = crate::resource_limits::current().max_factor_degree;
    fn monomial(e: &Expr, var: &str, cap: usize) -> Option<(usize, BigRational)> {
        match e {
            Expr::Num(n) => Some((0, n.to_bigrational()?)),
            Expr::Sym(s) if s.name() == var => Some((1, BigRational::one())),
            Expr::Pow(b, x) => match (&**b, &**x) {
                (Expr::Sym(s), Expr::Num(Number::Int(k)))
                    if s.name() == var && *k >= 1 && *k <= cap as i64 =>
                {
                    Some((*k as usize, BigRational::one()))
                }
                _ => None,
            },
            Expr::Mul(fs) => {
                let mut deg = 0usize;
                let mut coeff = BigRational::one();
                for f in fs {
                    let (d, c) = monomial(f, var, cap)?;
                    deg = deg.checked_add(d).filter(|deg| *deg <= cap)?;
                    coeff *= c;
                }
                Some((deg, coeff))
            }
            _ => None,
        }
    }
    let terms: Vec<&Expr> = match e {
        Expr::Add(ts) => ts.iter().collect(),
        other => vec![other],
    };
    let mut out: upoly::UPoly = Vec::new();
    for t in terms {
        let (d, c) = monomial(t, var, cap)?;
        if out.len() <= d {
            out.resize(d + 1, BigRational::zero());
        }
        out[d] += c;
    }
    upoly::trim(&mut out);
    Some(out)
}
