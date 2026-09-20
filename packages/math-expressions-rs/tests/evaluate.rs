//! Numeric evaluation: `evaluate` (complex-principal, with bindings) and
//! `evaluate_to_constant` (simplify-then-eval). Expected values verified against
//! `me.evaluate` / `me.evaluate_to_constant`.

use math_expressions::{
    evaluate_fast_f64, evaluate_to_constant, Expr, TextToAst, TextToAstOptions,
};
use std::collections::HashMap;

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn binds(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

/// Assert an evaluation equals `(re, im)` within tolerance.
fn approx(got: Option<num_complex::Complex64>, re: f64, im: f64) {
    let v = got.expect("expected a value, got None");
    assert!(
        (v.re - re).abs() < 1e-9 && (v.im - im).abs() < 1e-9,
        "got {v:?}, expected ({re}, {im})",
    );
}

#[test]
fn evaluate_real() {
    approx(
        evaluate_fast_f64(&parse("x^2"), &binds(&[("x", 3.0)])),
        9.0,
        0.0,
    );
    approx(
        evaluate_fast_f64(&parse("x + y"), &binds(&[("x", 1.0), ("y", 2.0)])),
        3.0,
        0.0,
    );
    approx(
        evaluate_fast_f64(&parse("sin(x)"), &binds(&[("x", 0.0)])),
        0.0,
        0.0,
    );
    approx(
        evaluate_fast_f64(&parse("abs(x)"), &binds(&[("x", -3.0)])),
        3.0,
        0.0,
    );
    approx(
        evaluate_fast_f64(&parse("exp(x)"), &binds(&[("x", 0.0)])),
        1.0,
        0.0,
    );
}

#[test]
fn evaluate_branch_choice() {
    // An odd root of a negative real is the *real* root — the branch
    // `simplify`'s radical cluster and `cbrt`/`nthroot` take, so every
    // spelling of the same root evaluates to the same number (see
    // `tests/odd_root_real_branch.rs`). This diverges from mathjs, which
    // answers the principal `1 + i√3` here.
    approx(
        evaluate_fast_f64(&parse("x^(1/3)"), &binds(&[("x", -8.0)])),
        -2.0,
        0.0,
    );
    // Even roots have no real branch and stay principal, matching mathjs.
    approx(
        evaluate_fast_f64(&parse("sqrt(x)"), &binds(&[("x", -4.0)])),
        0.0,
        2.0,
    );
}

#[test]
fn evaluate_none_cases() {
    assert!(evaluate_fast_f64(&parse("x^2"), &binds(&[])).is_none()); // unbound
    assert!(evaluate_fast_f64(&parse("x/y"), &binds(&[("x", 1.0), ("y", 0.0)])).is_none());
    // 1/0
}

#[test]
fn evaluate_mod_floored_matches_mathjs() {
    // mathjs `mod` is floored division: the result takes the sign of the
    // divisor, unlike Rust `rem_euclid` (always non-negative).
    approx(
        evaluate_fast_f64(&parse("mod(5, -3)"), &binds(&[])),
        -1.0,
        0.0,
    );
    approx(
        evaluate_fast_f64(&parse("mod(-5, 3)"), &binds(&[])),
        1.0,
        0.0,
    );
    approx(
        evaluate_fast_f64(&parse("mod(5, 3)"), &binds(&[])),
        2.0,
        0.0,
    );
    approx(
        evaluate_fast_f64(&parse("mod(-5, -3)"), &binds(&[])),
        -2.0,
        0.0,
    );
    // mathjs defines `mod(x, 0) = x` (not NaN).
    approx(
        evaluate_fast_f64(&parse("mod(5, 0)"), &binds(&[])),
        5.0,
        0.0,
    );
}

#[test]
fn evaluate_to_constant_cases() {
    approx(evaluate_to_constant(&parse("2 + 3")), 5.0, 0.0);
    approx(evaluate_to_constant(&parse("sin(pi/2)")), 1.0, 0.0);
    approx(evaluate_to_constant(&parse("sqrt(2)")), 2f64.sqrt(), 0.0);
    approx(evaluate_to_constant(&parse("2^10")), 1024.0, 0.0);
    approx(
        evaluate_to_constant(&parse("e^2")),
        std::f64::consts::E.powi(2),
        0.0,
    );
    // Real-domain reduction via simplify (contrast `evaluate`'s complex branch).
    approx(evaluate_to_constant(&parse("(-8)^(1/3)")), -2.0, 0.0);
    // Genuinely complex constant.
    approx(
        evaluate_to_constant(&parse("log(-1)")),
        0.0,
        std::f64::consts::PI,
    );
    // Not a constant at all — a free variable has no value to report.
    assert!(evaluate_to_constant(&parse("x + 1")).is_none());
    // Non-finite, but *known*: `±∞` is reported as the value it is, so that a
    // caller can tell an unbounded endpoint from an undecidable one
    // (DOENET_INTEGRATION item 1). An indeterminate form is *also* a computed
    // value — NaN — for the same reason: `None` crosses to JS as `null`, which
    // coerces to `0`, so declining would report an undefined result as a real
    // point at the origin.
    // (Compared exactly rather than through `approx`, whose `|v − re|` is NaN
    // when both sides are infinite.)
    assert_eq!(
        evaluate_to_constant(&parse("1/0")).map(|v| v.re),
        Some(f64::INFINITY)
    );
    assert!(evaluate_to_constant(&parse("0/0")).is_some_and(|v| v.re.is_nan()));
}
