//! Port of `spec/quick_trees.spec.js` — the raw JS-tree utility surface that
//! Doenet uses via `me.utils`: structural `equal`, `flatten`/`unflatten{Left,
//! Right}`, `substitute`, and default-mode template `match`. The Rust homes are
//! `js_match` (match/flatten/unflatten, operating on `serde_json::Value` trees),
//! `js_tree::to_js` (structural equality), and the crate `substitute` (on `Expr`).
//!
//! Only the **default** match mode is ported — the opt-in JS params
//! (`variables`, regex/function conditions, `allow_permutations`,
//! `allow_extended_match`) are deliberately unported (see `src/js_match.rs` docs
//! and JS_TEST_COVERAGE_AUDIT.md), so those spec cases are out of scope here.

use math_expressions::js_match::{flatten_tree, match_template, unflatten_left, unflatten_right};
use math_expressions::js_tree::to_js;
use math_expressions::{equals, substitute, EqOptions, Expr, TextToAst, TextToAstOptions};
use serde_json::{json, Value};
use std::collections::HashMap;

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

/// The JS `TREE(s)` helper: parse text and take the raw JS tree.
fn tree(s: &str) -> Value {
    to_js(&parse(s))
}

/// Structural tree equality (JS `trees.equal`) is JSON identity of the encoding.
fn equal(a: &Value, b: &Value) -> bool {
    a == b
}

fn eq_expr(a: &Expr, b: &Expr) -> bool {
    equals(a, b, &EqOptions::default())
}

// ---- tree basics ----

#[test]
fn structural_equality_is_exact_and_order_sensitive() {
    assert!(equal(&tree("cos x"), &tree("cos x")));
    assert!(!equal(&tree("cos x"), &tree("cos y")));
    // Structural equality does NOT allow order changes (that is `equals`).
    assert!(!equal(&tree("x+y"), &tree("y+x")));
}

#[test]
fn flatten_and_unflatten() {
    // unflattenRight: ["+",1,2,3] -> ["+",1,["+",2,3]]
    assert_eq!(unflatten_right(&json!(["+", 1, 2, 3])), json!(["+", 1, ["+", 2, 3]]));
    // unflattenLeft: ["+",1,2,3] -> ["+",["+",1,2],3]
    assert_eq!(unflatten_left(&json!(["+", 1, 2, 3])), json!(["+", ["+", 1, 2], 3]));
    // flatten both nestings back to the n-ary form.
    assert_eq!(flatten_tree(&json!(["+", 1, ["+", 2, 3]])), json!(["+", 1, 2, 3]));
    assert_eq!(flatten_tree(&json!(["+", ["+", 1, 2], 3])), json!(["+", 1, 2, 3]));
}

#[test]
fn substitute_symbols() {
    let sub = |e: &str, pairs: &[(&str, Expr)]| {
        let map: HashMap<String, Expr> =
            pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
        substitute(&parse(e), &map)
    };

    // x+y becomes 1+2 when x:=1 and y:=2
    assert!(eq_expr(
        &sub("x+y", &[("x", parse("1")), ("y", parse("2"))]),
        &parse("1+2")
    ));
    // simultaneous swap: x := y^2 and y := x^2
    assert!(eq_expr(
        &sub("x+y", &[("x", parse("y^2")), ("y", parse("x^2"))]),
        &parse("y^2 + x^2")
    ));
    // recurses through apply / div
    assert!(eq_expr(
        &sub("cos(x+y)/sin(x*y)", &[("x", parse("1")), ("y", parse("2"))]),
        &parse("cos(1+2)/sin(1*2)")
    ));
    // recurses through relations (chained inequality)
    assert!(eq_expr(
        &sub("x < y < z", &[("x", parse("a")), ("y", parse("b")), ("z", parse("c"))]),
        &parse("a < b < c")
    ));
    assert!(eq_expr(
        &sub("x < y <= z", &[("x", parse("a")), ("y", parse("b")), ("z", parse("c"))]),
        &parse("a < b <= c")
    ));
}

// ---- default-mode template matching ----

#[test]
fn match_binds_wildcards() {
    let m = match_template(&tree("x+y"), &tree("a+b")).expect("x+y matches a+b");
    assert_eq!(m.get("a"), Some(&json!("x")));
    assert_eq!(m.get("b"), Some(&json!("y")));
}

#[test]
fn match_requires_same_operator_and_whole_tree() {
    // x+y does not match a*b
    assert!(match_template(&tree("x+y"), &tree("a*b")).is_none());
    // a wildcard match must cover the entire tree
    assert!(match_template(&tree("x+y/z"), &tree("a/b")).is_none());
}

#[test]
fn match_must_be_consistent() {
    // x+y/z matches a+b/c (all distinct) ...
    assert!(match_template(&tree("x+y/z"), &tree("a+b/c")).is_some());
    // ... but not a+b/a (would need y/z's numerator == denominator)
    assert!(match_template(&tree("x+y/z"), &tree("a+b/a")).is_none());
    // x+y/x DOES match a+b/a (x bound consistently)
    assert!(match_template(&tree("x+y/x"), &tree("a+b/a")).is_some());
}

#[test]
fn match_multichar_placeholders_and_exact_numbers() {
    // multi-character pattern leaves are still wildcards by default
    assert!(match_template(&json!(["+", "x", "y"]), &json!(["+", "a", "bc"])).is_some());
    assert!(match_template(&json!(["+", "x", "bc"]), &json!(["+", "a", "bc"])).is_some());
    // numbers must match exactly
    assert!(match_template(&tree("3x+5"), &tree("ab+5")).is_some());
    assert!(match_template(&tree("3x+5"), &tree("ab+6")).is_none());
}

#[test]
fn match_addition_matches_subtraction_not_vice_versa() {
    // x-y is ["+","x",["-","y"]]; a wildcard b absorbs the negated term.
    assert!(match_template(&tree("x-y"), &tree("a+b")).is_some());
    // but x+y cannot match a-b (the second operand must be a negation)
    assert!(match_template(&tree("x+y"), &tree("a-b")).is_none());
}
