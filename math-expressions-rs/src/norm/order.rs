//! A total canonical order on `Expr` (PORTING_PLAN.md §7c, redesign note
//! §3.4 option B). Commutative operators sort their operands by this order, so
//! two equal canonical expressions have identical trees and structural
//! equality reduces to tree comparison.
//!
//! Unlike the JS `default_order` (which allocates a fresh nested sort-key array
//! per node per comparison), this is a direct typed comparator: no allocation,
//! and symbols compare by resolved name so the order is stable across sessions
//! (the `Sym` interner index is insertion-order, which is not).

use crate::expr::{Expr, MathConst, RelOp, SeqKind};
use crate::num::Number;
use std::cmp::Ordering;

/// Coarse ordering class: numbers first, then atoms, then operators by
/// increasing "structural weight". Only the relative order matters.
fn rank(e: &Expr) -> u8 {
    match e {
        Expr::Num(_) => 0,
        Expr::Const(_) => 1,
        Expr::Sym(_) => 2,
        Expr::Blank => 3,
        Expr::Ldots => 4,
        Expr::Pow(..) => 5,
        Expr::Prime(_) => 6,
        Expr::Index(..) => 7,
        Expr::Apply(..) => 8,
        Expr::Mul(_) => 9,
        Expr::Div(..) => 10,
        Expr::Neg(_) => 11,
        Expr::Add(_) => 12,
        Expr::And(_) => 13,
        Expr::Or(_) => 14,
        Expr::Not(_) => 15,
        Expr::Union(_) => 16,
        Expr::Intersect(_) => 17,
        Expr::Seq(..) => 18,
        Expr::Interval { .. } => 19,
        Expr::Relation { .. } => 20,
        Expr::Matrix { .. } => 21,
        Expr::OtherOp(..) => 22,
    }
}

fn const_index(c: MathConst) -> u8 {
    match c {
        MathConst::Pi => 0,
        MathConst::E => 1,
        MathConst::I => 2,
        MathConst::Inf => 3,
        MathConst::NegInf => 4,
        MathConst::NaN => 5,
    }
}

/// Total order on numbers: by numeric value, with an exact tiebreak (so two
/// distinct rationals with the same f64 still order deterministically).
pub(crate) fn number_cmp(a: &Number, b: &Number) -> Ordering {
    a.to_f64()
        .partial_cmp(&b.to_f64())
        .unwrap_or(Ordering::Equal)
        .then_with(|| a.js_string().cmp(&b.js_string()))
}

/// Lexicographic comparison of two operand slices, shorter first on a prefix.
fn slice_cmp(a: &[Expr], b: &[Expr]) -> Ordering {
    for (x, y) in a.iter().zip(b.iter()) {
        let c = cmp(x, y);
        if c != Ordering::Equal {
            return c;
        }
    }
    a.len().cmp(&b.len())
}

/// The canonical total order.
pub(crate) fn cmp(a: &Expr, b: &Expr) -> Ordering {
    let by_rank = rank(a).cmp(&rank(b));
    if by_rank != Ordering::Equal {
        return by_rank;
    }
    match (a, b) {
        (Expr::Num(x), Expr::Num(y)) => number_cmp(x, y),
        (Expr::Const(x), Expr::Const(y)) => const_index(*x).cmp(&const_index(*y)),
        (Expr::Sym(x), Expr::Sym(y)) => x.name().cmp(&y.name()),
        (Expr::Blank, Expr::Blank) | (Expr::Ldots, Expr::Ldots) => Ordering::Equal,

        (Expr::Pow(b1, e1), Expr::Pow(b2, e2)) => cmp(b1, b2).then_with(|| cmp(e1, e2)),
        (Expr::Prime(x), Expr::Prime(y)) | (Expr::Not(x), Expr::Not(y)) => cmp(x, y),
        (Expr::Index(a1, b1), Expr::Index(a2, b2)) => cmp(a1, a2).then_with(|| cmp(b1, b2)),
        (Expr::Neg(x), Expr::Neg(y)) => cmp(x, y),

        (Expr::Apply(h1, a1), Expr::Apply(h2, a2)) => cmp(h1, h2).then_with(|| slice_cmp(a1, a2)),

        (Expr::Mul(x), Expr::Mul(y))
        | (Expr::Add(x), Expr::Add(y))
        | (Expr::And(x), Expr::And(y))
        | (Expr::Or(x), Expr::Or(y))
        | (Expr::Union(x), Expr::Union(y))
        | (Expr::Intersect(x), Expr::Intersect(y)) => slice_cmp(x, y),

        (Expr::Div(a1, b1), Expr::Div(a2, b2)) => cmp(a1, a2).then_with(|| cmp(b1, b2)),

        (Expr::Seq(k1, x), Expr::Seq(k2, y)) => seq_index(*k1)
            .cmp(&seq_index(*k2))
            .then_with(|| slice_cmp(x, y)),
        (
            Expr::Interval {
                endpoints: e1,
                closed: c1,
            },
            Expr::Interval {
                endpoints: e2,
                closed: c2,
            },
        ) => cmp(&e1.0, &e2.0)
            .then_with(|| cmp(&e1.1, &e2.1))
            .then_with(|| c1.cmp(c2)),

        (
            Expr::Relation {
                operands: o1,
                ops: p1,
            },
            Expr::Relation {
                operands: o2,
                ops: p2,
            },
        ) => slice_cmp(o1, o2).then_with(|| {
            p1.iter()
                .map(|r| rel_index(*r))
                .cmp(p2.iter().map(|r| rel_index(*r)))
        }),

        (
            Expr::Matrix {
                rows: r1,
                cols: c1,
                entries: e1,
            },
            Expr::Matrix {
                rows: r2,
                cols: c2,
                entries: e2,
            },
        ) => r1
            .cmp(r2)
            .then_with(|| c1.cmp(c2))
            .then_with(|| slice_cmp(e1, e2)),

        (Expr::OtherOp(n1, a1), Expr::OtherOp(n2, a2)) => {
            n1.name().cmp(&n2.name()).then_with(|| slice_cmp(a1, a2))
        }

        // Different variants share a rank only if rank() is not 1:1 — it is,
        // so this is unreachable. Fall back to Equal for totality.
        _ => Ordering::Equal,
    }
}

fn seq_index(k: SeqKind) -> u8 {
    match k {
        SeqKind::Tuple => 0,
        SeqKind::Array => 1,
        SeqKind::List => 2,
        SeqKind::Set => 3,
        SeqKind::Vector => 4,
        SeqKind::AltVector => 5,
    }
}

fn rel_index(r: RelOp) -> u8 {
    match r {
        RelOp::Eq => 0,
        RelOp::Ne => 1,
        RelOp::Lt => 2,
        RelOp::Le => 3,
        RelOp::Gt => 4,
        RelOp::Ge => 5,
        RelOp::In => 6,
        RelOp::NotIn => 7,
        RelOp::Ni => 8,
        RelOp::NotNi => 9,
        RelOp::Subset => 10,
        RelOp::NotSubset => 11,
        RelOp::SubsetEq => 12,
        RelOp::NotSubsetEq => 13,
        RelOp::Superset => 14,
        RelOp::NotSuperset => 15,
        RelOp::SupersetEq => 16,
        RelOp::NotSupersetEq => 17,
    }
}
