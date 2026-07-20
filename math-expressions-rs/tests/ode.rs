//! ODE solver (ODE_PLAN.md O1–O3): analytic-solution conformance for the
//! endpoint AND the dense output, chunk chaining, guards, and the
//! expression-RHS front end.

use math_expressions::{solve_ode_exprs, solve_ode_with, Expr, TextToAst, TextToAstOptions};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

const TOL: f64 = 1e-6;

#[test]
fn exponential_growth_endpoint_and_dense() {
    // y′ = y, y(0) = 1 → y = eᵗ.
    let sol = solve_ode_with(
        |_t, y, out| {
            out[0] = y[0];
            true
        },
        0.0,
        2.0,
        &[1.0],
        TOL,
        10_000,
    );
    assert!(!sol.terminated_early);
    assert!((sol.last_t() - 2.0).abs() < 1e-12);
    assert!((sol.last_y()[0] - 2.0f64.exp()).abs() < 10.0 * TOL * 2.0f64.exp());
    // Dense output across the whole interval.
    for i in 0..=100 {
        let t = 2.0 * i as f64 / 100.0;
        let v = sol.at(t)[0];
        assert!(
            (v - t.exp()).abs() < 10.0 * TOL * t.exp(),
            "dense at {t}: {v} vs {}",
            t.exp()
        );
    }
}

#[test]
fn harmonic_oscillator_2d() {
    // y″ = −y as a system: (y, v)′ = (v, −y); y(0)=1, v(0)=0 → y = cos t.
    let sol = solve_ode_with(
        |_t, y, out| {
            out[0] = y[1];
            out[1] = -y[0];
            true
        },
        0.0,
        10.0,
        &[1.0, 0.0],
        TOL,
        10_000,
    );
    assert!(!sol.terminated_early);
    for i in 0..=200 {
        let t = 10.0 * i as f64 / 200.0;
        let y = sol.at(t);
        assert_eq!(y.len(), 2, "always a length-n vector (§5a)");
        assert!((y[0] - t.cos()).abs() < 1e-4, "cos at {t}: {}", y[0]);
        assert!((y[1] + t.sin()).abs() < 1e-4, "−sin at {t}: {}", y[1]);
    }
}

#[test]
fn logistic_against_closed_form() {
    // y′ = y(1 − y), y(0) = 1/2 → y = 1/(1 + e^{−t}).
    let sol = solve_ode_with(
        |_t, y, out| {
            out[0] = y[0] * (1.0 - y[0]);
            true
        },
        0.0,
        6.0,
        &[0.5],
        TOL,
        10_000,
    );
    assert!(!sol.terminated_early);
    for i in 0..=60 {
        let t = 6.0 * i as f64 / 60.0;
        let want = 1.0 / (1.0 + (-t).exp());
        assert!((sol.at(t)[0] - want).abs() < 1e-5);
    }
}

#[test]
fn backward_integration() {
    // y′ = y integrated from 1 back to 0: y(0) = y(1)/e.
    let sol = solve_ode_with(
        |_t, y, out| {
            out[0] = y[0];
            true
        },
        1.0,
        0.0,
        &[1.0f64.exp()],
        TOL,
        10_000,
    );
    assert!(!sol.terminated_early);
    assert!((sol.last_y()[0] - 1.0).abs() < 1e-4);
    assert!((sol.at(0.5)[0] - 0.5f64.exp()).abs() < 1e-4);
}

#[test]
fn blow_up_terminates_cleanly() {
    // y′ = y², y(0) = 1 blows up at t = 1: must stop before it, finite.
    let sol = solve_ode_with(
        |_t, y, out| {
            out[0] = y[0] * y[0];
            true
        },
        0.0,
        2.0,
        &[1.0],
        TOL,
        10_000,
    );
    assert!(sol.terminated_early, "must flag early termination");
    assert!(sol.last_t() < 1.0 + 1e-3, "stops at the singularity");
    assert!(sol.last_t() > 0.5, "but gets close to it");
    for i in 0..=50 {
        let t = sol.last_t() * i as f64 / 50.0;
        assert!(sol.at(t)[0].is_finite(), "no NaN samples (Doenet contract)");
    }
}

#[test]
fn step_budget_is_honest() {
    let sol = solve_ode_with(
        |_t, y, out| {
            out[0] = y[0];
            true
        },
        0.0,
        1.0,
        &[1.0],
        1e-12, // tight tolerance…
        3,     // …with a budget too small for it
    );
    assert!(sol.terminated_early);
    assert!(sol.last_t() < 1.0);
}

#[test]
fn chunk_chaining_matches_direct() {
    // ODESystem's pattern (§5b): integrate [0,1], continue [1,2] from
    // last_y — must agree with one [0,2] run.
    let f = |_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -0.5 * y[0] + (2.0 * _t).sin();
        true
    };
    let first = solve_ode_with(f, 0.0, 1.0, &[1.0], TOL, 10_000);
    let second = solve_ode_with(f, first.last_t(), 2.0, &first.last_y(), TOL, 10_000);
    let direct = solve_ode_with(f, 0.0, 2.0, &[1.0], TOL, 10_000);
    assert!((second.last_y()[0] - direct.last_y()[0]).abs() < 1e-5);
}

// ================= O2/O3: expression right-hand sides =================

#[test]
fn expr_rhs_via_tape() {
    // Kernel-only RHS compiles to the evaluation tape.
    let sol = solve_ode_exprs(
        &[parse("y")],
        "t",
        &["y".to_string()],
        0.0,
        1.0,
        &[1.0],
        TOL,
        10_000,
    )
    .expect("solvable");
    assert!(!sol.terminated_early);
    assert!((sol.last_y()[0] - 1.0f64.exp()).abs() < 1e-4);
}

#[test]
fn expr_rhs_system_with_time_dependence() {
    // (x, y)′ = (y, −sin(t) − x): 2-dimensional, t-dependent.
    let sol = solve_ode_exprs(
        &[parse("y"), parse("-sin(t) - x")],
        "t",
        &["x".to_string(), "y".to_string()],
        0.0,
        3.0,
        &[0.0, 1.0],
        TOL,
        10_000,
    )
    .expect("solvable");
    assert!(!sol.terminated_early);
    // Cross-check against the closure formulation.
    let direct = solve_ode_with(
        |t, y, out| {
            out[0] = y[1];
            out[1] = -t.sin() - y[0];
            true
        },
        0.0,
        3.0,
        &[0.0, 1.0],
        TOL,
        10_000,
    );
    for i in 0..=30 {
        let t = 3.0 * i as f64 / 30.0;
        let a = sol.at(t);
        let b = direct.at(t);
        assert!((a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6);
    }
}

#[test]
fn expr_rhs_rejects_unknown_variables() {
    assert!(solve_ode_exprs(
        &[parse("a*y")],
        "t",
        &["y".to_string()],
        0.0,
        1.0,
        &[1.0],
        TOL,
        1000
    )
    .is_none());
}

#[test]
fn expr_rhs_domain_failure_terminates_early() {
    // y′ = ln(y) from y(0) = 1/2 drives y down toward 0 where ln blows up:
    // must terminate cleanly, not panic or emit NaN.
    let sol = solve_ode_exprs(
        &[parse("ln(y)")],
        "t",
        &["y".to_string()],
        0.0,
        10.0,
        &[0.5],
        TOL,
        10_000,
    )
    .expect("constructible");
    assert!(sol.terminated_early);
    assert!(sol.last_y()[0].is_finite());
}
