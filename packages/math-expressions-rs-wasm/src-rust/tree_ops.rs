//! Core operations on a JS-tree AST that the compat layer needs *without* the
//! canonicalization an `Expression` method would bring with it (JSON in, JSON
//! out, per [`interop`](crate::interop)).
//!
//! Every one of these backs a piece of the assumption store or the compat
//! polynomial engine, which manipulate raw trees and must get back the tree
//! they handed in, rearranged only in the one way they asked for. That is why
//! they are free functions over the AST rather than methods: an `Expression`
//! pipeline would be free to normalize on the way through.
//!
//! An unreadable tree returns `undefined` rather than throwing. These run
//! inside normalization passes on trees built by JS-side surgery, and the
//! callers all have a "leave it as it is" branch; an exception there would turn
//! a tree the core simply does not model into a failed operation.

use math_expressions::expr::serde::{to_js, try_from_js};
use math_expressions::Expr;
use wasm_bindgen::prelude::*;

/// Parse a JS-tree AST, or `None` if the core cannot read it.
fn read(tree_json: &str) -> Option<Expr> {
    try_from_js(&serde_json::from_str::<serde_json::Value>(tree_json).ok()?).ok()
}

fn write(e: &Expr) -> String {
    to_js(e).to_string()
}

/// Order two trees by the JS library's `default_order` sort key: negative if
/// `a` sorts first, positive if `b` does, 0 if they tie. The port of the legacy
/// `trees/default_order.js` `compare_function`.
///
/// This is deliberately the *legacy* key rather than the core's own canonical
/// order: it decides which variable a multivariate polynomial is written in,
/// and that choice is observable in the polynomial ASTs the compat API returns.
#[wasm_bindgen]
pub fn cmp_default_order(a_json: &str, b_json: &str) -> Option<i32> {
    let (a, b) = (read(a_json)?, read(b_json)?);
    Some(match math_expressions::cmp_default_order(&a, &b) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    })
}

/// Push every `not` inward — De Morgan, double-negation collapse, and negation
/// of relations — and nothing else.
///
/// The `Expression::simplify_logical` method does this *and* simplifies and
/// canonicalizes, which restates `x > a` as `a < x`. The assumption store files
/// a fact under the variable the caller put on the left, so it needs the
/// pushdown on its own.
#[wasm_bindgen]
pub fn push_not_ast(tree_json: &str) -> Option<String> {
    Some(write(&math_expressions::push_not(&read(tree_json)?)))
}

/// Merge nested `and`/`or` into their parent and drop a connective left with a
/// single operand.
#[wasm_bindgen]
pub fn flatten_logical_ast(tree_json: &str) -> Option<String> {
    Some(write(&math_expressions::flatten_logical(&read(tree_json)?)))
}

/// Restate a relation with `variable` alone on the left (`a + b < 1` under `a`
/// becomes `a < 1 - b`). `undefined` when it is not linear in `variable`, or
/// when the coefficient's sign — which decides whether an inequality flips — is
/// not known.
///
/// No assumptions are consulted: the store calls this while deciding what to
/// file, so a conclusion drawn from the facts already on file would depend on
/// insertion order.
#[wasm_bindgen]
pub fn solve_linear_ast(tree_json: &str, variable: &str) -> Option<String> {
    let e = read(tree_json)?;
    math_expressions::solve_linear(&e, variable, &math_expressions::Assumptions::new())
        .as_ref()
        .map(write)
}

/// Decompose a tree as `b + Σ aᵢ·vᵢ` over `variables`, with every coefficient
/// and `b` a plain number. Returns `{"b": tree, "coefficients": [tree, …]}`
/// with the coefficients in the order the variables were given, or `undefined`
/// when the tree is not of that form.
#[wasm_bindgen]
pub fn linear_decomposition_ast(tree_json: &str, variables: Vec<String>) -> Option<String> {
    let e = read(tree_json)?;
    let (coefficients, b) = math_expressions::linear_decomposition(&e, &variables)?;
    Some(
        serde_json::json!({
            "b": to_js(&b),
            "coefficients": coefficients.iter().map(to_js).collect::<Vec<_>>(),
        })
        .to_string(),
    )
}
