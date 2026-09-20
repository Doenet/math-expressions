//! The canonical form a stored assumption is held in, and the two ways of
//! taking one apart again.
//!
//! Everything the tree store holds passes through [`clean_assumptions`] on the
//! way in and on the way out, which is what makes structural equality a usable
//! test between two facts: without it `a > b` and `b < a` are different trees
//! and the store would file, and hand back, both.

use super::expand::expand_relations;
use super::tree_store::Facts;
use crate::expr::Expr;
use crate::normalize::{default_order, flatten_logical, push_not, simplify};
use crate::ops::variables;

/// Does this spell as a JS *array*? The legacy passes key off `Array.isArray`,
/// which is false for the leaves that spell as a bare string, number, boolean
/// or tagged object — and each pass has its own answer for one of those, so the
/// distinction has to survive the port.
pub(super) fn is_js_leaf(e: &Expr) -> bool {
    matches!(
        e,
        Expr::Num(_) | Expr::Sym(_) | Expr::Const(_) | Expr::Bool(_) | Expr::Blank
    )
}

/// Normalize an incoming assumption before it is filed. The value here is the
/// arithmetic normalization (`5-5` to `0`) plus a consistent orientation of the
/// relation.
pub(super) fn normalize_assumption(tree: &Expr) -> Expr {
    simplify(tree)
}

/// Canonical form: relations expanded into comparisons, `not` pushed down,
/// operands ordered, and duplicates — including anything already in `known` —
/// dropped.
///
/// `None` when nothing is left after the `known` facts are removed.
pub(super) fn clean_assumptions(tree: &Expr, known: Option<&Expr>) -> Option<Expr> {
    if is_js_leaf(tree) {
        return Some(tree.clone());
    }

    let tree = flatten_logical(&default_order(&push_not(&expand_relations(tree))));

    let is_and = matches!(tree, Expr::And(_));
    let mut result = tree.clone();

    if let Expr::And(operands) | Expr::Or(operands) = &tree {
        let mut kept: Vec<Expr> = Vec::with_capacity(operands.len());
        for b in operands {
            if !kept.contains(b) {
                kept.push(b.clone());
            }
        }
        if is_and {
            if let Some(known) = known_operands(known) {
                kept.retain(|v| !known.contains(v));
            }
        }
        result = if kept.len() == 1 {
            kept.swap_remove(0)
        } else if is_and {
            Expr::And(kept)
        } else {
            Expr::Or(kept)
        };
    }

    // A single fact that is already known adds nothing. An `or` reaches this
    // too: the dedupe above works inside it, this asks about it whole.
    if !is_and {
        if let Some(known) = known_operands(known) {
            if known.contains(&result) {
                return None;
            }
        }
    }

    Some(result)
}

/// The individual facts `known` states, or `None` when it states none.
fn known_operands(known: Option<&Expr>) -> Option<Vec<Expr>> {
    let known = known?;
    if is_js_leaf(known) {
        return None;
    }
    Some(match known {
        Expr::And(xs) => xs.clone(),
        other => vec![other.clone()],
    })
}

/// The part of a fact that stays clear of `exclude_variables`, or `None` if
/// none of it does. Used to answer a query about one variable without dragging
/// in the variable that query came from.
pub(super) fn filter_assumptions_from_tree(
    tree: &Expr,
    exclude_variables: &[String],
) -> Option<Expr> {
    if is_js_leaf(tree) {
        return None;
    }

    if let Expr::And(operands) = tree {
        let kept: Vec<Expr> = operands
            .iter()
            .filter_map(|v| filter_assumptions_from_tree(v, exclude_variables))
            .collect();
        return match kept.len() {
            0 => None,
            1 => kept.into_iter().next(),
            _ => Some(Expr::And(kept)),
        };
    }

    let tree_variables = variables(tree);
    let contains_excluded = exclude_variables.iter().any(|v| tree_variables.contains(v));
    (!contains_excluded).then(|| tree.clone())
}

/// Drop `tree` — or its solved-for-a-variable spelling — from a fact. `None`
/// when there was nothing to remove, [`Facts::Empty`] when nothing is left.
pub(super) fn remove_from(current: &Expr, tree: &Expr, solved: Option<&Expr>) -> Option<Facts> {
    let matches = |v: &Expr| v == tree || solved.is_some_and(|s| v == s);

    if let Expr::And(operands) = current {
        let kept: Vec<Expr> = operands.iter().filter(|v| !matches(v)).cloned().collect();
        if kept.is_empty() {
            return Some(Facts::Empty);
        }
        if kept.len() == 1 {
            return Some(Facts::Tree(kept.into_iter().next().expect("len 1")));
        }
        if kept.len() < operands.len() {
            return Some(Facts::Tree(Expr::And(kept)));
        }
        return None;
    }

    matches(current).then_some(Facts::Empty)
}
