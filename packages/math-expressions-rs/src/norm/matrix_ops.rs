//! Literal-matrix helpers used by the `mul`/`pow` smart constructors
//! (MATRIX_PLAN §1a): matrix-valued detection, the identity, and symbolic
//! literal multiplication.

use super::{add, mul};
use crate::expr::Expr;

/// Is this canonical factor matrix-valued (a literal matrix, an unevaluated
/// matrix power, or an unfoldable matrix product)? Such factors must not
/// commute past each other and are excluded from scalar-only rewrites.
pub(crate) fn is_matrix_valued(e: &Expr) -> bool {
    match e {
        Expr::Matrix { .. } => true,
        Expr::Pow(b, _) => matches!(**b, Expr::Matrix { .. }),
        Expr::Mul(fs) => fs.iter().any(is_matrix_valued),
        _ => false,
    }
}

/// The n×n identity matrix.
pub(crate) fn identity_matrix(n: u32) -> Expr {
    let entries = (0..n)
        .flat_map(|r| (0..n).map(move |c| Expr::int(i64::from(r == c))))
        .collect();
    Expr::Matrix {
        rows: n,
        cols: n,
        entries,
    }
}

/// Multiply two literal matrices symbolically (entries built with the smart
/// constructors). `None` on dimension mismatch or when the work exceeds
/// `limits.max_expand_terms` (the caller keeps the product unevaluated).
pub(crate) fn matmul_literal(a: &Expr, b: &Expr) -> Option<Expr> {
    let (
        Expr::Matrix {
            rows: r1,
            cols: c1,
            entries: ea,
        },
        Expr::Matrix {
            rows: r2,
            cols: c2,
            entries: eb,
        },
    ) = (a, b)
    else {
        return None;
    };
    if c1 != r2 {
        return None;
    }
    let (r1, c1, c2) = (*r1 as usize, *c1 as usize, *c2 as usize);
    if r1.saturating_mul(c1).saturating_mul(c2) > crate::resource_limits::current().max_expand_terms {
        return None;
    }
    let mut entries = Vec::with_capacity(r1 * c2);
    for i in 0..r1 {
        for j in 0..c2 {
            let terms = (0..c1)
                .map(|k| mul(vec![ea[i * c1 + k].clone(), eb[k * c2 + j].clone()]))
                .collect();
            entries.push(add(terms));
        }
    }
    Some(Expr::Matrix {
        rows: r1 as u32,
        cols: c2 as u32,
        entries,
    })
}
