//! The complete rational-function engine.
//!
//! `∫ p/q dx` for p, q ∈ ℚ[x] decides *every* input under the degree caps:
//!
//! 1. polynomial part by division;
//! 2. **Ostrogradsky–Hermite**: the rational part `P₁/q₁` (q₁ = gcd(q, q′))
//!    by a linear solve — no iterated reductions to get wrong;
//! 3. **Rothstein–Trager** on the squarefree remainder: the resultant
//!    `R(t) = res_x(q₂, A − t·q₂′)` (computed by evaluation + Lagrange
//!    interpolation, which sidesteps subresultant bookkeeping), whose roots
//!    are the log residues; each residue class α contributes
//!    `α·ln(gcd(q₂, A − α·q₂′))`, the gcd taken over ℚ, ℚ(√m), or
//!    ℚ[t]/(F) as the residue ladder dictates (rational → quadratic closed
//!    form with the atan/ln real cleanup → `RootOf`).
//!
//! Every emitted antiderivative is verified upstream by the derivative gate.

use crate::expr::Expr;
use crate::num::Number;
use crate::upoly::{self, UPoly};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

use crate::norm::{add as cadd, mul as cmul, pow as cpow};

fn num(r: &BigRational) -> Expr {
    Expr::Num(Number::from_bigrational(r.clone()))
}

fn int(i: i64) -> Expr {
    Expr::Num(Number::Int(i))
}

// Layering note (FULL_SIMPLIFY §8, assessed 2026-07-22): this is the
// univariate rational-function converter — dense `(num, den)` `UPoly` pairs in
// one named variable, exactly what the LRT integrator consumes. It is
// deliberately NOT merged with `crate::ratform` (`together`/`cancel`), which is
// the multivariate Expr-level normal form over `poly::Rep` with
// opaque-kernelization; the two share no representation. If a `Rep`↔`UPoly`
// converter ever exists, revisit — until then use ratform for Expr rewriting
// and this for univariate coefficient extraction.
/// A canonical tree as a rational function in `x` over ℚ, returned as a
/// `(numerator, denominator)` `UPoly` pair with the denominator normalized
/// monic. `None` = not in ℚ(x) (symbolic coefficients, other functions, …).
pub(crate) fn expr_to_ratfun(e: &Expr, x: &str) -> Option<(UPoly, UPoly)> {
    let cap = crate::resource_limits::current().max_lrt_degree;
    fn conv(e: &Expr, x: &str, cap: usize) -> Option<(UPoly, UPoly)> {
        let one = || vec![BigRational::one()];
        match e {
            Expr::Num(n) => Some((vec![n.to_bigrational()?], one())),
            Expr::Sym(s) if s.name() == x => {
                Some((vec![BigRational::zero(), BigRational::one()], one()))
            }
            Expr::Add(ts) => {
                let mut acc = (Vec::new(), one());
                for t in ts {
                    let (c, d) = conv(t, x, cap)?;
                    let n = upoly::add_p(&upoly::mul(&acc.0, &d), &upoly::mul(&c, &acc.1));
                    let den = upoly::mul(&acc.1, &d);
                    if upoly::degree(&n) > cap || upoly::degree(&den) > cap {
                        return None;
                    }
                    // Keep sizes down: cancel the gcd as we fold.
                    let g = upoly::gcd(&n, &den);
                    acc = if upoly::degree(&g) >= 1 {
                        (upoly::divrem(&n, &g).0, upoly::divrem(&den, &g).0)
                    } else {
                        (n, den)
                    };
                }
                Some(acc)
            }
            Expr::Mul(fs) => {
                let mut acc = (one(), one());
                for f in fs {
                    let (c, d) = conv(f, x, cap)?;
                    acc = (upoly::mul(&acc.0, &c), upoly::mul(&acc.1, &d));
                    if upoly::degree(&acc.0) > cap || upoly::degree(&acc.1) > cap {
                        return None;
                    }
                }
                Some(acc)
            }
            Expr::Pow(b, k) => {
                let Expr::Num(Number::Int(k)) = &**k else {
                    return None;
                };
                let (n, d) = conv(b, x, cap)?;
                let (mut bn, mut bd) = if *k >= 0 { (n, d) } else { (d, n) };
                let mut kk = k.unsigned_abs();
                if kk as usize * upoly::degree(&bn).max(upoly::degree(&bd)) > cap {
                    return None;
                }
                let (mut rn, mut rd) = (one(), one());
                while kk > 0 {
                    if kk & 1 == 1 {
                        rn = upoly::mul(&rn, &bn);
                        rd = upoly::mul(&rd, &bd);
                    }
                    kk >>= 1;
                    if kk > 0 {
                        bn = upoly::mul(&bn, &bn);
                        bd = upoly::mul(&bd, &bd);
                    }
                }
                Some((rn, rd))
            }
            _ => None,
        }
    }
    let (n, d) = conv(e, x, cap)?;
    if upoly::is_zero(&d) {
        return None;
    }
    // Cancel and normalize the denominator monic.
    let g = upoly::gcd(&n, &d);
    let (n, d) = if upoly::degree(&g) >= 1 {
        (upoly::divrem(&n, &g).0, upoly::divrem(&d, &g).0)
    } else {
        (n, d)
    };
    let lc = d.last()?.clone();
    Some((
        n.iter().map(|c| c / &lc).collect(),
        d.iter().map(|c| c / &lc).collect(),
    ))
}

/// `∫ num/den dx`, complete over ℚ(x). `None` only on caps/ring failures.
pub(super) fn integrate_rational(num_p: &UPoly, den: &UPoly, x: &str) -> Option<Expr> {
    let xs = Expr::sym(x);
    if upoly::degree(den) == 0 {
        // Purely polynomial (den is a constant, made 1 by normalization).
        return Some(integrate_poly(num_p, &xs));
    }
    let (quot, rem) = upoly::divrem(num_p, den);
    let mut terms = vec![integrate_poly(&quot, &xs)];
    if upoly::is_zero(&rem) {
        return Some(cadd(terms));
    }
    // Ostrogradsky–Hermite: rem/den = (P1/q1)′ + P2/q2.
    let dq = upoly::derivative(den);
    let q1 = upoly::gcd(den, &dq);
    let (a_rem, q2) = if upoly::degree(&q1) >= 1 {
        let q2 = upoly::divrem(den, &q1).0;
        let (p1, p2) = ostrogradsky(&rem, &q1, &q2)?;
        terms.push(cmul(vec![
            poly_expr(&p1, &xs),
            cpow(poly_expr(&q1, &xs), int(-1)),
        ]));
        (p2, q2)
    } else {
        (rem.clone(), den.clone())
    };
    if !upoly::is_zero(&a_rem) {
        terms.push(log_part(&a_rem, &q2, &xs)?);
    }
    Some(cadd(terms))
}

fn integrate_poly(p: &UPoly, xs: &Expr) -> Expr {
    let terms: Vec<Expr> = p
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.is_zero())
        .map(|(i, c)| {
            let coeff = c / BigRational::from_integer(BigInt::from(i as i64 + 1));
            cmul(vec![num(&coeff), cpow(xs.clone(), int(i as i64 + 1))])
        })
        .collect();
    cadd(terms)
}

fn poly_expr(p: &UPoly, xs: &Expr) -> Expr {
    let terms: Vec<Expr> = p
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.is_zero())
        .map(|(i, c)| match i {
            0 => num(c),
            _ => cmul(vec![num(c), cpow(xs.clone(), int(i as i64))]),
        })
        .collect();
    cadd(terms)
}

/// Solve `p = P1′·q2 − P1·T + P2·q1` (T = q1′q2/q1, exact) for deg P1 <
/// deg q1, deg P2 < deg q2, by Gaussian elimination over ℚ.
#[allow(clippy::needless_range_loop)] // parallel row indexing in one matrix
fn ostrogradsky(p: &UPoly, q1: &UPoly, q2: &UPoly) -> Option<(UPoly, UPoly)> {
    let (a, b) = (upoly::degree(q1), upoly::degree(q2));
    let n = a + b; // deg q — also the equation count (deg p < n)
    let (t_num, t_rem) = upoly::divrem(&upoly::mul(&upoly::derivative(q1), q2), q1);
    if !upoly::is_zero(&t_rem) {
        return None; // cannot happen for q1 = gcd(q, q′); guard anyway
    }
    // Columns: P1 coefficients (a unknowns), then P2 coefficients (b).
    let mut m = vec![vec![BigRational::zero(); n + 1]; n]; // last col = rhs
    for j in 0..a {
        // Contribution of P1_j: derivative term j·x^{j−1}·q2 − x^j·T.
        if j > 0 {
            for (k, c) in q2.iter().enumerate() {
                if j - 1 + k < n {
                    m[j - 1 + k][j] += c * BigRational::from_integer(BigInt::from(j as i64));
                }
            }
        }
        for (k, c) in t_num.iter().enumerate() {
            if j + k < n {
                m[j + k][j] -= c;
            }
        }
    }
    for j in 0..b {
        for (k, c) in q1.iter().enumerate() {
            if j + k < n {
                m[j + k][a + j] += c;
            }
        }
    }
    for (k, c) in p.iter().enumerate() {
        if k < n {
            m[k][n] = c.clone();
        }
    }
    // Gaussian elimination.
    let mut row = 0;
    let mut pivots: Vec<(usize, usize)> = Vec::new();
    for col in 0..n {
        let Some(pr) = (row..n).find(|&r| !m[r][col].is_zero()) else {
            continue;
        };
        m.swap(row, pr);
        let inv = BigRational::one() / m[row][col].clone();
        for v in m[row].iter_mut() {
            *v = &*v * &inv;
        }
        for r in 0..n {
            if r != row && !m[r][col].is_zero() {
                let f = m[r][col].clone();
                for cidx in 0..=n {
                    let sub = &m[row][cidx] * &f;
                    m[r][cidx] -= sub;
                }
            }
        }
        pivots.push((row, col));
        row += 1;
    }
    // Any zero row with nonzero rhs → inconsistent (cannot happen).
    for r in row..n {
        if !m[r][n].is_zero() {
            return None;
        }
    }
    let mut sol = vec![BigRational::zero(); n];
    for (r, c) in pivots {
        sol[c] = m[r][n].clone();
    }
    let mut p1: UPoly = sol[..a].to_vec();
    let mut p2: UPoly = sol[a..].to_vec();
    upoly::trim(&mut p1);
    upoly::trim(&mut p2);
    Some((p1, p2))
}

// ================= Rothstein–Trager =================

/// `Σ_α α·ln(gcd(q, A − α·q′))` over the roots α of the Rothstein–Trager
/// resultant — `q` squarefree, deg A < deg q.
fn log_part(a: &UPoly, q: &UPoly, xs: &Expr) -> Option<Expr> {
    let dq = upoly::derivative(q);
    let r = rt_resultant(q, a, &dq)?;
    if upoly::degree(&r) == 0 {
        return upoly::is_zero(a).then(|| int(0));
    }
    // Distinct residues: the radical of R, primitive.
    let g = upoly::gcd(&r, &upoly::derivative(&r));
    let radical = if upoly::degree(&g) >= 1 {
        upoly::divrem(&r, &g).0
    } else {
        r
    };
    let mut terms: Vec<Expr> = Vec::new();
    let (rational_roots, mut rest) = upoly::rational_roots(&radical);
    for alpha in &rational_roots {
        if alpha.is_zero() {
            continue; // zero residue contributes nothing
        }
        let shifted = shift_by_scalar(a, &dq, alpha);
        let gcd_q = upoly::gcd(q, &shifted);
        terms.push(cmul(vec![
            num(alpha),
            log_expr(poly_expr(&upoly::monic(&gcd_q), xs)),
        ]));
    }
    if upoly::degree(&rest) == 1 {
        let alpha = -&rest[0] / &rest[1];
        if !alpha.is_zero() {
            let shifted = shift_by_scalar(a, &dq, &alpha);
            let gcd_q = upoly::gcd(q, &shifted);
            terms.push(cmul(vec![
                num(&alpha),
                log_expr(poly_expr(&upoly::monic(&gcd_q), xs)),
            ]));
        }
        rest = Vec::new();
    }
    if upoly::degree(&rest) == 2 {
        terms.push(quadratic_residues(&rest, a, q, &dq, xs)?);
    } else if upoly::degree(&rest) >= 3 {
        terms.push(rootof_residues(&rest, a, q, &dq, xs)?);
    }
    Some(cadd(terms))
}

/// `A − α·q′` for rational α.
fn shift_by_scalar(a: &UPoly, dq: &UPoly, alpha: &BigRational) -> UPoly {
    upoly::sub(a, &upoly::scale(dq, alpha))
}

/// Resultant `res_x(q, A − t·q′)` in ℚ[t] via evaluation at deg q + 1 good
/// points and Lagrange interpolation (resultants specialize at any t where
/// the x-degree does not drop).
fn rt_resultant(q: &UPoly, a: &UPoly, dq: &UPoly) -> Option<UPoly> {
    let deg_t = upoly::degree(q);
    // Generic x-degree of A − t·q′.
    let generic_deg = upoly::degree(a).max(upoly::degree(dq));
    let mut points: Vec<(BigRational, BigRational)> = Vec::new();
    let mut tj = BigRational::zero();
    let mut tries = 0;
    while points.len() < deg_t + 1 {
        tries += 1;
        if tries > 4 * (deg_t + 2) {
            return None;
        }
        let spec = shift_by_scalar(a, dq, &tj);
        tj += BigRational::one();
        if upoly::degree(&spec) != generic_deg || upoly::is_zero(&spec) {
            continue; // degree dropped at this t — skip the point
        }
        let val = resultant_q(q, &spec)?;
        points.push((&tj - BigRational::one(), val));
    }
    Some(lagrange(&points))
}

/// Resultant of two ℚ[x] polynomials by the Euclidean PRS formula.
fn resultant_q(a: &UPoly, b: &UPoly) -> Option<BigRational> {
    let (mut a, mut b) = (a.clone(), b.clone());
    let mut acc = BigRational::one();
    loop {
        if upoly::is_zero(&b) {
            return Some(if upoly::degree(&a) == 0 && !upoly::is_zero(&a) {
                acc
            } else {
                BigRational::zero()
            });
        }
        if upoly::degree(&b) == 0 {
            // res(A, c) = c^deg A.
            let c = b[0].clone();
            let mut p = BigRational::one();
            for _ in 0..upoly::degree(&a) {
                p *= &c;
            }
            return Some(acc * p);
        }
        let (da, db) = (upoly::degree(&a), upoly::degree(&b));
        let r = upoly::divrem(&a, &b).1;
        let dr = if upoly::is_zero(&r) { 0 } else { upoly::degree(&r) };
        if upoly::is_zero(&r) {
            return Some(BigRational::zero());
        }
        // res(A,B) = (−1)^(da·db) · lc(B)^(da − dr) · res(B, R).
        let lc = b[db].clone();
        let mut p = BigRational::one();
        for _ in 0..(da - dr) {
            p *= &lc;
        }
        acc *= p;
        if (da * db) % 2 == 1 {
            acc = -acc;
        }
        a = b;
        b = r;
    }
}

fn lagrange(points: &[(BigRational, BigRational)]) -> UPoly {
    let mut acc: UPoly = Vec::new();
    for (i, (xi, yi)) in points.iter().enumerate() {
        // Basis polynomial ∏_{j≠i} (t − xj)/(xi − xj), scaled by yi.
        let mut basis: UPoly = vec![yi.clone()];
        for (j, (xj, _)) in points.iter().enumerate() {
            if i == j {
                continue;
            }
            let denom = xi - xj;
            basis = upoly::mul(&basis, &[-(xj / &denom), BigRational::one() / denom]);
        }
        acc = upoly::add_p(&acc, &basis);
    }
    upoly::trim(&mut acc);
    acc
}

// ---- quadratic residue class: closed forms with the atan/ln cleanup ----

/// Elements of ℚ(√m): `u + v·√m` (m a non-square rational, possibly < 0).
type Qe = (BigRational, BigRational);

fn qe_mul(a: &Qe, b: &Qe, m: &BigRational) -> Qe {
    (
        &a.0 * &b.0 + &a.1 * &b.1 * m,
        &a.0 * &b.1 + &a.1 * &b.0,
    )
}

fn qe_inv(a: &Qe, m: &BigRational) -> Option<Qe> {
    let d = &a.0 * &a.0 - &a.1 * &a.1 * m;
    if d.is_zero() {
        return None;
    }
    Some((&a.0 / &d, -&a.1 / &d))
}

fn qe_is_zero(a: &Qe) -> bool {
    a.0.is_zero() && a.1.is_zero()
}

type QePoly = Vec<Qe>;

fn qep_trim(p: &mut QePoly) {
    while p.last().is_some_and(qe_is_zero) {
        p.pop();
    }
}

fn qep_divrem(a: &QePoly, b: &QePoly, m: &BigRational) -> Option<(QePoly, QePoly)> {
    let db = b.len().checked_sub(1)?;
    let lc_inv = qe_inv(&b[db], m)?;
    let mut r = a.clone();
    qep_trim(&mut r);
    let mut q: QePoly = Vec::new();
    while r.len() > db {
        let dr = r.len() - 1;
        let coeff = qe_mul(&r[dr], &lc_inv, m);
        if q.len() < dr - db + 1 {
            q.resize(dr - db + 1, (BigRational::zero(), BigRational::zero()));
        }
        q[dr - db] = coeff.clone();
        for i in 0..=db {
            let sub = qe_mul(&b[i], &coeff, m);
            r[dr - db + i] = (&r[dr - db + i].0 - sub.0, &r[dr - db + i].1 - sub.1);
        }
        qep_trim(&mut r);
    }
    Some((q, r))
}

fn qep_gcd(a: &QePoly, b: &QePoly, m: &BigRational) -> Option<QePoly> {
    let (mut x, mut y) = (a.clone(), b.clone());
    qep_trim(&mut x);
    qep_trim(&mut y);
    while !y.is_empty() {
        let (_, r) = qep_divrem(&x, &y, m)?;
        x = y;
        y = r;
    }
    // Monic.
    let lc_inv = qe_inv(x.last()?, m)?;
    Some(x.iter().map(|c| qe_mul(c, &lc_inv, m)).collect())
}

/// Both residues of an irreducible quadratic factor `c₂t² + c₁t + c₀`:
/// α = h ± k·√m. Complex pairs (m < 0) rewrite to the student form
/// `h·ln(U² + |m|k'²V²) − 2·h_im·atan(…)`; real pairs stay as two logs.
fn quadratic_residues(f: &UPoly, a: &UPoly, q: &UPoly, dq: &UPoly, xs: &Expr) -> Option<Expr> {
    let (c2, c1, c0) = (&f[2], &f[1], &f[0]);
    let h = -(c1 / (BigRational::from_integer(2.into()) * c2)); // real part
    let disc = c1 * c1 - BigRational::from_integer(4.into()) * c2 * c0;
    let m = &disc / (BigRational::from_integer(4.into()) * c2 * c2); // α = h ± √m
    // gcd over ℚ(√m) with α = h + √m  (k = 1 by construction).
    let alpha: Qe = (h.clone(), BigRational::one());
    // A − α·q′ as a ℚ(√m)[x] polynomial.
    let mut shifted: QePoly = Vec::new();
    let len = a.len().max(dq.len());
    for i in 0..len {
        let ac = a.get(i).cloned().unwrap_or_else(BigRational::zero);
        let dc = dq.get(i).cloned().unwrap_or_else(BigRational::zero);
        // ac − α·dc = (ac − h·dc) − √m·dc·1
        shifted.push((&ac - &alpha.0 * &dc, -(&alpha.1 * &dc)));
    }
    qep_trim(&mut shifted);
    let q_qe: QePoly = q
        .iter()
        .map(|c| (c.clone(), BigRational::zero()))
        .collect();
    let g = qep_gcd(&q_qe, &shifted, &m)?;
    // Split G = U(x) + √m·V(x).
    let u: UPoly = {
        let mut v: UPoly = g.iter().map(|c| c.0.clone()).collect();
        upoly::trim(&mut v);
        v
    };
    let v: UPoly = {
        let mut w: UPoly = g.iter().map(|c| c.1.clone()).collect();
        upoly::trim(&mut w);
        w
    };
    let (u_e, v_e) = (poly_expr(&u, xs), poly_expr(&v, xs));
    if m.is_negative() {
        // α = h ± i·s, s = √|m|:  h·ln(U² + s²V²) − 2s·atan(sV/U).
        let mm = -&m;
        let s = sqrt_expr(&mm);
        let usq_vsq = cadd(vec![
            cpow(u_e.clone(), int(2)),
            cmul(vec![num(&mm), cpow(v_e.clone(), int(2))]),
        ]);
        let mut terms = Vec::new();
        if !h.is_zero() {
            terms.push(cmul(vec![num(&h), log_expr(usq_vsq)]));
        }
        if !upoly::is_zero(&v) {
            // Orient the argument with the higher-degree polynomial on top:
            // atan(z) = −atan(1/z) up to a constant, and `atan(x)` is the
            // student form where `−atan(1/x)` is not.
            let (sign, arg) = if upoly::degree(&u) > upoly::degree(&v) {
                (
                    1,
                    cmul(vec![
                        u_e,
                        cpow(cmul(vec![s.clone(), v_e]), int(-1)),
                    ]),
                )
            } else {
                (-1, cmul(vec![s.clone(), v_e, cpow(u_e, int(-1))]))
            };
            terms.push(cmul(vec![
                int(2 * sign),
                s,
                Expr::Apply(Box::new(Expr::sym("atan")), vec![arg]),
            ]));
        }
        Some(cadd(terms))
    } else {
        // Real pair: (h + √m)·ln(U + √m·V) + (h − √m)·ln(U − √m·V).
        let s = sqrt_expr(&m);
        let mut terms = Vec::new();
        for sign in [1i64, -1] {
            let alpha_e = cadd(vec![num(&h), cmul(vec![int(sign), s.clone()])]);
            let arg = cadd(vec![u_e.clone(), cmul(vec![int(sign), s.clone(), v_e.clone()])]);
            terms.push(cmul(vec![alpha_e, log_expr(arg)]));
        }
        Some(cadd(terms))
    }
}

// ---- RootOf residue class: gcd over ℚ[t]/(F) ----

fn qr_reduce(p: &UPoly, f: &UPoly) -> UPoly {
    upoly::divrem(p, f).1
}

fn qr_mulm(a: &UPoly, b: &UPoly, f: &UPoly) -> UPoly {
    qr_reduce(&upoly::mul(a, b), f)
}

/// Inverse in ℚ[t]/(F) or the discovered factor.
fn qr_invm(x: &UPoly, f: &UPoly) -> Result<UPoly, UPoly> {
    let (g, s) = upoly::xgcd_mod(x, f);
    if upoly::degree(&g) == 0 && !upoly::is_zero(&g) {
        Ok(s)
    } else {
        Err(g)
    }
}

type RPoly = Vec<UPoly>; // dense in x, coefficients in ℚ[t]/(F)

fn rp_trim(p: &mut RPoly) {
    while p.last().is_some_and(|c| upoly::is_zero(c)) {
        p.pop();
    }
}

fn rp_divrem(a: &RPoly, b: &RPoly, f: &UPoly) -> Result<(RPoly, RPoly), UPoly> {
    let db = b.len() - 1;
    let lc_inv = qr_invm(&b[db], f)?;
    let mut r = a.clone();
    rp_trim(&mut r);
    let mut q: RPoly = Vec::new();
    while r.len() > db {
        let dr = r.len() - 1;
        let coeff = qr_mulm(&r[dr], &lc_inv, f);
        if q.len() < dr - db + 1 {
            q.resize(dr - db + 1, Vec::new());
        }
        q[dr - db] = coeff.clone();
        for i in 0..=db {
            let sub = qr_mulm(&b[i], &coeff, f);
            r[dr - db + i] = upoly::sub(&r[dr - db + i], &sub);
        }
        rp_trim(&mut r);
    }
    Ok((q, r))
}

fn rp_gcd(a: &RPoly, b: &RPoly, f: &UPoly) -> Result<RPoly, UPoly> {
    let (mut x, mut y) = (a.clone(), b.clone());
    rp_trim(&mut x);
    rp_trim(&mut y);
    while !y.is_empty() {
        let (_, r) = rp_divrem(&x, &y, f)?;
        x = y;
        y = r;
    }
    let Some(lc) = x.last() else {
        return Err(Vec::new());
    };
    let lc_inv = qr_invm(lc, f)?;
    Ok(x.iter().map(|c| qr_mulm(c, &lc_inv, f)).collect())
}

/// Residues that stay abstract: for each index k of `RootOf(F, ·)`,
/// `α_k · ln(G(α_k, x))` with `G = gcd_{ℚ[t]/(F)}(q, A − t·q′)`. A zero
/// divisor found during the gcd is a discovered factor of F: split and
/// recurse on both parts (strictly decreasing degree — bounded).
fn rootof_residues(f: &UPoly, a: &UPoly, q: &UPoly, dq: &UPoly, xs: &Expr) -> Option<Expr> {
    let f = {
        // Primitive/positive-lc canonical form so RootOf construction agrees.
        let ints = upoly::to_primitive_int(f);
        ints.iter()
            .map(|c| BigRational::from_integer(c.clone()))
            .collect::<UPoly>()
    };
    // A − t·q′ over ℚ[t]/(F): coefficient of x^i is (a_i) − t·(dq_i).
    let len = a.len().max(dq.len());
    let mut shifted: RPoly = Vec::new();
    for i in 0..len {
        let ac = a.get(i).cloned().unwrap_or_else(BigRational::zero);
        let dc = dq.get(i).cloned().unwrap_or_else(BigRational::zero);
        let mut c = vec![ac, -dc];
        upoly::trim(&mut c);
        shifted.push(qr_reduce(&c, &f));
    }
    rp_trim(&mut shifted);
    let q_r: RPoly = q.iter().map(|c| {
        let mut v = vec![c.clone()];
        upoly::trim(&mut v);
        v
    }).collect();
    match rp_gcd(&q_r, &shifted, &f) {
        Ok(g) => {
            let d = upoly::degree(&f);
            let root0 = crate::rootof::make_rootof(&f, 0)?;
            let Expr::RootOf { poly, .. } = &root0 else {
                unreachable!()
            };
            let mut terms = Vec::new();
            for k in 0..d {
                let alpha = Expr::RootOf {
                    poly: poly.clone(),
                    index: k as u32,
                };
                // G with t ↦ α_k, per x-power.
                let arg_terms: Vec<Expr> = g
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| !upoly::is_zero(c))
                    .map(|(i, c)| {
                        let coeff = crate::rootof::upoly_in_root(c, &alpha);
                        match i {
                            0 => coeff,
                            _ => cmul(vec![coeff, cpow(xs.clone(), int(i as i64))]),
                        }
                    })
                    .collect();
                terms.push(cmul(vec![alpha, log_expr(cadd(arg_terms))]));
            }
            Some(cadd(terms))
        }
        Err(gfac) => {
            // Discovered factor: split F = gfac·(F/gfac) and recurse the
            // residue ladder on each part.
            if upoly::degree(&gfac) < 1 || upoly::degree(&gfac) >= upoly::degree(&f) {
                return None;
            }
            let (rest, _) = upoly::divrem(&f, &gfac);
            let left = residues_dispatch(&gfac, a, q, dq, xs)?;
            let right = residues_dispatch(&rest, a, q, dq, xs)?;
            Some(cadd(vec![left, right]))
        }
    }
}

/// Route a residue factor through the ladder by degree (used on split).
fn residues_dispatch(f: &UPoly, a: &UPoly, q: &UPoly, dq: &UPoly, xs: &Expr) -> Option<Expr> {
    match upoly::degree(f) {
        0 => Some(int(0)),
        1 => {
            let alpha = -&f[0] / &f[1];
            if alpha.is_zero() {
                return Some(int(0));
            }
            let shifted = shift_by_scalar(a, dq, &alpha);
            let g = upoly::gcd(q, &shifted);
            Some(cmul(vec![
                num(&alpha),
                log_expr(poly_expr(&upoly::monic(&g), xs)),
            ]))
        }
        2 => quadratic_residues(f, a, q, dq, xs),
        _ => rootof_residues(f, a, q, dq, xs),
    }
}

fn log_expr(arg: Expr) -> Expr {
    // Canonical spelling (`log`, natural) — matching every other builder so
    // downstream string-matching on the head sees one name.
    Expr::Apply(Box::new(Expr::sym("log")), vec![arg])
}

/// Exact square root of a rational when both parts are perfect squares.
pub(super) fn rational_sqrt(r: &BigRational) -> Option<BigRational> {
    if r.is_negative() {
        return None;
    }
    let (n, d) = (r.numer(), r.denom());
    let (sn, sd) = (n.sqrt(), d.sqrt());
    (&sn * &sn == *n && &sd * &sd == *d).then(|| BigRational::new(sn, sd))
}

/// `√r` as an expression: exact rational when possible, else `r^(1/2)`.
pub(super) fn sqrt_expr(r: &BigRational) -> Expr {
    match rational_sqrt(r) {
        Some(s) => num(&s),
        None => cpow(num(r), Expr::Num(Number::rat(1, 2))),
    }
}
