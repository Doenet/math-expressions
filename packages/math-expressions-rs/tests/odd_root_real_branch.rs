//! An odd root of a negative real reads on the **real** branch on every
//! numeric path — `(-2)^(1/3)` is `-2^(1/3)`, the same number as `cbrt(-2)`
//! and `nthroot(-2, 3)` — while even roots and non-rational exponents stay on
//! the principal complex branch.
//!
//! This is a regression suite: the branch used to depend on whether the
//! radicand was a *perfect power* (`(-8)^(1/3)` folded to the real `-2` while
//! `(-2)^(1/3)` evaluated principal), so `equals` told the same number apart
//! from itself and four DoenetML `<answer>` cases that scored 1 on the legacy
//! JS engine scored 0 here. The fix is layered — `simplify`'s radical cluster
//! pulls the sign out (which is what the certified-constant path sees),
//! `eval_complex`'s `Pow` arm and `cbrt`/`nthroot`'s evaluators take the same
//! branch for sampling — and each layer's row is pinned below.

use math_expressions::{
    equals, evaluate_fast_f64, evaluate_many, evaluate_to_constant, simplify, to_text, EqOptions,
    Expr, LatexToAst, TextOpts, TextToAst, TextToAstOptions,
};
use std::collections::HashMap;

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn pl(s: &str) -> Expr {
    LatexToAst::new(Default::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse latex {s:?}: {e}"))
}

fn eq(a: &str, b: &str) -> bool {
    equals(&parse(a), &parse(b), &EqOptions::default())
}

fn simplified(s: &str) -> String {
    to_text(&simplify(&parse(s)), &TextOpts::default())
}

/// The real cube root of 2, `2^(1/3)`, negated — what every spelling of the
/// cube root of −2 must evaluate to.
const NEG_CBRT_2: f64 = -1.2599210498948732;

// ---- simplify pulls the sign out of a non-perfect-power odd root ----------

#[test]
fn simplify_pulls_the_sign_out_of_an_odd_root() {
    assert_eq!(simplified("(-2)^(1/3)"), "-2^(1/3)");
    // An even numerator squares the sign away.
    assert_eq!(simplified("(-2)^(2/3)"), "2^(2/3)");
    // Perfect powers still fold all the way (the case that always worked).
    assert_eq!(simplified("(-8)^(1/3)"), "-2");
    assert_eq!(simplified("(-8)^(2/3)"), "4");
}

#[test]
fn even_roots_of_negatives_do_not_move() {
    // No real branch to prefer: the sign stays under an even root.
    assert_eq!(simplified("(-2)^(1/2)"), "(-2)^(1/2)");
    // A decimal exponent is an exact rational with an even denominator
    // (`3333/10000`), so it is not an odd root and stays put.
    assert_eq!(simplified("(-8)^0.3333"), "(-8)^0.3333");
}

// ---- the four DoenetML `<answer>` rows that regressed ---------------------

#[test]
fn every_spelling_of_the_same_odd_root_compares_equal() {
    // expected `cbrt(-2)`, student typed `(-2)^{1/3}` — and the other three
    // measured rows from the migration ledger.
    assert!(eq("cbrt(-2)", "(-2)^(1/3)"));
    assert!(eq("nthroot(-2, 3)", "(-2)^(1/3)"));
    assert!(eq("(-2)^(1/3)", "-1.2599210498948732"));
    assert!(equals(
        &parse("(-2)^(1/3)"),
        &pl(r"\sqrt[3]{-2}"),
        &EqOptions::default()
    ));
    // The two rows that legacy got wrong and this engine fixed — they must
    // not be given back.
    assert!(eq("cbrt(-2)", "-cbrt(2)"));
    assert!(eq("cbrt(-2)", "-1.2599210498948732"));
}

/// The sharper form of the old bug: an algebraic identity that holds on
/// either branch taken *consistently* was false because the perfect-power
/// side folded real while the non-perfect side evaluated principal.
#[test]
fn branch_choice_does_not_depend_on_perfect_powers() {
    assert!(eq("(-8)^(1/3)", "(-2)^(1/3) * 4^(1/3)"));
    assert!(eq("(-8)^(1/5)", "(-2)^(1/5) * 4^(1/5)"));
}

// ---- numeric readout ------------------------------------------------------

#[test]
fn odd_roots_evaluate_to_the_real_root() {
    for s in ["(-2)^(1/3)", "cbrt(-2)", "nthroot(-2, 3)"] {
        let v = evaluate_to_constant(&parse(s)).unwrap_or_else(|| panic!("{s} has a value"));
        assert!(
            (v.re - NEG_CBRT_2).abs() < 1e-12 && v.im == 0.0,
            "{s}: got {v}"
        );
    }
    let v = evaluate_to_constant(&parse("(-8)^(1/5)")).expect("a value");
    let want = -(8f64.powf(0.2));
    assert!((v.re - want).abs() < 1e-12 && v.im == 0.0, "got {v}");
}

#[test]
fn even_and_non_rational_exponents_stay_principal() {
    // `(-8)^0.3333` is an even root (`3333/10000`): principal complex value.
    let v = evaluate_to_constant(&parse("(-8)^0.3333")).expect("a value");
    assert!(v.im != 0.0, "got {v}");
    // `nthroot(-8, 4)` has no real value either.
    let v = evaluate_to_constant(&parse("nthroot(-8, 4)")).expect("a value");
    assert!(v.im != 0.0, "got {v}");
}

// ---- sampling: both entry points, both tree shapes ------------------------

/// `evaluate_fast_f64` walks the *canonical* tree (exponent already a folded
/// rational); `evaluate_many` escalates a negative base off the tape and falls
/// back to the complex walk on the *raw* tree (exponent still a quotient
/// node). Both must land on the real branch, or a plotted `x^(1/3)` would
/// disagree with its own single-point readout.
#[test]
fn both_sampling_paths_take_the_real_branch() {
    let e = parse("x^(1/3)");
    let mut binds = HashMap::new();
    binds.insert("x".to_string(), -8.0);
    let v = evaluate_fast_f64(&e, &binds).expect("a value");
    assert!((v.re - -2.0).abs() < 1e-12 && v.im == 0.0, "got {v}");

    let many = evaluate_many(&e, "x", &[-8.0, 8.0]);
    assert!((many[0] - -2.0).abs() < 1e-12, "got {}", many[0]);
    assert!((many[1] - 2.0).abs() < 1e-12, "got {}", many[1]);

    // Even roots keep sampling principal: a negative point is a gap.
    let many = evaluate_many(&parse("x^(1/2)"), "x", &[-4.0]);
    assert!(many[0].is_nan(), "got {}", many[0]);
}

/// The root spellings sample on the same branch as the power spelling — the
/// split this suite exists to prevent, one level down.
#[test]
fn root_spellings_sample_like_the_power_spelling() {
    assert!(eq("x^(1/3)", "cbrt(x)"));
    assert!(eq("x^(1/3)", "nthroot(x, 3)"));
    assert!(eq("x^(1/2)", "sqrt(x)"));
}
