//! Literal-matrix helpers used by the `mul`/`pow` smart constructors:
//! matrix-valued detection, the identity, and symbolic literal multiplication.

use super::{add, mul};
use crate::expr::{Expr, Mat, SeqKind};

/// Is this canonical factor matrix-valued (a literal matrix, an unevaluated
/// matrix power, or an unfoldable matrix product)? Such factors must not
/// commute past each other and are excluded from scalar-only rewrites.
pub(crate) fn is_matrix_valued(e: &Expr) -> bool {
    match e {
        Expr::Matrix(_) => true,
        Expr::Pow(b, _) => matches!(**b, Expr::Matrix(_)),
        Expr::Mul(fs) => fs.iter().any(is_matrix_valued),
        _ => false,
    }
}

/// The n×n identity matrix.
pub(crate) fn identity_matrix(n: u32) -> Expr {
    Expr::Matrix(Mat::generate(n, n, |r, c| Expr::int(i64::from(r == c))))
}

/// Is this factor a coordinate vector — something a matrix *multiplies* rather
/// than something that scales it?
///
/// `M·(e,f)` used to partition `(e,f)` into `mul`'s scalar segment, which
/// distributed it into every entry and produced a matrix of `a·(e,f)`. A vector
/// is not a scalar; it belongs in the ordered segment beside the matrices, so
/// the product either contracts (in [`contract_pair`], under `expand`) or
/// stays written as it was.
pub(crate) fn is_vector_valued(e: &Expr) -> bool {
    matches!(e, Expr::Seq(k, _)
        if matches!(k, SeqKind::Tuple | SeqKind::Array | SeqKind::Vector | SeqKind::AltVector))
}

/// Multiply two literal matrices as `Mat`s (entries built with the smart
/// constructors). `None` on dimension mismatch or when the work exceeds
/// `limits.max_expand_terms`. This is the shared compute core: matrix·matrix in
/// the `mul` constructor and every matrix/vector product under `expand` (via
/// [`contract_pair`]) route through it — a vector is just a 1×N or N×1 `Mat`.
pub(crate) fn matmul_mats(ma: &Mat, mb: &Mat) -> Option<Mat> {
    if ma.cols() != mb.rows() {
        return None;
    }
    let (r1, c1, c2) = (ma.rows() as usize, ma.cols() as usize, mb.cols() as usize);
    if r1.saturating_mul(c1).saturating_mul(c2) > crate::resource_limits::current().max_expand_terms
    {
        return None;
    }
    // `i < r1`, `j < c2` and `k < c1`, so both flat indices are within
    // `rows * cols` — in bounds by `Mat`'s invariant, with no length check of
    // our own to get right.
    let (ea, eb) = (ma.entries(), mb.entries());
    Some(Mat::generate(ma.rows(), mb.cols(), |i, j| {
        let (i, j) = (i as usize, j as usize);
        add((0..c1)
            .map(|k| mul(vec![ea[i * c1 + k].clone(), eb[k * c2 + j].clone()]))
            .collect())
    }))
}

/// Multiply two literal matrices symbolically. Thin `Expr` wrapper over
/// [`matmul_mats`]; `None` on non-matrices or the shape/size failures above.
pub(crate) fn matmul_literal(a: &Expr, b: &Expr) -> Option<Expr> {
    let (Expr::Matrix(ma), Expr::Matrix(mb)) = (a, b) else {
        return None;
    };
    Some(Expr::Matrix(matmul_mats(ma, mb)?))
}

/// Which side of a `·` a factor sits on. A coordinate vector is a **row (1×N)**
/// as a left operand and a **column (N×1)** as a right operand — the positional
/// rule that makes `M·v` a column, `v·M` a row, and `v·w` a row·column dot.
#[derive(Clone, Copy)]
pub(crate) enum Side {
    Left,
    Right,
}

/// Lift a factor into matrix form for [`matmul_mats`], remembering the vector
/// notation (if any) so the product can be lowered back. A matrix is itself
/// (kind `None`); a coordinate vector becomes a 1×N row or N×1 column by `side`,
/// carrying its `SeqKind`. Anything else is not multipliable this way.
fn lift(e: &Expr, side: Side) -> Option<(Mat, Option<SeqKind>)> {
    match e {
        Expr::Matrix(m) => Some((m.clone(), None)),
        Expr::Seq(k, comps) if is_vector_valued(e) => {
            let n = comps.len() as u32;
            let m = match side {
                Side::Left => Mat::new(1, n, comps.clone()),
                Side::Right => Mat::new(n, 1, comps.clone()),
            }?;
            Some((m, Some(*k)))
        }
        _ => None,
    }
}

/// Lower a computed product back to the narrowest natural form. `kind` is the
/// coordinate-vector notation to restore *when a vector was involved*; a pure
/// matrix·matrix product (`kind == None`) always stays a matrix, even if it came
/// out 1×N. With a vector involved the result is only ever 1×1, 1×N or N×1:
/// `1×1` → the scalar entry (a dot product), a lone row/column → `Seq(kind, …)`.
fn lower(m: Mat, kind: Option<SeqKind>) -> Expr {
    let Some(k) = kind else {
        return Expr::Matrix(m);
    };
    if m.rows() == 1 && m.cols() == 1 {
        return m
            .into_entries()
            .into_iter()
            .next()
            .expect("1×1 has one entry");
    }
    if m.rows() == 1 || m.cols() == 1 {
        return Expr::Seq(k, m.into_entries());
    }
    Expr::Matrix(m)
}

/// Contract one ordered product step `left · right` where each factor is a
/// literal matrix or a coordinate vector. Vectors are lifted by position (see
/// [`Side`]), multiplied through the shared [`matmul_mats`] core, and lowered
/// back ([`lower`]). `None` when the shapes do not conform or a factor is not
/// liftable — the caller leaves the product written. Subsumes the old
/// `matvec_literal` (`M·v`) and adds `v·M` (row) and `v·w` (dot).
pub(crate) fn contract_pair(left: &Expr, right: &Expr) -> Option<Expr> {
    let (lm, lk) = lift(left, Side::Left)?;
    let (rm, rk) = lift(right, Side::Right)?;
    let product = matmul_mats(&lm, &rm)?;
    // A vector on either side sets the notation to restore; two vectors always
    // contract to a 1×1 scalar, so their kinds never compete.
    Some(lower(product, lk.or(rk)))
}
