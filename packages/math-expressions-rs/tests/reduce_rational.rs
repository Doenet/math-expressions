//! `reduce_rational` (§8 polynomial GCD consumer). Expected values are the JS
//! oracle outputs probed from `me.reduce_rational()`; comparison canonicalizes
//! both sides (the reduction must actually change the tree, not just be
//! mathematically equal — the presented output canonicalizes back to the
//! reduced form, never to the unreduced input).

use math_expressions::{canonicalize, equals, reduce_rational, EqOptions, Expr, TextToAst, TextToAstOptions};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

/// Assert reduce_rational(input) is canonically identical to `expected`.
fn red(input: &str, expected: &str) {
    let got = canonicalize(&reduce_rational(&parse(input)));
    let want = canonicalize(&parse(expected));
    assert_eq!(
        got, want,
        "reduce_rational({input:?}):\n  got  {got:?}\n  want {want:?}"
    );
}

/// Assert the input is left unchanged (still equal to its canonical form).
fn unchanged(input: &str) {
    let got = canonicalize(&reduce_rational(&parse(input)));
    let want = canonicalize(&parse(input));
    assert_eq!(got, want, "reduce_rational({input:?}) should be unchanged");
}

#[test]
fn univariate_cancellation() {
    red("(x^2-1)/(x-1)", "x+1");
    red("(x^2-4)/(x-2)", "x+2");
    red("(x^2+2*x+1)/(x+1)", "x+1");
    red("(x^3-1)/(x-1)", "x^2+x+1");
    red("(x^2-1)/(x+1)", "x-1");
    red("(2*x^2+4*x)/(2*x)", "x+2");
    red("(x^2-5*x+6)/(x^2-4)", "(x-3)/(x+2)");
}

#[test]
fn multivariate_cancellation() {
    red("(x^2-y^2)/(x-y)", "x+y");
    red("(x^2*y+x*y^2)/(x*y)", "x+y");
}

#[test]
fn irreducible_and_nonpolynomial_unchanged() {
    unchanged("(x+1)/(x+2)");
    unchanged("sin(x)/x");
    unchanged("x/(y+1)");
    unchanged("pi/x");
}

#[test]
fn reduces_nested_positions() {
    // Bottom-up: a reducible fraction inside a sum reduces in place.
    let got = canonicalize(&reduce_rational(&parse("1 + (x^2-1)/(x-1)")));
    let want = canonicalize(&parse("x + 2"));
    assert_eq!(got, want);
}

#[test]
fn value_is_preserved() {
    for s in [
        "(x^2-1)/(x-1)",
        "(x^2-5*x+6)/(x^2-4)",
        "(x^2-y^2)/(x-y)",
        "(2*x^2+4*x)/(2*x)",
    ] {
        let got = reduce_rational(&parse(s));
        assert!(
            equals(&got, &parse(s), &EqOptions::default()),
            "reduction changed the value of {s:?}"
        );
    }
}
