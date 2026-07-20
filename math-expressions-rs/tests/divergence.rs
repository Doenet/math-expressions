//! Divergence classification (DIVERGENCE_PLAN.md §7): the exact rational
//! tier, MVT/exact-point certificates, tail-bounded improper values, the
//! never-guess adversarial invariant, and front-end consistency.

use math_expressions::precise::{
    evaluate_to_precision, integrate_analyzed, integrate_to_precision, IntegralVerdict, Precise,
};
use math_expressions::{Expr, TextToAst, TextToAstOptions};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn analyze(f: &str, a: &str, b: &str, digits: usize) -> IntegralVerdict {
    integrate_analyzed(&parse(f), "x", &parse(a), &parse(b), digits)
}

fn expect_divergent(f: &str, a: &str, b: &str) -> Vec<f64> {
    match analyze(f, a, b, 8) {
        IntegralVerdict::Divergent { at } => at.iter().map(|p| p.location).collect(),
        other => panic!("∫ {f} over [{a},{b}]: expected Divergent, got {other:?}"),
    }
}

fn expect_value(f: &str, a: &str, b: &str, digits: usize) -> f64 {
    match analyze(f, a, b, digits) {
        IntegralVerdict::Value(p) => p.to_f64().expect("finite"),
        other => panic!("∫ {f} over [{a},{b}]: expected Value, got {other:?}"),
    }
}

// ================= suite 1: D1 exactness =================

#[test]
fn d1_interior_and_endpoint_poles() {
    let at = expect_divergent("1/x^2", "-1", "1");
    assert!(at.iter().any(|&x| x.abs() < 1e-9), "pole at 0: {at:?}");
    let at = expect_divergent("1/x", "0", "1");
    assert!(at.iter().any(|&x| x.abs() < 1e-12), "endpoint pole: {at:?}");
    let at = expect_divergent("1/(x - 1/3)^2", "0", "1");
    assert!(
        at.iter().any(|&x| (x - 1.0 / 3.0).abs() < 1e-9),
        "pole at 1/3: {at:?}"
    );
}

#[test]
fn d1_irrational_pole_location() {
    // x/(x²−2): the pole at √2 has no float representation — the exact
    // Sturm tier must still find and (as RootOf) name it.
    match analyze("x/(x^2 - 2)", "0", "2", 8) {
        IntegralVerdict::Divergent { at } => {
            assert_eq!(at.len(), 1);
            assert!((at[0].location - 2f64.sqrt()).abs() < 1e-9);
            let exact = at[0].exact.as_ref().expect("exact location");
            assert!(
                matches!(exact, Expr::RootOf { .. }),
                "irrational pole names a RootOf: {exact:?}"
            );
        }
        other => panic!("expected Divergent, got {other:?}"),
    }
}

#[test]
fn d1_rational_pole_reports_exact_location() {
    match analyze("1/(x - 1/3)", "0", "1", 8) {
        IntegralVerdict::Divergent { at } => {
            let exact = at[0].exact.as_ref().expect("exact");
            assert!(
                math_expressions::equals(
                    exact,
                    &parse("1/3"),
                    &math_expressions::EqOptions::default()
                ),
                "got {exact:?}"
            );
        }
        other => panic!("expected Divergent, got {other:?}"),
    }
}

#[test]
fn d1_no_pole_controls() {
    // Tolerances follow the digit contract: the packaged ±1-ulp grid sits
    // just above the relative digit target.
    let v = expect_value("1/x^2", "1", "2", 12);
    assert!((v - 0.5).abs() < 1e-10);
    let v = expect_value("1/(x^2+1)", "-5", "5", 12);
    assert!((v - 2.0 * 5f64.atan()).abs() < 1e-9);
}

#[test]
fn d1_planted_pole_property() {
    // Seeded: plant poles by construction; the verdict must be Divergent
    // exactly when a planted pole lies inside the interval.
    let mut state = 0x5eed5eedu64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for case in 0..20 {
        let c1 = (next() % 19) as i64 - 9; // pole locations in [-9, 9]
        let c2 = (next() % 19) as i64 - 9;
        let a = (next() % 21) as i64 - 10;
        let b = a + 1 + (next() % 6) as i64;
        if c1 == c2 {
            continue;
        }
        let f = format!("(x + 17)/((x - ({c1}))(x - ({c2})))");
        let inside = (a..=b).contains(&c1) || (a..=b).contains(&c2);
        // x = −17 cancels nothing (poles are in [−9,9]).
        match analyze(&f, &a.to_string(), &b.to_string(), 6) {
            IntegralVerdict::Divergent { .. } => {
                assert!(inside, "case {case}: {f} on [{a},{b}] wrongly divergent")
            }
            IntegralVerdict::Value(_) => {
                assert!(!inside, "case {case}: {f} on [{a},{b}] missed a pole")
            }
            IntegralVerdict::Unknown(w) => {
                panic!("case {case}: {f} on [{a},{b}] must be decided (D1): {w}")
            }
        }
    }
}

// ================= suite 2: D2 certificates =================

#[test]
fn d2_tan_pole() {
    let at = expect_divergent("tan(x)", "0", "2");
    assert!(
        at.iter()
            .any(|&x| (x - std::f64::consts::FRAC_PI_2).abs() < 1e-6),
        "pole at π/2: {at:?}"
    );
}

#[test]
fn d2_reciprocal_sine() {
    let at = expect_divergent("1/sin(x)", "-1", "2");
    assert!(at.iter().any(|&x| x.abs() < 1e-6), "pole at 0: {at:?}");
}

#[test]
fn d2_higher_order_odd_zero() {
    // x − sin x has a zero of order 3 at 0 (still a sign change): the MVT
    // certificate needs no order knowledge.
    expect_divergent("1/(x - sin(x))", "-1", "1");
}

#[test]
fn d2_even_order_zero_via_exact_probing() {
    // 1 − cos x ≥ 0: no sign change; the exact-point probe at x = 0 finds
    // the order-2 zero exactly.
    let at = expect_divergent("1/(1 - cos(x))", "-1", "1");
    assert!(at.iter().any(|&x| x.abs() < 1e-6), "pole at 0: {at:?}");
}

#[test]
fn d2_fractional_exponent_boundary() {
    // The convergent/divergent boundary is decided structurally.
    expect_divergent("x^(-1001/1000)", "0", "1");
    // Value (if the tail resolves) or honest Unknown — both fine; only
    // Divergent would be a wrong answer.
    if let IntegralVerdict::Divergent { .. } = analyze("x^(-999/1000)", "0", "1", 4) {
        panic!("x^(-999/1000) is integrable — must not be Divergent")
    }
}

// ================= suite 3: D3 improper values =================

fn digits_of(p: &Precise, d: usize) -> String {
    p.to_decimal_string(d)
        .unwrap()
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take(d)
        .collect()
}

#[test]
fn d3_improper_values_match_closed_forms() {
    for (f, closed) in [
        ("1/sqrt(x)", "2"),
        ("x^(-1/3)", "3/2"),
        ("ln(x)", "-1"),
    ] {
        let v = expect_value(f, "0", "1", 8);
        let want = evaluate_to_precision(&parse(closed), 12).to_f64().unwrap();
        assert!(
            ((v - want) / want).abs() < 1e-7,
            "∫₀¹ {f}: got {v}, want {want}"
        );
    }
}

#[test]
fn d3_upper_endpoint_singularity() {
    // ∫₀¹ dx/√(1−x²) = π/2, singular at the *upper* endpoint. Digits are
    // capped by f64 cancellation there: cells below ~5·10⁻¹⁵ of x = 1 are
    // unresolvable to interval arithmetic (tail ∝ √w ⇒ ~6-7 digits max);
    // deeper requests refuse rather than guess.
    let v = expect_value("1/sqrt(1 - x^2)", "0", "1", 6);
    assert!(
        (v - std::f64::consts::FRAC_PI_2).abs() < 1e-5,
        "got {v}, want π/2"
    );
    match analyze("1/sqrt(1 - x^2)", "0", "1", 12) {
        IntegralVerdict::Value(p) => {
            let v = p.to_f64().unwrap();
            assert!((v - std::f64::consts::FRAC_PI_2).abs() < 1e-11);
        }
        IntegralVerdict::Unknown(_) => {} // honest refusal at unreachable depth
        IntegralVerdict::Divergent { .. } => panic!("π/2 is not divergent"),
    }
}

#[test]
fn d3_digit_agreement_with_closed_form() {
    // Digit-for-digit against the arbitrary-precision path.
    let IntegralVerdict::Value(p) = analyze("1/sqrt(x)", "0", "1", 8) else {
        panic!("expected value")
    };
    assert_eq!(&digits_of(&p, 7), "2000000");
}

// ================= suite 4: never-guess adversarial invariant =================

#[test]
fn near_poles_are_never_judged_divergent() {
    // Smooth spikes: Value or Unknown are both honest; Divergent never is.
    for (f, digits) in [
        ("1/(x^2 + 10^(-12))", 8),
        ("1/((x - 1/2)^2 + 10^(-12))", 8),
        ("1/((x - 1/2)^2 + 10^(-30))", 4),
    ] {
        if let IntegralVerdict::Divergent { .. } = analyze(f, "0", "1", digits) {
            panic!("{f} is smooth — Divergent is a wrong answer")
        }
    }
}

#[test]
fn near_pole_spike_still_evaluates() {
    let v = expect_value("1/((x - 1/2)^2 + 10^(-6))", "0", "1", 8);
    let want = 2e3 * 500f64.atan();
    assert!(((v - want) / want).abs() < 1e-7, "got {v}, want {want}");
}

// ================= suite 5: honest unknowns =================

#[test]
fn unclassifiable_stays_unknown_fast() {
    use std::time::Instant;
    let t = Instant::now();
    let r = analyze("sin(1/x)/x", "0", "1", 6);
    assert!(
        !matches!(r, IntegralVerdict::Divergent { .. }),
        "no certificate exists for the oscillatory case: {r:?}"
    );
    assert!(t.elapsed().as_secs() < 10, "must be budget-fast");
}

// ================= suite 6: front-end consistency =================

#[test]
fn integrate_to_precision_reports_divergence() {
    for (f, a, b) in [
        ("1/x^2", "-1", "1"),
        ("1/x", "0", "1"),
        ("tan(x)", "0", "2"),
        ("1/(1 - cos(x))", "-1", "1"),
    ] {
        match integrate_to_precision(&parse(f), "x", &parse(a), &parse(b), 8) {
            Precise::Unknown(why) => {
                assert!(
                    why.contains("diverges"),
                    "∫ {f}: reason should name divergence, got {why:?}"
                );
            }
            other => panic!("∫ {f} over [{a},{b}]: divergent input returned {other:?}"),
        }
    }
}
