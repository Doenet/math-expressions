//! Determinant, inverse, rref, rank, and nullspace — the public entry points
//! that dispatch to the [`super::elimination`] elimination kernels.

use crate::assumptions::{is_nonzero, Assumptions};
use crate::expr::sym::Sym;
use crate::expr::{Expr, Mat};
use crate::normalize::{canonicalize, mul, pow};
use crate::num::Number;

use super::elimination::{
    as_numbers, det_bareiss, det_cofactor, det_rational, is_polynomial, is_zero, rref_core,
};

/// Determinant, tiered by entry type: exact elimination over `Number` for
/// all-rational entries, fraction-free Bareiss (divisions
/// cancelled through `reduce_rational`) for polynomial entries up to
/// `max_matrix_dim`, cofactor expansion for general symbolic entries up to
/// `max_symbolic_det_dim`. Anything else — non-matrix, non-square, over the
/// caps — stays an opaque `det(e)` node.
pub fn det(e: &Expr) -> Expr {
    let c = canonicalize(e);
    if let Expr::Matrix(m) = &c {
        if m.is_square() {
            let n = m.rows() as usize;
            let entries = m.entries();
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
/// gated on the assumptions system proving the determinant nonzero (no silent
/// case-guessing).
pub fn matrix_inverse(e: &Expr, assumptions: &Assumptions) -> Expr {
    let c = canonicalize(e);
    if let Expr::Matrix(m) = &c {
        if m.is_square() {
            let n = m.rows() as usize;
            let entries = m.entries();
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
                    return Expr::Matrix(Mat::generate(m.rows(), m.cols(), |i, j| {
                        // Adjugate: cofactor C(j, i) (transposed).
                        let cof = super::elimination::cofactor(entries, n, j as usize, i as usize);
                        mul(vec![dinv.clone(), cof])
                    }));
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
    if let Expr::Matrix(m) = &c {
        if let Some((reduced, _)) = rref_core(
            m.entries(),
            m.rows() as usize,
            m.cols() as usize,
            assumptions,
        ) {
            // `rref_core` returns a same-shape matrix; if it ever did not, the
            // opaque `rref(e)` node below is the graceful answer.
            if let Some(out) = Mat::new(m.rows(), m.cols(), reduced) {
                return Expr::Matrix(out);
            }
        }
    }
    Expr::OtherOp(Sym::new("rref"), vec![c])
}

/// Rank (number of pivots in the assumption-gated rref). `None` when the
/// input is not a literal matrix or a pivot decision is undecidable.
pub fn rank(e: &Expr, assumptions: &Assumptions) -> Option<u32> {
    let c = canonicalize(e);
    let Expr::Matrix(m) = &c else {
        return None;
    };
    let (_, pivots) = rref_core(
        m.entries(),
        m.rows() as usize,
        m.cols() as usize,
        assumptions,
    )?;
    Some(pivots.len() as u32)
}

/// Nullspace basis as n×1 column matrices (one per free column of the rref),
/// each normalized to a numeric leading 1 where possible. `None` under the
/// same conditions as [`rank`].
pub fn nullspace(e: &Expr, assumptions: &Assumptions) -> Option<Vec<Expr>> {
    let c = canonicalize(e);
    let Expr::Matrix(m) = &c else {
        return None;
    };
    let (rows, cols) = (m.rows() as usize, m.cols() as usize);
    let (reduced, pivots) = rref_core(m.entries(), rows, cols, assumptions)?;
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
        // `v` is built with exactly `cols` entries just above, which is the
        // entry count an n×1 column needs.
        if let Some(col) = Mat::new(cols as u32, 1, v) {
            basis.push(Expr::Matrix(col));
        }
    }
    Some(basis)
}

/// Invert an all-rational literal matrix by Gauss–Jordan over exact
/// `Number`s. `None` if not such a matrix or singular. Also used by the
/// canonical `pow` to fold `A^(-k)`.
pub(crate) fn invert_rational_literal(e: &Expr) -> Option<Expr> {
    let Expr::Matrix(m) = e else {
        return None;
    };
    if !m.is_square() {
        return None;
    }
    let entries = m.entries();
    let n = m.rows() as usize;
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
    // Square, so `n` is both dimensions; `inv` was built with `n * n` entries.
    Mat::new(n as u32, n as u32, inv.into_iter().map(Expr::Num).collect()).map(Expr::Matrix)
}
