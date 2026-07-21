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
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

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
            // Split the no-rational-root remainder into irreducibles over ℚ
            // (S4): `x⁶−1`'s `x⁴+x²+1` becomes `(x²+x+1)(x²−x+1)`.
            for piece in split_over_q(&cofactor) {
                factors.push(with_exponent(upoly_to_expr(&piece, &var), mult));
            }
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
    crate::equality::equals(&factored, e, &crate::equality::EqOptions::default()).then_some(factored)
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

// ============================ S4: irreducible splitting ============================

/// Split a squarefree cofactor (degree ≥ 2, no rational roots) into its
/// irreducible factors over ℚ. The product of the returned factors equals the
/// input **exactly**, so substituting these back into `factor` cannot change
/// the value. Falls back to `[coeffs]` unchanged when factoring exceeds the
/// Kronecker budget (a missed factoring, never a wrong one).
fn split_over_q(coeffs: &upoly::UPoly) -> Vec<upoly::UPoly> {
    if upoly::degree(coeffs) < 2 {
        return vec![coeffs.clone()];
    }
    let mut budget: i64 = 200_000;
    factor_int(coeffs, &mut budget)
}

/// Recursively factor a rational polynomial into irreducibles by Kronecker's
/// method. The product of the returned factors equals `p` exactly. Every level
/// re-normalizes its input to primitive integer coefficients (Gauss's lemma:
/// factors can be taken primitive) — required because an interpolated factor
/// may be a non-integer rational multiple of the true factor, and the divisor
/// enumeration below is only exact over ℤ.
fn factor_int(p: &upoly::UPoly, budget: &mut i64) -> Vec<upoly::UPoly> {
    if upoly::degree(p) <= 1 {
        return vec![p.clone()];
    }
    // p = scale · pp, with pp primitive over ℤ.
    let pp: upoly::UPoly = upoly::to_primitive_int(p)
        .iter()
        .map(|c| BigRational::from_integer(c.clone()))
        .collect();
    let (Some(cl), Some(pl)) = (p.last(), pp.last()) else {
        return vec![p.clone()];
    };
    if pl.is_zero() {
        return vec![p.clone()];
    }
    let scale = cl / pl;

    let mut pieces = match find_factor(&pp, budget) {
        Some(g) => {
            let (q, r) = upoly::divrem(&pp, &g);
            if !upoly::is_zero(&r) || upoly::degree(&q) < 1 {
                vec![pp.clone()] // defensive: not an exact split
            } else {
                let mut out = factor_int(&g, budget);
                out.extend(factor_int(&q, budget));
                out
            }
        }
        None => vec![pp.clone()],
    };
    // Reattach the rational scale to the first piece so ∏ pieces == p.
    if !scale.is_one() {
        if let Some(first) = pieces.first_mut() {
            *first = upoly::scale(first, &scale);
        }
    }
    pieces
}

/// Search for a proper factor of `p` of degree `1..=deg/2` by interpolating
/// through divisors of `p` at integer nodes (Kronecker). `None` when `p` is
/// irreducible or the search exceeds the divisor / combination budget.
fn find_factor(p: &upoly::UPoly, budget: &mut i64) -> Option<upoly::UPoly> {
    let n = upoly::degree(p);
    for d in 1..=n / 2 {
        let nodes: Vec<BigInt> = (0..=d).map(node).collect();
        let vals: Vec<BigInt> = nodes.iter().map(|x| eval_int(p, x)).collect();
        // A zero value is a rational root (removed upstream, handled defensively).
        if let Some(pos) = vals.iter().position(Zero::is_zero) {
            return Some(vec![
                -BigRational::from_integer(nodes[pos].clone()),
                BigRational::one(),
            ]);
        }
        // ± each positive divisor of every value.
        let mut divsets: Vec<Vec<BigInt>> = Vec::with_capacity(nodes.len());
        for v in &vals {
            let mut ds = upoly::divisors_capped(&v.abs())?;
            let neg: Vec<BigInt> = ds.iter().map(|d| -d.clone()).collect();
            ds.extend(neg);
            divsets.push(ds);
        }
        if let Some(g) = search_combos(&nodes, &divsets, p, budget) {
            return Some(g);
        }
    }
    None
}

/// Iterate the Cartesian product of divisor choices, interpolating a candidate
/// factor for each and accepting the first that divides `p` exactly.
fn search_combos(
    nodes: &[BigInt],
    divsets: &[Vec<BigInt>],
    p: &upoly::UPoly,
    budget: &mut i64,
) -> Option<upoly::UPoly> {
    let sizes: Vec<usize> = divsets.iter().map(Vec::len).collect();
    if sizes.contains(&0) {
        return None;
    }
    let xs: Vec<BigRational> = nodes
        .iter()
        .map(|x| BigRational::from_integer(x.clone()))
        .collect();
    let mut idx = vec![0usize; sizes.len()];
    loop {
        if *budget <= 0 {
            return None;
        }
        *budget -= 1;
        let ys: Vec<BigRational> = idx
            .iter()
            .enumerate()
            .map(|(i, &j)| BigRational::from_integer(divsets[i][j].clone()))
            .collect();
        if let Some(g) = interpolate(&xs, &ys) {
            let dg = upoly::degree(&g);
            if dg >= 1 && dg < upoly::degree(p) {
                let (q, r) = upoly::divrem(p, &g);
                if upoly::is_zero(&r) && upoly::degree(&q) >= 1 {
                    return Some(g);
                }
            }
        }
        // Mixed-radix increment.
        let mut i = 0;
        loop {
            if i == sizes.len() {
                return None; // product exhausted
            }
            idx[i] += 1;
            if idx[i] < sizes[i] {
                break;
            }
            idx[i] = 0;
            i += 1;
        }
    }
}

/// The `i`-th integer node: 0, 1, −1, 2, −2, … (small, to keep values small).
fn node(i: usize) -> BigInt {
    let m = i.div_ceil(2) as i64;
    let s = if i % 2 == 1 { 1 } else { -1 };
    BigInt::from(s * m)
}

/// `p(x)` at an integer point (`p` has integer coefficients).
fn eval_int(p: &upoly::UPoly, x: &BigInt) -> BigInt {
    upoly::eval_rat(p, &BigRational::from_integer(x.clone())).to_integer()
}

/// Lagrange interpolation through `(xs[i], ys[i])`. `None` on duplicate nodes.
fn interpolate(xs: &[BigRational], ys: &[BigRational]) -> Option<upoly::UPoly> {
    let n = xs.len();
    let mut acc: upoly::UPoly = Vec::new();
    for i in 0..n {
        let mut num: upoly::UPoly = vec![BigRational::one()];
        let mut den = BigRational::one();
        for j in 0..n {
            if i == j {
                continue;
            }
            num = upoly::mul(&num, &[-xs[j].clone(), BigRational::one()]);
            let diff = &xs[i] - &xs[j];
            if diff.is_zero() {
                return None;
            }
            den *= diff;
        }
        let scale = &ys[i] / &den;
        acc = upoly::add_p(&acc, &upoly::scale(&num, &scale));
    }
    upoly::trim(&mut acc);
    Some(acc)
}

// ============================ S4: factor_terms ============================

/// Pull the common numeric content and common (possibly kernel) factors out of
/// a sum — `factor_terms` (`6x²+9x → 3x(2x+3)`). Cheap and multivariate; the
/// S7 driver always tries it. Returns the input canonicalized when there is
/// nothing common to pull. The result is gate-checked against the input.
pub fn factor_terms(e: &Expr) -> Expr {
    factor_terms_opt(e).unwrap_or_else(|| norm::canonicalize(e))
}

fn factor_terms_opt(e: &Expr) -> Option<Expr> {
    let expanded = norm::canonicalize(&norm::expand(e));
    let terms: Vec<Expr> = match &expanded {
        Expr::Add(ts) => ts.clone(),
        _ => return None, // a single term has nothing to factor against
    };
    let parts: Vec<(BigRational, std::collections::HashMap<Expr, i64>)> =
        terms.iter().map(term_parts).collect();

    // Common numeric content and common factors (min exponent across terms).
    let mut g = parts[0].0.clone();
    for (c, _) in &parts[1..] {
        g = rational_gcd(&g, c);
    }
    let mut common = parts[0].1.clone();
    for (_, fac) in &parts[1..] {
        common.retain(|base, exp| match fac.get(base) {
            Some(e2) => {
                *exp = (*exp).min(*e2);
                *exp > 0
            }
            None => false,
        });
    }
    if (g.is_one() || g == -BigRational::one()) && common.is_empty() {
        return None; // nothing to pull
    }

    // Build the pulled-out product and the reduced remainder sum. HashMap
    // iteration order is nondeterministic, so every factor list is sorted by
    // the canonical order first — wasm and native must produce the same tree.
    let sorted = |m: &std::collections::HashMap<Expr, i64>| -> Vec<(Expr, i64)> {
        let mut v: Vec<(Expr, i64)> = m.iter().map(|(b, e)| (b.clone(), *e)).collect();
        v.sort_by(|(a, _), (b, _)| crate::norm::cmp(a, b));
        v
    };
    let mut pulled: Vec<Expr> = Vec::new();
    if !g.is_one() {
        pulled.push(Expr::Num(Number::from_bigrational(g.clone())));
    }
    for (base, exp) in sorted(&common) {
        pulled.push(with_exponent(base, exp as u32));
    }
    let remainder: Vec<Expr> = parts
        .iter()
        .map(|(c, fac)| {
            let mut fs: Vec<Expr> = Vec::new();
            let coeff = c / &g;
            let mut leading_num = None;
            if !coeff.is_one() {
                leading_num = Some(Expr::Num(Number::from_bigrational(coeff)));
            }
            for (base, exp) in sorted(fac) {
                let reduced = exp - common.get(&base).copied().unwrap_or(0);
                if reduced > 0 {
                    fs.push(with_exponent(base, reduced as u32));
                }
            }
            match (leading_num, fs.len()) {
                (None, 0) => Expr::int(1),
                (Some(n), 0) => n,
                (None, 1) => fs.into_iter().next().unwrap(),
                (Some(n), _) => {
                    let mut v = vec![n];
                    v.extend(fs);
                    Expr::Mul(v)
                }
                (None, _) => Expr::Mul(fs),
            }
        })
        .collect();

    pulled.push(Expr::Add(remainder));
    let factored = if pulled.len() == 1 {
        pulled.into_iter().next().unwrap()
    } else {
        Expr::Mul(pulled)
    };
    crate::equality::equals(&factored, e, &crate::equality::EqOptions::default()).then_some(factored)
}

/// Split a term into `(rational coefficient, {base → integer exponent})`.
fn term_parts(t: &Expr) -> (BigRational, std::collections::HashMap<Expr, i64>) {
    let mut coeff = BigRational::one();
    let mut fac: std::collections::HashMap<Expr, i64> = std::collections::HashMap::new();
    let factors: Vec<Expr> = match t {
        Expr::Mul(fs) => fs.clone(),
        other => vec![other.clone()],
    };
    for f in factors {
        match &f {
            Expr::Num(n) => match n.to_bigrational() {
                Some(q) => coeff *= q,
                None => *fac.entry(f.clone()).or_default() += 1,
            },
            Expr::Pow(b, x) if matches!(&**x, Expr::Num(Number::Int(_))) => {
                if let Expr::Num(Number::Int(k)) = &**x {
                    *fac.entry((**b).clone()).or_default() += *k;
                }
            }
            _ => *fac.entry(f.clone()).or_default() += 1,
        }
    }
    (coeff, fac)
}

/// Positive GCD of two rationals (`gcd(p1/q1, p2/q2) = gcd(p1q2, p2q1)/(q1q2)`).
fn rational_gcd(x: &BigRational, y: &BigRational) -> BigRational {
    fn int_gcd(mut a: BigInt, mut b: BigInt) -> BigInt {
        while !b.is_zero() {
            let r = &a % &b;
            a = b;
            b = r;
        }
        a.abs()
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
