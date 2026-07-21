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

    /// Does the expression contain any ± (plus-minus) operator?
    pub fn contains_pm(&self) -> bool {
        crate::pm::contains_pm(&self.0)
    }

    /// The number of ± (plus-minus) operators in the expression.
    pub fn count_pm(&self) -> usize {
        crate::pm::count_pm(&self.0)
    }

    /// Enumerate all `2^n` sign assignments of the ± operators (each `±x`
    /// becomes `x` or `-x`). Errors if there are too many ± operators to expand.
    pub fn expand_pm_signs(&self) -> Result<Vec<Expression>, JsError> {
        crate::pm::expand_pm_signs(&self.0)
            .map(|v| v.into_iter().map(Expression).collect())
            .map_err(|e| JsError::new(&e.to_string()))
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

    /// Put the expression over a single common denominator and reduce it to
    /// lowest terms (FULL_SIMPLIFY S2). Non-rational subtrees (`sin x`, `√x`,
    /// …) are held fixed as opaque kernels, so `1/sin(x) + 1/sin(x)` becomes
    /// `2/sin(x)` and `1/(x+1) + 1/(x-1)` becomes `2x/(x^2-1)`.
    pub fn together(&self) -> Expression {
        Expression(crate::ratform::together(&self.0))
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

    /// Remainder `self mod other` (JS `mod`).
    #[wasm_bindgen(js_name = "mod")]
    pub fn modulo(&self, other: &Expression) -> Expression {
        Expression(Expr::OtherOp(
            crate::sym::Sym::new("mod"),
            vec![self.0.clone(), other.0.clone()],
        ))
    }

    /// A structural copy (JS `copy`).
    pub fn copy(&self) -> Expression {
        Expression(self.0.clone())
    }
}

// ================= WHATS_LEFT A.2 / A.3: newly exposed surface =================

#[wasm_bindgen]
impl Expression {
    // ---- equality variants (items 10, 11) ----

    /// Numerical equality by sampling real points only, gated on both sides
    /// being analytic (JS `equalsViaReal`).
    pub fn equals_via_real(&self, other: &Expression) -> bool {
        crate::equals_via_real(&self.0, &other.0, &EqOptions::default())
    }

    /// Equality with grading options as JSON. Keys (all optional): numbers —
    /// `relativeTolerance`, `absoluteTolerance`, `toleranceForZero`,
    /// `allowedErrorInNumbers`; bools — `includeErrorInNumberExponents`,
    /// `allowedErrorIsAbsolute`, `allowBlanks`. So `3.14 == pi` becomes true
    /// with `{"allowedErrorInNumbers": 0.01}`.
    pub fn equals_with_options(&self, other: &Expression, options_json: &str) -> bool {
        let v: serde_json::Value = match serde_json::from_str(options_json) {
            Ok(v) => v,
            Err(_) => return self.equals(other),
        };
        let mut o = EqOptions::default();
        read_opt_f64(&v, "relativeTolerance", &mut o.relative_tolerance);
        read_opt_f64(&v, "absoluteTolerance", &mut o.absolute_tolerance);
        read_opt_f64(&v, "toleranceForZero", &mut o.tolerance_for_zero);
        read_opt_f64(&v, "allowedErrorInNumbers", &mut o.allowed_error_in_numbers);
        read_opt_bool(
            &v,
            "includeErrorInNumberExponents",
            &mut o.include_error_in_number_exponents,
        );
        read_opt_bool(&v, "allowedErrorIsAbsolute", &mut o.allowed_error_is_absolute);
        read_opt_bool(&v, "allowBlanks", &mut o.allow_blanks);
        crate::equals(&self.0, &other.0, &o)
    }

    // ---- certified zero-equivalence (FULL_SIMPLIFY S1) ----

    /// Certified test for `self ≡ 0`: `true` = provably zero, `false` =
    /// provably nonzero, `undefined` = undecided. Never certifies a wrong
    /// answer (adversarial almost-zeros return `undefined`, not `true`).
    pub fn is_zero(&self) -> Option<bool> {
        crate::exact::is_zero(&self.0, &Assumptions::new())
    }

    // ---- analyticity (item 7) ----

    /// Is this an analytic expression (only `+ - * / ^`, sequences, and
    /// analytic functions)? `allow_abs`/`allow_arg` permit those functions;
    /// `allow_relation` permits the order relations (JS `isAnalytic`).
    pub fn is_analytic(&self, allow_abs: bool, allow_arg: bool, allow_relation: bool) -> bool {
        crate::is_analytic(
            &self.0,
            &crate::AnalyticOpts {
                allow_abs,
                allow_arg,
                allow_relation,
            },
        )
    }

    // ---- simplification variants (items 9, 19, 8) ----

    /// Logical simplification: De Morgan / not-pushdown (JS `simplify_logical`).
    pub fn simplify_logical(&self) -> Expression {
        Expression(crate::simplify_logical(&self.0, &crate::Assumptions::new()))
    }

    /// Collect like terms and factors. Backed by the canonical simplifier
    /// (JS `collect_like_terms_factors`).
    pub fn collect_like_terms_factors(&self) -> Expression {
        Expression(rust_simplify(&self.0))
    }

    /// Simplify rational expressions by cancelling common factors — an alias of
    /// [`Self::reduce_rational`] (JS `simplify_ratios`).
    pub fn simplify_ratios(&self) -> Expression {
        Expression(reduce_rational(&self.0))
    }

    /// Factor a univariate polynomial over ℚ (item 8).
    pub fn factor(&self) -> Expression {
        Expression(crate::factor(&self.0))
    }

    // ---- number cleanup (item 17) ----

    /// Replace every number smaller than `tolerance` in magnitude with 0
    /// (JS `set_small_zero`).
    pub fn set_small_zero(&self, tolerance: f64) -> Expression {
        Expression(crate::set_small_zero(&self.0, tolerance))
    }

    // ---- units (item 14) ----

    /// Strip unit annotations. With `scale_based_on_unit`, scaling units are
    /// applied (`50%` → `1/2`); otherwise the bare value is kept.
    pub fn remove_units(&self, scale_based_on_unit: bool) -> Expression {
        Expression(crate::remove_units(&self.0, scale_based_on_unit))
    }

    /// Rewrite the scaling units `%`, `deg`, `$` into plain arithmetic
    /// (JS `remove_scaling_units`).
    pub fn remove_scaling_units(&self) -> Expression {
        Expression(crate::remove_scaling_units(&self.0))
    }

    /// Wrap this expression in `unit` (JS `add_unit`).
    pub fn add_unit(&self, unit: &str) -> Expression {
        Expression(crate::add_unit(&self.0, unit))
    }

    // ---- normalization passes (item 13) ----

    /// Fold alternate function spellings to canonical (`arcsin` → `asin`).
    pub fn normalize_function_names(&self) -> Expression {
        Expression(crate::normalize_function_names(&self.0))
    }

    /// Reinterpret tuples as vectors (JS `tuples_to_vectors`).
    pub fn tuples_to_vectors(&self) -> Expression {
        Expression(crate::tuples_to_vectors(&self.0))
    }

    /// Reinterpret alt-vectors as vectors (JS `altvectors_to_vectors`).
    pub fn altvectors_to_vectors(&self) -> Expression {
        Expression(crate::altvectors_to_vectors(&self.0))
    }

    /// Collapse subscripts into string symbols: `x_1` → the symbol `x_1`.
    pub fn subscripts_to_strings(&self) -> Expression {
        Expression(crate::subscripts_to_strings(&self.0))
    }

    /// Inverse of [`Self::subscripts_to_strings`].
    pub fn strings_to_subscripts(&self) -> Expression {
        Expression(crate::strings_to_subscripts(&self.0))
    }

    /// Convert 2-element tuples/arrays into interval notation (JS `to_intervals`).
    pub fn to_intervals(&self) -> Expression {
        Expression(crate::to_intervals(&self.0))
    }

    // ---- matrix / vector operations (item 12) ----

    pub fn transpose(&self) -> Expression {
        Expression(crate::transpose(&self.0))
    }
    pub fn trace(&self) -> Expression {
        Expression(crate::trace(&self.0))
    }
    pub fn matmul(&self, other: &Expression) -> Expression {
        Expression(crate::matmul(&self.0, &other.0))
    }
    /// Matrix inverse (opaque when singular or symbolic without a nonzero-det
    /// proof under the default assumptions).
    pub fn matrix_inverse(&self) -> Expression {
        Expression(crate::matrix_inverse(&self.0, &crate::Assumptions::new()))
    }
    /// Reduced row-echelon form.
    pub fn rref(&self) -> Expression {
        Expression(crate::rref(&self.0, &crate::Assumptions::new()))
    }
    /// Matrix rank, or `undefined` when not a literal matrix under the caps.
    pub fn rank(&self) -> Option<u32> {
        crate::rank(&self.0, &crate::Assumptions::new())
    }
    /// A basis for the null space as a JSON array of column vectors (each an
    /// array of text-syntax entries), or `undefined` on refusal.
    pub fn nullspace(&self) -> Option<String> {
        let basis = crate::nullspace(&self.0, &crate::Assumptions::new())?;
        let rows: Vec<serde_json::Value> = basis
            .iter()
            .map(|v| {
                let entries = match v {
                    Expr::Matrix { entries, .. } => entries.clone(),
                    other => vec![other.clone()],
                };
                serde_json::Value::Array(
                    entries
                        .iter()
                        .map(|e| serde_json::json!(to_text(e, &Default::default())))
                        .collect(),
                )
            })
            .collect();
        Some(serde_json::Value::Array(rows).to_string())
    }

    pub fn vector_add(&self, other: &Expression) -> Expression {
        Expression(crate::vector_add(&self.0, &other.0))
    }
    pub fn vector_sub(&self, other: &Expression) -> Expression {
        Expression(crate::vector_sub(&self.0, &other.0))
    }
    pub fn dot_prod(&self, other: &Expression) -> Expression {
        Expression(crate::dot_prod(&self.0, &other.0))
    }
    pub fn cross_prod(&self, other: &Expression) -> Expression {
        Expression(crate::cross_prod(&self.0, &other.0))
    }
}

fn read_opt_f64(v: &serde_json::Value, key: &str, target: &mut f64) {
    if let Some(x) = v.get(key).and_then(serde_json::Value::as_f64) {
        *target = x;
    }
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
    crate::finite_field_evaluate(&e.0, &bindings, i64::from(modulus))
        .map(|v| v.into_iter().map(|x| x as i32).collect())
}

// ================= assumptions handle (item 15) =================

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
        crate::is_real(&expr.0, &self.0)
    }
    pub fn is_complex(&self, expr: &Expression) -> Option<bool> {
        crate::is_complex(&expr.0, &self.0)
    }
    pub fn is_integer(&self, expr: &Expression) -> Option<bool> {
        crate::is_integer(&expr.0, &self.0)
    }
    pub fn is_nonzero(&self, expr: &Expression) -> Option<bool> {
        crate::is_nonzero(&expr.0, &self.0)
    }
    pub fn is_positive(&self, expr: &Expression) -> Option<bool> {
        crate::is_positive(&expr.0, &self.0)
    }
    pub fn is_negative(&self, expr: &Expression) -> Option<bool> {
        crate::is_negative(&expr.0, &self.0)
    }
    pub fn is_nonnegative(&self, expr: &Expression) -> Option<bool> {
        crate::is_nonnegative(&expr.0, &self.0)
    }
    pub fn is_nonpositive(&self, expr: &Expression) -> Option<bool> {
        crate::is_nonpositive(&expr.0, &self.0)
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

// ---- JS-tree AST boundary (Doenet interop) ----

/// Build an `Expression` from a JS-tree AST (JSON) — the port of
/// `me.fromAst`. Accepts the array AST format Doenet manipulates directly.
#[wasm_bindgen]
pub fn from_ast(tree_json: &str) -> Result<Expression, JsError> {
    let value: serde_json::Value =
        serde_json::from_str(tree_json).map_err(|e| JsError::new(&e.to_string()))?;
    crate::js_tree::try_from_js(&value)
        .map(Expression)
        .map_err(|e| JsError::new(&e))
}

/// Revive an expression serialized by [`Expression::to_serialized`] (or by
/// the JS library's `toJSON`) — the port of `me.reviver`'s object shape:
/// `{"objectType": "math-expression", "tree": ...}`.
#[wasm_bindgen]
pub fn from_serialized(json: &str) -> Result<Expression, JsError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| JsError::new(&e.to_string()))?;
    if value.get("objectType").and_then(serde_json::Value::as_str) != Some("math-expression") {
        return Err(JsError::new("not a serialized math-expression"));
    }
    let tree = value.get("tree").ok_or_else(|| JsError::new("missing tree"))?;
    crate::js_tree::try_from_js(tree)
        .map(Expression)
        .map_err(|e| JsError::new(&e))
}

/// Template match on JS-tree ASTs — the port of `me.utils.match` in its
/// default mode. Returns the bindings object as JSON (wildcard name →
/// subtree), or `undefined` if the tree does not match the pattern.
#[wasm_bindgen]
pub fn match_template(tree_json: &str, pattern_json: &str) -> Option<String> {
    let tree: serde_json::Value = serde_json::from_str(tree_json).ok()?;
    let pattern: serde_json::Value = serde_json::from_str(pattern_json).ok()?;
    crate::js_match::match_template(&tree, &pattern)
        .map(|m| serde_json::Value::Object(m).to_string())
}

/// `me.utils.flatten` on a JS-tree AST (JSON in, JSON out).
#[wasm_bindgen]
pub fn flatten_ast(tree_json: &str) -> Option<String> {
    let tree: serde_json::Value = serde_json::from_str(tree_json).ok()?;
    Some(crate::js_match::flatten_tree(&tree).to_string())
}

/// `me.utils.unflattenLeft`.
#[wasm_bindgen]
pub fn unflatten_left(tree_json: &str) -> Option<String> {
    let tree: serde_json::Value = serde_json::from_str(tree_json).ok()?;
    Some(crate::js_match::unflatten_left(&tree).to_string())
}

/// `me.utils.unflattenRight`.
#[wasm_bindgen]
pub fn unflatten_right(tree_json: &str) -> Option<String> {
    let tree: serde_json::Value = serde_json::from_str(tree_json).ok()?;
    Some(crate::js_match::unflatten_right(&tree).to_string())
}

/// Parse text with JS-style parser parameters (the port of Doenet's
/// `new me.converters.textToAstObj({...})` pattern). `options_json` keys —
/// all optional, JS spellings: `splitSymbols`, `unsplitSymbols`,
/// `appliedFunctionSymbols`, `functionSymbols`, `operatorSymbols`,
/// `allowSimplifiedFunctionApplication`, `parseLeibnizNotation`,
/// `parseScientificNotation`.
#[wasm_bindgen]
pub fn parse_text_with_options(s: &str, options_json: &str) -> Result<Expression, JsError> {
    let v: serde_json::Value =
        serde_json::from_str(options_json).map_err(|e| JsError::new(&e.to_string()))?;
    let mut o = TextToAstOptions::default();
    read_opt_bool(&v, "splitSymbols", &mut o.split_symbols);
    read_opt_bool(
        &v,
        "allowSimplifiedFunctionApplication",
        &mut o.allow_simplified_function_application,
    );
    read_opt_bool(&v, "parseLeibnizNotation", &mut o.parse_leibniz_notation);
    read_opt_bool(&v, "parseScientificNotation", &mut o.parse_scientific_notation);
    read_opt_strings(&v, "unsplitSymbols", &mut o.unsplit_symbols);
    read_opt_strings(&v, "appliedFunctionSymbols", &mut o.applied_function_symbols);
    read_opt_strings(&v, "functionSymbols", &mut o.function_symbols);
    read_opt_strings(&v, "operatorSymbols", &mut o.operator_symbols);
    TextToAst::new(o)
        .convert(s)
        .map(Expression)
        .map_err(|e| JsError::new(&e.to_string()))
}

/// Parse LaTeX with JS-style parser parameters (keys: `allowedLatexSymbols`,
/// `appliedFunctionSymbols`, `functionSymbols`,
/// `allowSimplifiedFunctionApplication`, `parseLeibnizNotation`,
/// `parseScientificNotation`).
#[wasm_bindgen]
pub fn parse_latex_with_options(s: &str, options_json: &str) -> Result<Expression, JsError> {
    let v: serde_json::Value =
        serde_json::from_str(options_json).map_err(|e| JsError::new(&e.to_string()))?;
    let mut o = LatexToAstOptions::default();
    read_opt_bool(
        &v,
        "allowSimplifiedFunctionApplication",
        &mut o.allow_simplified_function_application,
    );
    read_opt_bool(&v, "parseLeibnizNotation", &mut o.parse_leibniz_notation);
    read_opt_bool(&v, "parseScientificNotation", &mut o.parse_scientific_notation);
    read_opt_strings(&v, "allowedLatexSymbols", &mut o.allowed_latex_symbols);
    read_opt_strings(&v, "appliedFunctionSymbols", &mut o.applied_function_symbols);
    read_opt_strings(&v, "functionSymbols", &mut o.function_symbols);
    LatexToAst::new(o)
        .convert(s)
        .map(Expression)
        .map_err(|e| JsError::new(&e.to_string()))
}

fn read_opt_bool(v: &serde_json::Value, key: &str, target: &mut bool) {
    if let Some(b) = v.get(key).and_then(serde_json::Value::as_bool) {
        *target = b;
    }
}

fn read_opt_strings(v: &serde_json::Value, key: &str, target: &mut Vec<String>) {
    if let Some(arr) = v.get(key).and_then(serde_json::Value::as_array) {
        *target = arr
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect();
    }
}

#[wasm_bindgen]
impl Expression {
    /// Serialize in the JS library's `toJSON` shape:
    /// `{"objectType": "math-expression", "tree": ...}` — revive with
    /// [`from_serialized`] (or the JS `me.reviver`).
    pub fn to_serialized(&self) -> String {
        serde_json::json!({
            "objectType": "math-expression",
            "tree": crate::js_tree::to_js(&self.0),
        })
        .to_string()
    }

    /// `me.round_numbers_to_precision_plus_decimals` — round to `digits`
    /// significant figures but at least `decimals` decimal places
    /// (`±Infinity` disable a mode, matching the JS callers).
    pub fn round_numbers_to_precision_plus_decimals(
        &self,
        digits: f64,
        decimals: f64,
    ) -> Expression {
        Expression(ops::round_numbers_to_precision_plus_decimals(
            &self.0, digits, decimals,
        ))
    }
}

// ---- f64 numeric utilities (the `me.math` replacements — see src/numeric.rs) ----

#[wasm_bindgen]
pub fn math_mod(x: f64, y: f64) -> f64 {
    crate::numeric::math_mod(x, y)
}
#[wasm_bindgen]
pub fn gcd(x: f64, y: f64) -> f64 {
    crate::numeric::gcd_f64(x, y)
}
#[wasm_bindgen]
pub fn lcm(x: f64, y: f64) -> f64 {
    crate::numeric::lcm_f64(x, y)
}
#[wasm_bindgen]
pub fn mean(data: Vec<f64>) -> f64 {
    crate::numeric::mean(&data)
}
#[wasm_bindgen]
pub fn median(data: Vec<f64>) -> f64 {
    crate::numeric::median(&data)
}
/// Unbiased sample variance (mathjs default).
#[wasm_bindgen]
pub fn variance(data: Vec<f64>) -> f64 {
    crate::numeric::variance(&data)
}
#[wasm_bindgen]
pub fn std(data: Vec<f64>) -> f64 {
    crate::numeric::std_dev(&data)
}
/// mathjs `quantileSeq` with linear interpolation.
#[wasm_bindgen]
pub fn quantile_seq(data: Vec<f64>, prob: f64) -> f64 {
    crate::numeric::quantile_seq(&data, prob)
}

/// Solve `A·x = b` for an n×n row-major matrix — the mathjs `lusolve`
/// replacement. `undefined` if singular or mis-sized.
#[wasm_bindgen]
pub fn lusolve(a: Vec<f64>, b: Vec<f64>, n: usize) -> Option<Vec<f64>> {
    crate::numeric::lusolve(&a, &b, n)
}

/// Numeric eigendecomposition of a real n×n row-major matrix — the mathjs
/// `eigs` replacement. Returns JSON in the mathjs result shape Doenet reads:
/// `{"values": [num | {"re","im"}...], "eigenvectors": [{"value": ...,
/// "vector": [...]}]}`. `undefined` when iteration fails to converge.
#[wasm_bindgen]
pub fn eigs(a: Vec<f64>, n: usize) -> Option<String> {
    let pairs = crate::numeric::eigs(&a, n)?;
    fn num(c: num_complex::Complex64) -> serde_json::Value {
        if c.im == 0.0 {
            serde_json::json!(c.re)
        } else {
            serde_json::json!({"re": c.re, "im": c.im})
        }
    }
    let values: Vec<_> = pairs.iter().map(|p| num(p.value)).collect();
    let eigenvectors: Vec<_> = pairs
        .iter()
        .map(|p| {
            serde_json::json!({
                "value": num(p.value),
                "vector": p.vector.iter().map(|&v| num(v)).collect::<Vec<_>>(),
            })
        })
        .collect();
    Some(serde_json::json!({"values": values, "eigenvectors": eigenvectors}).to_string())
}

#[wasm_bindgen]
impl Expression {
    /// Evaluate a constant expression to `digits` significant decimal digits
    /// (arbitrary precision). Returns the digits in normalized scientific
    /// form (`"1.4142…e0"`), `"re + im i"` for complex values, or
    /// `undefined` when not decidable within budget.
    pub fn evaluate_to_precision(&self, digits: usize) -> Option<String> {
        let p = crate::precise::evaluate_to_precision(&self.0, digits);
        p.to_decimal_string(digits)
    }
}

#[wasm_bindgen]
impl Expression {
    /// Determinant (tiered — MATRIX_PLAN §1b). Always an expression: an
    /// opaque `det(…)` node when not decidable.
    pub fn determinant(&self) -> Expression {
        Expression(crate::matrix::det(&self.0))
    }

    /// Characteristic polynomial in `var`, or `undefined` when the receiver
    /// is not a square literal matrix under the caps.
    pub fn char_poly(&self, var: &str) -> Option<Expression> {
        crate::matrix::char_poly(&self.0, var).map(Expression)
    }

    /// Eigenvalues with algebraic multiplicities as JSON
    /// `[{"value": <text>, "multiplicity": n}, …]` in canonical order
    /// (MATRIX_PLAN §2c), or `undefined` on an honest refusal.
    pub fn eigenvalues(&self) -> Option<String> {
        let vals = crate::matrix::eigenvalues(&self.0, &crate::Assumptions::new())?;
        let rows: Vec<serde_json::Value> = vals
            .iter()
            .map(|(v, m)| {
                serde_json::json!({
                    "value": to_text(v, &Default::default()),
                    "multiplicity": m,
                })
            })
            .collect();
        Some(serde_json::Value::Array(rows).to_string())
    }

    /// Eigenvectors as JSON `[{"value", "multiplicity", "basis": [[…]]}, …]`
    /// (basis entries in text syntax; geometric multiplicity is the basis
    /// length), or `undefined` on an honest refusal (MATRIX_PLAN §3).
    pub fn eigenvectors(&self) -> Option<String> {
        let pairs = crate::matrix::eigenvectors(&self.0, &crate::Assumptions::new())?;
        let rows: Vec<serde_json::Value> = pairs
            .iter()
            .map(|p| {
                let basis: Vec<Vec<String>> = p
                    .basis
                    .iter()
                    .map(|v| v.iter().map(|e| to_text(e, &Default::default())).collect())
                    .collect();
                serde_json::json!({
                    "value": to_text(&p.value, &Default::default()),
                    "multiplicity": p.alg_mult,
                    "basis": basis,
                })
            })
            .collect();
        Some(serde_json::Value::Array(rows).to_string())
    }
}

#[wasm_bindgen]
impl Expression {
    /// Indefinite integral in `var` (INTEGRATION_PLAN I1+I2), gate-verified
    /// by differentiation; `undefined` = no elementary form found.
    pub fn integrate(&self, var: &str) -> Option<Expression> {
        crate::integrate::integrate(&self.0, var, &crate::Assumptions::new()).map(Expression)
    }

    /// Certified definite integral over [a, b] to `digits` significant
    /// digits (guaranteed accuracy or `undefined` — never an estimate).
    pub fn integrate_to_precision(
        &self,
        var: &str,
        a: &Expression,
        b: &Expression,
        digits: usize,
    ) -> Option<String> {
        crate::precise::integrate_to_precision(&self.0, var, &a.0, &b.0, digits)
            .to_decimal_string(digits)
    }
}

// ================= ODE solving (ODE_PLAN O1+O2) =================

/// A computed trajectory with dense output — the `numeric.dopri` result
/// contract: `at(t)` always returns a length-n Float64Array (§5a), and
/// `last_t`/`last_y` support ODESystem's chunk chaining (§5b).
#[wasm_bindgen]
pub struct OdeSolution(crate::ode::OdeSolution);

#[wasm_bindgen]
impl OdeSolution {
    pub fn at(&self, t: f64) -> Vec<f64> {
        self.0.at(t)
    }

    /// Batch sampling for plotting: the states at each `ts[i]`, flattened
    /// row-major (`n` values per abscissa).
    pub fn at_many(&self, ts: Vec<f64>) -> Vec<f64> {
        let mut out = Vec::with_capacity(ts.len() * self.0.dim());
        for t in ts {
            out.extend(self.0.at(t));
        }
        out
    }

    pub fn dim(&self) -> usize {
        self.0.dim()
    }
    pub fn last_t(&self) -> f64 {
        self.0.last_t()
    }
    pub fn last_y(&self) -> Vec<f64> {
        self.0.last_y()
    }
    /// True when integration stopped before t1 (blow-up / budget) —
    /// Doenet's warning path, never an exception or NaN samples.
    pub fn terminated_early(&self) -> bool {
        self.0.terminated_early
    }
    /// The accepted step times (diagnostics).
    pub fn times(&self) -> Vec<f64> {
        self.0.times().to_vec()
    }
}

/// Drop-in for `numeric.dopri(t0, t1, y0, f, tol, maxit)`: `f` is a JS
/// closure `(t, yArray) -> array`. One boundary crossing per RK stage; for
/// expression right-hand sides prefer [`solve_ode_expressions`], which
/// evaluates entirely inside wasm.
#[wasm_bindgen]
pub fn solve_ode(
    f: &js_sys::Function,
    t0: f64,
    t1: f64,
    y0: Vec<f64>,
    tol: f64,
    max_steps: usize,
) -> OdeSolution {
    let this = JsValue::NULL;
    let sol = crate::ode::solve_ode_with(
        |t, y, out| {
            let arr = js_sys::Float64Array::from(y);
            match f.call2(&this, &JsValue::from_f64(t), &arr.into()) {
                Ok(v) => {
                    let a = js_sys::Array::from(&v);
                    if a.length() as usize != out.len() {
                        return false;
                    }
                    for (i, slot) in out.iter_mut().enumerate() {
                        match a.get(i as u32).as_f64() {
                            Some(x) => *slot = x,
                            None => return false,
                        }
                    }
                    true
                }
                Err(_) => false,
            }
        },
        t0,
        t1,
        &y0,
        tol,
        max_steps,
    );
    OdeSolution(sol)
}

/// Expression-RHS solver (plan §5c): `rhs` is a tuple/vector Expression with
/// one component per state variable (or a single expression for n = 1),
/// evaluated inside wasm via the compiled tape — no boundary crossings.
/// `undefined` when the expressions reference unknown variables.
#[wasm_bindgen]
pub fn solve_ode_expressions(
    rhs: &Expression,
    ind_var: &str,
    state_vars: Vec<String>,
    t0: f64,
    t1: f64,
    y0: Vec<f64>,
    tol: f64,
    max_steps: usize,
) -> Option<OdeSolution> {
    let comps: Vec<Expr> = match &rhs.0 {
        Expr::Seq(_, xs) => xs.clone(),
        other => vec![other.clone()],
    };
    crate::ode::solve_ode_exprs(&comps, ind_var, &state_vars, t0, t1, &y0, tol, max_steps)
        .map(OdeSolution)
}

#[wasm_bindgen]
impl Expression {
    /// Three-way definite-integral analysis (DIVERGENCE_PLAN): JSON
    /// `{"status":"value","value":…}` |
    /// `{"status":"divergent","singularities":[{"location":…,"exact":…?}]}` |
    /// `{"status":"unknown","reason":…}`.
    pub fn integrate_analyzed(
        &self,
        var: &str,
        a: &Expression,
        b: &Expression,
        digits: usize,
    ) -> String {
        use crate::precise::IntegralVerdict;
        let v = crate::precise::integrate_analyzed(&self.0, var, &a.0, &b.0, digits);
        match v {
            IntegralVerdict::Value(p) => serde_json::json!({
                "status": "value",
                "value": p.to_f64(),
                "digits": p.to_decimal_string(digits),
            })
            .to_string(),
            IntegralVerdict::Divergent { at } => {
                let sing: Vec<serde_json::Value> = at
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "location": s.location,
                            "exact": s.exact.as_ref().map(|e| to_text(e, &Default::default())),
                        })
                    })
                    .collect();
                serde_json::json!({"status": "divergent", "singularities": sing}).to_string()
            }
            IntegralVerdict::Unknown(reason) => {
                serde_json::json!({"status": "unknown", "reason": reason}).to_string()
            }
        }
    }
}
