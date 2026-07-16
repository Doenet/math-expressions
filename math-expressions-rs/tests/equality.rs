//! Equality-tester cases, mirroring the equivalence pairs in the JS
//! `slow_math-expressions.spec.js`. Equal pairs resolve either at the exact
//! canonical stage (stage 1) or by numerical sampling (stage 3).

use math_expressions::{equals, EqOptions, Expr, TextToAst, TextToAstOptions};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn eq(a: &str, b: &str) -> bool {
    equals(&parse(a), &parse(b), &EqOptions::default())
}

#[test]
fn exact_stage_equalities() {
    // These resolve structurally (stage 1), no sampling needed.
    assert!(eq("1/3 + 1/6", "1/2"));
    assert!(eq("0.1 + 0.2", "0.3"));
    assert!(eq("x + x", "2x"));
    assert!(eq("x + y", "y + x"));
    assert!(eq("a*b*c", "c*a*b"));
    assert!(eq("x*x", "x^2"));
    assert!(eq("x - x", "0"));
    assert!(eq("2*3 + 4", "10"));
}

#[test]
fn numerical_stage_equalities() {
    // Algebraic identities canonicalize differently but agree numerically.
    assert!(eq("(x+1)^2", "x^2 + 2x + 1"));
    assert!(eq("x^2 - 1", "(x-1)(x+1)"));
    assert!(eq("(x+y)^2", "x^2 + 2x y + y^2"));
    assert!(eq("sin^2 x + cos^2 x", "1"));
    assert!(eq("2 sin(x) cos(x)", "sin(2x)"));
    assert!(eq("exp(x) exp(y)", "exp(x+y)"));
}

#[test]
fn inequalities() {
    assert!(!eq("x", "y"));
    assert!(!eq("x + 1", "x + 2"));
    assert!(!eq("1/3", "1/2"));
    assert!(!eq("sin(x)", "cos(x)"));
    assert!(!eq("(x+1)^2", "x^2 + 1"));
    assert!(!eq("x^2", "x^3"));
    assert!(!eq("2x", "3x"));
}

#[test]
fn exactness_beats_float_slop() {
    // §3a payoff: these are distinct exact integers even though they collapse
    // to the same f64. The JS float path calls them equal; we do not.
    assert!(!eq("10^20 + 1", "10^20 + 2"));
    // ...but the genuinely-equal huge integers still compare equal.
    assert!(eq("10^20 + 1", "1 + 10^20"));
}

#[test]
fn blanks_are_never_equal() {
    // A missing operand (`x^` → x^blank) makes equality undefined.
    assert!(!eq("x^", "x^"));
    let opts = EqOptions {
        allow_blanks: true,
        ..EqOptions::default()
    };
    // With allow_blanks, identical blank-bearing trees compare structurally.
    assert!(equals(&parse("x^"), &parse("x^"), &opts));
}

#[test]
fn commutativity_and_tuple_coercion() {
    // Tuple/array coercion on by default.
    assert!(eq("(1, 2)", "[1, 2]"));
    let no_coerce = EqOptions {
        coerce_tuples_arrays: false,
        ..EqOptions::default()
    };
    assert!(!equals(&parse("(1,2)"), &parse("[1,2]"), &no_coerce));
}
