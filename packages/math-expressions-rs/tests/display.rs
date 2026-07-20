//! Display form of user-facing operations (`norm::present`): simplified
//! output should read like a calculus student's "simplest form" — polynomial
//! term order (descending degree, constants last), division instead of
//! negative exponents, explicit minus signs — while staying canonically equal
//! to the plain-canonical result.

use math_expressions::{
    derivative, equals, expand, reduce_rational, simplify, to_text, EqOptions, Expr, TextOpts,
    TextToAst, TextToAstOptions,
};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn txt(e: &Expr) -> String {
    to_text(e, &TextOpts::default())
}

fn simp(s: &str) -> String {
    txt(&simplify(&parse(s)))
}

#[test]
fn polynomials_order_by_descending_degree() {
    assert_eq!(simp("1 + 2x + x^2"), "x^2 + 2 x + 1");
    assert_eq!(simp("x^2 + 2x + 1"), "x^2 + 2 x + 1");
    assert_eq!(simp("x^3 + x"), "x^3 + x");
    assert_eq!(simp("x^2 - 2x"), "x^2 - 2 x");
    // Every symbol counts toward the degree, so parameter-coefficient forms
    // read conventionally too (matches the JS oracle).
    assert_eq!(simp("c x^2 + b x + a"), "c x^2 + b x + a");
}

#[test]
fn ties_break_graded_lexicographically() {
    assert_eq!(simp("x^2 + x y + y^2"), "x^2 + x y + y^2");
    assert_eq!(simp("z^2 + x y"), "x y + z^2");
    assert_eq!(simp("y x + x^2 y^3"), "x^2 y^3 + x y");
}

#[test]
fn functions_and_constants_sort_with_degree_zero() {
    assert_eq!(simp("sin(x) + x^2"), "x^2 + sin(x)");
    assert_eq!(simp("x + sin(x)"), "x + sin(x)");
    assert_eq!(simp("e^x + x^2"), "x^2 + e^x");
    // Negative exponents are negative degrees: constants sort above them.
    assert_eq!(simp("2 + x^(-3)"), "2 + 1/x^3");
    assert_eq!(simp("x^(-1) + x"), "x + 1/x");
}

#[test]
fn negative_exponents_display_as_division() {
    assert_eq!(simp("1/x"), "1/x");
    assert_eq!(simp("3/x^2"), "3/x^2");
    assert_eq!(simp("x/y"), "x/y");
    assert_eq!(simp("2/(3x)"), "2/(3 x)");
    assert_eq!(simp("a/(b c)"), "a/(b c)");
    assert_eq!(simp("-x^(-2)"), "-1/x^2");
    assert_eq!(simp("1/(2 sqrt(x))"), "1/(2 sqrt(x))");
    // Symbolic negative exponents too.
    assert_eq!(simp("x^(-n)"), "1/x^n");
}

#[test]
fn rational_coefficients_join_the_fraction_bar() {
    assert_eq!(simp("x/2"), "x/2");
    assert_eq!(simp("-x/2"), "-x/2");
    assert_eq!(simp("-(x+1)/2"), "-(x + 1)/2");
    assert_eq!(simp("3/2 x"), "3 x/2");
}

#[test]
fn fractional_exponents_stay_fractions() {
    assert_eq!(simp("x^(3/2)"), "x^(3/2)");
    assert_eq!(simp("x^(2/3) + x^(1/2)"), "x^(2/3) + x^(1/2)");
    // But additive rational constants keep the §3a decimal round-trip.
    assert_eq!(simp("0.5 + x"), "x + 0.5");
}

#[test]
fn mul_factors_sort_alphabetically_by_base() {
    assert_eq!(simp("y x^2"), "x^2 y");
    assert_eq!(simp("z y x"), "x y z");
}

#[test]
fn other_presented_surfaces() {
    // expand, derivative, and reduce_rational share the display pass.
    assert_eq!(txt(&expand(&parse("(x+1)^2"))), "x^2 + 2 x + 1");
    assert_eq!(txt(&derivative(&parse("1/x"), "x")), "-1/x^2");
    assert_eq!(
        txt(&reduce_rational(&parse("(x^2-5x+6)/(x^2-4)"))),
        "(x - 3)/(x + 2)"
    );
}

/// The pass is display-only: canonically the presented tree is the same
/// expression, and presenting is idempotent.
#[test]
fn presentation_is_sound_and_idempotent() {
    for s in [
        "1 + 2x + x^2",
        "2/(3x)",
        "x^(-1) + x",
        "-(x+1)/2",
        "x^(3/2)",
        "c x^2 + b x + a",
        "sin(x)/cos(x)",
        "x^(1-n)",
        "(x^2-1)/(x-1)",
        "2 - x",
        "pi + x",
    ] {
        let e = parse(s);
        let simp = simplify(&e);
        assert!(
            equals(&e, &simp, &EqOptions::default()),
            "simplify changed meaning of {s:?}: {}",
            txt(&simp)
        );
        assert_eq!(
            simplify(&simp),
            simp,
            "simplify not idempotent on {s:?}: {}",
            txt(&simp)
        );
    }
}
