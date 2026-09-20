//! Filing and unfiling a single assumption.
//!
//! A fact is recorded once per variable it mentions, and it is recorded *solved
//! for* that variable where possible — `a + b < 1` is filed under `a` as
//! `a < 1 - b` — so that a query about `a` can be answered with a statement
//! about `a` rather than a relation the caller has to rearrange. That is also
//! why removal tries both spellings: removing `5 < z` has to find the `z > 5`
//! that was filed.

use super::{Facts, TreeStore};
use crate::assumptions::clean::{clean_assumptions, is_js_leaf, normalize_assumption, remove_from};
use crate::assumptions::derive::calculate_derived_assumptions;
use crate::assumptions::Assumptions;
use crate::expr::Expr;
use crate::ops::{substitute, variables};
use std::collections::HashMap;

impl TreeStore {
    /// File an assumption. Unless `exclude_generic`, any variable meeting the
    /// store for the first time also picks up the generic assumption.
    ///
    /// Returns the number of facts recorded — 0 for an empty assumption, which
    /// is a no-op rather than an error.
    pub fn add_assumption(&mut self, tree: &Expr, exclude_generic: bool) -> u32 {
        self.apply(tree, |s, t| s.add_sub(t, exclude_generic))
    }

    /// File a generic assumption: one written in terms of `x`, standing for any
    /// variable that has no assumptions of its own.
    pub fn add_generic_assumption(&mut self, tree: &Expr) -> u32 {
        self.apply(tree, TreeStore::add_generic_sub)
    }

    pub fn remove_assumption(&mut self, tree: &Expr) -> u32 {
        self.apply(tree, TreeStore::remove_sub)
    }

    pub fn remove_generic_assumption(&mut self, tree: &Expr) -> u32 {
        self.apply(tree, TreeStore::remove_generic_sub)
    }

    /// The shape every mutation shares: normalize the assumption, hand it to
    /// `sub`, and recompute the derived facts if anything moved. The derived
    /// facts are a function of the whole store, so any change invalidates all
    /// of them.
    fn apply(&mut self, tree: &Expr, sub: impl FnOnce(&mut Self, &Expr) -> u32) -> u32 {
        if is_js_leaf(tree) {
            return 0;
        }
        let Some(cleaned) = clean_assumptions(&normalize_assumption(tree), None) else {
            return 0;
        };
        if is_js_leaf(&cleaned) {
            return 0;
        }

        let n = sub(self, &cleaned);

        if n > 0 {
            self.derived = calculate_derived_assumptions(self);
        }

        n
    }

    fn add_sub(&mut self, tree: &Expr, exclude_generic: bool) -> u32 {
        // Split an `and` so each conjunct is filed under its own variables.
        if let Expr::And(operands) = tree {
            return operands
                .iter()
                .map(|v| self.add_sub(v, exclude_generic))
                .sum();
        }

        let vars = variables(tree);
        if vars.is_empty() {
            return 0;
        }

        let mut n_added = 0;

        if !exclude_generic {
            if let Some(generic) = self.generic.tree().cloned() {
                let generic_vars = variables(&generic);
                for v in &vars {
                    if self.by_var.get(v).is_some_and(|f| *f != Facts::Absent) {
                        continue;
                    }
                    // A variable named in the generic assumption itself is not
                    // one the generic assumption speaks about (`x < y` says
                    // nothing about `y`).
                    if v == "x" || !generic_vars.contains(v) {
                        let subs = HashMap::from([("x".to_string(), Expr::sym(v))]);
                        self.add_sub(&substitute(&generic, &subs), true);
                        n_added += 1;
                    }
                }
            }
        }

        for variable in &vars {
            let solved = solve_for(tree, variable);
            let mut new_a = solved.unwrap_or_else(|| tree.clone());

            let current_a = self.by_var.get(variable).cloned().unwrap_or_default();

            if let Facts::Tree(current) = &current_a {
                new_a = Expr::And(vec![current.clone(), new_a]);
            }

            let new_a = Facts::from(clean_assumptions(&new_a, None));

            if new_a != current_a {
                self.by_var.set(variable, new_a);
                n_added += 1;
            }
        }

        n_added
    }

    fn add_generic_sub(&mut self, tree: &Expr) -> u32 {
        if let Expr::And(operands) = tree {
            return operands.iter().map(|v| self.add_generic_sub(v)).sum();
        }

        if !variables(tree).iter().any(|v| v == "x") {
            return 0;
        }

        let solved = solve_for(tree, "x");
        let mut new_a = solved.unwrap_or_else(|| tree.clone());

        if let Facts::Tree(current) = &self.generic {
            new_a = Expr::And(vec![current.clone(), new_a]);
        }

        let new_a = Facts::from(clean_assumptions(&new_a, None));

        if new_a == self.generic {
            return 0;
        }

        self.generic = new_a;

        1
    }

    fn remove_sub(&mut self, tree: &Expr) -> u32 {
        if let Expr::And(operands) = tree {
            return operands.iter().map(|v| self.remove_sub(v)).sum();
        }

        let vars = variables(tree);
        if vars.is_empty() {
            return 0;
        }

        let mut n_removed = 0;

        for variable in &vars {
            let solved = solve_for(tree, variable);

            let Some(current) = self.by_var.get(variable).and_then(Facts::tree).cloned() else {
                continue;
            };

            let Some(result) = remove_from(&current, tree, solved.as_ref()) else {
                continue;
            };

            n_removed += 1;
            self.by_var.set(variable, result);
        }

        n_removed
    }

    fn remove_generic_sub(&mut self, tree: &Expr) -> u32 {
        if let Expr::And(operands) = tree {
            return operands.iter().map(|v| self.remove_generic_sub(v)).sum();
        }

        if !variables(tree).iter().any(|v| v == "x") {
            return 0;
        }

        let Some(current) = self.generic.tree().cloned() else {
            return 0;
        };

        let solved = solve_for(tree, "x");

        let Some(result) = remove_from(&current, tree, solved.as_ref()) else {
            return 0;
        };

        self.generic = result;

        1
    }
}

/// Restate `tree` with `variable` alone on the left.
///
/// No assumptions are consulted: the store calls this while deciding what to
/// file, so a conclusion drawn from the facts already on file would depend on
/// insertion order.
fn solve_for(tree: &Expr, variable: &str) -> Option<Expr> {
    if is_js_leaf(tree) {
        return None;
    }
    crate::grade::solve_linear(tree, variable, &Assumptions::new())
}
