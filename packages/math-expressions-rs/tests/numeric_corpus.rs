//! Differential corpus for the f64 mathjs_compat module and Doenet-interop
//! utilities, generated from the JS oracle (`me.math`, `me.utils.match`,
//! `me.round_numbers_to_precision_plus_decimals`) by
//! `scripts/generate-numeric-corpus.mjs`.

use math_expressions::{expr, mathjs_compat, ops};
use serde_json::Value;

fn corpus() -> Value {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/numeric-corpus.json"
    ))
    .expect("run scripts/generate-numeric-corpus.mjs first");
    serde_json::from_str(&text).unwrap()
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}
fn fs(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(f).collect()
}

fn assert_close(got: f64, want: f64, tol: f64, ctx: &str) {
    let scale = 1.0f64.max(want.abs());
    assert!(
        (got - want).abs() <= tol * scale || (got.is_nan() && want.is_nan()),
        "{ctx}: got {got}, JS oracle {want}"
    );
}

#[test]
fn scalar_matches_mathjs() {
    for case in corpus()["scalar"].as_array().unwrap() {
        let (x, y) = (f(&case["x"]), f(&case["y"]));
        let got = match case["op"].as_str().unwrap() {
            "mod" => mathjs_compat::math_mod(x, y),
            "gcd" => mathjs_compat::gcd_f64(x, y),
            "lcm" => mathjs_compat::lcm_f64(x, y),
            other => panic!("unknown op {other}"),
        };
        assert_close(got, f(&case["expected"]), 1e-12, &format!("{case}"));
    }
}

#[test]
fn statistics_match_mathjs() {
    for case in corpus()["stats"].as_array().unwrap() {
        let data = fs(&case["data"]);
        assert_close(mathjs_compat::mean(&data), f(&case["mean"]), 1e-12, "mean");
        assert_close(
            mathjs_compat::median(&data),
            f(&case["median"]),
            1e-12,
            "median",
        );
        assert_close(
            mathjs_compat::variance(&data),
            f(&case["variance"]),
            1e-10,
            "variance",
        );
        assert_close(mathjs_compat::std_dev(&data), f(&case["std"]), 1e-10, "std");
        assert_close(
            mathjs_compat::quantile_seq(&data, f(&case["prob"])),
            f(&case["quantile"]),
            1e-10,
            "quantile",
        );
    }
}

#[test]
fn lusolve_matches_mathjs() {
    for case in corpus()["lusolve"].as_array().unwrap() {
        let n = case["n"].as_u64().unwrap() as usize;
        let x = mathjs_compat::lusolve(&fs(&case["a"]), &fs(&case["b"]), n)
            .unwrap_or_else(|| panic!("lusolve failed on JS-solvable system {case}"));
        let want = fs(&case["x"]);
        for i in 0..n {
            assert_close(x[i], want[i], 1e-6, "lusolve component");
        }
    }
}

#[test]
fn eigenvalues_match_mathjs() {
    for case in corpus()["eigs"].as_array().unwrap() {
        let n = case["n"].as_u64().unwrap() as usize;
        let a = fs(&case["a"]);
        let norm = a.iter().map(|v| v.abs()).fold(1.0f64, f64::max);
        let pairs = mathjs_compat::eigs(&a, n).expect("eigs converges where mathjs did");
        // Multiset comparison: each JS value must have a close Rust value
        // (greedy nearest, each used once). Ordering conventions differ.
        let mut ours: Vec<(f64, f64)> = pairs.iter().map(|p| (p.value.re, p.value.im)).collect();
        for jsv in case["values"].as_array().unwrap() {
            let (re, im) = (f(&jsv["re"]), f(&jsv["im"]));
            let (idx, dist) = ours
                .iter()
                .enumerate()
                .map(|(i, &(r, m))| (i, ((r - re).powi(2) + (m - im).powi(2)).sqrt()))
                .min_by(|p, q| p.1.partial_cmp(&q.1).unwrap())
                .expect("value left to match");
            assert!(
                dist <= 1e-6 * norm,
                "eigenvalue {re}+{im}i unmatched (nearest at distance {dist}) in {case}"
            );
            ours.remove(idx);
        }
        // And every eigenpair satisfies its own definition (residual check).
        for p in &pairs {
            let mut max = 0.0f64;
            for i in 0..n {
                let mut av = num_complex::Complex64::new(0.0, 0.0);
                for j in 0..n {
                    av += a[i * n + j] * p.vector[j];
                }
                max = max.max((av - p.value * p.vector[i]).norm());
            }
            assert!(max <= 1e-6 * norm, "residual {max} too large in {case}");
        }
    }
}

#[test]
fn combined_rounding_matches_js() {
    let inf = |v: &Value| match v {
        Value::String(s) if s == "-Infinity" => f64::NEG_INFINITY,
        other => f(other),
    };
    for case in corpus()["round"].as_array().unwrap() {
        let expr = expr::serde::try_from_js(&case["tree"]).unwrap();
        let rounded = ops::round_numbers_to_precision_plus_decimals(
            &expr,
            inf(&case["digits"]),
            inf(&case["decimals"]),
        );
        let got = expr::serde::to_js(&rounded);
        // Exactly, not within a tolerance like the numerical routines above.
        // Both sides round the float's exact binary value and then take the
        // nearest f64 to the decimal that produces, so there is no room for the
        // two to disagree — and while there was (`(v · 10^d).round() / 10^d`),
        // the disagreement was large, not last-ulp: `2e21` came back as
        // `1.9999999999999997e21`. A tolerance here would hide the next one.
        assert!(
            got == case["expected"],
            "round({}, {}, {}): got {got}, JS {}",
            case["tree"],
            case["digits"],
            case["decimals"],
            case["expected"]
        );
    }
}
