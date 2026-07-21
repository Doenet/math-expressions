//! M4: eigenvectors via nullspace over the quotient ring ℚ[t]/(f), where `t`
//! is the abstract eigenvalue (MATRIX_PLAN §3).

use crate::expr::Expr;
use crate::norm::canonicalize;
use crate::upoly::{self, UPoly};
use num_traits::One;
use num_rational::BigRational;

use super::eigen::{charpoly_rational, eigen_items};
use super::kernels::{as_rationals, square_literal};

/// One eigenvalue with its eigenspace (MATRIX_PLAN §3). Geometric
/// multiplicity is `basis.len()`; a defective eigenvalue shows
/// `basis.len() < alg_mult` (never "repaired" — §0 decision 5).
#[derive(Debug, Clone)]
pub struct EigenPair {
    pub value: Expr,
    pub alg_mult: u32,
    pub basis: Vec<Vec<Expr>>,
}

/// Ring element ops: dense polys of degree < deg f, reduced after multiply.
fn qmul(x: &[BigRational], y: &[BigRational], f: &UPoly) -> UPoly {
    if upoly::degree(f) == 0 {
        return Vec::new();
    }
    upoly::divrem(&upoly::mul(x, y), f).1
}

/// Inverse in ℚ[t]/(f), or the discovered factor when `x` is a zero divisor.
fn qinv(x: &[BigRational], f: &UPoly) -> Result<UPoly, UPoly> {
    let (g, s) = upoly::xgcd_mod(x, f);
    if upoly::degree(&g) == 0 && !upoly::is_zero(&g) {
        Ok(s)
    } else {
        Err(g)
    }
}

/// Nullspace of `A − tI` over ℚ[t]/(f): reduced echelon elimination with
/// ext-Euclid inverses. `Err(g)` reports a discovered factor of `f`
/// (MATRIX_PLAN §3: split and restart, bounded). Basis vectors have their
/// first nonzero component normalized to 1.
fn quotient_nullspace(
    a: &[BigRational],
    n: usize,
    f: &UPoly,
) -> Result<Vec<Vec<UPoly>>, UPoly> {
    // Entries of A − t·I as ring elements.
    let mut m: Vec<UPoly> = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            let mut p: UPoly = vec![a[i * n + j].clone()];
            if i == j {
                p.push(-BigRational::one());
            }
            upoly::trim(&mut p);
            m.push(upoly::divrem(&p, f).1);
        }
    }
    let mut pivots: Vec<usize> = Vec::new();
    let mut row = 0usize;
    for col in 0..n {
        if row == n {
            break;
        }
        let Some(pr) = (row..n).find(|&r| !upoly::is_zero(&m[r * n + col])) else {
            continue;
        };
        if pr != row {
            for k in 0..n {
                m.swap(row * n + k, pr * n + k);
            }
        }
        let inv = qinv(&m[row * n + col].clone(), f)?;
        for k in 0..n {
            m[row * n + k] = qmul(&m[row * n + k], &inv, f);
        }
        for r in 0..n {
            if r == row || upoly::is_zero(&m[r * n + col]) {
                continue;
            }
            let factor = m[r * n + col].clone();
            for k in 0..n {
                let prod = qmul(&factor, &m[row * n + k], f);
                m[r * n + k] = upoly::sub(&m[r * n + k], &prod);
            }
        }
        pivots.push(col);
        row += 1;
    }
    let mut basis = Vec::new();
    for free in (0..n).filter(|c| !pivots.contains(c)) {
        let mut v: Vec<UPoly> = vec![Vec::new(); n];
        v[free] = vec![BigRational::one()];
        for (r, &pcol) in pivots.iter().enumerate() {
            v[pcol] = upoly::sub(&[], &m[r * n + free]);
        }
        // Normalize the first nonzero component to 1 (ring inverse — a zero
        // divisor here is another discovered factor).
        if let Some(first) = v.iter().position(|c| !upoly::is_zero(c)) {
            if v[first] != vec![BigRational::one()] {
                let inv = qinv(&v[first].clone(), f)?;
                for c in v.iter_mut() {
                    *c = qmul(c, &inv, f);
                }
            }
        }
        basis.push(v);
    }
    Ok(basis)
}

/// Eigenvectors (MATRIX_PLAN §3): for each eigenvalue, the nullspace of
/// `A − λI` computed over ℚ[t]/(minimal factor), components emerging as
/// polynomials in the abstract eigenvalue. Rational literal matrices only;
/// `None` = honest refusal.
pub fn eigenvectors(e: &Expr, _assumptions: &crate::assumptions::Assumptions) -> Option<Vec<EigenPair>> {
    let c = canonicalize(e);
    let (n, entries) = square_literal(&c)?;
    let rats = as_rationals(entries)?;
    let p = charpoly_rational(&rats, n);
    let mut splits: Vec<UPoly> = Vec::new();
    // Each restart strictly refines a factor, so deg p bounds the restarts.
    for _attempt in 0..=upoly::degree(&p) {
        let items = eigen_items(&p, &splits)?;
        let mut pairs = Vec::with_capacity(items.len());
        let mut discovered: Option<UPoly> = None;
        'items: for item in &items {
            match quotient_nullspace(&rats, n, &item.factor) {
                Ok(vectors) => {
                    let basis = vectors
                        .into_iter()
                        .map(|v| {
                            v.into_iter()
                                .map(|coeffs| crate::rootof::upoly_in_root(&coeffs, &item.value))
                                .collect::<Vec<Expr>>()
                        })
                        .collect();
                    pairs.push(EigenPair {
                        value: item.value.clone(),
                        alg_mult: item.mult,
                        basis,
                    });
                }
                Err(g) => {
                    discovered = Some(g);
                    break 'items;
                }
            }
        }
        match discovered {
            Some(g) => splits.push(g),
            None => return Some(pairs),
        }
    }
    None
}
