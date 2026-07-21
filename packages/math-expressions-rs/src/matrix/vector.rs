//! Vector operations (JS `vector_add`/`sub`/`dot`/`cross`).

use crate::expr::Expr;
use crate::norm::{add, canonicalize, mul};
use crate::sym::Sym;

/// The components of a vector-shaped expression: a `Seq` of any vector kind
/// (`vector`/`altvector`/`tuple`), or a single-row/single-column literal
/// matrix. `None` for anything else.
fn as_vector(e: &Expr) -> Option<(crate::expr::SeqKind, Vec<Expr>)> {
    use crate::expr::SeqKind;
    match e {
        Expr::Seq(k @ (SeqKind::Vector | SeqKind::AltVector | SeqKind::Tuple), xs) => {
            Some((*k, xs.clone()))
        }
        Expr::Matrix {
            rows,
            cols,
            entries,
        } if *rows == 1 || *cols == 1 => Some((SeqKind::Vector, entries.clone())),
        _ => None,
    }
}

/// Entrywise vector sum. Literal same-length vectors add eagerly (keeping the
/// left operand's kind); anything else stays an opaque `vector_add(a, b)`.
pub fn vector_add(a: &Expr, b: &Expr) -> Expr {
    let (ca, cb) = (canonicalize(a), canonicalize(b));
    if let (Some((k, xa)), Some((_, xb))) = (as_vector(&ca), as_vector(&cb)) {
        if xa.len() == xb.len() {
            let out = xa
                .into_iter()
                .zip(xb)
                .map(|(x, y)| add(vec![x, y]))
                .collect();
            return Expr::Seq(k, out);
        }
    }
    Expr::OtherOp(Sym::new("vector_add"), vec![ca, cb])
}

/// Entrywise vector difference `a − b`.
pub fn vector_sub(a: &Expr, b: &Expr) -> Expr {
    let (ca, cb) = (canonicalize(a), canonicalize(b));
    if let (Some((k, xa)), Some((_, xb))) = (as_vector(&ca), as_vector(&cb)) {
        if xa.len() == xb.len() {
            let out = xa
                .into_iter()
                .zip(xb)
                .map(|(x, y)| add(vec![x, mul(vec![Expr::int(-1), y])]))
                .collect();
            return Expr::Seq(k, out);
        }
    }
    Expr::OtherOp(Sym::new("vector_sub"), vec![ca, cb])
}

/// Dot product `a · b` (sum of entrywise products), a scalar. Opaque
/// `dot_prod(a, b)` on mismatched lengths or non-vectors.
pub fn dot_prod(a: &Expr, b: &Expr) -> Expr {
    let (ca, cb) = (canonicalize(a), canonicalize(b));
    if let (Some((_, xa)), Some((_, xb))) = (as_vector(&ca), as_vector(&cb)) {
        if xa.len() == xb.len() {
            let terms = xa
                .into_iter()
                .zip(xb)
                .map(|(x, y)| mul(vec![x, y]))
                .collect();
            return add(terms);
        }
    }
    Expr::OtherOp(Sym::new("dot_prod"), vec![ca, cb])
}

/// Cross product `a × b` of two 3-vectors. Opaque `cross_prod(a, b)` unless
/// both are literal 3-component vectors.
pub fn cross_prod(a: &Expr, b: &Expr) -> Expr {
    let (ca, cb) = (canonicalize(a), canonicalize(b));
    if let (Some((k, xa)), Some((_, xb))) = (as_vector(&ca), as_vector(&cb)) {
        if xa.len() == 3 && xb.len() == 3 {
            let comp = |i: usize, j: usize| {
                add(vec![
                    mul(vec![xa[i].clone(), xb[j].clone()]),
                    mul(vec![Expr::int(-1), xa[j].clone(), xb[i].clone()]),
                ])
            };
            return Expr::Seq(k, vec![comp(1, 2), comp(2, 0), comp(0, 1)]);
        }
    }
    Expr::OtherOp(Sym::new("cross_prod"), vec![ca, cb])
}
