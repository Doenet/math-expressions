//! Equality-tester cases, mirroring the equivalence pairs in the JS
//! `slow_math-expressions.spec.js`. Equal pairs resolve either at the exact
//! canonical stage (stage 1) or by numerical sampling (stage 3).

use math_expressions::{equals, equals_syntactic, EqOptions, Expr, TextToAst, TextToAstOptions};

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

#[test]
fn scaling_units() {
    // `%` and `deg` scale away to plain numbers (units.js: `only_scales`), so
    // they are equal to their scaled values under full equality.
    assert!(eq("50%", "1/2"));
    assert!(eq("50% + 1", "1.5"));
    assert!(eq("x%", "x/100"));
    assert!(eq("180 deg", "pi"));

    // `$` is a `prefix` unit that only marks its value: it becomes a free
    // factor, so like-`$` quantities combine (`$3 + $2 = $5`)...
    assert!(eq("$5", "$3+$2"));
    assert!(eq("$5", "$9-$4"));
    assert!(eq("$xy+a$b", "$(xy+ab)"));
    // ...but a `$` quantity is never equal to a bare number.
    assert!(!eq("$5", "5"));
    assert!(!eq("$x", "x"));
}

#[test]
fn scaling_units_stay_syntactically_distinct() {
    // Syntactic (`equalsViaSyntax`) equality does NOT desugar units, so a
    // scaled unit and its numeric value remain structurally different — even
    // though full `equals` treats them as equal above.
    let o = EqOptions::default();
    assert!(!equals_syntactic(&parse("50%"), &parse("1/2"), &o));
    assert!(!equals_syntactic(&parse("180 deg"), &parse("pi"), &o));
}

#[test]
fn equation_and_inequality_equivalence() {
    // Equations compare by standard form up to any nonzero scalar: `a=b` ≡ `c=d`
    // when `a-b` is proportional to `c-d`.
    assert!(eq("5x + 2y = 3", "6-4y = 10x")); // factor -1/2
    assert!(eq("5x + 2y = 3", "-(6-4y) = -10x")); // factor 1/2

    // Inequalities need a *positive* factor: a negative one reverses direction.
    assert!(eq("5q-9z < 2u+9z", "27z -5q > -4u + 5q-9z"));
    assert!(eq("5q-9z <= 2u+9z", "27z -5q >= -4u + 5q-9z"));

    // Same coefficients, opposite direction: factor -1, so not equal.
    assert!(!eq("5q < 9z", "5q > 9z"));
    assert!(!eq("5q <= 9z", "-5q <= -9z"));
    // Different constants are not proportional.
    assert!(!eq("x > 1000", "x > 1001"));
    // A shift by a free constant / tiny number is not proportional either.
    assert!(!eq("e^(10x)=0", "e^(10x)+C=0"));
    assert!(!eq("cos(10x) < 0", "cos(10x)+0.0000001 < 0"));
}

#[test]
fn equation_form_is_preserved_for_syntactic_equality() {
    // The proportional-standard-form equivalence above is a *mathematical*
    // check; it must NOT leak into syntactic equality, so a teacher grading the
    // required form still distinguishes `5x+2y=3` from its rearrangement.
    let o = EqOptions::default();
    assert!(!equals_syntactic(
        &parse("5x + 2y = 3"),
        &parse("6-4y = 10x"),
        &o
    ));
    assert!(!equals_syntactic(
        &parse("5q-9z < 2u+9z"),
        &parse("27z -5q > -4u + 5q-9z"),
        &o
    ));
}

#[test]
fn coercion_reaches_nested_positions() {
    // Sequence coercion must apply inside relations (and other containers),
    // not just at the top level.
    assert!(eq("(1,2) = x", "[1,2] = x"));
    assert!(eq("x = y", "y = x"));
}
