//! The mutable `Assumptions` handle (item 15) and the assumption-adjacent
//! builders: finite-field evaluation and discrete-infinite-set construction.

use super::Expression;
use math_expressions::{
    create_discrete_infinite_set, simplify_with as rust_simplify_with, Assumptions, TextToAst,
    TextToAstOptions,
};
use wasm_bindgen::prelude::*;

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

    /// Simplify `expr` under these assumptions.
    pub fn simplify(&self, expr: &Expression) -> Expression {
        Expression(rust_simplify_with(&expr.0, &self.0))
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
}

impl Default for WasmAssumptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a discrete infinite set (periodic solution set) from offsets and
/// periods expressions (either may be a comma list). `undefined` on
/// mismatched list lengths.
#[wasm_bindgen]
pub fn discrete_infinite_set(offsets: &Expression, periods: &Expression) -> Option<Expression> {
    create_discrete_infinite_set(&offsets.0, &periods.0, None, None).map(Expression)
}
