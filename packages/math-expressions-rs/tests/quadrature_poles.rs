//! Quadrature behavior at and near poles. Documented contract: divergent
//! and improper integrals are **honest, fast refusals** (there is no
//! divergence *test* — budget exhaustion is deliberately indistinguishable
//! from slow convergence); merely near-singular but smooth integrands must
//! converge to certified digits, however sharp the spike.

use math_expressions::precise::{evaluate_to_precision, integrate_to_precision, Precise};
use math_expressions::{Expr, TextToAst, TextToAstOptions};
use std::time::Instant;

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn quad(f: &str, a: &str, b: &str, digits: usize) -> Precise {
    integrate_to_precision(&parse(f), "x", &parse(a), &parse(b), digits)
}

#[test]
fn divergent_and_improper_integrals_refuse_fast() {
    // Divergent (non-integrable poles) and convergent-improper (integrable
    // endpoint singularities) alike: the certified machinery cannot bound
    // them, so both refuse — quickly, via the budget/domain caps.
    let cases = [
        ("1/x^2", "-1", "1"),      // divergent, interior pole
        ("1/x", "0", "1"),         // divergent, endpoint pole
        ("tan(x)", "0", "2"),      // divergent, pole at π/2
        ("1/(x - 1/3)^2", "0", "1"), // divergent, interior rational pole
        ("1/sqrt(x)", "0", "1"),   // CONVERGENT improper — still refused
        ("ln(x)", "0", "1"),       // convergent improper — still refused
    ];
    for (f, a, b) in cases {
        let t = Instant::now();
        let p = quad(f, a, b, 8);
        let dt = t.elapsed();
        assert!(
            matches!(p, Precise::Unknown(_)),
            "∫ {f} over [{a},{b}] must refuse, got {p:?}"
        );
        assert!(
            dt.as_millis() < 2_000,
            "refusal must be budget-fast, took {dt:?} for {f}"
        );
    }
}

#[test]
fn near_pole_spikes_converge_certified() {
    // 1/((x−½)² + ε²) is smooth; its integral over [0,1] is
    // (2/ε)·atan(1/(2ε)). The adaptive splitter must climb the spike:
    // this is the case whose huge per-segment remainder bounds once drove
    // the incremental error tracker into catastrophic cancellation (a
    // spurious 0 ± 10⁸ "answer" — now a drift-checked break).
    for (eps_pow, closed) in [(3, "2*10^3*atan(5*10^2)"), (6, "2*10^6*atan(5*10^5)")] {
        let f = format!("1/((x-1/2)^2 + 10^(-{}))", 2 * eps_pow);
        let p = integrate_to_precision(&parse(&f), "x", &parse("0"), &parse("1"), 8);
        let v = p.to_f64().unwrap_or_else(|| panic!("{f}: {p:?}"));
        let want = evaluate_to_precision(&parse(closed), 12)
            .to_f64()
            .expect("closed form");
        assert!(
            ((v - want) / want).abs() < 1e-7,
            "{f}: got {v}, want {want}"
        );
    }
}

#[test]
fn regular_integrand_with_pole_outside_interval() {
    let p = quad("1/x^2", "1", "2", 10);
    let v = p.to_f64().expect("finite");
    assert!((v - 0.5).abs() < 1e-10);
}
