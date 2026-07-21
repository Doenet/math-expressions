//! Transformation passes: simplification variants, factoring, number cleanup,
//! units, and the normalization rewrites (items 8, 9, 13, 14, 17, 19).

use super::Expression;
use math_expressions::reduce_rational;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Expression {
    // ---- simplification variants (items 9, 19, 8) ----

    /// Logical simplification: De Morgan / not-pushdown (JS `simplify_logical`).
    pub fn simplify_logical(&self) -> Expression {
        Expression(math_expressions::simplify_logical(&self.0, &math_expressions::Assumptions::new()))
    }

    /// Collect like terms and factors. Backed by the canonical simplifier
    /// (JS `collect_like_terms_factors`).
    pub fn collect_like_terms_factors(&self) -> Expression {
        Expression(math_expressions::simplify(&self.0))
    }

    /// Simplify rational expressions by cancelling common factors — an alias of
    /// [`Self::reduce_rational`] (JS `simplify_ratios`).
    pub fn simplify_ratios(&self) -> Expression {
        Expression(reduce_rational(&self.0))
    }

    /// Factor a univariate polynomial over ℚ (item 8).
    pub fn factor(&self) -> Expression {
        Expression(math_expressions::factor(&self.0))
    }

    // ---- number cleanup (item 17) ----

    /// Replace every number smaller than `tolerance` in magnitude with 0
    /// (JS `set_small_zero`).
    pub fn set_small_zero(&self, tolerance: f64) -> Expression {
        Expression(math_expressions::set_small_zero(&self.0, tolerance))
    }

    // ---- units (item 14) ----

    /// Strip unit annotations. With `scale_based_on_unit`, scaling units are
    /// applied (`50%` → `1/2`); otherwise the bare value is kept.
    pub fn remove_units(&self, scale_based_on_unit: bool) -> Expression {
        Expression(math_expressions::remove_units(&self.0, scale_based_on_unit))
    }

    /// Rewrite the scaling units `%`, `deg`, `$` into plain arithmetic
    /// (JS `remove_scaling_units`).
    pub fn remove_scaling_units(&self) -> Expression {
        Expression(math_expressions::remove_scaling_units(&self.0))
    }

    /// Wrap this expression in `unit` (JS `add_unit`).
    pub fn add_unit(&self, unit: &str) -> Expression {
        Expression(math_expressions::add_unit(&self.0, unit))
    }

    // ---- normalization passes (item 13) ----

    /// Fold alternate function spellings to canonical (`arcsin` → `asin`).
    pub fn normalize_function_names(&self) -> Expression {
        Expression(math_expressions::normalize_function_names(&self.0))
    }

    /// Reinterpret tuples as vectors (JS `tuples_to_vectors`).
    pub fn tuples_to_vectors(&self) -> Expression {
        Expression(math_expressions::tuples_to_vectors(&self.0))
    }

    /// Reinterpret alt-vectors as vectors (JS `altvectors_to_vectors`).
    pub fn altvectors_to_vectors(&self) -> Expression {
        Expression(math_expressions::altvectors_to_vectors(&self.0))
    }

    /// Collapse subscripts into string symbols: `x_1` → the symbol `x_1`.
    pub fn subscripts_to_strings(&self) -> Expression {
        Expression(math_expressions::subscripts_to_strings(&self.0))
    }

    /// Inverse of [`Self::subscripts_to_strings`].
    pub fn strings_to_subscripts(&self) -> Expression {
        Expression(math_expressions::strings_to_subscripts(&self.0))
    }

    /// Convert 2-element tuples/arrays into interval notation (JS `to_intervals`).
    pub fn to_intervals(&self) -> Expression {
        Expression(math_expressions::to_intervals(&self.0))
    }
}
