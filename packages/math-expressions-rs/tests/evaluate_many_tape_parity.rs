//! `evaluate_many` runs a compiled Tier-0 tape and falls back to `eval_complex`
//! wherever that tape escalates. This suite pins the contract that makes the
//! fast path safe: **for every expression and every point, the batched result
//! is what the single-point complex evaluator would have said.**
//!
//! `reference` below is the pre-tape body of `evaluate_many`, spelled out
//! against the public single-point `evaluate`. If a tape change ever makes the
//! two disagree, that is a behaviour change reaching JS callers, not a rounding
//! detail — which is why the corpus leans on the cases where a real-f64 tape
//! and a complex-principal-branch walk are most likely to part ways: negative
//! bases under fractional powers, poles, log/sqrt domain edges, and overflow.

use math_expressions::{evaluate_fast_f64, evaluate_many, Expr, TextToAst};
use std::collections::HashMap;

fn t(s: &str) -> Expr {
    TextToAst::new(Default::default()).convert(s).unwrap()
}

/// What `evaluate_many` returned before the tape existed: the complex walk,
/// with a point discarded as `NaN` unless its imaginary part is negligible
/// against the real one's scale.
fn reference(e: &Expr, var: &str, x: f64) -> f64 {
    let mut bindings = HashMap::new();
    bindings.insert(var.to_string(), x);
    match evaluate_fast_f64(e, &bindings) {
        Some(v) if v.im.abs() <= 1e-10 * v.re.abs().max(1.0) => v.re,
        _ => f64::NAN,
    }
}

/// Agreement to a relative tolerance, with `NaN` a value that must match `NaN`.
/// The tape and the complex walk associate their arithmetic differently, so the
/// last ulp is allowed to differ; anything larger is a real divergence.
fn agrees(fast: f64, slow: f64) -> bool {
    if slow.is_nan() || fast.is_nan() {
        return slow.is_nan() && fast.is_nan();
    }
    if fast == slow {
        return true;
    }
    (fast - slow).abs() <= 1e-12 * fast.abs().max(slow.abs()).max(1.0)
}

/// Points chosen to straddle the interesting boundaries: signs, zero, poles at
/// 0 and ±1, integer and non-integer, and magnitudes that overflow `exp`.
fn probe_points() -> Vec<f64> {
    let mut xs = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        -2.0,
        8.0,
        -8.0,
        0.5,
        -0.5,
        1e-9,
        -1e-9,
        1e9,
        -1e9,
        710.0,
        -710.0,
        std::f64::consts::PI,
        -std::f64::consts::E,
    ];
    // A dense sweep as well, so structural agreement is not an artifact of
    // hand-picked abscissas.
    for i in 0..=400 {
        xs.push(-20.0 + 40.0 * (i as f64) / 400.0);
    }
    xs
}

const CORPUS: &[&str] = &[
    // Polynomial / rational — the shapes a plotter sees most.
    "x",
    "x+1",
    "x^2-3*x+1",
    "x/3",
    "-x",
    "1/2",
    "1/x",
    "1/(x-1)",
    "(x^2-1)/(x-1)",
    "x^5 - 4*x^3 + x",
    "1/(1+x^2)",
    // Transcendental.
    "sin(x)",
    "cos(2*x)",
    "tan(x)",
    "exp(x)",
    "e^x",
    "sin(x)*exp(-x/3) - 1/2",
    "sin(x)*exp(-x/3) - 1/2 + cos(2*x)*sin(3*x)/(1+x^2)",
    "sin(x)/x",
    "atan(x)",
    // Domain edges — where a real tape must escalate rather than invent a value.
    "sqrt(x)",
    "log(x)",
    "ln(x)",
    "sqrt(x-1)",
    "log(x^2)",
    "arcsin(x/8)",
    "arctanh(x/8)",
    "asin(x)",
    "acos(x)",
    // Fractional powers of a possibly-negative base. Odd roots read on the
    // real branch (`(-8)^(1/3) = -2`), even roots stay principal (a gap
    // here); either way the tape escalates at a negative base and the
    // fallback must agree with the reference walk — including on the raw
    // `Div`-node exponent shape the fallback sees.
    "x^(1/3)",
    "x^(1/2)",
    "x^(2/3)",
    "x^x",
    "(-8)^(1/3)",
    // Constants and other-variable cases (the fast path must decline these).
    "3",
    "pi",
    "2*pi + 1",
    "y",
    "x + y",
    "sin(y)*x",
];

#[test]
fn tape_fast_path_matches_complex_reference() {
    let xs = probe_points();
    let mut divergences = Vec::new();
    for src in CORPUS {
        let e = t(src);
        let fast = evaluate_many(&e, "x", &xs);
        assert_eq!(fast.len(), xs.len(), "{src}: one result per point");
        for (i, &x) in xs.iter().enumerate() {
            let slow = reference(&e, "x", x);
            if !agrees(fast[i], slow) {
                divergences.push(format!("  {src}  at x={x}:  tape {}  vs  {slow}", fast[i]));
            }
        }
    }
    assert!(
        divergences.is_empty(),
        "evaluate_many diverged from the complex reference at {} point(s):\n{}",
        divergences.len(),
        divergences.join("\n")
    );
}

/// The fast path is keyed to the variable being swept. An expression in a
/// *different* free variable has no binding, so every point is a gap — the tape
/// must decline the batch rather than bind its lone slot to the wrong values.
#[test]
fn other_variable_stays_unbound() {
    let xs = vec![0.0, 1.0, 2.0, -3.5];
    let out = evaluate_many(&t("y^2"), "x", &xs);
    assert!(
        out.iter().all(|v| v.is_nan()),
        "sweeping x must not bind y: {out:?}"
    );
}

/// A constant expression compiles to a tape with no variable slots; every point
/// still gets that constant back, one result per point asked about.
#[test]
fn constant_expression_repeats_its_value() {
    let xs = vec![-1.0, 0.0, 7.0];
    let out = evaluate_many(&t("2*pi"), "x", &xs);
    assert_eq!(out.len(), 3);
    for v in out {
        assert!((v - 2.0 * std::f64::consts::PI).abs() < 1e-12, "got {v}");
    }
}

/// Non-finite results stay gaps rather than becoming `±inf` values: a pole is a
/// hole in the sample set, which is what the `NaN` marker means to consumers.
#[test]
fn poles_are_gaps() {
    let out = evaluate_many(&t("1/x"), "x", &[0.0, 1.0, -1.0]);
    assert!(out[0].is_nan(), "1/0 must be a gap, got {}", out[0]);
    assert_eq!(out[1], 1.0);
    assert_eq!(out[2], -1.0);
}

/// An empty request is an empty answer, not a panic.
#[test]
fn empty_input_is_empty_output() {
    assert!(evaluate_many(&t("sin(x)"), "x", &[]).is_empty());
}

/// The compiled tape is memoized, so the failure mode to rule out is a stale
/// hit: expression B answered with expression A's tape. Alternate between
/// expressions — and between *variables* on the same expression, since entries
/// are keyed by both — and demand the right answer every time.
#[test]
fn cache_does_not_answer_from_the_wrong_expression() {
    let xs = [0.5, 1.0, 2.0, -3.0];
    let (sq, cu) = (t("x^2"), t("x^3"));
    for _ in 0..4 {
        assert_eq!(evaluate_many(&sq, "x", &xs), vec![0.25, 1.0, 4.0, 9.0]);
        assert_eq!(evaluate_many(&cu, "x", &xs), vec![0.125, 1.0, 8.0, -27.0]);
    }
    // Same expression, different swept variable: `y` is bound, `x` is not.
    let e = t("y^2");
    for _ in 0..3 {
        assert_eq!(evaluate_many(&e, "y", &xs), vec![0.25, 1.0, 4.0, 9.0]);
        assert!(evaluate_many(&e, "x", &xs).iter().all(|v| v.is_nan()));
    }
    // An expression the tape declines must not leave another tape in play.
    assert!(evaluate_many(&t("z+1"), "x", &[2.0])[0].is_nan());
    assert_eq!(evaluate_many(&sq, "x", &[3.0]), vec![9.0]);
}

/// Interleaving more distinct curves than the cache can hold is the case a
/// one-slot cache got wrong — and correctness must not depend on capacity, so
/// this sweeps well past it and checks every value on every pass.
#[test]
fn interleaved_curves_past_cache_capacity_stay_correct() {
    let xs: Vec<f64> = (-30..=30).map(|i| i as f64 / 3.0).collect();
    // Each curve carries its index into its value, so a stale tape shows up as
    // a wrong number rather than a coincidence.
    let curves: Vec<(usize, Expr)> = (1..=20)
        .map(|k| (k, t(&format!("{k}*x^2 + {k}"))))
        .collect();
    for _ in 0..3 {
        for (k, e) in &curves {
            let got = evaluate_many(e, "x", &xs);
            for (i, &x) in xs.iter().enumerate() {
                let want = (*k as f64) * x * x + (*k as f64);
                assert!(
                    (got[i] - want).abs() <= 1e-13 * want.abs().max(1.0),
                    "curve {k} at x={x}: got {} want {want}",
                    got[i]
                );
            }
        }
    }
}

/// `ln` is a registered alias of `log`, and both evaluators now resolve it.
/// This used to be a silent `null`/`NaN` for every input — `evaluate_fast_f64(ln(2))`
/// returned `None` while `log(2)` evaluated — because the registry matches
/// evaluation rules on the canonical spelling and neither entry point
/// normalized first. The batch and single-point paths must agree, and both
/// must produce the logarithm.
#[test]
fn alias_spellings_evaluate() {
    let xs = [0.5, 1.0, 2.0, 8.0];
    for (alias, canonical) in [
        ("ln(x)", "log(x)"),
        ("arcsin(x/8)", "asin(x/8)"),
        ("arctan(x)", "atan(x)"),
        ("arccos(x/8)", "acos(x/8)"),
        ("arcsinh(x)", "asinh(x)"),
    ] {
        let (a, c) = (t(alias), t(canonical));
        let (ba, bc) = (evaluate_many(&a, "x", &xs), evaluate_many(&c, "x", &xs));
        for i in 0..xs.len() {
            assert!(
                agrees(ba[i], bc[i]),
                "{alias} vs {canonical} at x={}: {} vs {}",
                xs[i],
                ba[i],
                bc[i]
            );
            assert!(
                agrees(ba[i], reference(&a, "x", xs[i])),
                "{alias} batch/point"
            );
        }
    }
    // The concrete regression: a real value, not a gap.
    let v = evaluate_many(&t("ln(x)"), "x", &[2.0])[0];
    assert!(
        (v - std::f64::consts::LN_2).abs() < 1e-15,
        "ln(2) should be {}, got {v}",
        std::f64::consts::LN_2
    );
}
