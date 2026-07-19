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
