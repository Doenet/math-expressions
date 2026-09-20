//! Regression tests for formatter correctness fixes: logical-operator
//! parenthesization, power-tower parens, LaTeX `%` escaping, Leibniz spacing,
//! `perp` in text, and the bracket notations applied to a tuple. Trees are
//! built from the JS AST shape via `try_from_js`.

use math_expressions::expr::serde::try_from_js;
use math_expressions::{to_latex, to_text, LatexOpts, TextOpts};
use serde_json::json;

fn lx(tree: serde_json::Value) -> String {
    to_latex(
        &try_from_js(&tree).expect("fixture tree"),
        &LatexOpts::default(),
    )
}
fn tx(tree: serde_json::Value) -> String {
    to_text(
        &try_from_js(&tree).expect("fixture tree"),
        &TextOpts::default(),
    )
}

#[test]
fn lnot_parenthesizes_compound_operand() {
    // `¬(x = y)`, not the ambiguous `¬x = y`.
    assert_eq!(
        lx(json!(["not", ["=", "x", "y"]])),
        r"\lnot \left(x = y\right)"
    );
    assert_eq!(tx(json!(["not", ["=", "x", "y"]])), "¬(x = y)");
    // a bare atom operand keeps no parens
    assert_eq!(lx(json!(["not", "A"])), r"\lnot A");
}

#[test]
fn logical_connectives_parenthesize_compound_operands() {
    assert_eq!(
        lx(json!(["or", ["and", "A", "B"], "C"])),
        r"\left(A \land B\right) \lor C"
    );
    assert_eq!(tx(json!(["or", ["and", "A", "B"], "C"])), "(A and B) or C");
    assert_eq!(
        lx(json!(["or", "A", ["and", "B", "C"]])),
        r"A \lor \left(B \land C\right)"
    );
}

#[test]
fn power_tower_parenthesizes_inner_power() {
    // bare `x^{y}^{z}` is invalid LaTeX (double superscript)
    assert_eq!(
        lx(json!(["^", ["^", "x", "y"], "z"])),
        r"\left(x^{y}\right)^{z}"
    );
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
    assert_eq!(
        lx(json!(["^", ["apply", "sqrt", 2], 3])),
        r"\left(\sqrt{2}\right)^{3}"
    );
    assert_eq!(
        lx(json!(["^", ["apply", "cbrt", 2], 3])),
        r"\left(\sqrt[3]{2}\right)^{3}"
    );
    assert_eq!(
        lx(json!(["^", ["apply", "nthroot", ["tuple", 2, 4]], 3])),
        r"\left(\sqrt[4]{2}\right)^{3}"
    );
    // a plain radical (not raised) keeps no parens
    assert_eq!(lx(json!(["apply", "sqrt", 2])), r"\sqrt{2}");
}

#[test]
fn units_in_a_product_are_parenthesized() {
    assert_eq!(
        lx(json!(["*", ["unit", "x", "%"], "y"])),
        r"\left(x \%\right) y"
    );
    assert_eq!(
        lx(json!(["*", ["unit", "$", "x"], "y"])),
        r"\left(\$ x\right) y"
    );
    assert_eq!(
        lx(json!(["*", ["unit", "x", "deg"], "y"])),
        r"\left(x^{\circ}\right) y"
    );
    assert_eq!(tx(json!(["*", ["unit", "x", "%"], "y"])), "(x %) y");
    // a standalone unit keeps no parens; `$` gets a space
    assert_eq!(lx(json!(["unit", "$", "x"])), r"\$ x");
    assert_eq!(tx(json!(["unit", "$", "x"])), "$ x");
}

/// The bracket notations wrap the whole argument, and in the JS AST a
/// multi-argument application is an application to a *tuple*. Rendering only
/// the one-argument case with brackets and falling through otherwise emitted
/// LaTeX commands that do not exist — `\abs`, and `\sqrt` with no braced
/// argument — for a tree the parsers produce from `abs(x, y)` or `|(x, y)|`.
#[test]
fn bracket_notations_wrap_a_tuple_argument() {
    let xy = json!(["tuple", "x", "y"]);
    assert_eq!(
        lx(json!(["apply", "abs", xy])),
        r"\left|\left( x, y \right)\right|"
    );
    assert_eq!(
        lx(json!(["apply", "floor", ["tuple", "x", "y"]])),
        r"\left\lfloor \left( x, y \right) \right\rfloor"
    );
    assert_eq!(
        lx(json!(["apply", "ceil", ["tuple", "x", "y"]])),
        r"\left\lceil \left( x, y \right) \right\rceil"
    );
    assert_eq!(
        lx(json!(["apply", "sqrt", ["tuple", "x", "y"]])),
        r"\sqrt{\left( x, y \right)}"
    );
    assert_eq!(
        lx(json!(["apply", "cbrt", ["tuple", "x", "y"]])),
        r"\sqrt[3]{\left( x, y \right)}"
    );
    assert_eq!(
        lx(json!(["apply", "factorial", ["tuple", "x", "y"]])),
        r"\left( x, y \right)!"
    );
    // `nthroot` is the exception: its second argument is the index, not part
    // of what the radical wraps.
    assert_eq!(
        lx(json!(["apply", "nthroot", ["tuple", "x", "y"]])),
        r"\sqrt[y]{x}"
    );
    // the one-argument spellings are unchanged
    assert_eq!(lx(json!(["apply", "abs", "x"])), r"\left|x\right|");
    assert_eq!(
        lx(json!(["apply", "floor", "x"])),
        r"\left\lfloor x \right\rfloor"
    );
}

/// `nthroot` renders as a radical at *every* arity — the arity only chooses
/// whether there is an index to raise.
///
/// Only the two-argument spelling has one, so the others are a plain
/// `\sqrt{…}` over the whole argument. Falling through to the generic
/// application form instead printed `\operatorname{nthroot}` for a tree the
/// rest of the engine already treats as a square root: `normalize::
/// canonicalize` rewrites `nthroot(x)` to `sqrt(x)`, so the same expression
/// displayed as a function and compared equal to a radical. Values are the
/// legacy library's, head by head.
#[test]
fn nthroot_is_a_radical_at_every_arity() {
    assert_eq!(
        lx(json!(["apply", "nthroot", ["tuple", "x", 3]])),
        r"\sqrt[3]{x}"
    );
    assert_eq!(lx(json!(["apply", "nthroot", "x"])), r"\sqrt{x}");
    assert_eq!(
        lx(json!(["apply", "nthroot", ["tuple", "x", 3, "z"]])),
        r"\sqrt{\left( x, 3, z \right)}"
    );
    assert_eq!(
        lx(json!(["apply", "nthroot", ["tuple"]])),
        r"\sqrt{\left(  \right)}"
    );
    // a non-tuple sequence is the sole argument, so it goes inside whole
    assert_eq!(
        lx(json!(["apply", "nthroot", ["array", "x", 3]])),
        r"\sqrt{\left[ x, 3 \right]}"
    );
}

/// A radical raised to a power is parenthesized — at every arity, and for
/// every radical head.
///
/// `is_radical` decides this, and it has to agree with the `sqrt`/`cbrt`/
/// `nthroot` arms of the LaTeX writer. It used to be guarded on
/// `args.len() == 1`, which stopped matching those arms once they learned to
/// wrap a multi-argument list; the disagreement is silent, because the radical
/// still renders and only the parentheses go missing.
#[test]
fn a_radical_raised_to_a_power_is_parenthesized_at_every_arity() {
    assert_eq!(
        lx(json!(["^", ["apply", "sqrt", ["tuple", "x", "y"]], 3])),
        r"\left(\sqrt{\left( x, y \right)}\right)^{3}"
    );
    assert_eq!(
        lx(json!(["^", ["apply", "cbrt", ["tuple", "x", "y"]], 3])),
        r"\left(\sqrt[3]{\left( x, y \right)}\right)^{3}"
    );
    assert_eq!(
        lx(json!(["^", ["apply", "nthroot", "x"], 3])),
        r"\left(\sqrt{x}\right)^{3}"
    );
    assert_eq!(
        lx(json!([
            "^",
            ["apply", "nthroot", ["tuple", "x", 3, "z"]],
            3
        ])),
        r"\left(\sqrt{\left( x, 3, z \right)}\right)^{3}"
    );
    // the one-argument spellings are unchanged
    assert_eq!(
        lx(json!(["^", ["apply", "sqrt", "x"], 3])),
        r"\left(\sqrt{x}\right)^{3}"
    );
    assert_eq!(
        lx(json!(["^", ["apply", "nthroot", ["tuple", "x", 3]], 3])),
        r"\left(\sqrt[3]{x}\right)^{3}"
    );
}

/// The *text* writer has the same two bracket notations, and had the same
/// `args.len() == 1` guard on them — so a multi-argument `abs`/`factorial`
/// silently lost its notation and printed as a bare function call. Legacy
/// wrote `|(x, y)|` and `(x, y)!`.
#[test]
fn text_bracket_notations_wrap_a_tuple_argument() {
    assert_eq!(tx(json!(["apply", "abs", ["tuple", "x", "y"]])), "|(x, y)|");
    assert_eq!(
        tx(json!(["apply", "factorial", ["tuple", "x", "y"]])),
        "(x, y)!"
    );
    assert_eq!(tx(json!(["apply", "abs", ["tuple"]])), "|()|");
    // a non-tuple sequence is already the sole argument and is unaffected
    assert_eq!(tx(json!(["apply", "abs", ["array", "x", "y"]])), "|[x, y]|");
    assert_eq!(
        tx(json!(["apply", "abs", ["vector", "x", "y"]])),
        "|(x, y)|"
    );
    // the one-argument spellings are unchanged
    assert_eq!(tx(json!(["apply", "abs", "x"])), "|x|");
    assert_eq!(tx(json!(["apply", "factorial", "x"])), "x!");
}
