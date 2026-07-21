//! Plus-minus (±) equality — port of the `.equals` cases in
//! `spec/quick_pm.spec.js`. `a ± b` denotes the value set `{a+b, a-b}`.

use math_expressions::{
    equals, expand, simplify, to_latex, to_text, EqOptions, Expr, LatexOpts, LatexToAst,
    LatexToAstOptions, TextOpts, TextToAst, TextToAstOptions,
};

fn text(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse text {s:?}: {e}"))
}

fn latex(s: &str) -> Expr {
    LatexToAst::new(LatexToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse latex {s:?}: {e}"))
}

fn eq(a: &Expr, b: &Expr) -> bool {
    equals(a, b, &EqOptions::default())
}

fn eq_err(a: &Expr, b: &Expr, allowed: f64) -> bool {
    equals(
        a,
        b,
        &EqOptions {
            allowed_error_in_numbers: allowed,
            ..EqOptions::default()
        },
    )
}

#[test]
fn scaling_and_symmetry_via_numeric() {
    // 2(a ± b) == 2a ± 2b   (distribute a non-pm factor: value set unchanged)
    assert!(eq(&latex(r"2(a \pm b)"), &latex(r"2 a \pm 2 b")));
    // 5 ± 3 == 5 ± (-3)     (same value set {8, 2})
    assert!(eq(&latex(r"5 \pm 3"), &latex(r"5 \pm (-3)")));
    // reordering independent pm terms
    assert!(eq(&latex(r"5 \pm 3 \pm 4"), &latex(r"5 \pm 4 \pm 3")));
}

#[test]
fn distinct_value_sets_are_unequal() {
    assert!(!eq(&latex(r"5 \pm 3 \pm 4"), &latex(r"5 \pm 7")));
    assert!(!eq(&latex(r"\pm 3 + \pm 3"), &latex(r"2 \pm 3")));
    assert!(!eq(
        &latex(r"(a \pm b)(c \pm d)"),
        &latex(r"a c \pm a d \pm b c \pm b d")
    ));
    // pm vs no pm
    assert!(!eq(&latex(r"5 \pm 3"), &latex("5")));
}

#[test]
fn pm_inside_containers() {
    // identical containers agree structurally
    assert!(eq(&text("(5 ± 3, 4)"), &text("(5 ± 3, 4)")));
    assert!(eq(
        &latex(r"\langle 5 \pm 3, 4 \rangle"),
        &latex(r"\langle 5 \pm 3, 4 \rangle")
    ));
    // order / value-set differences
    assert!(!eq(&text("(5 ± 3, 4)"), &text("(4, 5 ± 3)")));
    assert!(!eq(&text("(5 ± 3, 4)"), &text("(8, 4)")));
    assert!(!eq(&text("(5 ± 3, 4)"), &text("(3 ± 5, 4)")));
}

#[test]
fn pm_in_container_compared_componentwise() {
    // The pm component is equal but NOT structurally identical (`2(a±b)` vs
    // `2a±2b` don't canonicalize together), so this only passes if the tuple is
    // compared componentwise, re-entering the numeric pm path per component.
    assert!(eq(&latex(r"(2(a \pm b), 4)"), &latex(r"(2 a \pm 2 b, 4)")));
    assert!(eq(
        &latex(r"\langle 2(a \pm b), 4 \rangle"),
        &latex(r"\langle 2 a \pm 2 b, 4 \rangle")
    ));
    // still order- and value-sensitive
    assert!(!eq(&latex(r"(2(a \pm b), 4)"), &latex(r"(4, 2 a \pm 2 b)")));
}

#[test]
fn equations_compare_by_standard_form() {
    // x = 5 ± 3  <=>  x - 5 = ± 3   (solution set {2, 8})
    assert!(eq(&text("x = 5 ± 3"), &text("x - 5 = ± 3")));
    assert!(eq(&text("y = 5 ± 3"), &text("y = 5 ± 3")));
    assert!(!eq(&text("y = 5 ± 3"), &text("y = 8")));
}

#[test]
fn equations_are_proportional() {
    // same solution set {2, 8}, scaled equation
    assert!(eq(&text("x = 5 ± 3"), &text("2x = 10 ± 6")));
    assert!(eq(&text("x = 5 ± 3"), &text("3x = 15 ± 9")));
    // scaled but different solutions {3, 7}
    assert!(!eq(&text("x = 5 ± 3"), &text("2x = 10 ± 4")));
}

#[test]
fn vacuous_pm_zero_dedups() {
    // x = 5 ± 0 dedups its duplicate branches to (x-5), proportional to x = 5
    assert!(eq(&text("x = 5 ± 0"), &text("x = 5")));
    assert!(eq(&text("2x = 10 ± 0"), &text("x = 5")));
}

#[test]
fn allowed_error_tolerance() {
    // within tolerance
    assert!(eq_err(&text("5 ± 3"), &text("5.05 ± 3"), 0.1));
    // no tolerance → exact value sets differ
    assert!(!eq(&text("5 ± 3"), &text("5.05 ± 3")));
    // error exceeds the allowance
    assert!(!eq_err(&text("5 ± 3"), &text("5.5 ± 3"), 0.01));
}

#[test]
fn per_variant_tolerance() {
    // The − variant (value 2) has a tighter derivative-based tolerance (0.02)
    // than the + variant (value 8, tolerance 0.08). A 0.04 perturbation of the
    // literal `3` is accepted by the + variant but must be rejected by the −
    // variant, so a single (loose) tolerance would be wrong.
    assert!(!eq_err(&text("5 ± 3"), &text("5 ± 3.04"), 0.01));
    // A smaller 0.01 perturbation is admitted by both variants.
    assert!(eq_err(&text("5 ± 3"), &text("5 ± 3.01"), 0.01));
}

#[test]
fn matching_is_order_independent() {
    // 95 ± 5 → {100, 90}; 95 ± (-4) → {91, 99}. The only valid pairing is
    // 100↔99 and 90↔91 — a greedy first-fit would fail, bipartite matching
    // must not.
    let lhs = Expr::Add(vec![Expr::int(95), pm(Expr::int(5))]);
    let rhs = Expr::Add(vec![Expr::int(95), pm(Expr::int(-4))]);
    assert!(eq_err(&lhs, &rhs, 0.1));
    assert!(!eq(&lhs, &rhs));
}

fn pm(inner: Expr) -> Expr {
    // ["pm", inner]
    math_expressions::parse::common::other_op("pm", vec![inner])
}

// --- simplify canonicalization rules ---------------------------------------

#[test]
fn simplify_negation_absorbs() {
    // −(±x) → ±x
    let neg_pm = Expr::Neg(Box::new(pm(Expr::sym("x"))));
    assert_eq!(simplify(&neg_pm), simplify(&pm(Expr::sym("x"))));
}

#[test]
fn simplify_scaling_pulls_constant_inside() {
    // 2·±b → ±(2b)
    let scaled = Expr::Mul(vec![Expr::int(2), pm(Expr::sym("b"))]);
    let inside = pm(Expr::Mul(vec![Expr::int(2), Expr::sym("b")]));
    assert_eq!(simplify(&scaled), simplify(&inside));
}

#[test]
fn simplify_does_not_combine_independent_pm() {
    // ±3 + ±3 must NOT collapse to 2·±3 (value sets {6,0,−6} ≠ {6,−6}); the
    // simplified result must still equal the original (meaning preserved).
    let e = latex(r"\pm 3 + \pm 3");
    assert!(equals(&e, &simplify(&e), &EqOptions::default()));
    let two_pm3 = Expr::Mul(vec![Expr::int(2), pm(Expr::int(3))]);
    assert!(!equals(&e, &two_pm3, &EqOptions::default()));
}

// --- expand must not duplicate a ± -----------------------------------------

#[test]
fn expand_preserves_pm_meaning() {
    // Safe: a non-pm factor distributes over a sum containing pm.
    let safe = latex(r"x(y \pm z)");
    assert!(equals(&safe, &expand(&safe), &EqOptions::default()));
    // Unsafe distributions must be left un-distributed (meaning preserved).
    let cases = [
        Expr::Mul(vec![
            pm(Expr::sym("x")),
            Expr::Add(vec![Expr::sym("y"), Expr::sym("z")]),
        ]), // (±x)(y+z)
        Expr::Pow(
            Box::new(Expr::Add(vec![pm(Expr::sym("k")), Expr::sym("m")])),
            Box::new(Expr::int(2)),
        ), // (±k+m)^2
        latex(r"(a \pm b)(c \pm d)"),
    ];
    for e in &cases {
        assert!(
            equals(e, &expand(e), &EqOptions::default()),
            "expand changed the value set of {e:?}"
        );
    }
}

// --- rendering: `± ` is the connective, not `+ ±` --------------------------

#[test]
fn renders_pm_without_stray_plus() {
    let tx = |e: &Expr| to_text(e, &TextOpts::default());
    let lx = |e: &Expr| to_latex(e, &LatexOpts::default());
    let five_pm_three = Expr::Add(vec![Expr::int(5), pm(Expr::int(3))]);
    assert_eq!(lx(&five_pm_three), r"5 \pm 3");
    assert_eq!(tx(&five_pm_three), "5 ± 3");
    let chain = Expr::Add(vec![Expr::int(5), pm(Expr::int(3)), pm(Expr::int(4))]);
    assert_eq!(lx(&chain), r"5 \pm 3 \pm 4");
    assert_eq!(tx(&chain), "5 ± 3 ± 4");
}
