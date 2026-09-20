//! The assumption store as a source of *trees*.
//!
//! [`super::Assumptions`] answers predicates (`is_real(x+y)`); this answers the
//! other question the JS API asks — "what is known about `x`?", as an AST with
//! `x` on the left. That needs its own storage: a fact is filed once per
//! variable it mentions, *solved for* that variable where possible (`a + b < 1`
//! is filed under `a` as `a < 1 - b`), the consequences of combining the stored
//! facts are recomputed after every change ([`super::derive`]), and a generic
//! assumption written in terms of `x` stands in for any variable with no facts
//! of its own.
//!
//! Ported from the legacy `lib/assumptions/assumptions.js`.

mod mutate;

use super::clean::{clean_assumptions, filter_assumptions_from_tree};
use super::derive::get_assumptions_for_expr;
use crate::expr::Expr;
use crate::ops::{substitute, variables};
use std::collections::HashMap;

/// What the store holds for one variable. The three states are the legacy
/// object's three values, and they are distinguishable there: a variable that
/// has met the store but has nothing known about it ([`Facts::Empty`]) does
/// *not* pick up the generic assumption, while one the store has never seen
/// does.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Facts {
    /// Recorded, but with no fact — the legacy `undefined` value.
    #[default]
    Absent,
    /// Met, with nothing known: the legacy `[]`.
    Empty,
    Tree(Expr),
}

impl From<Option<Expr>> for Facts {
    fn from(value: Option<Expr>) -> Self {
        match value {
            Some(t) => Facts::Tree(t),
            None => Facts::Absent,
        }
    }
}

impl Facts {
    /// The fact itself, if there is one.
    pub fn tree(&self) -> Option<&Expr> {
        match self {
            Facts::Tree(t) => Some(t),
            _ => None,
        }
    }
}

/// Facts by variable, in the order the variables were first filed.
///
/// Insertion-ordered rather than hashed because the legacy object is iterated
/// in that order when the derived facts are recomputed, and the order reaches
/// the shape of the conjunction that pass builds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VarMap(Vec<(String, Facts)>);

impl VarMap {
    pub fn get(&self, var: &str) -> Option<&Facts> {
        self.0.iter().find(|(k, _)| k == var).map(|(_, v)| v)
    }

    pub(crate) fn set(&mut self, var: &str, facts: Facts) {
        match self.0.iter_mut().find(|(k, _)| k == var) {
            Some(slot) => slot.1 = facts,
            None => self.0.push((var.to_string(), facts)),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &(String, Facts)> {
        self.0.iter()
    }
}

/// Assumption trees, filed per variable.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeStore {
    by_var: VarMap,
    derived: VarMap,
    generic: Facts,
}

impl Default for TreeStore {
    fn default() -> Self {
        TreeStore {
            by_var: VarMap::default(),
            derived: VarMap::default(),
            // "met, nothing known" — the legacy initial `[]`, which the
            // generic-assumption paths test with `.length`.
            generic: Facts::Empty,
        }
    }
}

impl TreeStore {
    pub fn clear(&mut self) {
        *self = TreeStore::default();
    }

    pub fn is_empty(&self) -> bool {
        *self == TreeStore::default()
    }

    pub fn by_var(&self) -> &VarMap {
        &self.by_var
    }

    pub fn derived(&self) -> &VarMap {
        &self.derived
    }

    pub fn generic(&self) -> &Facts {
        &self.generic
    }

    /// The assumptions on `expr`, restated in terms of `expr` itself.
    pub fn assumptions_for_tree(&self, expr: &Expr, exclude_variables: &[String]) -> Option<Expr> {
        get_assumptions_for_expr(self, expr, exclude_variables)
    }

    /// The assumptions on a list of variables, answered from the stored facts
    /// — or, for a variable with none of its own, from the generic assumption.
    ///
    /// `None` when nothing is known.
    pub fn facts_for_variables(
        &self,
        vars: &[String],
        exclude_variables: &[String],
        omit_derived: bool,
    ) -> Option<Expr> {
        let mut collected: Vec<Expr> = Vec::new();

        for v in vars {
            let by_var = self.by_var.get(v);
            let derived = self.derived.get(v);
            // "Recorded at all", which `Facts::Empty` counts as: a variable the
            // store has met keeps the generic assumption off.
            let recorded = matches!(by_var, Some(Facts::Empty | Facts::Tree(_)))
                || matches!(derived, Some(Facts::Empty | Facts::Tree(_)));

            if recorded {
                if let Some(t) = by_var.and_then(Facts::tree) {
                    if let Some(kept) = filter_assumptions_from_tree(t, exclude_variables) {
                        collected.push(kept);
                    }
                }
                if !omit_derived {
                    if let Some(t) = derived.and_then(Facts::tree) {
                        if let Some(kept) = filter_assumptions_from_tree(t, exclude_variables) {
                            collected.push(kept);
                        }
                    }
                }
            } else if let Some(generic) = self.generic.tree() {
                // The generic assumption is written in terms of `x`.
                // Substituting a different variable into it would be wrong if
                // that variable is named in the generic assumption itself
                // (`x < y` says nothing about `y`).
                if v == "x" || !variables(generic).contains(v) {
                    let subs = HashMap::from([("x".to_string(), Expr::sym(v))]);
                    collected.push(substitute(generic, &subs));
                }
            }
        }

        let combined = match collected.len() {
            0 => return None,
            1 => collected.into_iter().next().expect("len 1"),
            _ => Expr::And(collected),
        };
        clean_assumptions(&combined, None)
    }
}
