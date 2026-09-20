//! The mutable `Assumptions` handle (item 15) and the assumption-adjacent
//! builders: finite-field evaluation and discrete-infinite-set construction.

use super::Expression;
use math_expressions::assumptions::tree_store::{Facts, VarMap};
use math_expressions::expr::serde::{to_js, try_from_js};
use math_expressions::{
    canonicalize, create_discrete_infinite_set, simplify_with as rust_simplify_with,
    solve_linear as rust_solve_linear, Assumptions, EqOptions, Expr, TextToAst, TextToAstOptions,
};
use wasm_bindgen::prelude::*;

/// Parse a JS-tree AST, or `None` if the core cannot read it.
fn read(tree_json: &str) -> Option<Expr> {
    try_from_js(&serde_json::from_str::<serde_json::Value>(tree_json).ok()?).ok()
}

fn facts_json(facts: &Facts) -> serde_json::Value {
    match facts {
        Facts::Absent => serde_json::Value::Null,
        Facts::Empty => serde_json::json!([]),
        Facts::Tree(t) => to_js(t),
    }
}

fn var_map_json(map: &VarMap) -> String {
    serde_json::Value::Object(
        map.iter()
            .map(|(k, v)| (k.clone(), facts_json(v)))
            .collect(),
    )
    .to_string()
}

/// Evaluate `e` in ℤ/`modulus`ℤ with real integer bindings (item 16). Returns
/// the possible residues, or `undefined` when the field can't represent it.
#[wasm_bindgen]
pub fn finite_field_evaluate(
    e: &Expression,
    vars: Vec<String>,
    values: Vec<i32>,
    modulus: i32,
) -> Option<Vec<i32>> {
    if vars.len() != values.len() {
        return None;
    }
    let bindings: std::collections::HashMap<String, i64> = vars
        .into_iter()
        .zip(values.into_iter().map(i64::from))
        .collect();
    math_expressions::finite_field_evaluate(&e.0, &bindings, i64::from(modulus))
        .map(|v| v.into_iter().map(|x| x as i32).collect())
}

/// A mutable assumptions set. Relations are given in text syntax
/// (`"x > 0"`, `"n elementof Z"`). Port of the JS `Assumptions` context.
#[wasm_bindgen(js_name = Assumptions)]
pub struct WasmAssumptions(Assumptions);

#[wasm_bindgen(js_class = Assumptions)]
impl WasmAssumptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmAssumptions {
        WasmAssumptions(Assumptions::new())
    }

    /// Add a relation (or an `and` of relations). Returns `false` if it fails
    /// to parse.
    pub fn add(&mut self, relation: &str) -> bool {
        match TextToAst::new(TextToAstOptions::default()).convert(relation) {
            Ok(e) => {
                self.0.add(&e);
                true
            }
            Err(_) => false,
        }
    }

    /// Remove a previously-added relation.
    pub fn remove(&mut self, relation: &str) {
        if let Ok(e) = TextToAst::new(TextToAstOptions::default()).convert(relation) {
            self.0.remove(&e);
        }
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    // ---- the assumption *trees* (JSON in / JSON out) ----
    //
    // The methods above answer predicates from a relation's text spelling. The
    // ones below drive the store's other half — the per-variable facts the JS
    // API hands *back* as trees — and take an AST, because that is what a
    // caller holding `me.from(...)` has and because a text round trip would
    // re-parse a tree the caller built by hand.
    //
    // An unreadable tree is a no-op rather than an error: these run behind
    // `me.add_assumption(...)`, whose legacy contract is to count what it filed
    // and report 0 when it filed nothing.

    /// File an assumption. Unless `exclude_generic`, a variable meeting the
    /// store for the first time also picks up the generic assumption. Returns
    /// the number of facts recorded.
    pub fn add_ast(&mut self, tree_json: &str, exclude_generic: bool) -> u32 {
        match read(tree_json) {
            Some(e) => self.0.trees_mut().add_assumption(&e, exclude_generic),
            None => 0,
        }
    }

    /// File a generic assumption: one written in terms of `x`, standing for
    /// every variable with no assumptions of its own.
    pub fn add_generic_ast(&mut self, tree_json: &str) -> u32 {
        match read(tree_json) {
            Some(e) => self.0.trees_mut().add_generic_assumption(&e),
            None => 0,
        }
    }

    pub fn remove_ast(&mut self, tree_json: &str) -> u32 {
        match read(tree_json) {
            Some(e) => self.0.trees_mut().remove_assumption(&e),
            None => 0,
        }
    }

    pub fn remove_generic_ast(&mut self, tree_json: &str) -> u32 {
        match read(tree_json) {
            Some(e) => self.0.trees_mut().remove_generic_assumption(&e),
            None => 0,
        }
    }

    /// Everything known about `query_json`, as a tree stating it.
    ///
    /// The query is either a variable name, an array holding an array of names
    /// (`[["a","b"]]`), or an expression — and the first two are not
    /// expressions, so the shape is decided here rather than by the caller.
    /// `undefined` when nothing is known.
    pub fn get_ast(
        &self,
        query_json: &str,
        exclude_variables: Vec<String>,
        omit_derived: bool,
    ) -> Option<String> {
        let value: serde_json::Value = serde_json::from_str(query_json).ok()?;
        let trees = self.0.trees();

        let vars: Option<Vec<String>> = match &value {
            serde_json::Value::String(s) => Some(vec![s.clone()]),
            serde_json::Value::Array(items) => match items.first() {
                Some(serde_json::Value::Array(names)) => Some(
                    names
                        .iter()
                        .filter_map(|n| n.as_str().map(str::to_string))
                        .collect(),
                ),
                _ => None,
            },
            _ => return None,
        };

        let result = match vars {
            Some(vars) => trees.facts_for_variables(&vars, &exclude_variables, omit_derived),
            None => trees.assumptions_for_tree(&try_from_js(&value).ok()?, &exclude_variables),
        };
        result.as_ref().map(|e| to_js(e).to_string())
    }

    /// The per-variable facts, as `{variable: tree}` — the legacy `byvar`,
    /// `derived` and `generic` inspection surface. A variable recorded with no
    /// fact is `null` (the legacy `undefined`), one met with nothing known is
    /// `[]`.
    pub fn byvar_ast(&self) -> String {
        var_map_json(self.0.trees().by_var())
    }

    pub fn derived_ast(&self) -> String {
        var_map_json(self.0.trees().derived())
    }

    pub fn generic_ast(&self) -> String {
        facts_json(self.0.trees().generic()).to_string()
    }

    /// Simplify `expr` under these assumptions.
    pub fn simplify(&self, expr: &Expression) -> Expression {
        expr.derive(rust_simplify_with(&expr.0, &self.0))
    }

    /// Restate a relation with `variable` alone on the left (`3x+4 = 2` under
    /// `x` becomes `x = -2/3`). `undefined` when the relation is not linear in
    /// `variable`, when the coefficient cannot be shown nonzero, or — for an
    /// inequality — when the coefficient's sign is unknown, since that is what
    /// decides whether the direction flips.
    ///
    /// On the store rather than on `Expression` because the assumptions are the
    /// argument that matters: `2uv-v = 3u+q` has no answer in `u` until `v < 0`
    /// makes `2v-3` provably nonzero. The free [`solve_linear_ast`] is the
    /// assumption-*free* entry point the store uses while deciding what to file,
    /// and must stay that way — see its note on insertion order.
    ///
    /// [`solve_linear_ast`]: crate::tree_ops::solve_linear_ast
    pub fn solve_linear(&self, expr: &Expression, variable: &str) -> Option<Expression> {
        rust_solve_linear(&expr.0, variable, &self.0).map(|e| expr.derive(e))
    }

    // The eight three-valued predicates (`true` / `false` / `undefined`).
    pub fn is_real(&self, expr: &Expression) -> Option<bool> {
        math_expressions::is_real(&expr.0, &self.0)
    }
    pub fn is_complex(&self, expr: &Expression) -> Option<bool> {
        math_expressions::is_complex(&expr.0, &self.0)
    }
    pub fn is_integer(&self, expr: &Expression) -> Option<bool> {
        math_expressions::is_integer(&expr.0, &self.0)
    }
    pub fn is_nonzero(&self, expr: &Expression) -> Option<bool> {
        math_expressions::is_nonzero(&expr.0, &self.0)
    }
    pub fn is_positive(&self, expr: &Expression) -> Option<bool> {
        math_expressions::is_positive(&expr.0, &self.0)
    }
    pub fn is_negative(&self, expr: &Expression) -> Option<bool> {
        math_expressions::is_negative(&expr.0, &self.0)
    }
    pub fn is_nonnegative(&self, expr: &Expression) -> Option<bool> {
        math_expressions::is_nonnegative(&expr.0, &self.0)
    }
    pub fn is_nonpositive(&self, expr: &Expression) -> Option<bool> {
        math_expressions::is_nonpositive(&expr.0, &self.0)
    }

    // ---- the equality stage that needs an assumption store ----
    //
    // `Expression::equals` is assumption-free, and for every stage but one that
    // is the right answer. Discrete infinite sets are the exception: comparing
    // them divides by the period, so a symbolic period (`c`) means nothing
    // until `c != 0` is *stated* — the JS took the assumptions from the
    // expression's context for exactly this reason. These two live on the
    // store rather than on `Expression` because the store is the argument that
    // matters.

    /// Full `equals`, routed through the discrete-infinite-set stage with these
    /// assumptions when either side is such a set, and through the ordinary
    /// (assumption-free) chain otherwise. `options_json` is the grading-options
    /// object shared with [`Expression::equals_with_options`]; `undefined` for
    /// the defaults.
    pub fn equals_expressions(
        &self,
        a: &Expression,
        b: &Expression,
        options_json: Option<String>,
    ) -> Result<bool, JsError> {
        let mut opts = match &options_json {
            Some(json) => super::grading::eq_options_from_json(json)?,
            None => EqOptions::default(),
        };
        // Hand the live assumption store to the numeric stage so it can sample a
        // variable known to be an integer over the integers (`(-1)^n·(-1)^n = 1`
        // under `n ∈ Z`). The discrete-infinite-set stage already receives it
        // separately.
        opts.assumptions = self.0.clone();
        Ok(
            match math_expressions::equals_discrete_infinite_sets(&a.0, &b.0, &opts, &self.0) {
                Some(answer) => answer,
                None => math_expressions::equals(&a.0, &b.0, &opts),
            },
        )
    }

    /// How much of the discrete infinite set `a` is matched by `b` (another
    /// set, or a list ending in `…`): 1 for equal, 0 for no match, and — under
    /// `match_partial` — the fraction of residue classes covered, which is the
    /// partial-credit grading signal a boolean `equals` cannot express.
    ///
    /// Both sides are canonicalized first, mirroring what the `equals` chain
    /// does before it reaches this stage; without it a set built from
    /// unnormalized text would score differently here than through `equals`.
    pub fn match_discrete_infinite_set(
        &self,
        a: &Expression,
        b: &Expression,
        match_partial: bool,
    ) -> f64 {
        math_expressions::match_discrete_infinite(
            &canonicalize(&a.0),
            &canonicalize(&b.0),
            &EqOptions::default(),
            match_partial,
            &self.0,
        )
    }
}

impl Default for WasmAssumptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a discrete infinite set (periodic solution set) from offsets and
/// periods expressions (either may be a comma list). `undefined` on
/// mismatched list lengths, or on index bounds that do not parse.
///
/// The bounds arrive as JS-tree JSON rather than as `Expression` handles
/// because they are optional and wasm-bindgen has no by-reference
/// `Option<&Expression>`: the owned `Option<Expression>` it does support would
/// move the handle out of the caller's wrapper, and the JS side shares handles
/// between wrappers (the atom cache), so that would null a live expression.
#[wasm_bindgen]
pub fn discrete_infinite_set(
    offsets: &Expression,
    periods: &Expression,
    min_index_json: Option<String>,
    max_index_json: Option<String>,
) -> Option<Expression> {
    let min = match &min_index_json {
        Some(json) => Some(read(json)?),
        None => None,
    };
    let max = match &max_index_json {
        Some(json) => Some(read(json)?),
        None => None,
    };
    create_discrete_infinite_set(&offsets.0, &periods.0, min.as_ref(), max.as_ref())
        .map(|e| offsets.derive(e))
}
