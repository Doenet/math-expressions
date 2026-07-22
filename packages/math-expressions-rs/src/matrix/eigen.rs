//! Characteristic polynomial and eigenvalues. The eigenvector stage
//! (nullspace over ℚ[t]/(f)) lives in [`super::eigenvectors`].

use crate::expr::Expr;
use crate::norm::{add, canonicalize, mul, pow};
use crate::num::Number;
use crate::upoly::{self, UPoly};
use num_complex::Complex64;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};

use super::kernels::{as_rationals, det_cofactor, is_zero, square_literal};

/// Characteristic polynomial `det(λI − A)` in `var` (monic). Rational
/// entries go through Faddeev–LeVerrier exactly at any dimension under
/// `max_matrix_dim`; symbolic entries through cofactor expansion of the
/// shifted matrix under `max_symbolic_det_dim`. `None` otherwise.
pub fn char_poly(e: &Expr, var: &str) -> Option<Expr> {
    let c = canonicalize(e);
    let (n, entries) = square_literal(&c)?;
    if let Some(rats) = as_rationals(entries) {
        let p = charpoly_rational(&rats, n);
        return Some(upoly_in_var(&p, var));
    }
    if n <= crate::resource_limits::current().max_symbolic_det_dim {
        let lambda = Expr::sym(var);
        let mut shifted = Vec::with_capacity(n * n);
        for i in 0..n {
            for j in 0..n {
                let neg = mul(vec![Expr::int(-1), entries[i * n + j].clone()]);
                shifted.push(if i == j {
                    add(vec![lambda.clone(), neg])
                } else {
                    neg
                });
            }
        }
        return Some(det_cofactor(&shifted, n));
    }
    None
}

/// Faddeev–LeVerrier: `M₁ = A, c₁ = −tr M₁; Mₖ = A(Mₖ₋₁ + cₖ₋₁I),
/// cₖ = −tr Mₖ / k`. Only ring ops and division by integers — exact over ℚ.
/// Returns dense monic coefficients, low → high.
pub(super) fn charpoly_rational(a: &[BigRational], n: usize) -> UPoly {
    let tr = |m: &[BigRational]| -> BigRational {
        (0..n).map(|i| m[i * n + i].clone()).sum()
    };
    let matmul_r = |x: &[BigRational], y: &[BigRational]| -> Vec<BigRational> {
        let mut out = vec![BigRational::zero(); n * n];
        for i in 0..n {
            for k in 0..n {
                if x[i * n + k].is_zero() {
                    continue;
                }
                for j in 0..n {
                    out[i * n + j] += &x[i * n + k] * &y[k * n + j];
                }
            }
        }
        out
    };
    let mut m = a.to_vec();
    let mut cs: Vec<BigRational> = Vec::with_capacity(n);
    for k in 1..=n {
        if k > 1 {
            let c_prev = cs[k - 2].clone();
            let mut shifted = m;
            for i in 0..n {
                shifted[i * n + i] += &c_prev;
            }
            m = matmul_r(a, &shifted);
        }
        let ck = -tr(&m) / BigRational::from_integer(num_bigint::BigInt::from(k));
        cs.push(ck);
    }
    // p(λ) = λⁿ + c₁λ^{n−1} + … + cₙ.
    let mut out: UPoly = Vec::with_capacity(n + 1);
    for i in (0..n).rev() {
        out.push(cs[i].clone());
    }
    out.push(BigRational::one());
    // Leading side is already trimmed (monic); low coefficients may be zero.
    out
}

fn upoly_in_var(p: &UPoly, var: &str) -> Expr {
    let terms: Vec<Expr> = p
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.is_zero())
        .map(|(i, c)| {
            let num = Expr::Num(Number::from_bigrational(c.clone()));
            match i {
                0 => num,
                _ => mul(vec![
                    num,
                    pow(Expr::sym(var), Expr::int(i as i64)),
                ]),
            }
        })
        .collect();
    add(terms)
}

/// One eigenvalue of a rational matrix, with everything the eigenvector
/// stage needs: its exact `Expr`, algebraic multiplicity, the monic
/// squarefree factor it is a root of, and a numeric value (used only for the
/// canonical output ordering).
pub(super) struct EigenItem {
    pub(super) value: Expr,
    pub(super) mult: u32,
    pub(super) factor: UPoly,
    pub(super) z: Complex64,
}

fn numeric_of(value: &Expr) -> Option<Complex64> {
    crate::eval::eval_complex(value, &std::collections::HashMap::new())
}

fn sort_key(z: Complex64) -> (u8, f64, f64, f64) {
    (u8::from(z.im != 0.0), z.re, z.im.abs(), z.im)
}

/// Split the factors of the squarefree decomposition further by an
/// accumulated list of discovered factors (the split-restart refinement).
fn refine_by_splits(f: UPoly, splits: &[UPoly]) -> Vec<UPoly> {
    let mut pieces = vec![f];
    for s in splits {
        let mut next = Vec::new();
        for piece in pieces {
            let g = upoly::gcd(&piece, s);
            let dg = upoly::degree(&g);
            if dg >= 1 && dg < upoly::degree(&piece) {
                let (q, _) = upoly::divrem(&piece, &g);
                next.push(g);
                next.push(q);
            } else {
                next.push(piece);
            }
        }
        pieces = next;
    }
    pieces
}

// Design note: this root-finding ladder is deliberately not collapsed into a
// single `factor()` call. Today's `crate::factor` does strictly *less*
// (content + Yun + rational-root deflation — no quadratic closed forms, no
// ordered `RootOf` tail), so the rewire would lose capability. Revisit once
// full factorization over ℚ lands (FULL_SIMPLIFY §8, chunk S4); until then
// this ladder is the more powerful implementation.
/// The full root list of a monic rational char poly: closed forms where
/// honest (rational roots, quadratic formula), `RootOf` elsewhere, in the
/// canonical order. `None` on any cap or certification refusal.
pub(super) fn eigen_items(p: &UPoly, splits: &[UPoly]) -> Option<Vec<EigenItem>> {
    let mut items: Vec<EigenItem> = Vec::new();
    for (f, m) in upoly::squarefree_decomposition(p) {
        let (rats, rest) = upoly::rational_roots(&f);
        for r in &rats {
            let value = Expr::Num(Number::from_bigrational(r.clone()));
            let z = Complex64::new(r.to_f64()?, 0.0);
            items.push(EigenItem {
                value,
                mult: m,
                factor: vec![-r.clone(), BigRational::one()],
                z,
            });
        }
        for piece in refine_by_splits(rest, splits) {
            match upoly::degree(&piece) {
                0 => {}
                1 => {
                    let r = -&piece[0] / &piece[1];
                    let value = Expr::Num(Number::from_bigrational(r.clone()));
                    let z = Complex64::new(r.to_f64()?, 0.0);
                    items.push(EigenItem {
                        value,
                        mult: m,
                        factor: vec![-r, BigRational::one()],
                        z,
                    });
                }
                2 => {
                    let (a, b, c) = (&piece[2], &piece[1], &piece[0]);
                    let disc = b * b - BigRational::from_integer(4.into()) * a * c;
                    let sq = pow(
                        Expr::Num(Number::from_bigrational(disc)),
                        Expr::Num(Number::rat(1, 2)),
                    );
                    let half_inv = Expr::Num(Number::from_bigrational(
                        BigRational::one() / (BigRational::from_integer(2.into()) * a),
                    ));
                    let neg_b = Expr::Num(Number::from_bigrational(-b));
                    for sign in [-1i64, 1] {
                        let value = mul(vec![
                            half_inv.clone(),
                            add(vec![
                                neg_b.clone(),
                                mul(vec![Expr::int(sign), sq.clone()]),
                            ]),
                        ]);
                        let z = numeric_of(&value)?;
                        items.push(EigenItem {
                            value,
                            mult: m,
                            factor: upoly::monic(&piece),
                            z,
                        });
                    }
                }
                d => {
                    let root0 = crate::rootof::make_rootof(&piece, 0)?;
                    let Expr::RootOf { poly, .. } = &root0 else {
                        unreachable!()
                    };
                    for k in 0..d {
                        let value = Expr::RootOf {
                            poly: poly.clone(),
                            index: k as u32,
                        };
                        // Ordering certification: every index must evaluate.
                        let z = crate::rootof::numeric_root(poly, k as u32)?;
                        items.push(EigenItem {
                            value,
                            mult: m,
                            factor: upoly::monic(&piece),
                            z,
                        });
                    }
                }
            }
        }
    }
    items.sort_by(|x, y| {
        sort_key(x.z)
            .partial_cmp(&sort_key(y.z))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Some(items)
}

/// Eigenvalues with algebraic multiplicities: closed forms where honest,
/// `RootOf` elsewhere, real ascending then conjugate pairs (negative
/// imaginary part first). Symbolic entries get quadratic closed forms for
/// 2×2 only (`RootOf` carries ℚ coefficients, so it cannot represent roots
/// of a symbolic polynomial). `None` = honest refusal (shape, caps, or
/// uncertifiable ordering).
pub fn eigenvalues(e: &Expr, _assumptions: &crate::assumptions::Assumptions) -> Option<Vec<(Expr, u32)>> {
    let c = canonicalize(e);
    let (n, entries) = square_literal(&c)?;
    if let Some(rats) = as_rationals(entries) {
        let p = charpoly_rational(&rats, n);
        let items = eigen_items(&p, &[])?;
        return Some(items.into_iter().map(|it| (it.value, it.mult)).collect());
    }
    if n == 2 {
        // Symbolic 2×2: λ = (tr ± √(tr² − 4·det))/2.
        let (a, b, cc, d) = (
            entries[0].clone(),
            entries[1].clone(),
            entries[2].clone(),
            entries[3].clone(),
        );
        let tr = add(vec![a.clone(), d.clone()]);
        let det = add(vec![
            mul(vec![a, d]),
            mul(vec![Expr::int(-1), b, cc]),
        ]);
        let disc = add(vec![
            pow(tr.clone(), Expr::int(2)),
            mul(vec![Expr::int(-4), det]),
        ]);
        if is_zero(&disc) {
            let value = mul(vec![Expr::Num(Number::rat(1, 2)), tr]);
            return Some(vec![(value, 2)]);
        }
        let sq = pow(disc, Expr::Num(Number::rat(1, 2)));
        let half = Expr::Num(Number::rat(1, 2));
        let minus = mul(vec![
            half.clone(),
            add(vec![tr.clone(), mul(vec![Expr::int(-1), sq.clone()])]),
        ]);
        let plus = mul(vec![half, add(vec![tr, sq])]);
        return Some(vec![(minus, 1), (plus, 1)]);
    }
    None
}
