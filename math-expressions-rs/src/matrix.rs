//! Matrix operations (MATRIX_PLAN.md Layer 1): `transpose`, `trace`,
//! `matmul`. The arithmetic itself (entrywise sums, segmented
//! non-commutative products, powers) lives in the canonical layer's smart
//! constructors (`norm::add`/`mul`/`pow`); these are the eager eponymous
//! operations, which evaluate on literal matrices and return an opaque
//! `OtherOp` on anything else (same policy as the derivative catch-all:
//! never a wrong answer, always a renderable residual).

use crate::expr::Expr;
use crate::norm::{add, canonicalize};
use crate::sym::Sym;

/// Matrix transpose. Literal matrices transpose eagerly; anything else stays
/// an opaque `transpose(e)` node.
pub fn transpose(e: &Expr) -> Expr {
    let c = canonicalize(e);
    if let Expr::Matrix {
        rows,
        cols,
        entries,
    } = &c
    {
        let (r, k) = (*rows as usize, *cols as usize);
        let mut out = Vec::with_capacity(r * k);
        for j in 0..k {
            for i in 0..r {
                out.push(entries[i * k + j].clone());
            }
        }
        return Expr::Matrix {
            rows: *cols,
            cols: *rows,
            entries: out,
        };
    }
    Expr::OtherOp(Sym::new("transpose"), vec![c])
}

/// Matrix trace (sum of the diagonal). Square literal matrices evaluate
/// eagerly; anything else (including non-square matrices) stays an opaque
/// `trace(e)` node.
pub fn trace(e: &Expr) -> Expr {
    let c = canonicalize(e);
    if let Expr::Matrix {
        rows,
        cols,
        entries,
    } = &c
    {
        if rows == cols {
            let n = *rows as usize;
            return add((0..n).map(|i| entries[i * n + i].clone()).collect());
        }
    }
    Expr::OtherOp(Sym::new("trace"), vec![c])
}

/// The canonical product `a·b` (folds literal matrices, keeps order for
/// unfoldable ones — see `norm::mul`'s matrix segmentation).
pub fn matmul(a: &Expr, b: &Expr) -> Expr {
    canonicalize(&Expr::Mul(vec![a.clone(), b.clone()]))
}

// ================= §1b: det / inverse / rref / rank / nullspace =================

use crate::assumptions::{is_nonzero, Assumptions};
use crate::norm::{mul, pow};
use crate::num::Number;

/// Determinant (MATRIX_PLAN §1b), tiered by entry type: exact elimination
/// over `Number` for all-rational entries, fraction-free Bareiss (divisions
/// cancelled through `reduce_rational`) for polynomial entries up to
/// `max_matrix_dim`, cofactor expansion for general symbolic entries up to
/// `max_symbolic_det_dim`. Anything else — non-matrix, non-square, over the
/// caps — stays an opaque `det(e)` node.
pub fn det(e: &Expr) -> Expr {
    let c = canonicalize(e);
    if let Expr::Matrix {
        rows,
        cols,
        entries,
    } = &c
    {
        if rows == cols {
            let n = *rows as usize;
            let lim = crate::limits::current();
            if n <= lim.max_matrix_dim {
                if let Some(nums) = as_numbers(entries) {
                    return Expr::Num(det_rational(nums, n));
                }
                if n <= lim.max_symbolic_det_dim {
                    return det_cofactor(entries, n);
                }
                if entries.iter().all(is_polynomial) {
                    if let Some(d) = det_bareiss(entries, n) {
                        return d;
                    }
                }
            }
        }
    }
    Expr::OtherOp(Sym::new("det"), vec![c])
}

/// Matrix inverse: exact Gauss–Jordan for all-rational entries (opaque when
/// singular); adjugate/det for symbolic entries up to `max_symbolic_det_dim`,
/// gated on the assumptions system proving the determinant nonzero
/// (MATRIX_PLAN §0 decision 4 — no silent case-guessing).
pub fn matrix_inverse(e: &Expr, assumptions: &Assumptions) -> Expr {
    let c = canonicalize(e);
    if let Expr::Matrix {
        rows,
        cols,
        entries,
    } = &c
    {
        if rows == cols {
            let n = *rows as usize;
            let lim = crate::limits::current();
            if n <= lim.max_matrix_dim && as_numbers(entries).is_some() {
                if let Some(inv) = invert_rational_literal(&c) {
                    return inv;
                }
                // Singular rational matrix: fall through to opaque.
            } else if n <= lim.max_symbolic_det_dim {
                let d = det(&c);
                if is_nonzero(&d, assumptions) == Some(true) {
                    let dinv = pow(d, Expr::int(-1));
                    let mut out = Vec::with_capacity(n * n);
                    for i in 0..n {
                        for j in 0..n {
                            // Adjugate: cofactor C(j, i) (transposed).
                            let cof = cofactor(entries, n, j, i);
                            out.push(mul(vec![dinv.clone(), cof]));
                        }
                    }
                    return Expr::Matrix {
                        rows: *rows,
                        cols: *cols,
                        entries: out,
                    };
                }
            }
        }
    }
    Expr::OtherOp(Sym::new("inverse"), vec![c])
}

/// Reduced row echelon form with assumption-gated pivots: a pivot is taken
/// only from entries *provably* nonzero; a column whose only candidates have
/// unknown zero-status makes the whole operation opaque (never a guessed
/// elimination). Returns the rref matrix or an opaque `rref(e)` node.
pub fn rref(e: &Expr, assumptions: &Assumptions) -> Expr {
    let c = canonicalize(e);
    if let Expr::Matrix {
        rows,
        cols,
        entries,
    } = &c
    {
        if let Some((reduced, _)) = rref_core(entries, *rows as usize, *cols as usize, assumptions)
        {
            return Expr::Matrix {
                rows: *rows,
                cols: *cols,
                entries: reduced,
            };
        }
    }
    Expr::OtherOp(Sym::new("rref"), vec![c])
}

/// Rank (number of pivots in the assumption-gated rref). `None` when the
/// input is not a literal matrix or a pivot decision is undecidable.
pub fn rank(e: &Expr, assumptions: &Assumptions) -> Option<u32> {
    let c = canonicalize(e);
    let Expr::Matrix {
        rows,
        cols,
        entries,
    } = &c
    else {
        return None;
    };
    let (_, pivots) = rref_core(entries, *rows as usize, *cols as usize, assumptions)?;
    Some(pivots.len() as u32)
}

/// Nullspace basis as n×1 column matrices (one per free column of the rref),
/// each normalized to a numeric leading 1 where possible. `None` under the
/// same conditions as [`rank`].
pub fn nullspace(e: &Expr, assumptions: &Assumptions) -> Option<Vec<Expr>> {
    let c = canonicalize(e);
    let Expr::Matrix {
        rows,
        cols,
        entries,
    } = &c
    else {
        return None;
    };
    let (rows, cols) = (*rows as usize, *cols as usize);
    let (reduced, pivots) = rref_core(entries, rows, cols, assumptions)?;
    let mut basis = Vec::new();
    for free in (0..cols).filter(|c| !pivots.contains(c)) {
        let mut v = vec![Expr::int(0); cols];
        v[free] = Expr::int(1);
        for (r, &p) in pivots.iter().enumerate() {
            v[p] = mul(vec![Expr::int(-1), reduced[r * cols + free].clone()]);
        }
        // Normalize the first structurally-nonzero component to 1 when it is
        // numeric (dividing by a symbolic entry could divide by zero).
        if let Some(Expr::Num(n)) = v.iter().find(|e| !is_zero(e)) {
            if !n.is_one() {
                let scale = pow(Expr::Num(n.clone()), Expr::int(-1));
                v = v.into_iter().map(|e| mul(vec![scale.clone(), e])).collect();
            }
        }
        basis.push(Expr::Matrix {
            rows: cols as u32,
            cols: 1,
            entries: v,
        });
    }
    Some(basis)
}

/// Invert an all-rational literal matrix by Gauss–Jordan over exact
/// `Number`s. `None` if not such a matrix or singular. Also used by the
/// canonical `pow` to fold `A^(-k)`.
pub(crate) fn invert_rational_literal(e: &Expr) -> Option<Expr> {
    let Expr::Matrix {
        rows,
        cols,
        entries,
    } = e
    else {
        return None;
    };
    if rows != cols {
        return None;
    }
    let n = *rows as usize;
    if n > crate::limits::current().max_matrix_dim {
        return None;
    }
    let mut m: Vec<Number> = as_numbers(entries)?;
    let mut inv: Vec<Number> = (0..n * n)
        .map(|i| Number::Int(i64::from(i / n == i % n)))
        .collect();
    for col in 0..n {
        let pivot_row = (col..n).find(|&r| !m[r * n + col].is_zero())?;
        if pivot_row != col {
            for k in 0..n {
                m.swap(col * n + k, pivot_row * n + k);
                inv.swap(col * n + k, pivot_row * n + k);
            }
        }
        let p = m[col * n + col].clone();
        for k in 0..n {
            m[col * n + k] = m[col * n + k].checked_div(&p)?;
            inv[col * n + k] = inv[col * n + k].checked_div(&p)?;
        }
        for r in 0..n {
            if r == col || m[r * n + col].is_zero() {
                continue;
            }
            let f = m[r * n + col].clone();
            for k in 0..n {
                m[r * n + k] = m[r * n + k].sub(&f.mul(&m[col * n + k]));
                inv[r * n + k] = inv[r * n + k].sub(&f.mul(&inv[col * n + k]));
            }
        }
    }
    Some(Expr::Matrix {
        rows: *rows,
        cols: *cols,
        entries: inv.into_iter().map(Expr::Num).collect(),
    })
}

// ---- kernels ----

fn as_numbers(entries: &[Expr]) -> Option<Vec<Number>> {
    entries
        .iter()
        .map(|e| match e {
            Expr::Num(n) => Some(n.clone()),
            _ => None,
        })
        .collect()
}

fn is_zero(e: &Expr) -> bool {
    matches!(e, Expr::Num(n) if n.is_zero())
}

/// Exact elimination over `Number`: det = sign · ∏ pivots.
fn det_rational(mut m: Vec<Number>, n: usize) -> Number {
    let mut sign_neg = false;
    let mut result = Number::Int(1);
    for col in 0..n {
        let Some(pivot_row) = (col..n).find(|&r| !m[r * n + col].is_zero()) else {
            return Number::Int(0);
        };
        if pivot_row != col {
            sign_neg = !sign_neg;
            for k in 0..n {
                m.swap(col * n + k, pivot_row * n + k);
            }
        }
        let p = m[col * n + col].clone();
        result = result.mul(&p);
        for r in col + 1..n {
            if m[r * n + col].is_zero() {
                continue;
            }
            let Some(f) = m[r * n + col].checked_div(&p) else {
                return Number::Int(0); // unreachable: p nonzero
            };
            for k in col..n {
                m[r * n + k] = m[r * n + k].sub(&f.mul(&m[col * n + k]));
            }
        }
    }
    if sign_neg {
        result.neg()
    } else {
        result
    }
}

/// Cofactor expansion along the first row (general symbolic entries; the
/// n ≤ `max_symbolic_det_dim` cap bounds the n! growth).
fn det_cofactor(entries: &[Expr], n: usize) -> Expr {
    if n == 1 {
        return entries[0].clone();
    }
    let mut terms = Vec::with_capacity(n);
    for j in 0..n {
        if is_zero(&entries[j]) {
            continue;
        }
        let minor = minor_entries(entries, n, 0, j);
        let mut factors = vec![entries[j].clone(), det_cofactor(&minor, n - 1)];
        if j % 2 == 1 {
            factors.push(Expr::int(-1));
        }
        terms.push(mul(factors));
    }
    add(terms)
}

/// The (i, j) cofactor `(−1)^(i+j)·det(minor)`.
fn cofactor(entries: &[Expr], n: usize, i: usize, j: usize) -> Expr {
    let minor = minor_entries(entries, n, i, j);
    let d = det_cofactor(&minor, n - 1);
    if (i + j) % 2 == 1 {
        mul(vec![Expr::int(-1), d])
    } else {
        d
    }
}

fn minor_entries(entries: &[Expr], n: usize, skip_row: usize, skip_col: usize) -> Vec<Expr> {
    let mut out = Vec::with_capacity((n - 1) * (n - 1));
    for r in 0..n {
        if r == skip_row {
            continue;
        }
        for c in 0..n {
            if c == skip_col {
                continue;
            }
            out.push(entries[r * n + c].clone());
        }
    }
    out
}

/// Is `e` a polynomial expression (the Bareiss tier's domain: exact division
/// is decidable there and a canonical zero is a true zero)?
fn is_polynomial(e: &Expr) -> bool {
    match e {
        Expr::Num(_) | Expr::Sym(_) => true,
        Expr::Add(ts) | Expr::Mul(ts) => ts.iter().all(is_polynomial),
        Expr::Pow(b, x) => {
            is_polynomial(b) && matches!(&**x, Expr::Num(Number::Int(k)) if *k >= 0)
        }
        _ => false,
    }
}

/// Fraction-free Bareiss over polynomial entries: every division is exact in
/// the polynomial ring; `reduce_rational` performs the cancellation. `None`
/// if a division fails to cancel (non-polynomial residue — conservative).
fn det_bareiss(entries: &[Expr], n: usize) -> Option<Expr> {
    let mut m: Vec<Expr> = entries.to_vec();
    let mut prev = Expr::int(1);
    let mut sign_neg = false;
    for k in 0..n - 1 {
        if is_zero(&m[k * n + k]) {
            let swap = (k + 1..n).find(|&r| !is_zero(&m[r * n + k]))?;
            // All-zero column would have returned via the entries being zero:
            // a canonical polynomial is zero iff it is the zero polynomial.
            sign_neg = !sign_neg;
            for c in 0..n {
                m.swap(k * n + c, swap * n + c);
            }
        }
        for i in k + 1..n {
            for j in k + 1..n {
                let num = add(vec![
                    mul(vec![m[i * n + j].clone(), m[k * n + k].clone()]),
                    mul(vec![Expr::int(-1), m[i * n + k].clone(), m[k * n + j].clone()]),
                ]);
                let q = crate::ops::reduce_rational(&Expr::Div(
                    Box::new(num),
                    Box::new(prev.clone()),
                ));
                if !is_polynomial(&canonicalize(&q)) {
                    return None;
                }
                m[i * n + j] = canonicalize(&q);
            }
            m[i * n + k] = Expr::int(0);
        }
        prev = m[k * n + k].clone();
    }
    let d = m[(n - 1) * n + (n - 1)].clone();
    Some(if sign_neg {
        mul(vec![Expr::int(-1), d])
    } else {
        d
    })
}

/// Gauss–Jordan over `Expr` with tri-state pivot gating. Returns the reduced
/// entries and the pivot columns, or `None` when a pivot decision needs an
/// unavailable assumption.
fn rref_core(
    entries: &[Expr],
    rows: usize,
    cols: usize,
    assumptions: &Assumptions,
) -> Option<(Vec<Expr>, Vec<usize>)> {
    let lim = crate::limits::current();
    if rows > lim.max_matrix_dim || cols > lim.max_matrix_dim {
        return None;
    }
    let mut m: Vec<Expr> = entries.to_vec();
    let mut pivots: Vec<usize> = Vec::new();
    let mut row = 0;
    for col in 0..cols {
        if row == rows {
            break;
        }
        // Classify candidates in this column below `row`.
        let mut pivot_row = None;
        let mut undecidable = false;
        for r in row..rows {
            let e = &m[r * cols + col];
            if is_zero(e) {
                continue;
            }
            match is_nonzero(e, assumptions) {
                Some(true) => {
                    pivot_row = Some(r);
                    break;
                }
                Some(false) => {} // provably zero (e.g. under assumptions)
                None => undecidable = true,
            }
        }
        let Some(p) = pivot_row else {
            if undecidable {
                return None; // §0 decision 4: never guess a pivot
            }
            continue; // genuinely free column
        };
        if p != row {
            for c in 0..cols {
                m.swap(row * cols + c, p * cols + c);
            }
        }
        let scale = pow(m[row * cols + col].clone(), Expr::int(-1));
        for c in 0..cols {
            m[row * cols + c] = mul(vec![scale.clone(), m[row * cols + c].clone()]);
        }
        for r in 0..rows {
            if r == row || is_zero(&m[r * cols + col]) {
                continue;
            }
            let f = m[r * cols + col].clone();
            for c in 0..cols {
                let sub = mul(vec![Expr::int(-1), f.clone(), m[row * cols + c].clone()]);
                m[r * cols + c] = add(vec![m[r * cols + c].clone(), sub]);
            }
        }
        pivots.push(col);
        row += 1;
    }
    Some((m, pivots))
}

// ================= M3/M4: char poly, eigenvalues, eigenvectors =================

use crate::upoly::{self, UPoly};
use num_complex::Complex64;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};

/// One eigenvalue with its eigenspace (MATRIX_PLAN §3). Geometric
/// multiplicity is `basis.len()`; a defective eigenvalue shows
/// `basis.len() < alg_mult` (never "repaired" — §0 decision 5).
#[derive(Debug, Clone)]
pub struct EigenPair {
    pub value: Expr,
    pub alg_mult: u32,
    pub basis: Vec<Vec<Expr>>,
}

fn as_rationals(entries: &[Expr]) -> Option<Vec<BigRational>> {
    entries
        .iter()
        .map(|e| match e {
            Expr::Num(n) => n.to_bigrational(),
            _ => None,
        })
        .collect()
}

/// A canonical square literal matrix with its dimension, under the dim cap.
fn square_literal(c: &Expr) -> Option<(usize, &[Expr])> {
    let Expr::Matrix {
        rows,
        cols,
        entries,
    } = c
    else {
        return None;
    };
    let n = *rows as usize;
    if rows != cols || n == 0 || n > crate::limits::current().max_matrix_dim {
        return None;
    }
    Some((n, entries))
}

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
    if n <= crate::limits::current().max_symbolic_det_dim {
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
fn charpoly_rational(a: &[BigRational], n: usize) -> UPoly {
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
struct EigenItem {
    value: Expr,
    mult: u32,
    factor: UPoly,
    z: Complex64,
}

fn numeric_of(value: &Expr) -> Option<Complex64> {
    crate::eval::eval_complex(value, &std::collections::HashMap::new())
}

fn sort_key(z: Complex64) -> (u8, f64, f64, f64) {
    (u8::from(z.im != 0.0), z.re, z.im.abs(), z.im)
}

/// Split the factors of the squarefree decomposition further by an
/// accumulated list of discovered factors (M4 split-restart).
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

/// The full root list of a monic rational char poly: closed forms where
/// honest (rational roots, quadratic formula), `RootOf` elsewhere, in the
/// canonical order. `None` on any cap or certification refusal.
fn eigen_items(p: &UPoly, splits: &[UPoly]) -> Option<Vec<EigenItem>> {
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

/// Eigenvalues with algebraic multiplicities (MATRIX_PLAN §2c): closed forms
/// where honest, `RootOf` elsewhere, real ascending then conjugate pairs
/// (negative imaginary part first). Symbolic entries: quadratic closed forms
/// for 2×2 only (§8 Q1 — `RootOf` is ℚ-coefficients only). `None` = honest
/// refusal (shape, caps, or uncertifiable ordering).
pub fn eigenvalues(e: &Expr, _assumptions: &Assumptions) -> Option<Vec<(Expr, u32)>> {
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

// ---- M4: nullspace over the quotient ring ℚ[t]/(f) ----

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
pub fn eigenvectors(e: &Expr, _assumptions: &Assumptions) -> Option<Vec<EigenPair>> {
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
