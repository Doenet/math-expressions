//! Certified numeric integration (`integrate_to_precision`). The primary
//! oracle is the library itself: integrals with elementary closed forms are
//! checked digit-for-digit against `evaluate_to_precision` of the closed
//! form — an independent code path (symbolic + MpFix vs f64 quadrature).

use math_expressions::precise::{evaluate_to_precision, integrate_to_precision, Precise};
use math_expressions::{Expr, TextToAst, TextToAstOptions};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn quad(f: &str, a: &str, b: &str, digits: usize) -> Precise {
    integrate_to_precision(&parse(f), "x", &parse(a), &parse(b), digits)
}

fn digits_of(p: &Precise, d: usize) -> String {
    let s = p
        .to_decimal_string(d)
        .unwrap_or_else(|| panic!("expected digits from {p:?}"));
    s.chars().filter(|c| c.is_ascii_digit()).take(d).collect()
}

/// Compare quadrature to a closed form at `digits`, allowing the final digit
/// to differ by the two paths' independent roundings (compare digits−1).
fn assert_matches_closed_form(f: &str, a: &str, b: &str, closed: &str, digits: usize) {
    let q = quad(f, a, b, digits);
    let want = evaluate_to_precision(&parse(closed), digits + 5);
    let (qd, wd) = (digits_of(&q, digits - 1), digits_of(&want, digits - 1));
    assert_eq!(
        qd, wd,
        "∫ {f} over [{a},{b}] @ {digits} digits: quad {qd} vs closed form {closed} = {wd}"
    );
}

#[test]
fn integral_of_polynomial() {
    // ∫₀¹ x³ = 1/4.
    let p = quad("x^3", "0", "1", 12);
    let v = p.to_f64().expect("finite");
    assert!((v - 0.25).abs() < 1e-12, "got {v}");
}

#[test]
fn arctan_integral_gives_pi() {
    // ∫₀¹ 4/(1+x²) = π.
    assert_matches_closed_form("4/(1+x^2)", "0", "1", "pi", 12);
}

#[test]
fn sine_integral() {
    // ∫₀¹ sin x = 1 − cos 1.
    assert_matches_closed_form("sin(x)", "0", "1", "1 - cos(1)", 12);
}

#[test]
fn gaussian_integral_known_digits() {
    // ∫₀¹ e^(−x²) = (√π/2)·erf(1) = 0.74682413281242702539946743613185…
    // (reference verified via the exact alternating series Σ(−1)ⁿ/(n!(2n+1))).
    let truth = 0.746_824_132_812_427;
    let p = quad("exp(-x^2)", "0", "1", 12);
    let v = p.to_f64().expect("finite");
    assert!(
        (v - truth).abs() <= truth * 10f64.powi(-11),
        "12-digit request: |{v} − {truth}| too large"
    );
}

#[test]
fn oscillatory_and_composite() {
    // ∫₀¹ eˣ·cos 3x = (e(cos 3 + 3 sin 3) − 1)/10.
    assert_matches_closed_form(
        "exp(x) * cos(3x)",
        "0",
        "1",
        "(e*(cos(3) + 3 sin(3)) - 1)/10",
        11,
    );
    // Wider interval, more oscillation.
    assert_matches_closed_form("sin(10 x)", "0", "2", "(1 - cos(20))/10", 10);
}

#[test]
fn symbolic_endpoints() {
    // ∫₀^π sin x = 2, with a symbolic endpoint.
    let p = integrate_to_precision(&parse("sin(x)"), "x", &parse("0"), &parse("pi"), 11);
    let v = p.to_f64().expect("finite");
    assert!((v - 2.0).abs() < 1e-10, "got {v}");
}

#[test]
fn reversed_endpoints_negate() {
    let fwd = quad("x^2", "0", "2", 10).to_f64().unwrap();
    let rev = quad("x^2", "2", "0", 10).to_f64().unwrap();
    assert!((fwd + rev).abs() < 1e-12 && fwd > 0.0);
}

#[test]
fn accuracy_is_guaranteed_at_every_requested_digit_count() {
    // The certified contract: at `d` digits, the result differs from the
    // true value by less than one unit in the d-th significant digit.
    let truth = std::f64::consts::PI;
    for d in 4..=12 {
        let p = quad("4/(1+x^2)", "0", "1", d);
        let v = p.to_f64().unwrap_or_else(|| panic!("digits {d}: {p:?}"));
        let tol = truth.abs() * 10f64.powi(1 - d as i32);
        assert!(
            (v - truth).abs() <= tol,
            "digits {d}: |{v} − π| > {tol}"
        );
    }
}

#[test]
fn honest_refusals() {
    // Pole inside the interval.
    assert!(matches!(quad("tan(x)", "0", "2", 8), Precise::Unknown(_)));
    // Endpoint singularity (unbounded derivative and value).
    assert!(matches!(
        quad("1/sqrt(x)", "0", "1", 8),
        Precise::Unknown(_)
    ));
    // Beyond the f64-node digit cap.
    assert!(matches!(quad("sin(x)", "0", "1", 20), Precise::Unknown(_)));
    // Free variable that is not the integration variable.
    assert!(matches!(quad("a*x", "0", "1", 8), Precise::Unknown(_)));
    // A zero integral cannot deliver *significant* digits.
    assert!(matches!(quad("sin(x)", "-1", "1", 8), Precise::Unknown(_)));
}

#[test]
fn rootof_constant_in_integrand() {
    // ∫₀¹ ρ·x dx = ρ/2 where ρ is the plastic number (RootOf reaches the
    // interval evaluator and the certified node path).
    let f = parse("rootof(t^3 - t - 1, 0) * x");
    let p = integrate_to_precision(&f, "x", &parse("0"), &parse("1"), 10);
    let v = p.to_f64().expect("finite");
    assert!((v - 1.324_717_957_244_746 / 2.0).abs() < 1e-9, "got {v}");
}
