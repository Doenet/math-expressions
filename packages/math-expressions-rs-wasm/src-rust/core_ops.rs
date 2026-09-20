//! Core `Expression` methods: rendering, equality, the primary simplify/expand/
//! derivative surface, evaluation, number cleanup, and the arithmetic builders.

use super::Expression;
use math_expressions::{
    constants_to_floats, default_order as rust_default_order, derivative as rust_derivative,
    equals as rust_equals, evaluate_numbers, evaluate_numbers_evaluate_functions,
    evaluate_numbers_evaluate_functions_with_digits, evaluate_numbers_preserve_order,
    evaluate_numbers_preserve_order_with_digits, evaluate_numbers_with_digits,
    evaluate_to_constant as rust_evc, expand as rust_expand, ops, reduce_rational,
    round_numbers_to_decimals, round_numbers_to_precision, simplify as rust_simplify,
    simplify_with as rust_simplify_with, to_latex, to_text, Assumptions, EqOptions, Expr,
    LatexOpts, MaxDigits, TextOpts, TextToAst, TextToAstOptions,
};
use wasm_bindgen::prelude::*;

/// A component path from JS. Indices arrive as `u32` because that is what
/// `wasm_bindgen` marshals a JS number array into; the core takes `usize`.
fn to_path(path: Vec<u32>) -> Vec<usize> {
    path.into_iter().map(|i| i as usize).collect()
}

/// Read the five shared number/blank/multiplication render options DoenetML
/// passes to `toLatex`/`toString` (`padToDigits`, `padToDecimals`, `showBlanks`,
/// `explicitMultiplicationSymbols`, `avoidScientificNotation`). Keys absent
/// from `v` leave the defaults.
fn read_render_opts(
    v: &serde_json::Value,
    pad_to_digits: &mut Option<u32>,
    pad_to_decimals: &mut Option<u32>,
    show_blanks: &mut bool,
    explicit_multiplication_symbols: &mut bool,
    avoid_scientific_notation: &mut bool,
) {
    super::parse::read_opt_u32(v, "padToDigits", pad_to_digits);
    super::parse::read_opt_u32(v, "padToDecimals", pad_to_decimals);
    super::parse::read_opt_bool(v, "showBlanks", show_blanks);
    super::parse::read_opt_bool(
        v,
        "explicitMultiplicationSymbols",
        explicit_multiplication_symbols,
    );
    super::parse::read_opt_bool(v, "avoidScientificNotation", avoid_scientific_notation);
}

/// Validate a `matrixEnvironment` name against the environments the LaTeX
/// *parser* accepts, so rendered output keeps round-tripping — and so the option
/// can never inject arbitrary LaTeX.
fn matrix_environment(env: &str) -> Result<&str, JsError> {
    match env {
        "matrix" | "pmatrix" | "bmatrix" => Ok(env),
        _ => Err(JsError::new(&format!(
            "unsupported matrixEnvironment {env:?} (expected matrix, pmatrix or bmatrix)"
        ))),
    }
}

#[wasm_bindgen]
impl Expression {
    /// Render back to text syntax — in the notation this expression was
    /// parsed with (see the `Expression` doc).
    pub fn to_text(&self) -> String {
        to_text(
            &self.0,
            &TextOpts {
                notation: self.1.clone(),
                ..Default::default()
            },
        )
    }

    /// Render to LaTeX — in the notation this expression was parsed with.
    pub fn to_latex(&self) -> String {
        to_latex(
            &self.0,
            &LatexOpts {
                notation: self.1.clone(),
                ..Default::default()
            },
        )
    }

    /// Render to text under an explicit options object (keys: `unicode` bool,
    /// and a `notation` sub-object — see `parse_text_with_options`). Options
    /// start from the expression's carried notation; given keys override.
    pub fn to_text_with_options(&self, options_json: &str) -> Result<String, JsError> {
        let v: serde_json::Value =
            serde_json::from_str(options_json).map_err(|e| JsError::new(&e.to_string()))?;
        let mut o = TextOpts {
            notation: self.1.clone(),
            ..Default::default()
        };
        super::parse::read_opt_bool(&v, "unicode", &mut o.unicode);
        super::parse::read_notation(&v, &mut o.notation).map_err(|e| JsError::new(&e))?;
        read_render_opts(
            &v,
            &mut o.pad_to_digits,
            &mut o.pad_to_decimals,
            &mut o.show_blanks,
            &mut o.explicit_multiplication_symbols,
            &mut o.avoid_scientific_notation,
        );
        Ok(to_text(&self.0, &o))
    }

    /// Render to LaTeX under an explicit options object (key: `notation`
    /// sub-object — see `parse_text_with_options`). Options start from the
    /// expression's carried notation; given keys override.
    pub fn to_latex_with_options(&self, options_json: &str) -> Result<String, JsError> {
        let v: serde_json::Value =
            serde_json::from_str(options_json).map_err(|e| JsError::new(&e.to_string()))?;
        let mut o = LatexOpts {
            notation: self.1.clone(),
            ..Default::default()
        };
        super::parse::read_notation(&v, &mut o.notation).map_err(|e| JsError::new(&e))?;
        read_render_opts(
            &v,
            &mut o.pad_to_digits,
            &mut o.pad_to_decimals,
            &mut o.show_blanks,
            &mut o.explicit_multiplication_symbols,
            &mut o.avoid_scientific_notation,
        );
        if let Some(env) = v.get("matrixEnvironment").and_then(|e| e.as_str()) {
            o.matrix_environment = matrix_environment(env)?.to_string();
        }
        Ok(to_latex(&self.0, &o))
    }

    /// The parse tree serialised to the JS `Tree` JSON shape
    /// (e.g. `["+", 1, "x", 3]`), so it lines up with the JS library's
    /// `expr.tree`. Intended for inspection/tooling (§13 `to_json`).
    pub fn tree_json(&self) -> String {
        math_expressions::expr::serde::to_js(&self.0).to_string()
    }

    /// Mathematical equality with another expression.
    pub fn equals(&self, other: &Expression) -> bool {
        rust_equals(&self.0, &other.0, &EqOptions::default())
    }

    /// Canonical simplification — the *aggressive* simplifier
    /// (FULL_SIMPLIFY_PLAN), which goes beyond the JS `.simplify()`: on top of
    /// the canonical reductions it folds `exp(ln x) → x`, the trig/exp/log
    /// special values (`cos(π/3) → 1/2`), and rational cancellation, iterated
    /// to a fixpoint. Always value-equal to the input, so the result may be a
    /// different (smaller) tree than the JS library returns.
    pub fn simplify(&self) -> Expression {
        self.derive(rust_simplify(&self.0))
    }

    /// Simplify under the given `assumptions` — each a relation in text syntax
    /// (e.g. `"x > 0"`, `"n elementof Z"`). Assumptions that fail to parse are
    /// ignored. This runs the same aggressive pipeline as [`Self::simplify`]
    /// plus the assumption-aware rules, so with an empty list it is exactly
    /// [`Self::simplify`].
    pub fn simplify_with_assumptions(&self, assumptions: Vec<String>) -> Expression {
        let mut a = Assumptions::new();
        // Assumption strings are parsed in THIS expression's notation — under
        // comma notation `"x > 1,5"` must read as x > 3/2, not misparse.
        let opts = TextToAstOptions {
            notation: self.1.clone(),
            ..Default::default()
        };
        for s in &assumptions {
            if let Ok(e) = TextToAst::new(opts.clone()).convert(s) {
                a.add(&e);
            }
        }
        self.derive(rust_simplify_with(&self.0, &a))
    }

    /// Sort into the JS library's default order, without evaluating anything:
    /// DoenetML's `simplify="normalizeOrder"`. Every term survives, including
    /// `0x^2` and unfolded constants — that is the difference from `simplify`,
    /// and the reason the attribute exists.
    pub fn default_order(&self) -> Expression {
        self.derive(rust_default_order(&self.0))
    }

    /// Distribute products and powers of sums.
    pub fn expand(&self) -> Expression {
        self.derive(rust_expand(&self.0))
    }

    /// Does the expression contain any ± (plus-minus) operator?
    pub fn contains_pm(&self) -> bool {
        math_expressions::contains_pm(&self.0)
    }

    /// The number of ± (plus-minus) operators in the expression.
    pub fn count_pm(&self) -> usize {
        math_expressions::count_pm(&self.0)
    }

    /// Enumerate all `2^n` sign assignments of the ± operators (each `±x`
    /// becomes `x` or `-x`). Errors if there are too many ± operators to expand.
    pub fn expand_pm_signs(&self) -> Result<Vec<Expression>, JsError> {
        math_expressions::expand_pm_signs(&self.0)
            .map(|v| v.into_iter().map(|e| self.derive(e)).collect())
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Symbolic derivative with respect to `var`.
    pub fn derivative(&self, var: &str) -> Expression {
        self.derive(rust_derivative(&self.0, var))
    }

    /// The critical points with respect to `var` — the real solutions of
    /// `d/dvar = 0` — exactly, in increasing order, or `undefined` where the
    /// method does not reach.
    ///
    /// The three outcomes are distinct and a caller holding a numerical
    /// fallback needs all three: an array of points, an *empty* array meaning
    /// provably none, and `undefined` meaning undecided — sample instead.
    /// Undecided covers a derivative that is not rational in `var` (`cos(x)`),
    /// one carrying a free parameter (`d/dx a·x²`), and a constant-zero
    /// derivative, where every point is critical. Points where `f'` does not
    /// exist (the corner of `|x|`) are not reported; they are critical in the
    /// textbook sense but are not roots of a polynomial.
    pub fn critical_points(&self, var: &str) -> Option<Vec<Expression>> {
        math_expressions::critical_points(&self.0, var)
            .map(|pts| pts.into_iter().map(|e| self.derive(e)).collect())
    }

    /// The free variable names, in first-appearance order.
    pub fn variables(&self) -> Vec<String> {
        ops::variables(&self.0)
    }

    /// Evaluate a closed expression to a real number, or `undefined` (JS side)
    /// when it has free variables or is not purely real — preserving the
    /// upstream null-vs-value distinction. The imaginary tolerance is
    /// *relative* to the magnitude, since complex-arithmetic float noise scales
    /// with it (`1e8·e^(iπ)` has im ≈ 1e-8 yet is real).
    ///
    /// `±∞` is a real value and comes back as one; an infinite real part makes
    /// the relative tolerance infinite, so the imaginary part is compared
    /// exactly there instead of being swallowed. A non-real result is not lost
    /// either — the JS wrapper falls back to [`Self::evaluate_to_complex`].
    pub fn evaluate_to_constant(&self) -> Option<f64> {
        let v = rust_evc(&self.0)?;
        let real = if v.re.is_finite() {
            v.im.abs() <= 1e-10 * v.re.abs().max(1.0)
        } else {
            v.im == 0.0
        };
        real.then_some(v.re)
    }

    /// Evaluate a closed expression to a complex constant, returned as the pair
    /// `[re, im]`, or `undefined` (JS side) when it has free variables or no
    /// value at all. Unlike [`Self::evaluate_to_constant`], this keeps a
    /// non-real result instead of discarding it.
    pub fn evaluate_to_complex(&self) -> Option<Vec<f64>> {
        let v = rust_evc(&self.0)?;
        (!v.re.is_nan() && !v.im.is_nan()).then(|| vec![v.re, v.im])
    }

    /// Replace `pi` and `e` with their floating-point values.
    pub fn constants_to_floats(&self) -> Expression {
        self.derive(constants_to_floats(&self.0))
    }

    /// Round every number to `decimals` decimal places.
    pub fn round_numbers_to_decimals(&self, decimals: i32) -> Expression {
        self.derive(round_numbers_to_decimals(&self.0, decimals))
    }

    /// Round every number to `sig_figs` significant figures.
    pub fn round_numbers_to_precision(&self, sig_figs: i32) -> Expression {
        self.derive(round_numbers_to_precision(&self.0, sig_figs))
    }

    /// `me.round_numbers_to_precision_plus_decimals` — round to `digits`
    /// significant figures but at least `decimals` decimal places
    /// (`±Infinity` disable a mode, matching the JS callers).
    pub fn round_numbers_to_precision_plus_decimals(
        &self,
        digits: f64,
        decimals: f64,
    ) -> Expression {
        self.derive(ops::round_numbers_to_precision_plus_decimals(
            &self.0, digits, decimals,
        ))
    }

    /// Fold numeric subexpressions exactly (`4+x-2` → `x+2`).
    pub fn evaluate_numbers(&self) -> Expression {
        self.derive(evaluate_numbers(&self.0))
    }

    /// Fold numeric subexpressions **without reordering operands** — the
    /// `skip_ordering` form, backing DoenetML's
    /// `simplify="numberspreserveorder"`. `1+x+2` stays `1+x+2`, where
    /// [`Self::evaluate_numbers`] gives `x+3`.
    pub fn evaluate_numbers_preserve_order(&self) -> Expression {
        self.derive(evaluate_numbers_preserve_order(&self.0))
    }

    /// [`Self::evaluate_numbers`] with function applications evaluated at
    /// numeric arguments (`sin(0)+2` → `2`) — the `evaluate_functions` form,
    /// backing DoenetML's `simplify="full"`.
    pub fn evaluate_numbers_evaluate_functions(&self) -> Expression {
        self.derive(evaluate_numbers_evaluate_functions(&self.0))
    }

    /// The three `evaluate_numbers` forms above under a `max_digits` budget.
    /// `max_digits` is a JS number: `Infinity` spends without limit (`π`, `e`
    /// and every folded rational become floats, so a variable-free subtree
    /// collapses to one number — `2π + π + 6 → 15.42477796076938`), while a
    /// finite value converts only the rationals whose decimal fits that many
    /// significant figures (`1/2 → 0.5`, but `1/3` stays exact). A negative or
    /// non-integer finite value is clamped to `0` ("integers only").
    ///
    /// One entry point rather than three more, because the JS option object
    /// crosses two independent flags and the product of them is not worth six
    /// exports. `skip_ordering` wins over `evaluate_functions` if a caller
    /// somehow passes both, matching the compat layer's dispatch order.
    pub fn evaluate_numbers_to_floats(
        &self,
        skip_ordering: bool,
        evaluate_functions: bool,
        max_digits: f64,
    ) -> Expression {
        let d = if max_digits.is_infinite() && max_digits > 0.0 {
            MaxDigits::Unlimited
        } else if max_digits.is_finite() && max_digits >= 0.0 {
            MaxDigits::Finite(max_digits as u32)
        } else {
            MaxDigits::Finite(0)
        };
        self.derive(if skip_ordering {
            evaluate_numbers_preserve_order_with_digits(&self.0, d)
        } else if evaluate_functions {
            evaluate_numbers_evaluate_functions_with_digits(&self.0, d)
        } else {
            evaluate_numbers_with_digits(&self.0, d)
        })
    }

    /// Cancel common polynomial factors in fractions
    /// (`(x^2-1)/(x-1)` → `x+1`).
    pub fn reduce_rational(&self) -> Expression {
        self.derive(reduce_rational(&self.0))
    }

    /// Put the expression over a single common denominator and reduce it to
    /// lowest terms (FULL_SIMPLIFY S2). Non-rational subtrees (`sin x`, `√x`,
    /// …) are held fixed as opaque kernels, so `1/sin(x) + 1/sin(x)` becomes
    /// `2/sin(x)` and `1/(x+1) + 1/(x-1)` becomes `2x/(x^2-1)`.
    pub fn together(&self) -> Expression {
        self.derive(math_expressions::together(&self.0))
    }

    /// Replace `var` with `value` everywhere (no simplification).
    pub fn substitute_var(&self, var: &str, value: &Expression) -> Expression {
        let map = std::collections::HashMap::from([(var.to_string(), value.0.clone())]);
        self.derive(ops::substitute(&self.0, &map))
    }

    /// Replace several variables at once, from a `{name: tree}` JSON object.
    ///
    /// **Simultaneously**, which is the whole reason this exists: applying the
    /// bindings one at a time captures. `sin(x+y)` with `x → 10y` and
    /// `y → -pi` is `sin(10y − π)`, but substituting `x` first puts a fresh `y`
    /// in the tree for the second substitution to replace, giving
    /// `sin(-10π − π)` — a wrong answer, silently, and one that depends on
    /// key order.
    pub fn substitute_map(&self, map_json: &str) -> Result<Expression, JsError> {
        let value: serde_json::Value =
            serde_json::from_str(map_json).map_err(|e| JsError::new(&e.to_string()))?;
        let obj = value
            .as_object()
            .ok_or_else(|| JsError::new("substitute_map: expected an object of {name: tree}"))?;
        let mut map = std::collections::HashMap::with_capacity(obj.len());
        for (name, tree) in obj {
            let expr = math_expressions::expr::serde::try_from_js(tree)
                .map_err(|e| JsError::new(&format!("substitute_map: {name}: {e}")))?;
            map.insert(name.clone(), expr);
        }
        Ok(self.derive(ops::substitute(&self.0, &map)))
    }

    /// Evaluate at many values of one variable in a single call.
    ///
    /// Sampling — plotting a curve, bracketing an extremum, hunting a root —
    /// asks for the same expression at thousands of points, and paying the
    /// boundary crossing and the variable-name marshalling per point dominates
    /// the arithmetic by orders of magnitude. This pays both once and returns a
    /// `Float64Array` of the same length as `values`.
    ///
    /// Any other variable is left unbound; `substitute` it first. A point that
    /// does not evaluate to a finite real — unbound variable, pole, complex
    /// value — comes back as `NaN`, so the result lines up index-for-index with
    /// the input and gaps carry the marker consumers already handle.
    pub fn evaluate_many(&self, var: &str, values: Vec<f64>) -> Vec<f64> {
        ops::evaluate_many(&self.0, var, &values)
    }

    /// Evaluate at real bindings given as parallel arrays; `undefined` on an
    /// unbound variable, non-finite, or non-real result.
    pub fn evaluate(&self, vars: Vec<String>, values: Vec<f64>) -> Option<f64> {
        if vars.len() != values.len() {
            return None;
        }
        let bindings: std::collections::HashMap<String, f64> =
            vars.into_iter().zip(values).collect();
        let v = ops::evaluate_fast_f64(&self.0, &bindings)?;
        (v.im.abs() <= 1e-10 * v.re.abs().max(1.0)).then_some(v.re)
    }

    /// The applied function names, first-appearance order.
    pub fn functions(&self) -> Vec<String> {
        ops::functions(&self.0)
    }

    /// The operator heads used, first-appearance order, in the JS tree's
    /// spelling (`+`, `*`, `^`, `and`, …).
    pub fn operators(&self) -> Vec<String> {
        ops::operators(&self.0)
    }

    // ---- component access (JS `get_component` / `substitute_component`) ----

    /// The component at a 0-based `path` into the operand lists of the JS tree
    /// (`tree_json`) — component `i` of `["tuple", a, b, c]` is `tree[i + 1]`.
    /// A path of more than one index walks into nested components, so a matrix
    /// entry is `[1, row, col]` (its component 0 is the dimension pair).
    /// `undefined` when a step is out of range, or lands on a leaf or on one of
    /// the shapes whose JS spelling carries boolean flags (`interval`, mixed
    /// relation chains), which have no component list.
    pub fn get_component(&self, path: Vec<u32>) -> Option<Expression> {
        ops::get_component(&self.0, &to_path(path)).map(|e| self.derive(e))
    }

    /// The expression with the component at `path` replaced by `value`.
    /// `undefined` under the same conditions as `get_component`.
    pub fn substitute_component(&self, path: Vec<u32>, value: &Expression) -> Option<Expression> {
        ops::substitute_component(&self.0, &to_path(path), &value.0).map(|e| self.derive(e))
    }

    // ---- arithmetic builders (JS `add`/`subtract`/`multiply`/`divide`/`pow`) ----

    pub fn add(&self, other: &Expression) -> Expression {
        self.derive(Expr::Add(vec![self.0.clone(), other.0.clone()]))
    }
    pub fn subtract(&self, other: &Expression) -> Expression {
        self.derive(Expr::Add(vec![
            self.0.clone(),
            Expr::Neg(Box::new(other.0.clone())),
        ]))
    }
    pub fn multiply(&self, other: &Expression) -> Expression {
        self.derive(Expr::Mul(vec![self.0.clone(), other.0.clone()]))
    }
    pub fn divide(&self, other: &Expression) -> Expression {
        self.derive(Expr::Div(
            Box::new(self.0.clone()),
            Box::new(other.0.clone()),
        ))
    }
    pub fn pow(&self, other: &Expression) -> Expression {
        self.derive(Expr::Pow(
            Box::new(self.0.clone()),
            Box::new(other.0.clone()),
        ))
    }

    /// Remainder `self mod other` (JS `mod`).
    ///
    /// Built as an application, not an `OtherOp`, so it is the same tree the
    /// parser makes from `mod(a, b)` — `["apply", "mod", ["tuple", a, b]]`.
    /// A builder that spelled it differently would make `a.mod(b)` unequal to
    /// the parse of its own printed form.
    #[wasm_bindgen(js_name = "mod")]
    pub fn modulo(&self, other: &Expression) -> Expression {
        self.derive(Expr::Apply(
            Box::new(Expr::sym("mod")),
            vec![self.0.clone(), other.0.clone()],
        ))
    }

    /// A structural copy (JS `copy`).
    pub fn copy(&self) -> Expression {
        self.derive(self.0.clone())
    }
}
