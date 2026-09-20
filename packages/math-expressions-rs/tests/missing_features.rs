//! Core-level coverage for the feature gaps closed against the js-compat suite:
//! differentiating `nthroot`, honoring the sequence-coercion equality flags
//! (and union member matching), and sampling an integer-assumed variable over
//! the integers. The wasm/JS-only gaps (`allow_extended_match`, graceful
//! invalid match conditions) are pinned by their spec files instead.

use math_expressions::{
    derivative, equals, is_integer, Assumptions, EqOptions, Expr, TextToAst, TextToAstOptions,
};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn eq(a: &str, b: &str, opts: &EqOptions) -> bool {
    equals(&parse(a), &parse(b), opts)
}

#[test]
fn nthroot_differentiates_like_the_power_it_denotes() {
    // `nthroot(x, k)` is `x^(1/k)`; on the faithful layer it had no derivative
    // rule and left a formal `nthroot'`. Now it differentiates to the same thing
    // as the explicit power.
    let a = derivative(&parse("nthroot(x, 4) + c"), "x");
    let b = derivative(&parse("x^(1/4)"), "x");
    assert!(
        equals(&a, &b, &EqOptions::default()),
        "d/dx nthroot(x,4) should equal d/dx x^(1/4)"
    );
    // A general index, and one that is not a fourth root.
    let a = derivative(&parse("nthroot(x, 7)"), "x");
    let b = derivative(&parse("x^(1/7)"), "x");
    assert!(equals(&a, &b, &EqOptions::default()));
}

#[test]
fn the_coercion_flags_gate_the_right_pairs() {
    let tuple = "(a,b)";
    // Default: tuple and array coerce together.
    assert!(eq(tuple, "[a,b]", &EqOptions::default()));

    // coerce_tuples_arrays governs tuple↔array; turning it off keeps them
    // distinct even though coerce_vectors is still on (the two flags are
    // independent — the bug this guards against was array/vector riding on the
    // wrong flag).
    let no_ta = EqOptions {
        coerce_tuples_arrays: false,
        ..EqOptions::default()
    };
    assert!(!eq(tuple, "[a,b]", &no_ta), "array must stay distinct");

    // coerce_vectors does not govern tuple↔array, so it leaves that pair alone.
    let no_v = EqOptions {
        coerce_vectors: false,
        ..EqOptions::default()
    };
    assert!(
        eq(tuple, "[a,b]", &no_v),
        "array↔tuple unaffected by coerce_vectors"
    );
}

// Union member matching (pairwise coercion across tuple/vector/interval members)
// needs a vector and a closed-interval member, which the *text* parser cannot
// spell; it is covered by `slow_math-expressions.spec.ts` ("unions of tuples,
// vectors, intervals, altvectors, arrays"), which passes.

#[test]
fn an_integer_assumption_constrains_the_equality_sampler() {
    // `(-1)^n·(-1)^n = (-1)^{2n} = 1` only for integer n. The sampler must draw n
    // over the integers when the assumptions prove it integer.
    let mut a = Assumptions::new();
    a.trees_mut().add_assumption(&parse("n elementof Z"), false);
    assert_eq!(is_integer(&parse("n"), &a), Some(true));

    let with_int = EqOptions {
        assumptions: a,
        ..EqOptions::default()
    };
    assert!(eq("(-1)^n * (-1)^n", "1", &with_int), "equal under n ∈ Z");
    // Without the assumption, n ranges over the complex disk and they differ.
    assert!(!eq("(-1)^n * (-1)^n", "1", &EqOptions::default()));
}
