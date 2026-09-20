//! Re-spelling a relation. Two maps, and they are not the same one.
//!
//! [`read_from_right`] is the relation seen from its other operand — the
//! question "the store knows `a < x`; what does that say about `x`?".
//! [`reversed_form`] is the operators that exist only as the mirror of another,
//! and so are *written* by swapping the operands instead.

use crate::expr::RelOp;

/// The same relation read with its operands swapped, or `None` for an operator
/// that reads the same either way (`=`, `≠`).
pub(super) fn read_from_right(op: RelOp) -> Option<RelOp> {
    Some(match op {
        RelOp::Lt => RelOp::Gt,
        RelOp::Le => RelOp::Ge,
        RelOp::In => RelOp::Ni,
        RelOp::NotIn => RelOp::NotNi,
        RelOp::Subset => RelOp::Superset,
        RelOp::NotSubset => RelOp::NotSuperset,
        _ => return None,
    })
}

/// The operators a composed relation is not stated with: `x > a` is written
/// `a < x`, so that the expression being spoken about stays on the left.
pub(super) fn reversed_form(op: RelOp) -> Option<RelOp> {
    Some(match op {
        RelOp::Gt => RelOp::Lt,
        RelOp::Ge => RelOp::Le,
        RelOp::Ni => RelOp::In,
        RelOp::NotNi => RelOp::NotIn,
        RelOp::Superset => RelOp::Subset,
        RelOp::NotSuperset => RelOp::NotSubset,
        _ => return None,
    })
}
