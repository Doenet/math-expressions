//! Restating what is known about variables as a statement about an expression
//! built from them.
//!
//! Asking for the assumptions on `q - x` has to answer `q - x > 0` from a
//! stored `q > x`. That works whenever the expression is linear in its
//! variables: each stored relation on a variable is substituted back into the
//! expression, which turns a fact about the variable into a fact about the
//! whole.

use super::combine::combine_assumptions;
use super::relops::read_from_right;
use crate::assumptions::clean::{clean_assumptions, is_js_leaf};
use crate::assumptions::tree_store::TreeStore;
use crate::expr::{Expr, RelOp};
use crate::grade::linear_decomposition;
use crate::normalize::simplify;
use crate::num::Number;
use crate::ops::{substitute, variables};
use std::collections::HashMap;

/// Assumptions that can be stated about `expr` itself, skipping anything that
/// mentions `exclude_variables`.
///
/// When `expr` is linear in its variables, a fact about one of them can be
/// rewritten as a fact about `expr`: with `q > x` on file, `q - x` is `> 0`.
/// Otherwise the assumptions on the individual variables are returned as they
/// stand.
pub(crate) fn get_assumptions_for_expr(
    store: &TreeStore,
    expr: &Expr,
    exclude_variables: &[String],
) -> Option<Expr> {
    let vars: Vec<String> = variables(expr)
        .into_iter()
        .filter(|v| !exclude_variables.contains(v))
        .collect();

    if vars.is_empty() {
        return None;
    }

    let Some((coefficients, b)) = linear_decomposition(expr, &vars) else {
        // Not linear: fall back to the assumptions on each variable.
        let results: Vec<Expr> = variables(expr)
            .iter()
            .filter_map(|v| get_assumptions_for_expr(store, &Expr::sym(v), exclude_variables))
            .collect();
        return match results.len() {
            0 => None,
            1 => results.into_iter().next(),
            _ => Some(Expr::And(results)),
        };
    };

    // `expr` is the variable itself, so a containment (`x ∈ A`) can be carried
    // over verbatim; for any other linear combination it could not.
    let identity = is_number(&b, 0) && is_number(&coefficients[0], 1) && vars.len() == 1;

    let new_assumptions = store.facts_for_variables(&vars, exclude_variables, false)?;

    let ctx = Ctx {
        store,
        expr,
        vars: &vars,
        coefficients: &coefficients,
        exclude_variables,
        identity,
    };
    clean_assumptions(&ctx.process(&new_assumptions)?, None)
}

/// What the rewriting below needs to know: the expression being restated, its
/// linear decomposition, and the query it is answering.
struct Ctx<'a> {
    store: &'a TreeStore,
    expr: &'a Expr,
    vars: &'a [String],
    coefficients: &'a [Expr],
    exclude_variables: &'a [String],
    identity: bool,
}

impl Ctx<'_> {
    fn process(&self, new_as: &Expr) -> Option<Expr> {
        if is_js_leaf(new_as) {
            return None;
        }

        if let Expr::And(operands) | Expr::Or(operands) = new_as {
            let is_or = matches!(new_as, Expr::Or(_));
            let results: Vec<Expr> = operands.iter().filter_map(|v| self.process(v)).collect();

            if results.is_empty() {
                return None;
            }
            if is_or {
                // An `or` survives only if every branch does; a partial
                // disjunction would claim more than is known.
                return (results.len() == operands.len()).then_some(Expr::Or(results));
            }
            if results.len() == 1 {
                return results.into_iter().next();
            }
            return Some(Expr::And(results));
        }

        let rewritable = match new_as {
            Expr::Relation { ops, .. } => match ops.as_slice() {
                [RelOp::Eq | RelOp::Ne | RelOp::Lt | RelOp::Le] => true,
                [RelOp::In | RelOp::NotIn | RelOp::Subset | RelOp::NotSubset] => self.identity,
                _ => false,
            },
            _ => false,
        };
        if !rewritable {
            return Some(self.with_assumptions_on_other_variables(new_as));
        }
        let Expr::Relation { operands, ops } = new_as else {
            unreachable!("checked above")
        };
        let operator = ops[0];

        let mut results: Vec<Expr> = Vec::new();

        for ind in 0..2 {
            let (next_var, next_rhs) = (&operands[ind], &operands[1 - ind]);
            let Expr::Sym(name) = next_var else { continue };
            let name = name.name();
            let Some(pos) = self.vars.iter().position(|v| *v == name) else {
                continue;
            };

            let subs = HashMap::from([(name.clone(), next_rhs.clone())]);
            let new_expr = simplify(&substitute(self.expr, &subs));

            // Two things can reverse the relation: a negative coefficient in
            // `expr`, and reading the stored relation from its right-hand side.
            // Both at once cancel out.
            let mut flip = false;
            let mut operator_eff = operator;
            if let Some(coefficient) = numeric_coefficient(&self.coefficients[pos]) {
                if (ind == 1 && coefficient.is_positive())
                    || (ind == 0 && coefficient.is_negative())
                {
                    if let Some(reversed) = read_from_right(operator) {
                        flip = true;
                        operator_eff = reversed;
                    }
                }
            }

            results.push(Expr::Relation {
                operands: if flip {
                    vec![new_expr.clone(), self.expr.clone()]
                } else {
                    vec![self.expr.clone(), new_expr.clone()]
                },
                ops: vec![operator],
            });

            // Chase whatever is known about the substituted expression.
            let mut new_exclude = self.exclude_variables.to_vec();
            new_exclude.push(name);
            let res = get_assumptions_for_expr(self.store, &new_expr, &new_exclude);
            if let Some(res) = combine_assumptions(self.expr, operator_eff, &new_expr, res.as_ref())
            {
                results.push(res);
            }
        }

        match results.len() {
            0 => Some(self.with_assumptions_on_other_variables(new_as)),
            1 => results.into_iter().next(),
            _ => Some(Expr::And(results)),
        }
    }

    /// A fact that could not be restated in terms of `expr` is kept as it is,
    /// together with what is known about the *other* variables it mentions.
    fn with_assumptions_on_other_variables(&self, new_as: &Expr) -> Expr {
        let mut new_exclude = self.exclude_variables.to_vec();
        new_exclude.extend(variables(self.expr));

        let mut results = vec![new_as.clone()];
        for v in variables(new_as) {
            if new_exclude.contains(&v) {
                continue;
            }
            if let Some(res) = get_assumptions_for_expr(self.store, &Expr::sym(&v), &new_exclude) {
                results.push(res);
            }
        }
        if results.len() == 1 {
            return new_as.clone();
        }
        Expr::And(results)
    }
}

/// A coefficient whose sign is readable — only a bare number counts, since the
/// sign is what decides whether an inequality flips.
fn numeric_coefficient(e: &Expr) -> Option<Number> {
    match e {
        Expr::Num(n) => Some(n.clone()),
        Expr::Neg(inner) => match &**inner {
            Expr::Num(n) => Some(n.neg()),
            _ => None,
        },
        _ => None,
    }
}

fn is_number(e: &Expr, v: i64) -> bool {
    matches!(e, Expr::Num(n) if *n == Number::Int(v))
}
