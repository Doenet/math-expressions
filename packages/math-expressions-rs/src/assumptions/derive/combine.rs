//! Composing two relations into one.
//!
//! The store knows `x < a` and, separately, `a < b`. Chaining them into `x < b`
//! is a question about the *operators*: which pairs compose, into what, and
//! which say nothing together (`x < a` with `a > b` bounds `x` from neither
//! side). That table is this module.

use super::relops::{read_from_right, reversed_form};
use crate::assumptions::clean::is_js_leaf;
use crate::expr::{Expr, RelOp};

/// Given the assumption `expr1 op1 expr2` plus the assumptions `new_as` about
/// `expr2`, state what follows about `expr1`.
///
/// `None` when `new_as` says nothing about `expr1`, and `new_as` itself when it
/// bears on `expr1` but cannot be reduced to a relation on it.
pub(crate) fn combine_assumptions(
    expr1: &Expr,
    op1: RelOp,
    expr2: &Expr,
    new_as: Option<&Expr>,
) -> Option<Expr> {
    // The ⊆/⊇ family has no place in the table below; a fact stated with one
    // is passed through rather than composed.
    if matches!(
        op1,
        RelOp::SubsetEq | RelOp::NotSubsetEq | RelOp::SupersetEq | RelOp::NotSupersetEq
    ) {
        return new_as.cloned();
    }

    let new_as = new_as?;
    if is_js_leaf(new_as) {
        return None;
    }

    if let Expr::And(operands) | Expr::Or(operands) = new_as {
        let is_or = matches!(new_as, Expr::Or(_));
        let results: Vec<Expr> = operands
            .iter()
            .filter_map(|v| combine_assumptions(expr1, op1, expr2, Some(v)))
            .collect();

        if results.is_empty() {
            return None;
        }
        if is_or {
            // A disjunction only follows if every branch of it does.
            return (results.len() == operands.len()).then_some(Expr::Or(results));
        }
        if results.len() == 1 {
            return results.into_iter().next();
        }
        return Some(Expr::And(results));
    }

    let Expr::Relation { operands, ops } = new_as else {
        return Some(new_as.clone());
    };
    let [op2] = ops.as_slice() else {
        return Some(new_as.clone());
    };
    if !matches!(
        op2,
        RelOp::Eq
            | RelOp::Ne
            | RelOp::Lt
            | RelOp::Le
            | RelOp::In
            | RelOp::NotIn
            | RelOp::Subset
            | RelOp::NotSubset
    ) {
        return Some(new_as.clone());
    }

    // Reading the far relation from its right-hand side reverses it.
    let (rhs, op2_eff) = if operands[0] == *expr2 {
        (operands[1].clone(), *op2)
    } else if operands[1] == *expr2 {
        (operands[0].clone(), read_from_right(*op2).unwrap_or(*op2))
    } else {
        return Some(new_as.clone());
    };

    let combined_op = if op1 == RelOp::Eq {
        op2_eff
    } else if op2_eff == RelOp::Eq {
        op1
    } else {
        match combine_ops(op1, op2_eff)? {
            Some(op) => op,
            None => {
                // A membership relation on the far side still constrains
                // `expr1`, it just cannot be folded into a single relation.
                if matches!(op1, RelOp::Lt | RelOp::Le | RelOp::Gt | RelOp::Ge)
                    && matches!(op2_eff, RelOp::In | RelOp::NotIn)
                {
                    return Some(new_as.clone());
                }
                return None;
            }
        }
    };

    // A composed relation is stated with `expr1` on the left, so the reversed
    // operators are re-spelled with their operands swapped.
    Some(match reversed_form(combined_op) {
        Some(rev) => Expr::Relation {
            operands: vec![rhs, expr1.clone()],
            ops: vec![rev],
        },
        None => Expr::Relation {
            operands: vec![expr1.clone(), rhs],
            ops: vec![combined_op],
        },
    })
}

/// How `expr1 op1 expr2` composes with `expr2 op2 rhs`.
///
/// The outer `None` is an operator with no row at all — `ne`, which composes
/// with nothing and, unlike a row that simply has no entry for `op2`, gets no
/// membership fallback either. The inner `None` is a row that has no entry: the
/// two say nothing together (`x < a` with `a > b` bounds `x` from neither
/// side), or they interact in a way that cannot be stated about `expr1`.
fn combine_ops(op1: RelOp, op2: RelOp) -> Option<Option<RelOp>> {
    use RelOp::*;
    Some(match op1 {
        Lt => match op2 {
            Lt | Le => Some(Lt),
            _ => None,
        },
        Le => match op2 {
            Lt => Some(Lt),
            Le => Some(Le),
            _ => None,
        },
        Gt => match op2 {
            Gt | Ge => Some(Gt),
            _ => None,
        },
        Ge => match op2 {
            Gt => Some(Gt),
            Ge => Some(Ge),
            _ => None,
        },
        In => (op2 == Subset).then_some(In),
        NotIn => (op2 == Superset).then_some(NotIn),
        Ni => (op2 == NotIn).then_some(NotSubset),
        NotNi => (op2 == In).then_some(NotSuperset),
        Subset => match op2 {
            Subset => Some(Subset),
            NotNi => Some(NotNi),
            NotSuperset => Some(NotSuperset),
            _ => None,
        },
        NotSubset => (op2 == Superset).then_some(NotSubset),
        Superset => match op2 {
            Superset => Some(Superset),
            Ni => Some(Ni),
            NotSubset => Some(NotSubset),
            _ => None,
        },
        NotSuperset => (op2 == Subset).then_some(NotSuperset),
        _ => return None,
    })
}
