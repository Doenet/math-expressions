//! Number-normalization ops: `constants_to_floats`, `round_numbers_to_decimals`,
//! `round_numbers_to_precision`. Expected values verified against the JS methods.

use math_expressions::{
    constants_to_floats, equals, round_numbers_to_decimals, round_numbers_to_precision, to_text,
    EqOptions, Expr, TextToAst, TextToAstOptions,
};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn txt(e: &Expr) -> String {
    to_text(e, &Default::default())
}

#[test]
fn constants_to_floats_cases() {
    assert!(txt(&constants_to_floats(&parse("pi"))).starts_with("3.14159"));
    assert!(txt(&constants_to_floats(&parse("e"))).starts_with("2.71828"));
    // Recurses; `i` is left alone.
    assert_eq!(txt(&constants_to_floats(&parse("i"))), "i");
    assert!(txt(&constants_to_floats(&parse("2*pi"))).contains("3.14159"));
}

#[test]
fn round_to_decimals_cases() {
    assert_eq!(txt(&round_numbers_to_decimals(&parse("3.14159"), 2)), "3.14");
    assert_eq!(txt(&round_numbers_to_decimals(&parse("3.7"), 0)), "4");
    assert_eq!(txt(&round_numbers_to_decimals(&parse("-2.567"), 2)), "-2.57");
    assert_eq!(txt(&round_numbers_to_decimals(&parse("100"), 2)), "100");
    // Exact-rational rounding: 2.345 is exactly 469/200, so ties go away → 2.35.
    assert_eq!(txt(&round_numbers_to_decimals(&parse("2.345"), 2)), "2.35");
    assert_eq!(txt(&round_numbers_to_decimals(&parse("2.355"), 2)), "2.36");
    // Recurses into subexpressions.
    assert_eq!(txt(&round_numbers_to_decimals(&parse("x + 1.9999"), 2)), "x + 2");
}

#[test]
fn round_to_precision_cases() {
    assert_eq!(txt(&round_numbers_to_precision(&parse("1234.5"), 3)), "1230");
    assert_eq!(txt(&round_numbers_to_precision(&parse("0.0012345"), 3)), "0.00123");
    assert_eq!(txt(&round_numbers_to_precision(&parse("5"), 3)), "5");
    // Recurses; keeps the variable factor.
    assert_eq!(txt(&round_numbers_to_precision(&parse("3.14159*x"), 2)), "3.1 x");
}

#[test]
fn rounding_extreme_magnitudes_is_bounded() {
    // A pasted `1E-400` is an exact nonzero rational below f64 range; deriving
    // the sig-fig position from f64 used to overflow i32 (panic/DoS). It must
    // return promptly and keep the value.
    let tiny = parse("1E-400");
    let r = round_numbers_to_precision(&tiny, 3);
    assert_eq!(txt(&r), txt(&tiny));

    // A 60-digit integer literal (to_f64 finite here; the 350-digit inf-path
    // is covered by magnitude_log10's bit fallback) rounds to 3 sig figs
    // without huge allocations.
    let big = parse(&format!("1234{}", "0".repeat(56)));
    let r = round_numbers_to_precision(&big, 3);
    assert_eq!(txt(&r), format!("123{}", "0".repeat(57)));

    // A ~350-digit literal exceeds f64 range (to_f64 = inf): bit-length
    // fallback path; must not panic or blow up, and must round sanely.
    let huge = parse(&format!("9{}", "0".repeat(349)));
    let r = round_numbers_to_precision(&huge, 2);
    assert_eq!(txt(&r), txt(&huge)); // 9000…0 is already 1-2 sig figs

    // Hostile decimals argument: clamped, no gigabyte BigInt.
    let r = round_numbers_to_decimals(&parse("1.5"), i32::MAX);
    assert_eq!(txt(&r), "1.5");
    let r = round_numbers_to_decimals(&parse("1.5"), i32::MIN);
    assert_eq!(txt(&r), "0");
}

// ---- "don't round" edge guards (spec/quick_rounding.spec.js) ----
//
// Rounding must touch only inexact decimal literals: exact rational fractions
// (`3/7`) and the constants π / e are left intact while a neighboring decimal
// coefficient is rounded. Compared via `equals` exactly as the JS spec does.

fn eq(a: &Expr, b: &str) -> bool {
    equals(a, &parse(b), &EqOptions::default())
}

#[test]
fn precision_does_not_round_fractions() {
    let e = parse("3/7x + 381439619649.253 y");
    assert!(eq(&round_numbers_to_precision(&e, 100), "3/7x + 381439619649.253 y"));
    assert!(eq(&round_numbers_to_precision(&e, 14), "3/7x + 381439619649.25 y"));
    assert!(eq(&round_numbers_to_precision(&e, 10), "3/7x + 381439619600 y"));
    assert!(eq(&round_numbers_to_precision(&e, 4), "3/7x + 381400000000 y"));
}

#[test]
fn precision_does_not_round_pi_or_e() {
    let e = parse("3/7e + 381439619649.253 pi");
    assert!(eq(&round_numbers_to_precision(&e, 100), "3/7exp(1) + 381439619649.253 pi"));
    assert!(eq(&round_numbers_to_precision(&e, 14), "3/7exp(1) + 381439619649.25 pi"));
    assert!(eq(&round_numbers_to_precision(&e, 10), "3/7exp(1) + 381439619600 pi"));
    assert!(eq(&round_numbers_to_precision(&e, 4), "3/7exp(1) + 381400000000 pi"));
}
