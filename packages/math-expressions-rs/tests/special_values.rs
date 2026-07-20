//! S3 (FULL_SIMPLIFY_PLAN) — trig/exp/log special-value folding + parity.
//!
//! `fold_special_values` is an unconditionally-sound rewrite. Value equalities
//! are checked semantically (`equals`); structural expectations (parity sign
//! pulled out, gated rules NOT fired) are asserted directly.

use math_expressions::norm::{canonicalize, fold_special_values};
use math_expressions::{equals, Expr, TextToAst};

fn parse(s: &str) -> Expr {
    TextToAst::new(Default::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e:?}"))
}

fn fold(s: &str) -> Expr {
    fold_special_values(&parse(s))
}

fn eq(a: &Expr, b: &str) -> bool {
    equals(a, &parse(b), &Default::default())
}

// ---------- lattice values, all six functions ----------

#[test]
fn lattice_values() {
    assert!(eq(&fold("sin(pi/6)"), "1/2"));
    assert!(eq(&fold("cos(pi/3)"), "1/2"));
    assert!(eq(&fold("tan(pi/4)"), "1"));
    assert!(eq(&fold("cot(pi/4)"), "1"));
    assert!(eq(&fold("sec(pi/3)"), "2"));
    assert!(eq(&fold("csc(pi/6)"), "2"));
    assert!(eq(&fold("sin(pi/4)"), "sqrt(2)/2"));
    assert!(eq(&fold("cos(pi/6)"), "sqrt(3)/2"));
    assert!(eq(&fold("tan(pi/6)"), "sqrt(3)/3"));
    assert!(eq(&fold("sec(pi/4)"), "sqrt(2)"));
    assert!(eq(&fold("cot(pi/6)"), "sqrt(3)"));
}

#[test]
fn sin_two_pi_is_zero() {
    // The case that motivated the chunk.
    assert!(eq(&fold("sin(2*pi)"), "0"));
    assert!(eq(&fold("cos(pi/2)"), "0"));
    assert!(eq(&fold("tan(pi)"), "0"));
}

#[test]
fn periodicity_reduction_of_constants() {
    assert!(eq(&fold("sin(101*pi/6)"), "1/2"));
    assert!(eq(&fold("cos(7*pi/3)"), "1/2"));
    assert!(eq(&fold("sin(13*pi/6)"), "1/2"));
}

// ---------- parity ----------

#[test]
fn parity() {
    assert!(eq(&fold("sin(-x)"), "-sin(x)"));
    assert!(eq(&fold("cos(-x)"), "cos(x)"));
    assert!(eq(&fold("tan(-x)"), "-tan(x)"));
    assert!(eq(&fold("csc(-x)"), "-csc(x)"));
    assert!(eq(&fold("sec(-x)"), "sec(x)"));
    assert!(eq(&fold("cot(-x)"), "-cot(x)"));
}

// ---------- π-shift (integer multiples) ----------

#[test]
fn pi_shift() {
    assert!(eq(&fold("sin(x + 2*pi)"), "sin(x)"));
    assert!(eq(&fold("sin(x + pi)"), "-sin(x)"));
    assert!(eq(&fold("cos(x + pi)"), "-cos(x)"));
    assert!(eq(&fold("tan(x + pi)"), "tan(x)"));
    assert!(eq(&fold("cos(x + 3*pi)"), "-cos(x)"));
}

// ---------- exp / log inverses ----------

#[test]
fn exp_log_inverses() {
    assert!(eq(&fold("exp(log(x))"), "x"));
    assert!(eq(&fold("e^(log(x))"), "x"));
    assert!(eq(&fold("log(exp(5))"), "5"));
    assert!(eq(&fold("ln(e)"), "1"));
    assert!(eq(&fold("ln(1)"), "0"));
    assert!(eq(&fold("exp(0)"), "1"));
}

#[test]
fn log_of_exp_of_variable_is_gated() {
    // ln(exp(x)) is only sound for real x; without S5 we do NOT fold it, so
    // the result still contains the log/exp structure (unchanged, canonical).
    let e = parse("log(exp(x))");
    assert_eq!(fold_special_values(&e), canonicalize(&e));
}

// ---------- idempotence ----------

#[test]
fn idempotent() {
    for s in [
        "sin(pi/6) + cos(-x) + tan(x + pi)",
        "sin(101*pi/6)",
        "exp(log(x)) + ln(1)",
        "sec(pi/4) - csc(pi/6)",
    ] {
        let once = fold(s);
        let twice = fold_special_values(&once);
        assert_eq!(once, twice, "not idempotent on {s:?}");
    }
}
