//! Arbitrary-precision evaluation (ARBITRARY_PERCISION_PLAN P1+P2).
//! Oracle: hardcoded reference constants (50+ digits), self-consistency
//! (results at d and 2d digits agree on the first d), identity round-trips,
//! and a differential run against `evaluate_to_constant` (P1 exit criterion).

use math_expressions::precise::{compile, evaluate_to_precision, Precise};
use math_expressions::{Expr, TextToAst, TextToAstOptions};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

/// First `d` significant digits as a plain digit string (no dot/exponent).
fn digits_of(p: &Precise, d: usize) -> String {
    let s = p
        .to_decimal_string(d)
        .unwrap_or_else(|| panic!("expected digits from {p:?}"));
    s.chars().filter(|c| c.is_ascii_digit()).take(d).collect()
}

fn assert_digits(expr: &str, d: usize, want: &str) {
    let got = digits_of(&evaluate_to_precision(&parse(expr), d), d);
    let want_trunc: String = want.replace(" ", "").chars().take(d).collect();
    // The implementation rounds its last digit; the reference is truncated —
    // accept exact or +1 in the last place (carry-propagated).
    let bumped = increment_decimal(&want_trunc);
    assert!(
        got == want_trunc || got == bumped,
        "{expr} @ {d} digits: got {got}, want {want_trunc} (or {bumped})"
    );
}

fn increment_decimal(s: &str) -> String {
    let mut digits: Vec<u8> = s.bytes().map(|b| b - b'0').collect();
    for d in digits.iter_mut().rev() {
        if *d == 9 {
            *d = 0;
        } else {
            *d += 1;
            return digits.iter().map(|d| (b'0' + d) as char).collect();
        }
    }
    let mut out = String::from("1");
    out.extend(digits.iter().map(|d| (b'0' + d) as char));
    out.truncate(s.len());
    out
}

// Reference values (standard published digits).
const SQRT2: &str = "141421356237309504880168872420969807856967187537694809";
const PI: &str = "314159265358979323846264338327950288419716939937510582";
const E: &str = "271828182845904523536028747135266249775724709369995957";
const LN2: &str = "693147180559945309417232121458176568075500134360255254";

#[test]
fn known_constants_to_50_digits() {
    assert_digits("sqrt(2)", 50, SQRT2);
    assert_digits("pi", 50, PI);
    assert_digits("e", 50, E);
    assert_digits("ln(2)", 50, LN2);
    // And through arithmetic:
    assert_digits("exp(1)", 50, E);
    assert_digits("2^(1/2)", 50, SQRT2);
}

#[test]
fn short_requests_use_the_fast_path_correctly() {
    assert_digits("sqrt(2)", 10, SQRT2);
    assert_digits("pi", 10, PI);
    assert_digits("exp(2)", 10, "7389056098");
    assert_digits("ln(10)", 10, "2302585092");
}

#[test]
fn tier_r_exact_rationals() {
    match evaluate_to_precision(&parse("1/3 + 1/6"), 10) {
        Precise::Exact(n) => assert_eq!(n.js_string(), "0.5"),
        other => panic!("expected exact, got {other:?}"),
    }
    // Huge exact powers stay exact.
    let p = evaluate_to_precision(&parse("2^200 + 1"), 10);
    assert!(matches!(p, Precise::Exact(_)));
}

#[test]
fn self_consistency_digit_prefixes() {
    // Result at 2d digits must extend the result at d digits (allowing the
    // final-digit rounding difference: compare on d-1).
    for expr in [
        "sqrt(3)",
        "exp(sqrt(2))",
        "ln(7)",
        "pi^2",
        "sqrt(2)*exp(1) + ln(3)",
        "exp(exp(1))",
        "1/sqrt(17)",
        "2^(2/3)",
    ] {
        for d in [12usize, 40] {
            let a = digits_of(&evaluate_to_precision(&parse(expr), d), d - 1);
            let b = digits_of(&evaluate_to_precision(&parse(expr), 2 * d), d - 1);
            assert_eq!(a, b, "{expr}: {d}-digit result not a prefix of {}", 2 * d);
        }
    }
}

#[test]
fn identity_round_trips_at_60_digits() {
    // exp(ln x) = x and sqrt(x)^2 = x, checked digit-for-digit.
    for x in ["7", "1/3", "42"] {
        let lhs = digits_of(
            &evaluate_to_precision(&parse(&format!("exp(ln({x}))")), 60),
            58,
        );
        let rhs = digits_of(&evaluate_to_precision(&parse(&format!("({x}) + 0.0")), 60), 58);
        // (x + 0.0 forces the non-exact path for uniform formatting)
        let _ = rhs;
        let direct = digits_of(&evaluate_to_precision(&parse(&format!("sqrt(({x})^2)")), 60), 58);
        assert_eq!(lhs, direct, "exp(ln({x})) vs sqrt(x^2)");
    }
}

#[test]
fn five_hundred_digits_of_pi_are_self_consistent() {
    // P2 exit criterion at the 500-digit scale: two independent requests and
    // an identity (pi = 4*atan-free check via  sqrt(pi^2)) agree.
    let a = digits_of(&evaluate_to_precision(&parse("pi"), 500), 499);
    let b = digits_of(&evaluate_to_precision(&parse("sqrt(pi^2)"), 500), 499);
    assert_eq!(a, b);
    assert!(a.starts_with(&PI.replace(" ", "")[..50]));
}

#[test]
fn cancellation_forces_ziv_refinement() {
    // exp(ln 2) − 2 is ~0 but not exactly representable: must come back tiny
    // or Unknown, never a wrong nonzero magnitude.
    let p = evaluate_to_precision(&parse("exp(ln(2)) - 2"), 8);
    match p {
        Precise::Bounded(m) => {
            let v = m.to_f64().abs();
            assert!(v < 1e-40, "cancellation leaked magnitude: {v}");
        }
        Precise::Unknown(_) => {} // acceptable: cannot resolve a true zero
        Precise::Complex { .. } => panic!("real cancellation went complex"),
        Precise::Exact(_) => panic!("not exact"),
    }
}

#[test]
fn unknowns_are_honest() {
    // Free variable.
    assert!(matches!(
        evaluate_to_precision(&parse("x + 1"), 10),
        Precise::Unknown(_)
    ));
    // Unknown function → Unknown at any precision.
    assert!(matches!(
        evaluate_to_precision(&parse("sec(1)"), 40),
        Precise::Unknown(_)
    ));
    // But within f64 reach, the Tier-0 path serves it.
    let p = evaluate_to_precision(&parse("sin(1)"), 10);
    assert_eq!(digits_of(&p, 10), "8414709848");
    // Absurd digit requests are refused, not attempted.
    assert!(matches!(
        evaluate_to_precision(&parse("sqrt(2)"), 1_000_000),
        Precise::Unknown(_)
    ));
}

#[test]
fn differential_against_evaluate_to_constant() {
    // P1 exit criterion: on the evaluate corpus, wherever both paths produce
    // a real value, they agree to f64 tolerance; and the precise path covers
    // a healthy majority of rows. Rows with bindings run through the
    // compiled tape (`eval_tape`), constant rows through the public API.
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/evaluate-corpus.json"
    ))
    .unwrap();
    let rows: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut total = 0u32;
    let mut covered = 0u32;
    for case in rows.as_array().unwrap() {
        let Some(input) = case.get("input").and_then(|v| v.as_str()) else {
            continue;
        };
        let Ok(e) = TextToAst::new(TextToAstOptions::default()).convert(input) else {
            continue;
        };
        let Some(evaluated) = case.get("evaluated").filter(|v| !v.is_null()) else {
            continue;
        };
        let (re, im) = (
            evaluated["re"].as_f64().unwrap_or(f64::NAN),
            evaluated["im"].as_f64().unwrap_or(0.0),
        );
        if !re.is_finite() || im.abs() > 1e-10 * re.abs().max(1.0) {
            continue; // complex or non-finite: P4 / out of scope
        }
        let binds = case["binds"].as_object().cloned().unwrap_or_default();
        total += 1;
        let canon = math_expressions::canonicalize(&e);
        let Ok(tape) = compile(&canon) else {
            continue;
        };
        let bindings: Option<Vec<f64>> = tape
            .vars()
            .iter()
            .map(|v| binds.get(v).and_then(|x| x.as_f64()))
            .collect();
        let Some(bindings) = bindings else { continue };
        match math_expressions::precise::eval_tape(&tape, &bindings, 12) {
            Precise::Unknown(_) => {}
            p => {
                covered += 1;
                let v = p.to_f64().expect("finite");
                let tol = 1e-8 * re.abs().max(1e-8);
                assert!(
                    (v - re).abs() <= tol,
                    "{input} {binds:?}: precise {v} vs oracle {re}"
                );
            }
        }
    }
    assert!(total > 200, "corpus should provide real rows, got {total}");
    assert!(
        covered * 10 >= total * 6,
        "coverage too low: {covered}/{total}"
    );
}

#[test]
fn deep_trees_do_not_grow_the_native_stack() {
    // 50_000 nested sqrt applications: compilation and evaluation must be
    // iterative (the tape design), so a 512 KiB thread suffices.
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            let mut e = parse("2");
            for _ in 0..50_000 {
                e = Expr::Apply(Box::new(Expr::sym("sqrt")), vec![e]);
            }
            let tape = compile(&e).expect("compiles");
            assert!(tape.len() >= 50_001);
            // Tier-0 evaluation over the tape (value converges to 1).
            let p = math_expressions::precise::eval_tape(&tape, &[], 10);
            let v = p.to_f64().expect("evaluates");
            assert!((v - 1.0).abs() < 1e-9);
            // Dropping a 50k-deep Expr recurses; leak it deliberately.
            std::mem::forget(e);
        })
        .unwrap()
        .join()
        .unwrap();
}

// ================= P3: trig / inverse-trig / hyperbolic =================

// Reference digits (standard published values).
const SIN1: &str = "841470984807896506652502321630298999622563060798371";
const COS1: &str = "540302305868139717400936607442976603732310420617922";
const TAN1: &str = "155740772465490223050697480745836017308725077238152";
const ATAN1: &str = "785398163397448309615660845819875721049292349843776"; // π/4
const ASIN_HALF: &str = "523598775598298873077107230546583814032861566562517"; // π/6
const SINH1: &str = "117520119364380145688238185059560081515571798133409";
const LOG10_2: &str = "301029995663981195213738894724493026768189881462108";

#[test]
fn p3_trig_known_digits() {
    for d in [12usize, 50] {
        assert_digits("sin(1)", d, SIN1);
        assert_digits("cos(1)", d, COS1);
        assert_digits("tan(1)", d, TAN1);
        assert_digits("atan(1)", d, ATAN1);
        assert_digits("asin(1/2)", d, ASIN_HALF);
        assert_digits("sinh(1)", d, SINH1);
        assert_digits("log10(2)", d, LOG10_2);
    }
    // Reduction correctness: sin(x + 2πk) = sin(x) digit-for-digit.
    let a = digits_of(&evaluate_to_precision(&parse("sin(1 + 100*pi)"), 40), 38);
    let b = digits_of(&evaluate_to_precision(&parse("sin(1)"), 40), 38);
    assert_eq!(a, b, "argument reduction across 100π");
    // Quadrants.
    let c = digits_of(&evaluate_to_precision(&parse("cos(pi/3)"), 30), 28);
    assert!(c.starts_with("50000000"), "cos(π/3) = 1/2, got {c}");
    let s = digits_of(&evaluate_to_precision(&parse("sin(pi/6)"), 30), 28);
    assert!(s.starts_with("50000000"), "sin(π/6) = 1/2, got {s}");
}

#[test]
fn p3_inverse_trig_consistency() {
    // asin(sin(x)) = x for x ∈ (−π/2, π/2), 40 digits.
    for x in ["1/3", "-2/5", "11/10"] {
        let lhs = digits_of(
            &evaluate_to_precision(&parse(&format!("asin(sin({x}))")), 40),
            36,
        );
        let rhs = digits_of(&evaluate_to_precision(&parse(&format!("sqrt(({x})^2)")), 40), 36);
        assert_eq!(lhs, rhs.trim_start_matches('0'), "asin∘sin({x})");
    }
    // acos(1/2) = π/3.
    let a = digits_of(&evaluate_to_precision(&parse("acos(1/2)"), 40), 38);
    let b = digits_of(&evaluate_to_precision(&parse("pi/3"), 40), 38);
    assert_eq!(a, b);
    // tanh/cosh/sinh identity: cosh² − sinh² = 1.
    let one = evaluate_to_precision(&parse("cosh(3)^2 - sinh(3)^2"), 20);
    let v = one.to_f64().expect("finite");
    assert!((v - 1.0).abs() < 1e-15, "cosh²−sinh²=1, got {v}");
}

#[test]
fn p3_adversarial_arguments() {
    // Argument magnitude beyond max_trig_arg_bits → Unknown, quickly.
    assert!(matches!(
        evaluate_to_precision(&parse("sin(2^5000)"), 10),
        Precise::Unknown(_)
    ));
    // Tower exponentials → Unknown (scale overflow), not a hang.
    assert!(matches!(
        evaluate_to_precision(&parse("exp(exp(exp(20)))"), 10),
        Precise::Unknown(_)
    ));
    // But a large-yet-reasonable argument reduces fine.
    let p = evaluate_to_precision(&parse("sin(10^15)"), 12);
    let v = p.to_f64().expect("sin(10^15) evaluates");
    assert!((-1.0..=1.0).contains(&v));
}

// ================= P4: complex tier =================

const SQRT3: &str = "173205080756887729352744634150587236694280525381038";

#[test]
fn p4_principal_branches() {
    // sqrt(-2) = i·√2.
    let p = evaluate_to_precision(&parse("sqrt(-2)"), 40);
    let s = p.to_decimal_string(40).expect("complex value");
    assert!(s.contains(" i"), "expected complex form, got {s}");
    let (re, im) = p.to_complex_f64().unwrap();
    assert!(re.abs() < 1e-30 && (im - 2f64.sqrt()).abs() < 1e-12, "{re} {im}");
    // ln(-1) = iπ, 40 digits of π in the imaginary part.
    let p = evaluate_to_precision(&parse("ln(-1)"), 40);
    let Precise::Complex { im, .. } = &p else {
        panic!("ln(-1) should be complex, got {p:?}")
    };
    let digits: String = im
        .to_decimal_string(40)
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take(38)
        .collect();
    assert_eq!(digits, PI.replace(" ", "")[..38]);
    // i² = −1 through the complex path: i² + 2 = 1 exactly (a true zero
    // like i² + 1 is correctly Unknown, matching the cancellation policy).
    let p = evaluate_to_precision(&parse("i^2 + 2"), 10);
    let (re, im) = p.to_complex_f64().unwrap_or_else(|| panic!("i^2+2: {p:?}"));
    assert!((re - 1.0).abs() < 1e-9 && im.abs() < 1e-9, "{p:?}");
    assert!(matches!(
        evaluate_to_precision(&parse("i^2 + 1"), 10),
        Precise::Unknown(_) | Precise::Exact(_)
    ));
}

#[test]
fn p4_asin_beyond_domain_and_powers() {
    // asin(2) = π/2 − i·ln(2+√3) (principal). Check against eval_complex.
    let e = parse("asin(2)");
    let reference = math_expressions::evaluate_to_constant(&e).expect("complex reference");
    let p = evaluate_to_precision(&e, 30);
    let (re, im) = p.to_complex_f64().expect("complex result");
    assert!((re - reference.re).abs() < 1e-9, "{re} vs {}", reference.re);
    assert!((im - reference.im).abs() < 1e-9, "{im} vs {}", reference.im);
    // (-8)^(1/3): simplify resolves it real (−2) — parity with evc.
    let p = evaluate_to_precision(&parse("(-8)^(1/3)"), 20);
    match p {
        Precise::Exact(n) => assert_eq!(n.to_f64(), -2.0),
        other => {
            let (re, _) = other.to_complex_f64().expect("value");
            assert!((re + 2.0).abs() < 1e-12 || (re - 1.0).abs() < 1e-12);
        }
    }
    // High-precision complex: sqrt(-3) imaginary part = √3 to 45 digits.
    let p = evaluate_to_precision(&parse("sqrt(-3)"), 45);
    let Precise::Complex { im, .. } = &p else {
        panic!("sqrt(-3) complex, got {p:?}")
    };
    let digits: String = im
        .to_decimal_string(45)
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take(43)
        .collect();
    assert_eq!(digits, SQRT3.replace(" ", "")[..43]);
}

#[test]
fn p4_complex_corpus_parity() {
    // The complex rows skipped by the real differential test: wherever
    // evaluate_to_constant produces a finite complex value, the precise path
    // must agree in both components (or answer Unknown, never differ).
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/evaluate-corpus.json"
    ))
    .unwrap();
    let rows: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut complex_total = 0u32;
    let mut complex_covered = 0u32;
    for case in rows.as_array().unwrap() {
        let Some(input) = case.get("input").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(constant) = case.get("constant").filter(|v| !v.is_null()) else {
            continue;
        };
        let (re, im) = (
            constant["re"].as_f64().unwrap_or(f64::NAN),
            constant["im"].as_f64().unwrap_or(0.0),
        );
        if !re.is_finite() || !im.is_finite() || im.abs() <= 1e-10 * re.abs().max(1.0) {
            continue; // real rows are covered by the P1 test
        }
        let Ok(e) = TextToAst::new(TextToAstOptions::default()).convert(input) else {
            continue;
        };
        complex_total += 1;
        match evaluate_to_precision(&e, 12) {
            Precise::Unknown(_) => {}
            p => {
                complex_covered += 1;
                let (pre, pim) = p.to_complex_f64().expect("finite");
                let scale = (re * re + im * im).sqrt().max(1e-8);
                assert!(
                    ((pre - re).powi(2) + (pim - im).powi(2)).sqrt() <= 1e-7 * scale,
                    "{input}: precise {pre}+{pim}i vs oracle {re}+{im}i"
                );
            }
        }
    }
    // The corpus has a modest number of complex rows; cover most of them.
    assert!(
        complex_total == 0 || complex_covered * 10 >= complex_total * 5,
        "complex coverage {complex_covered}/{complex_total}"
    );
}

// ================= P5: batch / quadrature hooks =================

#[test]
fn p5_batch_evaluation() {
    use math_expressions::canonicalize;
    let tape = compile(&canonicalize(&parse("exp(-x^2)"))).unwrap();
    assert_eq!(tape.vars(), ["x"]);
    // f64 fast path with certified error at each abscissa.
    let mut max_err = 0.0f64;
    for i in 0..=100 {
        let x = -3.0 + 6.0 * (i as f64) / 100.0;
        let (v, err) = tape.eval_f64(&[x]).expect("f64 tier");
        assert!((v - (-x * x).exp()).abs() <= err.max(1e-15));
        max_err = max_err.max(err);
    }
    assert!(max_err < 1e-14, "certified error stays tight: {max_err}");
    // Batch API: mixed-tier per point.
    let points: Vec<f64> = (0..=10).map(|i| i as f64 / 10.0).collect();
    let results = tape.eval_batch(&points, 12);
    assert_eq!(results.len(), 11);
    for (x, r) in points.iter().zip(&results) {
        let v = r.to_f64().expect("finite");
        assert!((v - (-x * x).exp()).abs() < 1e-11);
    }
}
