//! A total canonical order on `Expr`. Commutative operators sort their operands
//! by this order, so two equal canonical expressions have identical trees and
//! structural equality reduces to tree comparison.
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
        // Only the specials (`∞`, `NaN`, `None`) rank as constants of their own.
        // `π`/`e`/`i` rank with the symbols and compare by name, so the two
        // spellings of one constant sort in the same place — see `cmp`.
        Expr::Const(c) if c.symbol_name().is_none() => 1,
        Expr::Const(_) | Expr::Sym(_) => 2,
        // Between Sym and Pow (MATRIX_PLAN §2a): an atom that sorts with the
        // other irrational atoms, before compound expressions.
        Expr::RootOf { .. } => 3,
        Expr::Blank => 4,
        Expr::Ldots => 5,
        // An atom, not an operator: it sorts with the other leaves.
        Expr::Bool(_) => 6,
        Expr::Pow(..) => 7,
        Expr::Prime(_) => 8,
        Expr::Index(..) => 9,
        Expr::Apply(..) => 10,
        Expr::Mul(_) => 11,
        Expr::Div(..) => 12,
        Expr::Neg(_) => 13,
        Expr::Add(_) => 14,
        Expr::And(_) => 15,
        Expr::Or(_) => 16,
        Expr::Not(_) => 17,
        Expr::Union(_) => 18,
        Expr::Intersect(_) => 19,
        Expr::Seq(..) => 20,
        Expr::Interval { .. } => 21,
        Expr::Relation { .. } => 22,
        Expr::Matrix { .. } => 23,
        Expr::OtherOp(..) => 24,
    }
}

/// Order among the specials only — `π`/`e`/`i` never reach this, they compare
/// by name against the symbols.
fn const_index(c: MathConst) -> u8 {
    match c {
        MathConst::Inf => 0,
        MathConst::NegInf => 1,
        MathConst::NaN => 2,
        MathConst::None => 3,
        MathConst::Pi | MathConst::E | MathConst::I => 4,
    }
}

/// The name a rank-2 leaf sorts under: a symbol's own name, or a named
/// constant's spelling.
fn leaf_name(e: &Expr) -> String {
    match e {
        Expr::Sym(s) => s.name(),
        Expr::Const(c) => c.symbol_name().unwrap_or_default().to_string(),
        _ => String::new(),
    }
}

/// Tiebreak between the two spellings of one name, `Const` first.
///
/// [`leaf_name`] deliberately reports the same name for `Const(Pi)` and
/// `Sym("pi")` so they sort together — but they are only the *same value* while
/// the name is declared, and `canonicalize` collapses them then. Undeclared,
/// both can stand in one tree, and a comparator that called them `Equal` would
/// leave their relative position to the (stable) sort's input order — the sum
/// `Const(Pi) + Sym("pi")` and the sum `Sym("pi") + Const(Pi)` would
/// canonicalize to different trees, and `==` would call two identical
/// expressions unequal. The comparator has to be a total order over *values*,
/// so distinct spellings get distinct keys.
fn spelling_rank(e: &Expr) -> u8 {
    match e {
        Expr::Const(_) => 0,
        _ => 1,
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
        // Rank 1: the specials, which have no symbol spelling.
        (Expr::Const(x), Expr::Const(y)) if x.symbol_name().is_none() => {
            const_index(*x).cmp(&const_index(*y))
        }
        // Rank 2: symbols and the named constants, all by name. `Sym("pi")` and
        // `Const(Pi)` land together, and neither jumps ahead of the variables —
        // sorting is alphabetical whatever `constant_policy` declares, because
        // this comparator is what makes `==` on canonical trees mean equality
        // (and canonical trees are persisted in DoenetML document state). Which
        // is also why the two spellings still break their tie rather than
        // comparing `Equal` — see `spelling_rank`.
        (Expr::Const(_) | Expr::Sym(_), Expr::Const(_) | Expr::Sym(_)) => leaf_name(a)
            .cmp(&leaf_name(b))
            .then_with(|| spelling_rank(a).cmp(&spelling_rank(b))),
        (Expr::Bool(x), Expr::Bool(y)) => x.cmp(y),
        (Expr::Blank, Expr::Blank) | (Expr::Ldots, Expr::Ldots) => Ordering::Equal,
        (
            Expr::RootOf {
                poly: p1,
                index: i1,
            },
            Expr::RootOf {
                poly: p2,
                index: i2,
            },
        ) => p1
            .len()
            .cmp(&p2.len())
            .then_with(|| {
                p1.iter()
                    .zip(p2.iter())
                    .map(|(a, b)| number_cmp(a, b))
                    .find(|o| *o != Ordering::Equal)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| i1.cmp(i2)),

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

        (Expr::Matrix(m1), Expr::Matrix(m2)) => m1
            .rows()
            .cmp(&m2.rows())
            .then_with(|| m1.cols().cmp(&m2.cols()))
            .then_with(|| slice_cmp(m1.entries(), m2.entries())),

        (Expr::OtherOp(n1, a1), Expr::OtherOp(n2, a2)) => {
            n1.name().cmp(&n2.name()).then_with(|| slice_cmp(a1, a2))
        }

        // rank() is 1:1 on variants apart from `Const`/`Sym` sharing rank 2,
        // which the arm above handles, so this is unreachable. Fall back to
        // Equal for totality.
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
