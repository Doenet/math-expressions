//! The per-variable assumption store.
//!
//! Storage mirrors the JS `initialize_assumptions` shape: per-variable facts
//! (`by_var`) added via [`Assumptions::add`], retrieved with
//! [`Assumptions::get`], removed with [`Assumptions::remove`]. A fact is a
//! canonical relation `Expr` (`x > 0`, `n ∈ Z`, `x ≠ 0`, `x = 3`, optionally
//! wrapped in `not`), with chains split on `And` and on a negated `Or`.

use crate::expr::Expr;
use crate::normalize::canonicalize;
use std::collections::HashMap;

/// Per-variable assumption store, plus generic assumptions: patterns in the
/// designated variable `x` that apply to every variable with no specific
/// facts (JS `add_generic_assumption`).
#[derive(Debug, Clone, Default)]
pub struct Assumptions {
    by_var: HashMap<String, Vec<Expr>>,
    generic: Vec<Expr>,
    trees: super::TreeStore,
}

impl Assumptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// The same facts as *trees*, for the callers that ask what is known about
    /// a variable rather than whether an expression is real. Filed separately
    /// (see [`TreeStore`](super::TreeStore)): the predicate engine wants one
    /// canonical relation per fact, this wants the fact solved for the variable
    /// it was filed under.
    pub fn trees(&self) -> &super::TreeStore {
        &self.trees
    }

    pub fn trees_mut(&mut self) -> &mut super::TreeStore {
        &mut self.trees
    }

    /// Add an assumption (a relation, or an `And` of relations, in any parse
    /// form). Each conjunct is canonicalized and filed under every variable it
    /// mentions.
    pub fn add(&mut self, assumption: &Expr) {
        let canon = canonicalize(assumption);
        for conjunct in conjuncts(&canon) {
            let mut vars = std::collections::BTreeSet::new();
            crate::eval_numeric::complex::free_symbols(&conjunct, &mut vars);
            for v in vars {
                self.by_var.entry(v).or_default().push(conjunct.clone());
            }
        }
    }

    /// All facts mentioning `var`, combined with `And` (or the single fact),
    /// mirroring `get_assumptions`. `None` when nothing is known.
    pub fn get(&self, var: &str) -> Option<Expr> {
        let facts = self.by_var.get(var)?;
        match facts.as_slice() {
            [] => None,
            [one] => Some(one.clone()),
            many => Some(Expr::And(many.to_vec())),
        }
    }

    /// Remove a previously-added assumption (structural equality on the
    /// canonical form), from every variable it was filed under.
    pub fn remove(&mut self, assumption: &Expr) {
        let canon = canonicalize(assumption);
        for conjunct in conjuncts(&canon) {
            for facts in self.by_var.values_mut() {
                facts.retain(|f| *f != conjunct);
            }
        }
        self.by_var.retain(|_, v| !v.is_empty());
    }

    pub fn clear(&mut self) {
        self.by_var.clear();
        self.generic.clear();
        self.trees.clear();
    }

    /// No facts stored at all?
    pub fn is_empty(&self) -> bool {
        self.by_var.is_empty() && self.generic.is_empty() && self.trees.is_empty()
    }

    /// Add a generic assumption: a pattern in the variable `x` applied to any
    /// variable without specific facts (`x > 0` ⇒ every unassumed variable is
    /// positive). Conjuncts not mentioning `x` are ignored (JS parity).
    pub fn add_generic(&mut self, assumption: &Expr) {
        let canon = canonicalize(assumption);
        for conjunct in conjuncts(&canon) {
            let mut vars = std::collections::BTreeSet::new();
            crate::eval_numeric::complex::free_symbols(&conjunct, &mut vars);
            if vars.contains("x") {
                self.generic.push(conjunct.clone());
            }
        }
    }

    /// Remove a generic assumption added with [`Assumptions::add_generic`].
    pub fn remove_generic(&mut self, assumption: &Expr) {
        let canon = canonicalize(assumption);
        for conjunct in conjuncts(&canon) {
            self.generic.retain(|f| *f != conjunct);
        }
    }

    /// The facts in effect for `var`: its specific facts, or — when none exist
    /// — the generic patterns with `x` substituted by `var` (unless the
    /// pattern itself mentions `var` as a different symbol, JS parity).
    pub(super) fn facts_for(&self, var: &str) -> Vec<Expr> {
        if let Some(facts) = self.by_var.get(var) {
            return facts.clone();
        }
        if self.generic.is_empty() {
            return Vec::new();
        }
        let subs = HashMap::from([("x".to_string(), Expr::sym(var))]);
        self.generic
            .iter()
            .filter(|f| {
                if var == "x" {
                    return true;
                }
                let mut vs = std::collections::BTreeSet::new();
                crate::eval_numeric::complex::free_symbols(f, &mut vs);
                !vs.contains(var)
            })
            .map(|f| canonicalize(&crate::ops::substitute(f, &subs)))
            .collect()
    }
}

/// The conjuncts of an assumption. `and` splits, and — by De Morgan — so does
/// a negated `or`: `not(p or q)` is filed as the two facts `not p` and
/// `not q`, which is what lets the reader see a plain (negated) relation
/// instead of a compound it has to give up on.
fn conjuncts(e: &Expr) -> Vec<Expr> {
    match e {
        Expr::And(xs) => xs.iter().flat_map(conjuncts).collect(),
        Expr::Not(inner) => match &**inner {
            Expr::Or(xs) => xs
                .iter()
                .flat_map(|x| conjuncts(&Expr::Not(Box::new(x.clone()))))
                .collect(),
            _ => vec![e.clone()],
        },
        other => vec![other.clone()],
    }
}
