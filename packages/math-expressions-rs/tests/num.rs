//! Unit tests for the `Number` type: exact decimal literal parsing (§3a),
//! rational reduction/demotion, the f64 projection at the JS boundary, and
//! exact decimal rendering.

use math_expressions::Number;

/// `from_decimal_str` on a value that reduces/demotes to a plain integer.
fn parses_to_int(text: &str, expected: i64) {
    assert_eq!(
        Number::from_decimal_str(text),
        Number::Int(expected),
        "{text:?}"
    );
}

/// `from_decimal_str` producing an exact reduced rational.
fn parses_to_rat(text: &str, num: i64, den: i64) {
    assert_eq!(
        Number::from_decimal_str(text),
        Number::rat(num, den),
        "{text:?}"
    );
}

#[test]
fn integer_literals() {
    parses_to_int("0", 0);
    parses_to_int("42", 42);
    parses_to_int("007", 7);
    parses_to_int("1.", 1); // trailing dot, no fractional digits
    parses_to_int("3.0", 3); // fractional zeros
    parses_to_int("1.2E3", 1200); // scientific, lands on an integer
    parses_to_int("1.2E+3", 1200);
    parses_to_int("1E2", 100);
    parses_to_int("0.0", 0);
    parses_to_int(".0", 0);
}

#[test]
fn exact_rationals() {
    parses_to_rat("0.5", 1, 2);
    parses_to_rat(".5", 1, 2);
    parses_to_rat("0.1", 1, 10);
    parses_to_rat("0.2", 1, 5);
    parses_to_rat("0.3", 3, 10);
    parses_to_rat("0.25", 1, 4);
    parses_to_rat("0.6", 3, 5); // reduces 6/10 → 3/5
    parses_to_rat("1.5", 3, 2);
    parses_to_rat("3.1", 31, 10);
    parses_to_rat("1E-5", 1, 100000);
    parses_to_rat("2E-3", 1, 500); // 2/1000 → 1/500
    parses_to_rat("3.1E-3", 31, 10000);
}

#[test]
fn no_float_from_parsing() {
    for s in ["0.1", "3.14159", "0.333", "2E-9", ".7"] {
        assert!(
            !matches!(Number::from_decimal_str(s), Number::Float(_)),
            "{s:?} parsed to a Float"
        );
    }
}

#[test]
fn whitespace_is_trimmed() {
    // The sci-notation lexer folds trailing whitespace into the NUMBER token.
    parses_to_rat("3.1E-3 ", 31, 10000);
    parses_to_int("1E2  ", 100);
}

#[test]
fn overlong_literal_promotes_to_big_but_exact() {
    // 30 significant digits overflow i64; the value stays exact as a Big.
    let n = Number::from_decimal_str("1E30");
    assert!(matches!(n, Number::Big(_)));
    assert_eq!(
        n.terminating_decimal().unwrap(),
        format!("1{}", "0".repeat(30))
    );

    let frac = Number::from_decimal_str("0.123456789012345678901234567890");
    assert!(matches!(frac, Number::Big(_)));
    // Renders back digit-for-digit (an f64 projection would truncate).
    assert_eq!(
        frac.terminating_decimal().unwrap(),
        "0.12345678901234567890123456789"
    );
}

#[test]
fn rat_constructor_reduces_and_demotes() {
    assert_eq!(Number::rat(6, 10), Number::Rat(3, 5));
    assert_eq!(Number::rat(4, 2), Number::Int(2)); // demotes to Int
    assert_eq!(Number::rat(-1, -2), Number::Rat(1, 2)); // sign normalised
    assert_eq!(Number::rat(1, -2), Number::Rat(-1, 2)); // sign to numerator
    assert_eq!(Number::rat(0, 5), Number::Int(0));
}

#[test]
fn terminating_decimal_rendering() {
    assert_eq!(Number::rat(1, 2).terminating_decimal().unwrap(), "0.5");
    assert_eq!(Number::rat(1, 10).terminating_decimal().unwrap(), "0.1");
    assert_eq!(Number::rat(3, 5).terminating_decimal().unwrap(), "0.6");
    assert_eq!(
        Number::rat(1, 100000).terminating_decimal().unwrap(),
        "0.00001"
    );
    assert_eq!(Number::Int(-7).terminating_decimal().unwrap(), "-7");
    assert_eq!(Number::rat(-1, 2).terminating_decimal().unwrap(), "-0.5");
    // 1/3 does not terminate.
    assert_eq!(Number::rat(1, 3).terminating_decimal(), None);
    assert_eq!(Number::rat(1, 6).terminating_decimal(), None);
}

#[test]
fn f64_projection_matches_parsefloat() {
    // The JS-boundary projection must equal parseFloat of the same text, so
    // the tree fixtures stay valid. For small literals `n as f64 / d as f64`
    // is exact, which the direct float parse confirms.
    for s in ["0.5", "0.1", "0.0031", "1.2", "3.1", "1E-5", "0.6", "2E-3"] {
        let via_rat = Number::from_decimal_str(s).to_f64();
        let via_float: f64 = s.trim().parse().unwrap();
        assert_eq!(via_rat, via_float, "{s:?}");
    }
}

#[test]
fn js_string_projects_to_float_shortest() {
    // Sign-string emulation: `0.10` must yield the atom `0.1` (parseFloat's
    // shortest form), not the exact `1/10`.
    assert_eq!(Number::from_decimal_str("0.10").js_string(), "0.1");
    assert_eq!(Number::from_decimal_str("0.5").js_string(), "0.5");
    assert_eq!(Number::Int(42).js_string(), "42");
}

#[test]
fn round_trips_through_from_decimal_str_and_terminating_decimal() {
    // Any terminating-decimal string parses and renders back to itself.
    for s in [
        "0", "7", "0.5", "0.1", "0.25", "0.6", "1.5", "3.14159", "0.00001",
    ] {
        let n = Number::from_decimal_str(s);
        assert_eq!(n.terminating_decimal().unwrap(), s, "{s:?}");
    }
}

// ---- Exact arithmetic ----

#[test]
fn arithmetic_examples() {
    let n = Number::from_decimal_str;
    // Exact rational sums (the §3a payoff): stay exact, reduce, demote.
    assert_eq!(n("0.1").add(&n("0.2")), Number::rat(3, 10));
    assert_eq!(Number::rat(1, 3).add(&Number::rat(1, 6)), Number::rat(1, 2));
    assert_eq!(Number::rat(1, 2).add(&Number::rat(1, 2)), Number::Int(1)); // demotes
    assert_eq!(Number::rat(2, 3).mul(&Number::rat(3, 4)), Number::rat(1, 2));
    assert_eq!(Number::Int(7).sub(&Number::Int(10)), Number::Int(-3));
    assert_eq!(
        Number::Int(1).checked_div(&Number::Int(3)),
        Some(Number::rat(1, 3))
    );
}

#[test]
fn division_by_zero_is_none() {
    assert_eq!(Number::Int(1).checked_div(&Number::Int(0)), None);
    assert_eq!(Number::rat(1, 2).checked_div(&Number::Int(0)), None);
}

#[test]
fn integer_pow_rules() {
    assert_eq!(Number::Int(2).checked_pow_int(10), Some(Number::Int(1024)));
    assert_eq!(Number::Int(5).checked_pow_int(0), Some(Number::Int(1)));
    assert_eq!(Number::Int(0).checked_pow_int(0), Some(Number::Int(1))); // 0^0 == 1
    assert_eq!(Number::Int(0).checked_pow_int(-1), None); // 1/0
    assert_eq!(Number::Int(2).checked_pow_int(-2), Some(Number::rat(1, 4)));
    assert_eq!(
        Number::rat(2, 3).checked_pow_int(2),
        Some(Number::rat(4, 9))
    );
}

#[test]
fn overflow_promotes_to_big() {
    let big = Number::Int(i64::MAX).add(&Number::Int(i64::MAX));
    assert!(matches!(big, Number::Big(_)));
    // ...and demotes back when it fits again.
    assert_eq!(big.sub(&Number::Int(i64::MAX)), Number::Int(i64::MAX));
}

#[test]
fn float_is_contagious() {
    use math_expressions::num::F64;
    let half = Number::Float(F64::new(0.5));
    // A Float operand yields a Float (marks an inexact evaluation result).
    assert!(matches!(half.add(&Number::Int(1)), Number::Float(_)));
    assert!(matches!(Number::Int(2).mul(&half), Number::Float(_)));
}

// ---- Property-based arithmetic laws (exact tiers only) ----

use proptest::prelude::*;

/// Small exact rationals: covers Int and Rat, exercises reduction/promotion.
fn small_rat() -> impl Strategy<Value = Number> {
    (-1000i64..1000, 1i64..1000).prop_map(|(n, d)| Number::rat(n, d))
}

proptest! {
    #[test]
    fn add_is_commutative(a in small_rat(), b in small_rat()) {
        prop_assert_eq!(a.add(&b), b.add(&a));
    }

    #[test]
    fn mul_is_commutative(a in small_rat(), b in small_rat()) {
        prop_assert_eq!(a.mul(&b), b.mul(&a));
    }

    #[test]
    fn add_is_associative(a in small_rat(), b in small_rat(), c in small_rat()) {
        prop_assert_eq!(a.add(&b).add(&c), a.add(&b.add(&c)));
    }

    #[test]
    fn mul_is_associative(a in small_rat(), b in small_rat(), c in small_rat()) {
        prop_assert_eq!(a.mul(&b).mul(&c), a.mul(&b.mul(&c)));
    }

    #[test]
    fn distributive(a in small_rat(), b in small_rat(), c in small_rat()) {
        prop_assert_eq!(a.mul(&b.add(&c)), a.mul(&b).add(&a.mul(&c)));
    }

    #[test]
    fn add_identity(a in small_rat()) {
        prop_assert_eq!(a.add(&Number::zero()), a.clone());
        prop_assert_eq!(a.sub(&a), Number::zero());
    }

    #[test]
    fn mul_identity(a in small_rat()) {
        prop_assert_eq!(a.mul(&Number::one()), a.clone());
    }

    #[test]
    fn sub_is_add_neg(a in small_rat(), b in small_rat()) {
        prop_assert_eq!(a.sub(&b), a.add(&b.neg()));
    }

    #[test]
    fn div_inverts_mul(a in small_rat(), b in small_rat()) {
        prop_assume!(!b.is_zero());
        prop_assert_eq!(a.mul(&b).checked_div(&b), Some(a));
    }

    /// Every reduced Rat/Int satisfies the tier invariants.
    #[test]
    fn tier_invariants(a in small_rat()) {
        match a {
            Number::Rat(n, d) => {
                prop_assert!(d > 1);
                prop_assert_eq!(gcd(n.unsigned_abs(), d as u64), 1);
            }
            Number::Int(_) => {}
            _ => prop_assert!(false, "small_rat should only make Int/Rat"),
        }
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

// ---- Review regression tests: bounded computation on adversarial input ----

#[test]
fn absurd_exponents_do_not_hang_or_wrap() {
    // Exponent overflows i64: saturates, then approximates like parseFloat
    // (previously `unwrap_or(0)` silently read the value as 1).
    let huge = Number::from_decimal_str("1E99999999999999999999");
    assert!(matches!(huge, Number::Float(f) if f.get().is_infinite()));
    let tiny = Number::from_decimal_str("1E-99999999999999999999");
    assert!(matches!(tiny, Number::Float(f) if f.get() == 0.0));

    // Exact power folding refuses astronomically large results...
    assert_eq!(Number::Int(2).checked_pow_int(1_000_000_000_000), None);
    // ...but |1| bases are exempt (their powers stay small)...
    assert_eq!(
        Number::Int(-1).checked_pow_int(1_000_000_000_000),
        Some(Number::Int(1))
    );
    // ...and reasonable big powers still fold exactly.
    assert_eq!(
        Number::Int(10).checked_pow_int(20).unwrap().js_string(),
        "100000000000000000000"
    );
}
