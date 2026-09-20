//! Vector operations (JS `vector_add`/`sub`/`dot`/`cross`).

use crate::expr::sym::Sym;
use crate::expr::Expr;
use crate::normalize::{add, canonicalize, mul, present};

/// The components of a vector-shaped expression: a `Seq` of any vector kind
/// (`vector`/`altvector`/`tuple`), or a single-row/single-column literal
/// matrix. `None` for anything else.
fn as_vector(e: &Expr) -> Option<(crate::expr::SeqKind, Vec<Expr>)> {
    use crate::expr::SeqKind;
    match e {
        Expr::Seq(k @ (SeqKind::Vector | SeqKind::AltVector | SeqKind::Tuple), xs) => {
            Some((*k, xs.clone()))
        }
        Expr::Matrix(m) if m.rows() == 1 || m.cols() == 1 => {
            Some((SeqKind::Vector, m.entries().to_vec()))
        }
        _ => None,
    }
}

/// The kind a *named* vector operation (`vector_add`/`sub`, `cross_prod`) hands
/// back. These are explicit vector operations, so the result is a `vector` even
/// when the operands are tuples — only two altvectors keep the `⟨…⟩` spelling.
/// (Generic `+` differs: it preserves a `tuple` sum as a tuple; see
/// `folded_seq_kind`.)
fn named_op_kind(ka: crate::expr::SeqKind, kb: crate::expr::SeqKind) -> crate::expr::SeqKind {
    use crate::expr::SeqKind;
    if ka == SeqKind::AltVector && kb == SeqKind::AltVector {
        SeqKind::AltVector
    } else {
        SeqKind::Vector
    }
}

/// Entrywise vector sum. Literal same-length vectors add eagerly; anything else
/// stays an opaque `vector_add(a, b)`.
pub fn vector_add(a: &Expr, b: &Expr) -> Expr {
    let (ca, cb) = (canonicalize(a), canonicalize(b));
    if let (Some((ka, xa)), Some((kb, xb))) = (as_vector(&ca), as_vector(&cb)) {
        if xa.len() == xb.len() {
            let out = xa
                .into_iter()
                .zip(xb)
                .map(|(x, y)| add(vec![x, y]))
                .collect();
            return present(&Expr::Seq(named_op_kind(ka, kb), out));
        }
    }
    Expr::OtherOp(Sym::new("vector_add"), vec![ca, cb])
}

/// Entrywise vector difference `a − b`.
pub fn vector_sub(a: &Expr, b: &Expr) -> Expr {
    let (ca, cb) = (canonicalize(a), canonicalize(b));
    if let (Some((ka, xa)), Some((kb, xb))) = (as_vector(&ca), as_vector(&cb)) {
        if xa.len() == xb.len() {
            let out = xa
                .into_iter()
                .zip(xb)
                .map(|(x, y)| add(vec![x, mul(vec![Expr::int(-1), y])]))
                .collect();
            return present(&Expr::Seq(named_op_kind(ka, kb), out));
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
            return present(&add(terms));
        }
    }
    Expr::OtherOp(Sym::new("dot_prod"), vec![ca, cb])
}

/// Scalar × vector: each component scaled by `scalar`. `scalar` is the first
/// operand and `vector` the second (the `me.scalar_mul(scalar, vector)` order).
/// The result is a `vector`; an `altvector` keeps the `⟨…⟩` spelling. Opaque
/// `scalar_mul(scalar, vector)` when the second operand is not a vector.
pub fn scalar_mul(scalar: &Expr, vector: &Expr) -> Expr {
    use crate::expr::SeqKind;
    let (cs, cv) = (canonicalize(scalar), canonicalize(vector));
    if let Some((k, xs)) = as_vector(&cv) {
        let kind = if k == SeqKind::AltVector {
            SeqKind::AltVector
        } else {
            SeqKind::Vector
        };
        let out = xs.into_iter().map(|x| mul(vec![cs.clone(), x])).collect();
        return present(&Expr::Seq(kind, out));
    }
    Expr::OtherOp(Sym::new("scalar_mul"), vec![cs, cv])
}

/// Cross product `a × b`. Two 3-vectors give a 3-vector; two 2-vectors give the
/// scalar `a₁b₂ − a₂b₁` (the z-component of the embedded 3-D cross). Opaque
/// `cross_prod(a, b)` otherwise.
pub fn cross_prod(a: &Expr, b: &Expr) -> Expr {
    let (ca, cb) = (canonicalize(a), canonicalize(b));
    if let (Some((ka, xa)), Some((kb, xb))) = (as_vector(&ca), as_vector(&cb)) {
        let z = |i: usize, j: usize| {
            add(vec![
                mul(vec![xa[i].clone(), xb[j].clone()]),
                mul(vec![Expr::int(-1), xa[j].clone(), xb[i].clone()]),
            ])
        };
        if xa.len() == 3 && xb.len() == 3 {
            return present(&Expr::Seq(
                named_op_kind(ka, kb),
                vec![z(1, 2), z(2, 0), z(0, 1)],
            ));
        }
        if xa.len() == 2 && xb.len() == 2 {
            return present(&z(0, 1));
        }
    }
    Expr::OtherOp(Sym::new("cross_prod"), vec![ca, cb])
}
