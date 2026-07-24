//! Discrete infinite sets (equals stage 4) — ported from
//! `spec/quick_sets.spec.js`. Sets are built with
//! `create_discrete_infinite_set` and compared through the full `equals`
//! chain, exactly as the JS `set1.equals(set2)` does.

use math_expressions::{
    create_discrete_infinite_set, equals, match_discrete_infinite, EqOptions, Expr, TextToAst,
    TextToAstOptions,
};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

/// Build a set from text offsets/periods (mirrors the spec's helper usage).
fn set(offsets: &str, periods: &str) -> Expr {
    create_discrete_infinite_set(&parse(offsets), &parse(periods), None, None).unwrap()
}

fn set_from(offsets: &str, periods: &str, min_index: &str) -> Expr {
    create_discrete_infinite_set(
        &parse(offsets),
        &parse(periods),
        Some(&parse(min_index)),
        None,
    )
    .unwrap()
}

fn eq(a: &Expr, b: &Expr) -> bool {
    equals(a, b, &EqOptions::default())
}

fn partial(a: &Expr, b: &Expr) -> f64 {
    match_discrete_infinite(a, b, &EqOptions::default(), true)
}

#[test]
fn basic_equality() {
    // {π/4 + kπ} == {π/4 + 2kπ} ∪ {5π/4 + 2kπ}
    let set2 = set("pi/4, 5pi/4", "2pi");
    assert!(eq(&set("pi/4", "pi"), &set2));
    assert!(!eq(&set("pi/4", "2*pi"), &set2));
    // Offset shifted by a full period still matches.
    assert!(eq(&set("9*pi/4", "pi"), &set2));
    assert!(!eq(&set("7*pi/4", "pi"), &set2));
    // {−π/4 + kπ/2} == the four-offset union with period 2π.
    assert!(eq(
        &set("-pi/4", "pi/2"),
        &set("-pi/4, pi/4, 11pi/4, -11pi/4", "2pi"),
    ));
}

#[test]
fn overcounting_offsets() {
    let set1 = set("1", "5");
    assert!(eq(&set1, &set("1, 1, 6, 11, 16, 21", "10")));
    assert!(!eq(&set1, &set("1, 1, 6, 11, 16, 22", "10")));
}

#[test]
fn match_partial_scores() {
    let set1 = set("1", "5");
    assert_eq!(partial(&set1, &set("1, 16", "10")), 1.0);
    assert_eq!(partial(&set1, &set("1, 15", "10")), 0.5);
    let s = partial(&set1, &set("1, 16, 17", "10"));
    assert!((s - 2.0 / 3.0).abs() < 1e-12, "got {s}");
    assert_eq!(partial(&set1, &set("2, 15", "10")), 0.0);

    let set1 = set("1, 2", "5");
    assert_eq!(partial(&set1, &set("2, 15", "10")), 0.25);
    assert_eq!(partial(&set1, &set("2, 15, 17", "10")), 0.5);
    assert_eq!(partial(&set1, &set("2, 15, 16, 17", "10")), 0.75);
    assert_eq!(partial(&set1, &set("2, 15, 16, 17, 18", "10")), 0.6);
    assert_eq!(partial(&set1, &set("2, 15, 16", "10")), 0.5);
    assert_eq!(partial(&set1, &set("2, 15, 16, 18", "10")), 0.5);
    assert_eq!(partial(&set1, &set("2, 15, 16, 18, 19", "10")), 0.4);
}

#[test]
fn symbolic_offsets() {
    // Offsets may be symbolic: only differences must cancel numerically.
    let set2 = set("a, a+3, a+6", "9");
    assert!(eq(&set("a", "3"), &set2));
    assert!(!eq(&set("b", "3"), &set2));
}

#[test]
fn symbolic_period_folds_without_assumptions() {
    // JS needs an explicit `c != 0` assumption to fold 2c/c; our
    // assumption-free canonicalizer folds it unconditionally (documented
    // divergence, same class as x/x → 1).
    assert!(eq(&set("a", "c"), &set("a, a+c", "2c")));
}

#[test]
fn compare_with_listed_sequence() {
    let s = set_from("0", "7", "0");
    assert!(eq(&s, &parse("0, 7, 14, 21, ...")));
    assert!(!eq(&s, &parse("-14, -7, 0, 7, 14, 21, ...")));
    assert!(!eq(&s, &parse("0, 7, 14, 21"))); // no ldots
    assert!(!eq(&s, &parse("0, 7, ..."))); // too few elements

    let s = set_from("0", "7", "-2");
    assert!(!eq(&s, &parse("0, 7, 14, 21, ...")));
    assert!(eq(&s, &parse("-14, -7, 0, 7, 14, 21, ...")));
    assert!(!eq(&s, &parse("0, 7, 14, 21")));
    assert!(!eq(&s, &parse("0, 7, ...")));
}

#[test]
fn simplified_sets_still_compare() {
    use math_expressions::simplify;
    let set1 = set("pi/4, 3pi/4", "pi");
    let set2 = set("-pi/4", "pi/2");
    assert!(eq(&set1, &set2));
    assert!(eq(&set2, &set1));
    let (s1, s2) = (simplify(&set1), simplify(&set2));
    assert!(eq(&s1, &s2));
    assert!(eq(&s2, &s1));
}

#[test]
fn sets_never_equal_plain_expressions() {
    let s = set("pi/4", "pi");
    assert!(!eq(&s, &parse("pi/4")));
    assert!(!eq(&s, &parse("x")));
}
