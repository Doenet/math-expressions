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
