//! Assumption-gated extraction of variable powers from a root.
//!
//! With no assumptions a variable radicand never folds (the settled root spec);
//! these are the rows DoenetML's `simplify sqrt/cbrt/nth root of powers` tests
//! exercise once `x > 0` / `x ∈ R` is in scope, plus the neighbours that must
//! keep declining.

use math_expressions::{
    evaluate_fast_f64, evaluate_to_constant, expr, simplify, simplify_with, substitute,
    Assumptions, Expr, LatexToAst, TextToAst,
};
use std::collections::HashMap;

fn t(s: &str) -> Expr {
    TextToAst::new(Default::default()).convert(s).unwrap()
}

fn l(s: &str) -> Expr {
    LatexToAst::new(Default::default()).convert(s).unwrap()
}

fn js(e: &Expr) -> String {
    expr::serde::to_js(e).to_string()
}

fn assume(parts: &[&str]) -> Assumptions {
    let mut a = Assumptions::new();
    for p in parts {
        a.add(&t(p));
    }
    a
}

fn under(parts: &[&str], s: &str) -> String {
    js(&simplify_with(&t(s), &assume(parts)))
}

fn under_latex(parts: &[&str], s: &str) -> String {
    js(&simplify_with(&l(s), &assume(parts)))
}

fn bare(s: &str) -> String {
    js(&simplify(&t(s)))
}

#[test]
fn odd_root_of_a_matching_power_reduces_under_real() {
    // cbrt(x³) = x for every real x — an odd root has no sign ambiguity.
    assert_eq!(under(&["x > 0"], "cbrt(x^3)"), r#""x""#);
    assert_eq!(under(&["x elementof R"], "cbrt(x^3)"), r#""x""#);
    assert_eq!(under(&["x > 0"], "nthroot(x^5, 5)"), r#""x""#);
    assert_eq!(under(&["x elementof R"], "nthroot(x^5, 5)"), r#""x""#);
}

#[test]
fn even_root_keeps_the_magnitude() {
    // sqrt(x²) is |x|, and only drops the abs when the sign is pinned.
    assert_eq!(under(&["x > 0"], "sqrt(x^2)"), r#""x""#);
    assert_eq!(
        under(&["x elementof R"], "sqrt(x^2)"),
        r#"["apply","abs","x"]"#
    );
    // An even exponent on the way out is nonnegative by itself, so no abs.
    assert_eq!(under(&["x elementof R"], "sqrt(x^4)"), r#"["^","x",2]"#);
}

#[test]
fn a_partial_power_leaves_a_residual_under_the_radical() {
    // y⁵ = y⁴·y under a square root; the numeric 32 = 2⁵ contributes 4.
    assert_eq!(
        under(&["x > 0", "y > 0"], "sqrt(32 x^2 y^5)"),
        r#"["*",4,"x",["^","y",2],["apply","sqrt",["*",2,"y"]]]"#
    );
    // Same radicand under a cube root: 32 gives 2, y⁵ gives y, x² stays.
    assert_eq!(
        under(&["x > 0", "y > 0"], "cbrt(32 x^2 y^5)"),
        r#"["*",2,"y",["apply","cbrt",["*",4,["^","x",2],["^","y",2]]]]"#
    );
    // a⁷b⁶c²⁸ under a fifth root: a, b and c⁵ come out; a²bc³ stays.
    assert_eq!(
        under(&["a > 0", "b > 0", "c > 0"], "nthroot(a^7 b^6 c^28, 5)"),
        r#"["*","a","b",["^","c",5],["apply","nthroot",["tuple",["*",["^","a",2],"b",["^","c",3]],5]]]"#
    );
}

#[test]
fn a_negative_coefficient_still_takes_the_odd_root_sign() {
    // The extracted `x²` joins the `−2` the odd root already pulled out, and
    // the sign stays on that coefficient rather than becoming a `Neg` wrapper —
    // the presentation layer's doing (see `normalize::present`), not this
    // rule's, and it is how every negative product with a coefficient prints.
    assert_eq!(
        under_latex(&["x > 0"], r"\sqrt[3]{-24x^6}"),
        r#"["*",-2,["^","x",2],["apply","cbrt",3]]"#
    );
    assert_eq!(
        under_latex(&["x elementof R"], r"\sqrt[3]{-24x^6}"),
        r#"["*",-2,["^","x",2],["apply","cbrt",3]]"#
    );
}

#[test]
fn an_even_root_of_a_real_power_keeps_its_abs_beside_the_residual() {
    // 128x⁶ under a sixth root: 128 = 2⁶·2, so 2 and |x| come out and 2 stays.
    assert_eq!(
        under_latex(&["x > 0"], r"\sqrt[6]{128x^6}"),
        r#"["*",2,"x",["apply","nthroot",["tuple",2,6]]]"#
    );
    assert_eq!(
        under_latex(&["x elementof R"], r"\sqrt[6]{128x^6}"),
        r#"["*",2,["apply","abs","x"],["apply","nthroot",["tuple",2,6]]]"#
    );
}

#[test]
fn abs_distributes_over_a_power_only_when_the_exponent_is_real_too() {
    // `|bʷ| = |b|ʷ` holds because `|bʷ| = |b|ʷ·e^(−arg(b)·Im w)`, so it needs a
    // real exponent as much as a real base. This is what turns the `4·|x³|` an
    // even-root extraction yields into the conventional `4·|x|³`.
    assert_eq!(
        under(&["x elementof R"], "sqrt(16 x^6)"),
        r#"["*",4,["^",["apply","abs","x"],3]]"#
    );
    assert_eq!(
        under(&["x elementof R"], "abs(x^3)"),
        r#"["^",["apply","abs","x"],3]"#
    );
    // An imaginary exponent must not distribute: for real `b > 0`, `|b^i|` is 1
    // while `|b|^i` is not even real.
    assert_eq!(
        under(&["x elementof R"], "abs(x^i)"),
        r#"["apply","abs",["^","x","i"]]"#
    );
    // An exponent of unknown realness declines for the same reason.
    assert_eq!(
        under(&["x elementof R"], "abs(x^w)"),
        r#"["apply","abs",["^","x","w"]]"#
    );
}

#[test]
fn without_assumptions_a_variable_radicand_never_folds() {
    // The settled root spec: only the numeric coefficient moves.
    assert_eq!(bare("sqrt(x^2)"), r#"["apply","sqrt",["^","x",2]]"#);
    assert_eq!(bare("cbrt(x^3)"), r#"["apply","cbrt",["^","x",3]]"#);
    assert_eq!(
        bare("nthroot(x^5, 5)"),
        r#"["apply","nthroot",["tuple",["^","x",5],5]]"#
    );
    assert_eq!(
        bare("sqrt(32 x^2 y^5)"),
        r#"["*",4,["apply","sqrt",["*",2,["^","x",2],["^","y",5]]]]"#
    );
}

#[test]
fn an_unrelated_assumption_does_not_unlock_the_fold() {
    // Knowing about `y` says nothing about `x`, so nothing moves.
    assert_eq!(
        under(&["y > 0"], "cbrt(x^3)"),
        r#"["apply","cbrt",["^","x",3]]"#
    );
    assert_eq!(
        under(&["y > 0"], "sqrt(x^2)"),
        r#"["apply","sqrt",["^","x",2]]"#
    );
}

#[test]
fn an_exponent_below_the_root_degree_stays_put() {
    // Nothing to extract: the rule must decline rather than churn.
    assert_eq!(
        under(&["x > 0"], "cbrt(x^2)"),
        r#"["apply","cbrt",["^","x",2]]"#
    );
    assert_eq!(
        under(&["x > 0"], "nthroot(x^4, 5)"),
        r#"["apply","nthroot",["tuple",["^","x",4],5]]"#
    );
}

// ---- meaning preservation ----
//
// Every assertion above pins an output *shape*. A rule that extracted the
// wrong power, dropped a factor or flipped a sign would still produce some
// shape, so the shapes are re-derived here as numbers: substitute concrete
// values for the variables and check the two forms agree.

/// Bind each variable to a literal and evaluate — `evaluate_to_constant`, which
/// simplifies first and so reads odd roots of negatives on the **real** branch
/// (`cbrt(-8)` is `-2`, not `1 + i√3`; see `ops::evaluate`). That is the
/// convention these rules extract under, so it is the one they are checked
/// against. Returned as a pair to keep `num_complex` out of the test's
/// dependencies.
fn value_at(e: &Expr, bindings: &[(&str, &str)]) -> Option<(f64, f64)> {
    let subs: HashMap<String, Expr> = bindings
        .iter()
        .map(|(k, v)| ((*k).to_string(), t(v)))
        .collect();
    evaluate_to_constant(&substitute(e, &subs)).map(|z| (z.re, z.im))
}

/// The same substitution evaluated *without* simplifying first — straight off
/// the tree via `evaluate_fast_f64`. Independent of every simplify rule under
/// test, but only used on nonnegative radicands, where no branch choice
/// arises (see below).
fn principal_value_at(e: &Expr, bindings: &[(&str, &str)]) -> Option<(f64, f64)> {
    let subs: HashMap<String, Expr> = bindings
        .iter()
        .map(|(k, v)| ((*k).to_string(), t(v)))
        .collect();
    evaluate_fast_f64(&substitute(e, &subs), &HashMap::new()).map(|z| (z.re, z.im))
}

/// One substitution: `variable` → the literal to bind it to.
type Binding = [(&'static str, &'static str)];
/// A row of the value table: assumptions, expression, and the samples to try.
type ValueRow = (
    &'static [&'static str],
    &'static str,
    &'static [&'static Binding],
);

fn assert_agrees(what: &str, a: Option<(f64, f64)>, b: Option<(f64, f64)>) {
    match (a, b) {
        (Some((ar, ai)), Some((br, bi))) => assert!(
            (ar - br).abs() < 1e-9 && (ai - bi).abs() < 1e-9,
            "{what}: {a:?} vs {b:?}",
        ),
        _ => panic!("{what}: did not evaluate — {a:?} vs {b:?}"),
    }
}

#[test]
fn the_value_is_unchanged_under_the_assumptions() {
    // (assumptions, expression, sample bindings). The samples obey the
    // assumptions — that is the whole contract, so feeding `x > 0` a negative
    // would test nothing.
    let rows: &[ValueRow] = &[
        (&["x > 0"], "cbrt(x^3)", &[&[("x", "2")], &[("x", "1/8")]]),
        (
            &["x elementof R"],
            "cbrt(x^3)",
            &[&[("x", "2")], &[("x", "-3")], &[("x", "-1/2")]],
        ),
        (&["x > 0"], "nthroot(x^5, 5)", &[&[("x", "2")]]),
        (
            &["x elementof R"],
            "nthroot(x^5, 5)",
            &[&[("x", "2")], &[("x", "-3")]],
        ),
        (&["x > 0"], "sqrt(x^2)", &[&[("x", "5")], &[("x", "1/4")]]),
        (
            &["x elementof R"],
            "sqrt(x^2)",
            &[&[("x", "5")], &[("x", "-7")]],
        ),
        (
            &["x elementof R"],
            "sqrt(x^4)",
            &[&[("x", "3")], &[("x", "-3")]],
        ),
        (
            &["x > 0", "y > 0"],
            "sqrt(32 x^2 y^5)",
            &[&[("x", "2"), ("y", "3")]],
        ),
        (
            &["x > 0", "y > 0"],
            "cbrt(32 x^2 y^5)",
            &[&[("x", "2"), ("y", "3")]],
        ),
        (
            &["a > 0", "b > 0", "c > 0"],
            "nthroot(a^7 b^6 c^28, 5)",
            &[&[("a", "2"), ("b", "3"), ("c", "2")]],
        ),
        (
            &["x > 0"],
            "cbrt(-24 x^6)",
            &[&[("x", "1")], &[("x", "2")], &[("x", "1/2")]],
        ),
        (
            &["x elementof R"],
            "cbrt(-24 x^6)",
            &[&[("x", "2")], &[("x", "-2")]],
        ),
        (
            &["x elementof R"],
            "nthroot(128 x^6, 6)",
            &[&[("x", "2")], &[("x", "-2")]],
        ),
        // The rows that must *not* fold still have to hold their value.
        (&["y > 0"], "cbrt(x^3)", &[&[("x", "-2")]]),
        (&["x > 0"], "cbrt(x^2)", &[&[("x", "3")]]),
        (&["x > 0"], "nthroot(x^4, 5)", &[&[("x", "3")]]),
    ];

    for (asm, src, samples) in rows {
        let original = t(src);
        let simplified = simplify_with(&original, &assume(asm));
        for binding in *samples {
            assert_agrees(
                &format!("{src} under {asm:?} at {binding:?}"),
                value_at(&original, binding),
                value_at(&simplified, binding),
            );
        }
    }
}

#[test]
fn the_extracted_form_holds_up_against_an_unsimplified_evaluation() {
    // `value_at` above simplifies both sides, so it cannot by itself rule out
    // a rule that is wrong in the same way twice. These rows re-check the
    // extraction against the raw evaluator, which shares no code with the
    // rewrite. They are restricted to radicands that land nonnegative, where
    // no branch choice arises and the comparison is unambiguous — the
    // odd-root-of-a-negative rows are covered by `value_at` and by
    // `tests/odd_root_real_branch.rs`, which pins the branch itself.
    let rows: &[(&'static [&'static str], &'static str, &'static Binding)] = &[
        (&["x > 0"], "cbrt(x^3)", &[("x", "2")]),
        (&["x > 0"], "nthroot(x^5, 5)", &[("x", "2")]),
        (&["x > 0"], "sqrt(x^2)", &[("x", "5")]),
        (&["x elementof R"], "sqrt(x^2)", &[("x", "-7")]),
        (&["x elementof R"], "sqrt(x^4)", &[("x", "-3")]),
        (
            &["x > 0", "y > 0"],
            "sqrt(32 x^2 y^5)",
            &[("x", "2"), ("y", "3")],
        ),
        (
            &["x > 0", "y > 0"],
            "cbrt(32 x^2 y^5)",
            &[("x", "2"), ("y", "3")],
        ),
        (
            &["a > 0", "b > 0", "c > 0"],
            "nthroot(a^7 b^6 c^28, 5)",
            &[("a", "2"), ("b", "3"), ("c", "2")],
        ),
        (&["x elementof R"], "nthroot(128 x^6, 6)", &[("x", "-2")]),
    ];

    for (asm, src, binding) in rows {
        let original = t(src);
        let simplified = simplify_with(&original, &assume(asm));
        assert_agrees(
            &format!("{src} under {asm:?} at {binding:?}"),
            principal_value_at(&original, binding),
            principal_value_at(&simplified, binding),
        );
    }
}

#[test]
fn the_extraction_reaches_a_fixpoint() {
    // A rule that re-fires on its own output would spin the rewrite loop.
    for (asm, src) in [
        (&["x > 0"][..], "cbrt(x^3)"),
        (&["x elementof R"], "sqrt(x^2)"),
        (&["x elementof R"], "sqrt(x^4)"),
        (&["x > 0", "y > 0"], "sqrt(32 x^2 y^5)"),
        (&["x > 0"], "cbrt(-24 x^6)"),
        (&["x elementof R"], "nthroot(128 x^6, 6)"),
        (&["x > 0"], "cbrt(x^2)"),
    ] {
        let a = assume(asm);
        let once = simplify_with(&t(src), &a);
        let twice = simplify_with(&once, &a);
        assert_eq!(js(&once), js(&twice), "{src} under {asm:?} did not settle");
    }
}
