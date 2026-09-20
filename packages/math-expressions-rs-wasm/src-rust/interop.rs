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
            "tree": math_expressions::expr::serde::to_js(&self.0),
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
    math_expressions::expr::serde::try_from_js(&value)
        .map(Expression::with_default_notation)
        .map_err(|e| JsError::new(&e))
}

/// Build an `Expression` from a bare f64 — the fast path for `me.fromAst(n)`.
///
/// The general [`from_ast`] takes a JSON string, so passing a single number
/// through it costs a `JSON.stringify` on the JS side and a full JSON parse
/// here. Sampling drives this: evaluating an interpolated function over a
/// domain calls `fromAst` once per sample point on an arbitrary float, which
/// by design misses the compat layer's atom cache (caching sampled coordinates
/// is a measured loss — see `ATOM_HANDLES`). Those two JSON traversals were
/// the largest remaining allocation source in the extrema search.
///
/// This is a shortcut, not a second set of semantics, so it has to agree with
/// `from_ast` on every input. Integral values demote to `Int` the same way, and
/// non-finite ones become `Expr::Const`, which is what `expr::serde` produces
/// for the `{"$":"NaN"}` / `{"$":"Inf"}` tags the JSON path carries them in.
/// `Number::from_f64` would instead give a `Num(Float(NaN))`, which the ∞/NaN
/// folds in `normalize::simplify` and `normalize::constructors` do not match —
/// so `me.fromAst(NaN)` would simplify differently depending on which path it
/// took.
#[wasm_bindgen]
pub fn from_number(value: f64) -> Expression {
    use math_expressions::expr::MathConst;
    let expr = if value.is_nan() {
        math_expressions::Expr::Const(MathConst::NaN)
    } else if value == f64::INFINITY {
        math_expressions::Expr::Const(MathConst::Inf)
    } else if value == f64::NEG_INFINITY {
        math_expressions::Expr::Const(MathConst::NegInf)
    } else {
        math_expressions::Expr::Num(math_expressions::num::Number::from_f64(value))
    };
    Expression::with_default_notation(expr)
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
    let tree = value
        .get("tree")
        .ok_or_else(|| JsError::new("missing tree"))?;
    math_expressions::expr::serde::try_from_js(tree)
        .map(Expression::with_default_notation)
        .map_err(|e| JsError::new(&e))
}

/// Template match on JS-tree ASTs — the port of `me.utils.match` in its
/// default mode. Returns the bindings object as JSON (wildcard name →
/// subtree), or `undefined` if the tree does not match the pattern.
///
/// A search that runs past `MAX_MATCH_STEPS` is an **error**, not `undefined`:
/// "we gave up" and "it does not match" grade differently.
#[wasm_bindgen]
pub fn match_template(tree_json: &str, pattern_json: &str) -> Result<Option<String>, JsError> {
    let Ok(tree) = serde_json::from_str::<serde_json::Value>(tree_json) else {
        return Ok(None);
    };
    let Ok(pattern) = serde_json::from_str::<serde_json::Value>(pattern_json) else {
        return Ok(None);
    };
    match crate::js_match::match_template(&tree, &pattern) {
        Ok(m) => Ok(m.map(|m| serde_json::Value::Object(m).to_string())),
        Err(_) => Err(JsError::new(
            "match: search budget exceeded — the tree is too large to match \
             against this pattern (try without allow_permutations)",
        )),
    }
}

/// [`match_template`] with the JS `match` options honored.
///
/// `options_json` keys, all optional:
/// - `variables`: object mapping each declared parameter to its kind —
///   `true`/`"any"`, `"number"`, or `"variable"`. **Present and empty means no
///   parameters**, so nothing binds; absent keeps the legacy default where
///   every string leaf in the pattern is a wildcard.
/// - `allow_permutations`: match `+`/`*` operands in any order.
/// - `allow_implicit_identities`: array of parameter names that may take the
///   operator's identity when the tree has no operand for them, or `true` for
///   every declared parameter.
///
/// Malformed options are an error rather than a silent fall-back to the
/// defaults, for the same reason they are on the equality entry points: a
/// match that silently ignored its parameter list produced confidently wrong
/// bindings. That covers the *shape* as well as the JSON syntax — an
/// ill-typed `variables` used to fall back to "every string leaf in the
/// pattern is a wildcard", which is the most permissive mode there is, so a
/// caller who misspelled the option got looser matching and no warning.
#[wasm_bindgen]
pub fn match_template_with_options(
    tree_json: &str,
    pattern_json: &str,
    options_json: &str,
) -> Result<Option<String>, JsError> {
    let tree: serde_json::Value =
        serde_json::from_str(tree_json).map_err(|e| JsError::new(&e.to_string()))?;
    let pattern: serde_json::Value =
        serde_json::from_str(pattern_json).map_err(|e| JsError::new(&e.to_string()))?;
    let v: serde_json::Value =
        serde_json::from_str(options_json).map_err(|e| JsError::new(&e.to_string()))?;

    // `null` and `{}` both mean "no options"; anything else that is not an
    // object is a caller mistake.
    let empty = serde_json::Map::new();
    let obj = match &v {
        serde_json::Value::Null => &empty,
        serde_json::Value::Object(o) => o,
        other => {
            return Err(JsError::new(&format!(
                "match: options must be an object or null, got {other}"
            )))
        }
    };
    if let Some(unknown) = obj.keys().find(|k| {
        !matches!(
            k.as_str(),
            "variables" | "allow_permutations" | "allow_implicit_identities"
        )
    }) {
        return Err(JsError::new(&format!(
            "match: unknown option {unknown:?} (expected \"variables\", \
             \"allow_permutations\" or \"allow_implicit_identities\")"
        )));
    }

    let mut opts = crate::js_match::MatchOptions::default();
    if let Some(raw) = obj.get("variables") {
        let vars = raw.as_object().ok_or_else(|| {
            JsError::new(&format!(
                "match: 'variables' must be an object mapping each parameter \
                 name to its kind, got {raw}"
            ))
        })?;
        let mut declared = std::collections::HashMap::new();
        for (name, kind) in vars {
            let kind = match kind {
                serde_json::Value::String(s) => match s.as_str() {
                    "number" => crate::js_match::VarKind::Number,
                    "variable" => crate::js_match::VarKind::Variable,
                    "any" => crate::js_match::VarKind::Any,
                    // An unrecognized kind string is an unusable condition; a
                    // parameter carrying it can bind nothing, so the match fails
                    // gracefully rather than throwing (legacy did not crash on a
                    // condition it could not honor). See `VarKind::Nothing`.
                    _ => crate::js_match::VarKind::Nothing,
                },
                // `true` is the legacy "any subtree". `false` declares the name
                // and admits nothing — a valid, if useless, condition: the match
                // simply cannot succeed.
                serde_json::Value::Bool(true) => crate::js_match::VarKind::Any,
                serde_json::Value::Bool(false) => crate::js_match::VarKind::Nothing,
                // A `RegExp` arrives here as `{}` (JSON has no spelling for one)
                // and a predicate function likewise cannot cross the boundary.
                // Both are deprecated and wontfix (see `js_match`'s docs); treat
                // them as an unusable condition so the match fails gracefully
                // instead of throwing.
                _ => crate::js_match::VarKind::Nothing,
            };
            declared.insert(name.clone(), kind);
        }
        opts.variables = Some(declared);
    }
    if let Some(raw) = obj.get("allow_permutations") {
        opts.allow_permutations = raw.as_bool().ok_or_else(|| {
            JsError::new(&format!(
                "match: 'allow_permutations' must be a boolean, got {raw}"
            ))
        })?;
    }
    if let Some(raw) = obj.get("allow_implicit_identities") {
        match raw {
            // `true` means every declared parameter. Expanded here rather than
            // by the caller because the default wildcard set is the *pattern's*
            // string leaves, which only the matcher knows.
            serde_json::Value::Bool(all) => opts.implicit_identities_all = *all,
            serde_json::Value::Array(names) => {
                let mut set = std::collections::HashSet::new();
                for n in names {
                    let name = n.as_str().ok_or_else(|| {
                        JsError::new(&format!(
                            "match: 'allow_implicit_identities' entries must be \
                             parameter names, got {n}"
                        ))
                    })?;
                    set.insert(name.to_string());
                }
                opts.implicit_identities = set;
            }
            other => {
                return Err(JsError::new(&format!(
                    "match: 'allow_implicit_identities' must be an array of \
                     parameter names or true, got {other}"
                )))
            }
        }
    }

    match crate::js_match::match_template_with_options(&tree, &pattern, &opts) {
        Ok(m) => Ok(m.map(|m| serde_json::Value::Object(m).to_string())),
        Err(_) => Err(JsError::new(
            "match: search budget exceeded — the tree is too large to match \
             against this pattern (try without allow_permutations)",
        )),
    }
}

/// `me.utils.flatten` on a JS-tree AST (JSON in, JSON out).
#[wasm_bindgen]
pub fn flatten_ast(tree_json: &str) -> Option<String> {
    let tree: serde_json::Value = serde_json::from_str(tree_json).ok()?;
    Some(crate::js_match::flatten_tree(&tree).to_string())
}

/// `me.utils.unflattenLeft`.
///
/// Refuses a tree whose widest associative node exceeds
/// [`MAX_UNFLATTEN_OPERANDS`](crate::js_match::MAX_UNFLATTEN_OPERANDS): the
/// fold turns width into depth, and nothing downstream — serialization here,
/// `JSON.parse` and the tree walk on the JS side — survives an unbounded one.
/// An error rather than a silent pass-through, because the flat tree a caller
/// would get back is *not* the associated one it asked for, and the difference
/// is invisible until something later reads it as binary.
#[wasm_bindgen]
pub fn unflatten_left(tree_json: &str) -> Result<Option<String>, JsError> {
    let Ok(tree) = serde_json::from_str::<serde_json::Value>(tree_json) else {
        return Ok(None);
    };
    match crate::js_match::unflatten_left(&tree) {
        Some(v) => Ok(Some(v.to_string())),
        None => Err(too_wide_to_unflatten()),
    }
}

/// `me.utils.unflattenRight`. Bounded exactly like [`unflatten_left`].
#[wasm_bindgen]
pub fn unflatten_right(tree_json: &str) -> Result<Option<String>, JsError> {
    let Ok(tree) = serde_json::from_str::<serde_json::Value>(tree_json) else {
        return Ok(None);
    };
    match crate::js_match::unflatten_right(&tree) {
        Some(v) => Ok(Some(v.to_string())),
        None => Err(too_wide_to_unflatten()),
    }
}

fn too_wide_to_unflatten() -> JsError {
    JsError::new(&format!(
        "unflatten: an associative operator with more than {} operands would \
         produce a tree too deep to serialize or read back",
        crate::js_match::MAX_UNFLATTEN_OPERANDS
    ))
}
