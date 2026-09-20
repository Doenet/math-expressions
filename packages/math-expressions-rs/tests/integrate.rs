//! Symbolic integration (INTEGRATION_PLAN I1+I2). The engine gates every
//! result by `equals(derivative(F, x), f)` internally, so a `Some` is
//! already verified; these tests re-run the gate explicitly (hard error
//! here, per plan §4c) and pin expected closed forms where pedagogically
//! meaningful.

use math_expressions::{
    canonicalize, derivative, equals, integrate, Assumptions, EqOptions, Expr, TextToAst,
    TextToAstOptions,
};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn eq(a: &Expr, b: &Expr) -> bool {
    equals(a, b, &EqOptions::default())
}

/// Integrate and hard-verify the gate.
fn integ(f: &str) -> Expr {
    let fe = parse(f);
    let result = integrate(&fe, "x", &Assumptions::new())
        .unwrap_or_else(|| panic!("∫ {f} dx: no antiderivative found"));
    let back = derivative(&result, "x");
    assert!(
        eq(&back, &fe),
        "gate: d/dx of {result:?} does not equal {f}"
    );
    result
}

fn assert_integrates_to(f: &str, want: &str) {
    // The engine's answer is gate-verified inside `integ`. Antiderivatives
    // are unique only up to a constant (and the engine may pick a different
    // but equally valid spelling, e.g. −atan(1/x) vs atan(x)), so the row's
    // `want` is validated the same way: its derivative must equal f.
    let _got = integ(f);
    let dwant = derivative(&canonicalize(&parse(want)), "x");
    assert!(
        eq(&dwant, &parse(f)),
        "test row invalid: d/dx({want}) ≠ {f}"
    );
}

// ================= I1: the rational engine =================

#[test]
fn power_rule_and_polynomials() {
    assert_integrates_to("x^3", "x^4/4");
    assert_integrates_to("3x^2 + 2x + 1", "x^3 + x^2 + x");
    assert_integrates_to("x^(-2)", "-1/x");
}

#[test]
fn reciprocal_and_logs() {
    assert_integrates_to("1/x", "ln(x)");
    assert_integrates_to("1/(x+2)", "ln(x+2)");
    assert_integrates_to("(3x^2+1)/(x^3+x)", "ln(x^3+x)");
}

#[test]
fn arctangent_cluster() {
    assert_integrates_to("1/(x^2+1)", "atan(x)");
    // Completing the square: 1/(x²+x+1).
    assert_integrates_to("1/(x^2+x+1)", "2/sqrt(3) * atan((2x+1)/sqrt(3))");
}

#[test]
fn real_quadratic_log_pair() {
    // 1/(x²−2): residues ±1/(2√2) at the irrational roots ±√2.
    let f = parse("1/(x^2-2)");
    let result = integrate(&f, "x", &Assumptions::new()).expect("integrable");
    assert!(eq(&derivative(&result, "x"), &f));
}

#[test]
fn hermite_rational_part() {
    // 1/(x²+1)²: rational part x/(2(x²+1)) + atan(x)/2.
    assert_integrates_to("1/(x^2+1)^2", "x/(2(x^2+1)) + atan(x)/2");
    // Higher multiplicity pole.
    assert_integrates_to("1/(x-1)^3", "-1/(2(x-1)^2)");
}

#[test]
fn mixed_rational() {
    assert_integrates_to("x^5/(x^2+1)", "x^4/4 - x^2/2 + ln(x^2+1)/2");
    assert_integrates_to("(x^2+1)/(x(x+1))", "x + ln(x) - 2 ln(x+1)");
}

#[test]
fn rootof_log_sum() {
    // 1/(x³ − x − 1): the resultant has no rational or quadratic structure —
    // residues stay abstract as RootOf, and the gate still verifies.
    let f = parse("1/(x^3 - x - 1)");
    let result = integrate(&f, "x", &Assumptions::new()).expect("integrable");
    assert!(eq(&derivative(&result, "x"), &f), "gate on RootOf log sum");
    let text = math_expressions::to_text(&canonicalize(&result), &Default::default());
    assert!(
        text.contains("rootof"),
        "abstract residues expected: {text}"
    );
}

#[test]
fn rational_completeness_property() {
    // Seeded pseudo-random proper rational functions: the engine must never
    // fail under the caps, and every answer passes the gate.
    let mut state = 0x9e3779b97f4a7c15u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for case in 0..25 {
        let mut num = String::new();
        let dn = (next() % 3 + 1) as i64; // denominator degree 1..3
        let nn = next() % (dn as u64 + 1); // numerator degree < den + 1
        for i in 0..=nn {
            let c = (next() % 11) as i64 - 5;
            if c != 0 {
                num.push_str(&format!("+({c})*x^{i}"));
            }
        }
        if num.is_empty() {
            num = "1".into();
        }
        let mut den = format!("x^{dn}");
        for i in 0..dn {
            let c = (next() % 11) as i64 - 5;
            if c != 0 {
                den.push_str(&format!("+({c})*x^{i}"));
            }
        }
        let f = parse(&format!("({num})/({den})"));
        let result = integrate(&f, "x", &Assumptions::new())
            .unwrap_or_else(|| panic!("case {case}: ∫ ({num})/({den}) failed"));
        assert!(
            eq(&derivative(&result, "x"), &f),
            "case {case}: gate failed for ({num})/({den})"
        );
    }
}

// ================= I2: table + linear substitution =================

#[test]
fn elementary_table() {
    assert_integrates_to("sin(x)", "-cos(x)");
    assert_integrates_to("cos(x)", "sin(x)");
    assert_integrates_to("exp(x)", "exp(x)");
    assert_integrates_to("e^x", "e^x");
    assert_integrates_to("ln(x)", "x ln(x) - x");
    assert_integrates_to("sinh(x)", "cosh(x)");
    assert_integrates_to("cosh(x)", "sinh(x)");
    assert_integrates_to("sqrt(x)", "2/3 * x^(3/2)");
    assert_integrates_to("x^(5/2)", "2/7 * x^(7/2)");
    assert_integrates_to("atan(x)", "x atan(x) - ln(1+x^2)/2");
    assert_integrates_to("asin(x)", "x asin(x) + sqrt(1-x^2)");
}

#[test]
fn linear_inner_arguments() {
    assert_integrates_to("cos(2x+1)", "sin(2x+1)/2");
    assert_integrates_to("exp(3x)", "exp(3x)/3");
    assert_integrates_to("(2x+5)^7", "(2x+5)^8/16");
    assert_integrates_to("sqrt(1-x)", "-2/3*(1-x)^(3/2)");
    assert_integrates_to("2^x", "2^x/ln(2)");
    assert_integrates_to("sin(x/2)", "-2 cos(x/2)");
}

#[test]
fn sec_squared_shapes() {
    assert_integrates_to("1/cos(x)^2", "tan(x)");
    assert_integrates_to("tan(x)", "-ln(cos(x))");
}

#[test]
fn trig_powers() {
    // Power-reduction for ∫sinⁿ(u)/∫cosⁿ(u) with a linear argument — previously
    // "no elementary form found".
    assert_integrates_to("sin(x)^2", "(x - sin(x)*cos(x))/2");
    assert_integrates_to("cos(x)^2", "(x + sin(x)*cos(x))/2");
    assert_integrates_to("sin(x)^3", "cos(x)^3/3 - cos(x)");
    assert_integrates_to("cos(x)^3", "sin(x) - sin(x)^3/3");
    // Linear inner argument u = 2x carries the 1/b factor.
    assert_integrates_to("sin(2x)^2", "x/2 - sin(2x)*cos(2x)/4");
}

#[test]
fn simplify_collapsing_integrands() {
    // Sums that a trig identity collapses to something trivially integrable.
    // (The pieces also integrate individually now, but this pins the behavior;
    // the second row is the playground's default equation.)
    assert_integrates_to("sin(x)^2 + cos(x)^2", "x");
    assert_integrates_to("sin(x)^2 + cos(x)^2 + 1", "2x");
}

#[test]
fn u_substitution() {
    assert_integrates_to("x * exp(x^2)", "exp(x^2)/2");
    assert_integrates_to("2x/(x^2+1)", "ln(x^2+1)");
    assert_integrates_to("cos(x) * sin(x)^3", "sin(x)^4/4");
    assert_integrates_to("ln(x)/x", "ln(x)^2/2");
    assert_integrates_to("x/sqrt(1-x^4)", "asin(x^2)/2");
    assert_integrates_to("cos(sqrt(x))/sqrt(x)", "2 sin(sqrt(x))");
    // A candidate list where an *earlier* candidate leads nowhere: `u = x^2` is
    // proposed before `u = sin(x^2)`, and integrating in `u` fails for it. The
    // search must move on to the next candidate rather than abandon the whole
    // substitution stage.
    assert_integrates_to("x sin(x^2) cos(x^2)", "-cos(x^2)^2/4");
}

#[test]
fn symbolic_coefficients_via_table() {
    // Non-ℚ coefficients skip the rational engine but hit the table.
    assert_integrates_to("sin(a x)", "-cos(a x)/a");
    assert_integrates_to("(a + b x)^3", "(a + b x)^4/(4b)");
}

#[test]
fn honest_failures_within_fuel() {
    for f in ["exp(x^2)", "sin(x)/x", "exp(x)/x"] {
        assert!(
            integrate(&parse(f), "x", &Assumptions::new()).is_none(),
            "∫ {f} has no elementary form — must refuse"
        );
    }
}

/// Every stage the pipeline gained must answer to the declared budgets rather
/// than to its own recursion: the u-sub search to `max_integration_candidates`,
/// the whole dispatcher (including the re-entrant u-sub and the simplify-retry)
/// to `max_integration_steps`. Zero of either must make the search fail
/// cleanly, not loop or panic.
#[test]
fn new_stages_answer_to_the_resource_limits() {
    use math_expressions::resource_limits::{self, ResourceLimits};
    let integrates = |f: &str, lim: ResourceLimits| {
        resource_limits::with(lim, || {
            integrate(&parse(f), "x", &Assumptions::new()).is_some()
        })
    };
    // u-sub is the only stage that can do `x·sin(x²)·cos(x²)`; with no
    // candidate slots it has nothing to try.
    let no_candidates = ResourceLimits {
        max_integration_candidates: 0,
        ..Default::default()
    };
    assert!(!integrates("x sin(x^2) cos(x^2)", no_candidates));
    assert!(integrates("x sin(x^2) cos(x^2)", ResourceLimits::default()));
    // Step fuel gates the dispatcher itself, so even `∫ x dx` refuses at 0.
    let no_fuel = ResourceLimits {
        max_integration_steps: 0,
        ..Default::default()
    };
    assert!(!integrates("x", no_fuel));
    // The simplify-retry re-enters the pipeline with a *fresh* budget: one step
    // is enough for `sin²x + cos²x + 1`, whose first (unsimplified) attempt
    // already spent one and failed.
    let one_step = ResourceLimits {
        max_integration_steps: 1,
        ..Default::default()
    };
    assert!(integrates("sin(x)^2 + cos(x)^2 + 1", one_step));
}
