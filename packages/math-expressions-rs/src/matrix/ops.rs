//! Layer 1 eager operations (MATRIX_PLAN.md Layer 1): `transpose`, `trace`,
//! `matmul`.

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
