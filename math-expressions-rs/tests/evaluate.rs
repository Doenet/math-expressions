//! Numeric evaluation: `evaluate` (complex-principal, with bindings) and
//! `evaluate_to_constant` (simplify-then-eval). Expected values verified against
//! `me.evaluate` / `me.evaluate_to_constant`.

use math_expressions::{evaluate, evaluate_to_constant, Expr, TextToAst, TextToAstOptions};
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
    approx(evaluate(&parse("x^2"), &binds(&[("x", 3.0)])), 9.0, 0.0);
    approx(evaluate(&parse("x + y"), &binds(&[("x", 1.0), ("y", 2.0)])), 3.0, 0.0);
    approx(evaluate(&parse("sin(x)"), &binds(&[("x", 0.0)])), 0.0, 0.0);
    approx(evaluate(&parse("abs(x)"), &binds(&[("x", -3.0)])), 3.0, 0.0);
    approx(evaluate(&parse("exp(x)"), &binds(&[("x", 0.0)])), 1.0, 0.0);
}

#[test]
fn evaluate_complex_principal_branch() {
    // Matches mathjs: complex principal value, not the real root.
    approx(evaluate(&parse("x^(1/3)"), &binds(&[("x", -8.0)])), 1.0, 3f64.sqrt());
    approx(evaluate(&parse("sqrt(x)"), &binds(&[("x", -4.0)])), 0.0, 2.0);
}

#[test]
fn evaluate_none_cases() {
    assert!(evaluate(&parse("x^2"), &binds(&[])).is_none()); // unbound
    assert!(evaluate(&parse("x/y"), &binds(&[("x", 1.0), ("y", 0.0)])).is_none()); // 1/0
}

#[test]
fn evaluate_to_constant_cases() {
    approx(evaluate_to_constant(&parse("2 + 3")), 5.0, 0.0);
    approx(evaluate_to_constant(&parse("sin(pi/2)")), 1.0, 0.0);
    approx(evaluate_to_constant(&parse("sqrt(2)")), 2f64.sqrt(), 0.0);
    approx(evaluate_to_constant(&parse("2^10")), 1024.0, 0.0);
    approx(evaluate_to_constant(&parse("e^2")), std::f64::consts::E.powi(2), 0.0);
    // Real-domain reduction via simplify (contrast `evaluate`'s complex branch).
    approx(evaluate_to_constant(&parse("(-8)^(1/3)")), -2.0, 0.0);
    // Genuinely complex constant.
    approx(evaluate_to_constant(&parse("log(-1)")), 0.0, std::f64::consts::PI);
    // Not a finite constant.
    assert!(evaluate_to_constant(&parse("x + 1")).is_none());
    assert!(evaluate_to_constant(&parse("1/0")).is_none());
}
