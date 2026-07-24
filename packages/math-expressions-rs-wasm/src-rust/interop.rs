//! JS-tree AST boundary (Doenet interop): construction from / serialization to
//! the array-AST and `toJSON` shapes, plus the `me.utils` match/flatten ports.

use super::Expression;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Expression {
    /// Serialize in the JS library's `toJSON` shape:
    /// `{"objectType": "math-expression", "tree": ...}` — revive with
    /// [`from_serialized`] (or the JS `me.reviver`).
    pub fn to_serialized(&self) -> String {
        serde_json::json!({
            "objectType": "math-expression",
            "tree": math_expressions::js_tree::to_js(&self.0),
        })
        .to_string()
    }
}

/// Build an `Expression` from a JS-tree AST (JSON) — the port of
/// `me.fromAst`. Accepts the array AST format Doenet manipulates directly.
#[wasm_bindgen]
pub fn from_ast(tree_json: &str) -> Result<Expression, JsError> {
    let value: serde_json::Value =
        serde_json::from_str(tree_json).map_err(|e| JsError::new(&e.to_string()))?;
    math_expressions::js_tree::try_from_js(&value)
        .map(Expression::with_default_notation)
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
    math_expressions::js_tree::try_from_js(tree)
        .map(Expression::with_default_notation)
        .map_err(|e| JsError::new(&e))
}

/// Template match on JS-tree ASTs — the port of `me.utils.match` in its
/// default mode. Returns the bindings object as JSON (wildcard name →
/// subtree), or `undefined` if the tree does not match the pattern.
#[wasm_bindgen]
pub fn match_template(tree_json: &str, pattern_json: &str) -> Option<String> {
    let tree: serde_json::Value = serde_json::from_str(tree_json).ok()?;
    let pattern: serde_json::Value = serde_json::from_str(pattern_json).ok()?;
    math_expressions::js_match::match_template(&tree, &pattern)
        .map(|m| serde_json::Value::Object(m).to_string())
}

/// `me.utils.flatten` on a JS-tree AST (JSON in, JSON out).
#[wasm_bindgen]
pub fn flatten_ast(tree_json: &str) -> Option<String> {
    let tree: serde_json::Value = serde_json::from_str(tree_json).ok()?;
    Some(math_expressions::js_match::flatten_tree(&tree).to_string())
}

/// `me.utils.unflattenLeft`.
#[wasm_bindgen]
pub fn unflatten_left(tree_json: &str) -> Option<String> {
    let tree: serde_json::Value = serde_json::from_str(tree_json).ok()?;
    Some(math_expressions::js_match::unflatten_left(&tree).to_string())
}

/// `me.utils.unflattenRight`.
#[wasm_bindgen]
pub fn unflatten_right(tree_json: &str) -> Option<String> {
    let tree: serde_json::Value = serde_json::from_str(tree_json).ok()?;
    Some(math_expressions::js_match::unflatten_right(&tree).to_string())
}
