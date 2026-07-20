//! Assumptions storage + inference (PORTING_PLAN.md §11). Behaviors verified
//! against the JS reference (`element_of_sets.js` probe + spec subset).

use math_expressions::{
    equals_syntactic, is_integer, is_negative, is_nonnegative, is_nonzero, is_positive, is_real,
    Assumptions, EqOptions, Expr, TextToAst, TextToAstOptions,
};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn with(assume: &str) -> Assumptions {
    let mut a = Assumptions::new();
    a.add(&parse(assume));
    a
}

#[test]
fn add_get_remove() {
    let mut a = Assumptions::new();
    assert!(a.get("x").is_none());
    a.add(&parse("x > 0"));
    // Stored canonically; compare via syntactic equality on the canonical form.
    let got = a.get("x").unwrap();
    assert!(equals_syntactic(&got, &parse("0 < x"), &EqOptions::default()));
    a.remove(&parse("x > 0"));
    assert!(a.get("x").is_none());
    // And-splitting files each conjunct.
    a.add(&parse("x > 0 and x < 2"));
    assert!(matches!(a.get("x").unwrap(), Expr::And(_)));
    a.clear();
    assert!(a.get("x").is_none());
}

#[test]
fn literals_and_constants() {
    let a = Assumptions::new();
    assert_eq!(is_integer(&parse("5"), &a), Some(true));
    assert_eq!(is_integer(&parse("1/2"), &a), Some(false));
    assert_eq!(is_positive(&parse("pi"), &a), Some(true));
    assert_eq!(is_integer(&parse("pi"), &a), Some(false));
    assert_eq!(is_real(&parse("i"), &a), Some(false));
    assert_eq!(is_nonzero(&parse("i"), &a), Some(true));
    assert_eq!(is_nonzero(&parse("0"), &a), Some(false));
    // Unassumed variables are fully unknown — even composites (JS parity).
    assert_eq!(is_real(&parse("x"), &a), None);
    assert_eq!(is_nonnegative(&parse("x^2"), &a), None);
}

#[test]
fn sign_bounds() {
    assert_eq!(is_positive(&parse("x"), &with("x > 0")), Some(true));
    assert_eq!(is_real(&parse("x"), &with("x > 0")), Some(true));
    assert_eq!(is_positive(&parse("x^2"), &with("x < 0")), Some(true));
    assert_eq!(is_positive(&parse("x + 1"), &with("x > 0")), Some(true));
    assert_eq!(is_positive(&parse("1/x"), &with("x > 0")), Some(true));
    assert_eq!(is_negative(&parse("1/x"), &with("x < -1")), Some(true));
    assert_eq!(is_positive(&parse("sqrt(x)"), &with("x > 0")), Some(true));
    assert_eq!(is_positive(&parse("exp(x)"), &with("x > 0")), Some(true));
    assert_eq!(is_nonnegative(&parse("x"), &with("x >= 0")), Some(true));
    assert_eq!(is_positive(&parse("x"), &with("x >= 0")), None);
    // JS conservatisms mirrored: odd-power sign, no interval arithmetic.
    assert_eq!(is_negative(&parse("x^3"), &with("x < 0")), None);
    assert_eq!(is_nonzero(&parse("x^3"), &with("x < 0")), Some(true));
    assert_eq!(is_positive(&parse("x - 3"), &with("x > 4")), None);
}

#[test]
fn known_value_and_nonzero() {
    let a = with("x = 3");
    assert_eq!(is_integer(&parse("x"), &a), Some(true));
    assert_eq!(is_positive(&parse("x"), &a), Some(true));
    // x ≠ 0 gives only nonzero-ness (not realness), which propagates.
    let a = with("x != 0");
    assert_eq!(is_nonzero(&parse("x"), &a), Some(true));
    assert_eq!(is_real(&parse("x"), &a), None);
    assert_eq!(is_nonzero(&parse("2*x"), &a), Some(true));
    assert_eq!(is_nonzero(&parse("x^2"), &a), Some(true));
    assert_eq!(is_nonzero(&parse("abs(x)"), &a), Some(true));
}

#[test]
fn integer_closure() {
    let a = with("n elementof Z");
    assert_eq!(is_integer(&parse("n"), &a), Some(true));
    assert_eq!(is_integer(&parse("n + 1"), &a), Some(true));
    assert_eq!(is_integer(&parse("2*n"), &a), Some(true));
    assert_eq!(is_integer(&parse("n^2"), &a), Some(true));
    assert_eq!(is_integer(&parse("n/2"), &a), None);
    assert_eq!(is_nonnegative(&parse("n^2"), &a), Some(true));
    // int + single definite non-int → not an integer.
    assert_eq!(is_integer(&parse("n + pi"), &a), Some(false));
}
