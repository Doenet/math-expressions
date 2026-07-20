//! Symbolic differentiation (PORTING_PLAN.md §15 Phase 8). Each case asserts the
//! derivative is *mathematically* equal (via `equals`) to the expected form —
//! not tree-identical — so canonical-form differences don't matter.

use math_expressions::{derivative, equals, EqOptions, Expr, TextToAst, TextToAstOptions};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

/// Assert d/dx `input` equals `expected`.
fn d(input: &str, expected: &str) {
    let got = derivative(&parse(input), "x");
    assert!(
        equals(&got, &parse(expected), &EqOptions::default()),
        "d/dx {input:?}: got {got:?}, expected {expected:?}",
    );
}

#[test]
fn constants_and_variables() {
    d("5", "0");
    d("pi", "0");
    d("y", "0"); // other variable is constant w.r.t. x
    d("x", "1");
    d("3*x + 7", "3");
}

#[test]
fn sum_product_quotient() {
    d("x^2 + x", "2*x + 1");
    d("x*y", "y");
    d("x^2 * y", "2*x*y");
    d("x*sin(x)", "sin(x) + x*cos(x)"); // product rule
    d("x/y", "1/y");
    d("1/x", "-1/x^2");
    d("(x+1)/(x-1)", "-2/(x-1)^2"); // quotient rule
    d("x^2/(x+1)", "(x^2 + 2*x)/(x+1)^2");
}

#[test]
fn power_rule_cases() {
    d("x^3", "3*x^2");
    d("x^n", "n*x^(n-1)"); // symbolic constant exponent
    d("(x^2+1)^5", "10*x*(x^2+1)^4"); // chain rule
    d("exp(x)", "exp(x)");
    d("e^x", "e^x");
    d("e^(x^2)", "2*x*e^(x^2)");
    d("2^x", "2^x * log(2)"); // constant base
    d("x^x", "x^x * (log(x) + 1)"); // general u^v
}

#[test]
fn chain_rule_and_function_table() {
    d("sin(x^2)", "2*x*cos(x^2)");
    d("cos(x)", "-sin(x)");
    d("tan(x)", "sec(x)^2");
    d("log(x)", "1/x");
    d("log(x^2+1)", "2*x/(x^2+1)");
    d("sqrt(x)", "1/(2*sqrt(x))");
    d("sqrt(x^2+1)", "x/sqrt(x^2+1)");
    d("sinh(x)", "cosh(x)");
    d("cosh(x)", "sinh(x)");
    d("tanh(x)", "sech(x)^2");
    d("asin(x)", "1/sqrt(1-x^2)");
    d("arctan(x)", "1/(1+x^2)"); // arc-spelling aliases atan
    d("abs(x)", "abs(x)/x");
}

#[test]
fn nested_composition() {
    d("sin(cos(x))", "-sin(x)*cos(cos(x))");
    d("exp(sin(x))", "cos(x)*exp(sin(x))");
    d("log(sin(x))", "cos(x)/sin(x)");
}

#[test]
fn other_variable() {
    // Differentiate w.r.t. t, not x.
    let got = derivative(&parse("t^2 + x*t"), "t");
    assert!(equals(&got, &parse("2*t + x"), &EqOptions::default()));
}

#[test]
fn undifferentiable_shapes_are_opaque_not_zero() {
    // Shapes with no rule (tuples, relations, subscripts) must NOT claim
    // derivative 0 — they become an opaque derivative(…) node, which never
    // compares equal to 0 (and two identical ones compare equal).
    for input in ["(x, x^2)", "x < 2", "x_1"] {
        let d = derivative(&parse(input), "x");
        assert!(
            !equals(&d, &parse("0"), &EqOptions::default()),
            "derivative of {input:?} wrongly equals 0: {d:?}"
        );
        let d2 = derivative(&parse(input), "x");
        assert!(equals(&d, &d2, &EqOptions::default()));
    }
}
