//! Deciding `x ∈ {a, b, …}` when the members are concrete enough to settle it.

use crate::equality::{equals, EqOptions};
use crate::expr::{Expr, RelOp, SeqKind};
use crate::normalize::canonicalize;

/// Evaluate a finite-set membership relation to a truth value: `x ∈ {a, b, …}`
/// is `Some(true)` when `x` equals a member,
/// `Some(false)` when every membership comparison is decidably false and the
/// candidate is a closed (constant) expression, and `None` otherwise.
/// `∋`/`∌` orientations are handled by canonicalization; `∉` negates.
pub fn evaluate_membership(e: &Expr, opts: &EqOptions) -> Option<bool> {
    let canon = canonicalize(e);
    let Expr::Relation { operands, ops } = &canon else {
        return None;
    };
    let ([lhs, rhs], [op]) = (operands.as_slice(), ops.as_slice()) else {
        return None;
    };
    let negate = match op {
        RelOp::In => false,
        RelOp::NotIn => true,
        _ => return None,
    };
    let Expr::Seq(SeqKind::Set, members) = rhs else {
        return None;
    };
    if members.iter().any(|m| equals(lhs, m, opts)) {
        return Some(!negate);
    }
    // No member matched: definitive only when everything is closed (constant)
    // — a symbolic candidate might still equal a member.
    let closed = |x: &Expr| {
        crate::ops::variables(x)
            .iter()
            .all(|v| crate::expr::sym::is_constant_symbol(v))
    };
    if closed(lhs) && members.iter().all(closed) {
        return Some(negate);
    }
    None
}
