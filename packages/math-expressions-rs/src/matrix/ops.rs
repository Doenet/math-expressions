//! Eager matrix operations: `transpose`, `trace`, `matmul`.

use crate::expr::sym::Sym;
use crate::expr::{Expr, Mat};
use crate::normalize::{add, canonicalize};

/// Matrix transpose. Literal matrices transpose eagerly; anything else stays
/// an opaque `transpose(e)` node.
pub fn transpose(e: &Expr) -> Expr {
    let c = canonicalize(e);
    if let Expr::Matrix(m) = &c {
        // Transposed shape: the result is cols×rows, and reading (j, i) from
        // the source is in bounds for every cell of it.
        let entries = m.entries();
        let k = m.cols() as usize;
        return Expr::Matrix(Mat::generate(m.cols(), m.rows(), |j, i| {
            entries[i as usize * k + j as usize].clone()
        }));
    }
    Expr::OtherOp(Sym::new("transpose"), vec![c])
}

/// Matrix trace (sum of the diagonal). Square literal matrices evaluate
/// eagerly; anything else (including non-square matrices) stays an opaque
/// `trace(e)` node.
pub fn trace(e: &Expr) -> Expr {
    let c = canonicalize(e);
    if let Expr::Matrix(m) = &c {
        if m.is_square() {
            let (n, entries) = (m.rows() as usize, m.entries());
            return add((0..n).map(|i| entries[i * n + i].clone()).collect());
        }
    }
    Expr::OtherOp(Sym::new("trace"), vec![c])
}

/// The canonical product `a·b` (folds literal matrices, keeps order for
/// unfoldable ones — see `normalize::mul`'s matrix segmentation).
pub fn matmul(a: &Expr, b: &Expr) -> Expr {
    canonicalize(&Expr::Mul(vec![a.clone(), b.clone()]))
}
