//! WebAssembly / JavaScript bindings (PORTING_PLAN.md §13).
//!
//! A single opaque `Expression` handle owns a parsed tree; JS calls parse once
//! and then invokes methods. Read-only methods take `&self`; transforming
//! methods return a fresh `Expression`. Only primitives and strings cross the
//! boundary, so there is no tree-serialisation overhead. This module is
//! wasm32-only; the rest of the crate is a normal Rust library.

use crate::{
    derivative as rust_derivative, equals as rust_equals, evaluate_to_constant as rust_evc,
    expand as rust_expand, ops, simplify as rust_simplify, simplify_with as rust_simplify_with,
    to_latex, to_text, Assumptions, EqOptions, Expr, LatexToAst, LatexToAstOptions, TextToAst,
    TextToAstOptions,
};
use crate::{constants_to_floats, round_numbers_to_decimals, round_numbers_to_precision};
use crate::{create_discrete_infinite_set, evaluate_numbers, reduce_rational};
use wasm_bindgen::prelude::*;

/// An opaque handle to a parsed math expression.
#[wasm_bindgen]
pub struct Expression(Expr);

/// Parse text syntax (e.g. `"sin^2 x + cos^2 x"`).
#[wasm_bindgen]
pub fn parse_text(s: &str) -> Result<Expression, JsError> {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .map(Expression)
        .map_err(|e| JsError::new(&e.to_string()))
}

/// Parse LaTeX syntax (e.g. `"\\frac{1}{2}"`).
#[wasm_bindgen]
pub fn parse_latex(s: &str) -> Result<Expression, JsError> {
    LatexToAst::new(LatexToAstOptions::default())
        .convert(s)
        .map(Expression)
        .map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen]
impl Expression {
    /// Render back to text syntax.
    pub fn to_text(&self) -> String {
        to_text(&self.0, &Default::default())
    }

    /// Render to LaTeX.
    pub fn to_latex(&self) -> String {
        to_latex(&self.0, &Default::default())
    }

    /// The parse tree serialised to the JS `Tree` JSON shape
    /// (e.g. `["+", 1, "x", 3]`), so it lines up with the JS library's
    /// `expr.tree`. Intended for inspection/tooling (§13 `to_json`).
    pub fn tree_json(&self) -> String {
        crate::js_tree::to_js(&self.0).to_string()
    }

    /// Mathematical equality with another expression.
    pub fn equals(&self, other: &Expression) -> bool {
        rust_equals(&self.0, &other.0, &EqOptions::default())
    }

    /// Canonical simplification.
    pub fn simplify(&self) -> Expression {
        Expression(rust_simplify(&self.0))
    }

    /// Simplify under the given `assumptions` — each a relation in text syntax
    /// (e.g. `"x > 0"`, `"n elementof Z"`). Assumptions that fail to parse are
    /// ignored. With an empty list this equals [`Self::simplify`].
    pub fn simplify_with_assumptions(&self, assumptions: Vec<String>) -> Expression {
        let mut a = Assumptions::new();
        for s in &assumptions {
            if let Ok(e) = TextToAst::new(TextToAstOptions::default()).convert(s) {
                a.add(&e);
            }
        }
        Expression(rust_simplify_with(&self.0, &a))
    }

    /// Distribute products and powers of sums.
    pub fn expand(&self) -> Expression {
        Expression(rust_expand(&self.0))
    }

    /// Symbolic derivative with respect to `var`.
    pub fn derivative(&self, var: &str) -> Expression {
        Expression(rust_derivative(&self.0, var))
    }

    /// The free variable names, in first-appearance order.
    pub fn variables(&self) -> Vec<String> {
        ops::variables(&self.0)
    }

    /// Evaluate a closed expression to a real number, or `undefined` (JS side)
    /// when it has free variables, is non-finite, or is not purely real —
    /// preserving the upstream null-vs-value distinction. The imaginary
    /// tolerance is *relative* to the magnitude, since complex-arithmetic float
    /// noise scales with it (`1e8·e^(iπ)` has im ≈ 1e-8 yet is real). Complex
    /// results are a follow-up (they can be added without breaking this
    /// signature).
    pub fn evaluate_to_constant(&self) -> Option<f64> {
        let v = rust_evc(&self.0)?;
        let tol = 1e-10 * v.re.abs().max(1.0);
        (v.im.abs() <= tol).then_some(v.re)
    }

    /// Evaluate a closed expression to a complex constant, returned as the pair
    /// `[re, im]`, or `undefined` (JS side) when it has free variables or is
    /// non-finite. Unlike [`Self::evaluate_to_constant`], this keeps a non-real
    /// result instead of discarding it.
    pub fn evaluate_to_complex(&self) -> Option<Vec<f64>> {
        let v = rust_evc(&self.0)?;
        (v.re.is_finite() && v.im.is_finite()).then(|| vec![v.re, v.im])
    }

    /// Replace `pi` and `e` with their floating-point values.
    pub fn constants_to_floats(&self) -> Expression {
        Expression(constants_to_floats(&self.0))
    }

    /// Round every number to `decimals` decimal places.
    pub fn round_numbers_to_decimals(&self, decimals: i32) -> Expression {
        Expression(round_numbers_to_decimals(&self.0, decimals))
    }

    /// Round every number to `sig_figs` significant figures.
    pub fn round_numbers_to_precision(&self, sig_figs: i32) -> Expression {
        Expression(round_numbers_to_precision(&self.0, sig_figs))
    }

    /// Fold numeric subexpressions exactly (`4+x-2` → `x+2`).
    pub fn evaluate_numbers(&self) -> Expression {
        Expression(evaluate_numbers(&self.0))
    }

    /// Cancel common polynomial factors in fractions
    /// (`(x^2-1)/(x-1)` → `x+1`).
    pub fn reduce_rational(&self) -> Expression {
        Expression(reduce_rational(&self.0))
    }

    /// Replace `var` with `value` everywhere (no simplification).
    pub fn substitute_var(&self, var: &str, value: &Expression) -> Expression {
        let map = std::collections::HashMap::from([(var.to_string(), value.0.clone())]);
        Expression(ops::substitute(&self.0, &map))
    }

    /// Evaluate at real bindings given as parallel arrays; `undefined` on an
    /// unbound variable, non-finite, or non-real result.
    pub fn evaluate(&self, vars: Vec<String>, values: Vec<f64>) -> Option<f64> {
        if vars.len() != values.len() {
            return None;
        }
        let bindings: std::collections::HashMap<String, f64> =
            vars.into_iter().zip(values).collect();
        let v = ops::evaluate(&self.0, &bindings)?;
        (v.im.abs() <= 1e-10 * v.re.abs().max(1.0)).then_some(v.re)
    }

    /// The applied function names, first-appearance order.
    pub fn functions(&self) -> Vec<String> {
        ops::functions(&self.0)
    }

    // ---- arithmetic builders (JS `add`/`subtract`/`multiply`/`divide`/`pow`) ----

    pub fn add(&self, other: &Expression) -> Expression {
        Expression(Expr::Add(vec![self.0.clone(), other.0.clone()]))
    }
    pub fn subtract(&self, other: &Expression) -> Expression {
        Expression(Expr::Add(vec![
            self.0.clone(),
            Expr::Neg(Box::new(other.0.clone())),
        ]))
    }
    pub fn multiply(&self, other: &Expression) -> Expression {
        Expression(Expr::Mul(vec![self.0.clone(), other.0.clone()]))
    }
    pub fn divide(&self, other: &Expression) -> Expression {
        Expression(Expr::Div(
            Box::new(self.0.clone()),
            Box::new(other.0.clone()),
        ))
    }
    pub fn pow(&self, other: &Expression) -> Expression {
        Expression(Expr::Pow(
            Box::new(self.0.clone()),
            Box::new(other.0.clone()),
        ))
    }
}

/// Build a discrete infinite set (periodic solution set) from offsets and
/// periods expressions (either may be a comma list). `undefined` on
/// mismatched list lengths.
#[wasm_bindgen]
pub fn discrete_infinite_set(offsets: &Expression, periods: &Expression) -> Option<Expression> {
    create_discrete_infinite_set(&offsets.0, &periods.0, None, None).map(Expression)
}
