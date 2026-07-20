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
