//! Regression tests for formatter correctness fixes: logical-operator
//! parenthesization, power-tower parens, LaTeX `%` escaping, Leibniz spacing,
//! and `perp` in text. Trees are built from the JS AST shape via `from_js`.

use math_expressions::js_tree::from_js;
use math_expressions::{to_latex, to_text, LatexOpts, TextOpts};
use serde_json::json;

fn lx(tree: serde_json::Value) -> String {
    to_latex(&from_js(&tree), &LatexOpts::default())
}
fn tx(tree: serde_json::Value) -> String {
    to_text(&from_js(&tree), &TextOpts::default())
}

#[test]
fn lnot_parenthesizes_compound_operand() {
    // `¬(x = y)`, not the ambiguous `¬x = y`.
    assert_eq!(lx(json!(["not", ["=", "x", "y"]])), r"\lnot \left(x = y\right)");
    assert_eq!(tx(json!(["not", ["=", "x", "y"]])), "¬(x = y)");
    // a bare atom operand keeps no parens
    assert_eq!(lx(json!(["not", "A"])), r"\lnot A");
}

#[test]
fn logical_connectives_parenthesize_compound_operands() {
    assert_eq!(lx(json!(["or", ["and", "A", "B"], "C"])), r"\left(A \land B\right) \lor C");
    assert_eq!(tx(json!(["or", ["and", "A", "B"], "C"])), "(A and B) or C");
    assert_eq!(lx(json!(["or", "A", ["and", "B", "C"]])), r"A \lor \left(B \land C\right)");
}

#[test]
fn power_tower_parenthesizes_inner_power() {
    // bare `x^{y}^{z}` is invalid LaTeX (double superscript)
    assert_eq!(lx(json!(["^", ["^", "x", "y"], "z"])), r"\left(x^{y}\right)^{z}");
    assert_eq!(tx(json!(["^", ["^", "x", "y"], "z"])), "(x^y)^z");
    // a plain power is untouched
    assert_eq!(lx(json!(["^", "x", "y"])), r"x^{y}");
}

#[test]
fn latex_escapes_percent_and_dollar() {
    // a bare `%` starts a LaTeX comment — must be escaped
    assert_eq!(lx(json!("%")), r"\%");
    assert_eq!(lx(json!(["unit", "x", "%"])), r"x \%");
    assert_eq!(lx(json!("$")), r"\$");
}

#[test]
fn leibniz_has_no_double_space() {
    assert_eq!(
        lx(json!(["partial_derivative_leibniz", "x", ["tuple", "t"]])),
        r"\frac{\partial x}{\partial t}"
    );
    assert_eq!(
        lx(json!(["derivative_leibniz", "x", ["tuple", "t"]])),
        r"\frac{dx}{dt}"
    );
}

#[test]
fn perp_renders_as_unicode_in_text() {
    assert_eq!(tx(json!(["^", "x", "perp"])), "x^⟂");
    assert_eq!(tx(json!(["_", "x", "perp"])), "x_⟂");
}

#[test]
fn radical_raised_to_a_power_is_parenthesized() {
    assert_eq!(lx(json!(["^", ["apply", "sqrt", 2], 3])), r"\left(\sqrt{2}\right)^{3}");
    assert_eq!(lx(json!(["^", ["apply", "cbrt", 2], 3])), r"\left(\sqrt[3]{2}\right)^{3}");
    assert_eq!(
        lx(json!(["^", ["apply", "nthroot", ["tuple", 2, 4]], 3])),
        r"\left(\sqrt[4]{2}\right)^{3}"
    );
    // a plain radical (not raised) keeps no parens
    assert_eq!(lx(json!(["apply", "sqrt", 2])), r"\sqrt{2}");
}

#[test]
fn units_in_a_product_are_parenthesized() {
    assert_eq!(lx(json!(["*", ["unit", "x", "%"], "y"])), r"\left(x \%\right) y");
    assert_eq!(lx(json!(["*", ["unit", "$", "x"], "y"])), r"\left(\$ x\right) y");
    assert_eq!(lx(json!(["*", ["unit", "x", "deg"], "y"])), r"\left(x^{\circ}\right) y");
    assert_eq!(tx(json!(["*", ["unit", "x", "%"], "y"])), "(x %) y");
    // a standalone unit keeps no parens; `$` gets a space
    assert_eq!(lx(json!(["unit", "$", "x"])), r"\$ x");
    assert_eq!(tx(json!(["unit", "$", "x"])), "$ x");
}
