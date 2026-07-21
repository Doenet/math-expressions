//! Text/LaTeX parsing entry points (plain and with JS-style option objects),
//! plus the shared JSON option readers used here and in [`super::grading`].

use super::Expression;
use math_expressions::{LatexToAst, LatexToAstOptions, TextToAst, TextToAstOptions};
use wasm_bindgen::prelude::*;

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

pub(super) fn read_opt_f64(v: &serde_json::Value, key: &str, target: &mut f64) {
    if let Some(x) = v.get(key).and_then(serde_json::Value::as_f64) {
        *target = x;
    }
}

pub(super) fn read_opt_bool(v: &serde_json::Value, key: &str, target: &mut bool) {
    if let Some(b) = v.get(key).and_then(serde_json::Value::as_bool) {
        *target = b;
    }
}

pub(super) fn read_opt_strings(v: &serde_json::Value, key: &str, target: &mut Vec<String>) {
    if let Some(arr) = v.get(key).and_then(serde_json::Value::as_array) {
        *target = arr
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect();
    }
}
