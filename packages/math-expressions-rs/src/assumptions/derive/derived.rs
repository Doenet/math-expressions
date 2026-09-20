//! Derived assumptions: the facts that follow from combining the stored ones.
//!
//! The store files each fact under the variables it mentions, so `x < a` lands
//! under `x` and under `a`. Chaining them is this module's job: with `x < a`
//! and `a < b` on file, `x < b` holds and has to come back from a query about
//! `x` even though nobody stated it.

use super::combine::combine_assumptions;
use super::for_expr::get_assumptions_for_expr;
use super::relops::read_from_right;
use crate::assumptions::clean::{clean_assumptions, is_js_leaf};
use crate::assumptions::tree_store::{Facts, TreeStore, VarMap};
use crate::expr::serde::{to_js, try_from_js};
use crate::expr::{Expr, RelOp};
use crate::ops::variables;

/// Every assumption on the variables of the store's facts that follows from
/// combining them, keyed by variable, with anything already recorded on that
/// variable filtered out.
///
/// The derived facts are a function of the whole store, so this recomputes all
/// of them after any change.
pub(crate) fn calculate_derived_assumptions(store: &TreeStore) -> VarMap {
    let collected: Vec<Expr> = store
        .by_var()
        .iter()
        .filter_map(|(_, f)| match f {
            Facts::Tree(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    if collected.is_empty() {
        return VarMap::default();
    }
    let tree = if collected.len() == 1 {
        collected.into_iter().next().expect("len 1")
    } else {
        Expr::And(collected)
    };
    match clean_assumptions(&tree, None) {
        Some(tree) => derive_from(store, &tree),
        None => VarMap::default(),
    }
}

/// Everything `tree` entails about the variables it mentions.
fn derive_from(store: &TreeStore, tree: &Expr) -> VarMap {
    if is_js_leaf(tree) {
        return VarMap::default();
    }

    if let Expr::And(operands) | Expr::Or(operands) = tree {
        let is_and = matches!(tree, Expr::And(_));
        let results: Vec<VarMap> = operands.iter().map(|v| derive_from(store, v)).collect();

        let mut allvars: Vec<String> = Vec::new();
        for r in &results {
            for (v, _) in r.iter() {
                if !allvars.contains(v) {
                    allvars.push(v.clone());
                }
            }
        }

        let mut derived = VarMap::default();

        for v in allvars {
            let res: Vec<Expr> = results
                .iter()
                .filter_map(|b| match b.get(&v) {
                    Some(Facts::Tree(t)) => Some(t.clone()),
                    _ => None,
                })
                .collect();

            // An `or` only entails something about `v` if every branch does.
            if !is_and && res.len() != results.len() {
                continue;
            }

            let joined = |res: Vec<Expr>| {
                if res.len() > 1 {
                    Some(if is_and {
                        Expr::And(res)
                    } else {
                        Expr::Or(res)
                    })
                } else {
                    res.into_iter().next()
                }
            };
            let new_derived = match derived.get(&v) {
                Some(Facts::Tree(prev)) => {
                    let prev = prev.clone();
                    joined(res).map(|r| Expr::And(vec![prev, r]))
                }
                _ => joined(res),
            };

            let known = store.facts_for_variables(std::slice::from_ref(&v), &[], true);
            let cleaned = new_derived.and_then(|t| clean_assumptions(&t, known.as_ref()));
            derived.set(&v, Facts::from(cleaned));
        }

        return derived;
    }

    let mut derived = VarMap::default();

    if let Expr::Relation { operands, ops } = tree {
        if let [operator] = ops.as_slice() {
            if matches!(
                operator,
                RelOp::Eq
                    | RelOp::Ne
                    | RelOp::Lt
                    | RelOp::Le
                    | RelOp::In
                    | RelOp::Subset
                    | RelOp::NotIn
                    | RelOp::NotSubset
            ) {
                let mut addressed_assumption = false;

                // Only a side that *is* a variable can carry a derived fact
                // about it.
                for ind in 0..2 {
                    let (v, other) = (&operands[ind], &operands[1 - ind]);
                    let Expr::Sym(name) = v else { continue };
                    let name = name.name();
                    let other_var = variables(other);
                    if other_var.is_empty() || other_var.contains(&name) {
                        continue;
                    }

                    addressed_assumption = true;

                    // Reading the relation from the right-hand side reverses it.
                    let adjusted_op = if ind == 1 {
                        read_from_right(*operator).unwrap_or(*operator)
                    } else {
                        *operator
                    };

                    let result =
                        get_assumptions_for_expr(store, other, std::slice::from_ref(&name));
                    let Some(result) = combine_assumptions(v, adjusted_op, other, result.as_ref())
                    else {
                        continue;
                    };

                    let new_derived = match derived.get(&name) {
                        Some(Facts::Tree(prev)) => Expr::And(vec![prev.clone(), result]),
                        _ => result,
                    };

                    let known = store.facts_for_variables(std::slice::from_ref(&name), &[], true);
                    let cleaned = clean_assumptions(&new_derived, known.as_ref());
                    derived.set(&name, Facts::from(cleaned));
                }
                if addressed_assumption {
                    return derived;
                }
            }
        }
    }

    // Nothing could be combined, so carry over whatever is known about the
    // operands unchanged.
    let collected: Vec<Expr> = js_operands(tree)
        .iter()
        .filter_map(|op| get_assumptions_for_expr(store, op, &[]))
        .collect();

    if collected.is_empty() {
        return VarMap::default();
    }

    let results = if collected.len() == 1 {
        collected.into_iter().next().expect("len 1")
    } else {
        Expr::And(collected)
    };

    for v in variables(tree) {
        let known = store.facts_for_variables(std::slice::from_ref(&v), &[], true);
        derived.set(&v, Facts::from(clean_assumptions(&results, known.as_ref())));
    }

    derived
}

/// The operands of a node the pass above has no rule for — `tree.slice(1)` on
/// the JS spelling.
///
/// Taken through the JSON form rather than matched per variant, because "the
/// operands" is exactly what the JS tree says they are, and an `Expr` variant's
/// children need not line up with it (an interval's closure flags are operands
/// there and metadata here).
fn js_operands(e: &Expr) -> Vec<Expr> {
    let js = to_js(e);
    let Some(parts) = js.as_array() else {
        return Vec::new();
    };
    parts
        .iter()
        .skip(1)
        .filter_map(|v| try_from_js(v).ok())
        .collect()
}
