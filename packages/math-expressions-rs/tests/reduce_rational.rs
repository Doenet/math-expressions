//! `reduce_rational` (§8 polynomial GCD consumer). Expected values are the JS
//! oracle outputs probed from `me.reduce_rational()`; comparison canonicalizes
//! both sides (the reduction must actually change the tree, not just be
//! mathematically equal — the presented output canonicalizes back to the
//! reduced form, never to the unreduced input).

use math_expressions::{
    canonicalize, equals, reduce_rational, EqOptions, Expr, TextToAst, TextToAstOptions,
};

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
fn the_leftover_unit_does_not_land_in_the_denominator() {
    // A gcd is only defined up to a unit, and the sign convention is fixed by
    // the *main* variable — so which variable sorts first used to decide
    // whether the leftover `−1` ended up on top or underneath. Written with `y`
    // first, both sides lead negatively in `x` and this returned `−(−x − y)`:
    // the right value, spelled in a way nobody would accept. Same expression,
    // same reduction, different letters — that is the tell.
    red("(y^2-x^2)/(y-x)", "x+y");
    red("(b^2-a^2)/(b-a)", "a+b");
    red("(x^2-y^2)/(x-y)", "x+y");
    // Genuinely negative denominators keep their sign on the numerator rather
    // than being flipped away.
    red("(x^2-1)/(1-x)", "-x-1");
}

#[test]
fn constants_and_functions_cancel_as_opaque_kernels() {
    // The ring is over ℚ in named variables, so `e`, `π` and `cos x` are none
    // of coefficient, variable, or anything else it recognizes — and the
    // converter used to refuse the whole fraction on account of them. But
    // cancellation is a polynomial *identity*, and identities survive
    // specialization, so it costs nothing to let each be an indeterminate.
    // Until it did, `(a+b)(c+d) / ((e+f)(c+d))` came back uncancelled for no
    // better reason than the letter `e`.
    red("((a+b)(c+d))/((e+f)(c+d))", "(a+b)/(e+f)");
    red("(e*x + e)/e", "x+1");
    red("(x^2 - pi^2)/(x - pi)", "x + pi");
    red("(sin(x)^2 - 1)/(sin(x) - 1)", "sin(x) + 1");
    red(
        "((a+cos(x))(c+sin(y)))/((e+atan(z))(c+sin(y)))",
        "(a+cos(x))/(e+atan(z))",
    );
}

#[test]
fn kernels_are_independent_of_each_other() {
    // Each distinct opaque subtree is its *own* indeterminate, which is what
    // makes the identity argument work — and which is why relations among them
    // are invisible. That costs completeness, never correctness: the failure
    // mode is a cancellation missed, not one invented.
    unchanged("sin(x)/sin(y)");
    unchanged("(sin(x)+1)/(cos(x)+1)");
    // `sin²+cos² = 1` would make this `1/(sin(x)+1)`. We do not see it.
    unchanged("(sin(x)^2 + cos(x)^2)/(sin(x)+1)");
    // Nor is `i` a root of `t²+1` here — it is a free indeterminate like any
    // other kernel, so `x−i` is left on the table. Being transcendental is what
    // makes `π` and `e` lose nothing this way; `i` is algebraic, and does.
    unchanged("(x^2+1)/(x+i)");
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
        "(x^2 - pi^2)/(x - pi)",
        "(sin(x)^2 - 1)/(sin(x) - 1)",
        "((a+b)(c+d))/((e+f)(c+d))",
    ] {
        let got = reduce_rational(&parse(s));
        assert!(
            equals(&got, &parse(s), &EqOptions::default()),
            "reduction changed the value of {s:?}"
        );
    }
}
