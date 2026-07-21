//! §1b: determinant, inverse, rref, rank, and nullspace — the public
//! entry points that dispatch to the [`super::kernels`] elimination kernels.

use crate::assumptions::{is_nonzero, Assumptions};
use crate::expr::Expr;
use crate::norm::{canonicalize, mul, pow};
use crate::num::Number;
use crate::sym::Sym;

use super::kernels::{
    as_numbers, det_bareiss, det_cofactor, det_rational, is_polynomial, is_zero, rref_core,
};

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
            let lim = crate::resource_limits::current();
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
            let lim = crate::resource_limits::current();
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
                            let cof = super::kernels::cofactor(entries, n, j, i);
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
    if n > crate::resource_limits::current().max_matrix_dim {
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
