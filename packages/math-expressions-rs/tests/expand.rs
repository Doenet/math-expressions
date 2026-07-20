//! Expansion (`expand`). Each case asserts the expanded form is mathematically
//! equal (via `equals`) to the expected polynomial — not tree-identical.

use math_expressions::{equals, expand, EqOptions, Expr, TextToAst, TextToAstOptions};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn ex(input: &str, expected: &str) {
    let got = expand(&parse(input));
    assert!(
        equals(&got, &parse(expected), &EqOptions::default()),
        "expand {input:?}: got {got:?}, expected {expected:?}",
    );
}

#[test]
fn products_of_sums() {
    ex("(x+1)*(x+2)", "x^2 + 3*x + 2");
    ex("(a+1)*x", "a*x + x");
    ex("x*(y+z)", "x*y + x*z");
    ex("(x+y)*(x-y)", "x^2 - y^2");
    ex("(x+1)*(x+2)*(x+3)", "x^3 + 6*x^2 + 11*x + 6");
    ex("2*(x+3)", "2*x + 6");
}

#[test]
fn integer_powers_of_sums() {
    ex("(x+1)^2", "x^2 + 2*x + 1");
    ex("(x+1)^3", "x^3 + 3*x^2 + 3*x + 1");
    ex("(x+y)^2", "x^2 + 2*x*y + y^2");
    ex("((x+1)*(x+2))^2", "x^4 + 6*x^3 + 13*x^2 + 12*x + 4");
    ex("(x-1)^4", "x^4 - 4*x^3 + 6*x^2 - 4*x + 1");
}

#[test]
fn negation_and_division() {
    ex("-(x+1)", "-x - 1");
    ex("(x+1)/(x+2)", "x/(x+2) + 1/(x+2)");
    ex("(x^2+2*x+1)/(x+1)", "x^2/(x+1) + 2*x/(x+1) + 1/(x+1)");
}

#[test]
fn recurses_into_arguments() {
    ex("sin((x+1)*(x+2))", "sin(x^2 + 3*x + 2)");
    ex("exp((x+1)^2)", "exp(x^2 + 2*x + 1)");
}

#[test]
fn leaves_non_integer_and_negative_powers_intact() {
    // These are not distributed; expand must not change their value.
    ex("(x+1)^(-1)", "1/(x+1)");
    ex("(x+1)^(1/2)", "(x+1)^(1/2)");
    ex("x + 1", "x + 1");
}

#[test]
fn expansion_is_bounded_on_adversarial_input() {
    // Combining after each factor keeps powers of small sums cheap even at the
    // exponent cap: (a+b)^64 is 65 combined terms, never 2^64 clones. (This
    // exact shape once froze a dev container via the uncombined Cartesian
    // product — keep it fast.)
    let e = expand(&parse("(a+b)^64"));
    match &e {
        Expr::Add(ts) => assert_eq!(ts.len(), 65),
        other => panic!("expected expanded sum, got {other:?}"),
    }

    // A product of 13 distinct binomials would have 2^13 = 8192 distinct
    // monomials (> the term cap): expand must bail out and return the
    // unexpanded product — same value, bounded memory — not hang.
    let input = "(a+b)*(c+d)*(e+f)*(g+h)*(i+j)*(k+l)*(m+n)*(o+p)*(q+r)*(s+t)*(u+v)*(w+y)*(z+x)";
    let e = expand(&parse(input));
    assert!(
        matches!(e, Expr::Mul(_)),
        "expected unexpanded product fallback, got {e:?}"
    );
}

#[test]
fn division_denominator_stays_factored() {
    // The denominator is not expanded: 1/((x+1)(x+2)) keeps its factored
    // denominator (as x^-1-style factors), it does NOT become 1/(x^2+3x+2).
    let e = expand(&parse("1/((x+1)*(x+2))"));
    let text = math_expressions::to_text(&e, &Default::default());
    assert!(
        !text.contains("3 x") && !text.contains("x^2"),
        "denominator was expanded: {text}"
    );
    // Value is unchanged.
    ex("1/((x+1)*(x+2))", "1/((x+1)*(x+2))");
}
