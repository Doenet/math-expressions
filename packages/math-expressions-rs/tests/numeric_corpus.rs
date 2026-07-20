//! Differential corpus for the f64 numeric module and Doenet-interop
//! utilities, generated from the JS oracle (`me.math`, `me.utils.match`,
//! `me.round_numbers_to_precision_plus_decimals`) by
//! `scripts/generate-numeric-corpus.mjs`.

use math_expressions::{js_match, js_tree, numeric, ops};
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
            "mod" => numeric::math_mod(x, y),
            "gcd" => numeric::gcd_f64(x, y),
            "lcm" => numeric::lcm_f64(x, y),
            other => panic!("unknown op {other}"),
        };
        assert_close(got, f(&case["expected"]), 1e-12, &format!("{case}"));
    }
}

#[test]
fn statistics_match_mathjs() {
    for case in corpus()["stats"].as_array().unwrap() {
        let data = fs(&case["data"]);
        assert_close(numeric::mean(&data), f(&case["mean"]), 1e-12, "mean");
        assert_close(numeric::median(&data), f(&case["median"]), 1e-12, "median");
        assert_close(
            numeric::variance(&data),
            f(&case["variance"]),
            1e-10,
            "variance",
        );
        assert_close(numeric::std_dev(&data), f(&case["std"]), 1e-10, "std");
        assert_close(
            numeric::quantile_seq(&data, f(&case["prob"])),
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
        let x = numeric::lusolve(&fs(&case["a"]), &fs(&case["b"]), n)
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
        let pairs = numeric::eigs(&a, n).expect("eigs converges where mathjs did");
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
fn match_agrees_with_js_default_mode() {
    for case in corpus()["match"].as_array().unwrap() {
        let got = js_match::match_template(&case["tree"], &case["pattern"]);
        match (&case["bindings"], got) {
            (Value::Null, None) => {}
            (Value::Null, Some(m)) => panic!(
                "JS found no match but we bound {:?} in {case}",
                Value::Object(m)
            ),
            (expected, None) => panic!("JS bound {expected} but we found no match in {case}"),
            (expected, Some(m)) => {
                let exp = expected.as_object().unwrap();
                assert_eq!(
                    exp.len(),
                    m.len(),
                    "binding sets differ in {case}: JS {expected}, ours {:?}",
                    Value::Object(m.clone())
                );
                for (k, v) in exp {
                    assert_eq!(
                        m.get(k),
                        Some(v),
                        "binding {k} differs in {case}: ours {:?}",
                        Value::Object(m.clone())
                    );
                }
            }
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
        let expr = js_tree::try_from_js(&case["tree"]).unwrap();
        let rounded = ops::round_numbers_to_precision_plus_decimals(
            &expr,
            inf(&case["digits"]),
            inf(&case["decimals"]),
        );
        let got = js_tree::to_js(&rounded);
        assert!(
            trees_close(&got, &case["expected"]),
            "round({}, {}, {}): got {got}, JS {}",
            case["tree"],
            case["digits"],
            case["decimals"],
            case["expected"]
        );
    }
}

/// Structural equality with a tiny numeric tolerance on number leaves (JS
/// float rounding vs. our exact-rational rounding can differ in the last ulp).
fn trees_close(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            (x - y).abs() <= 1e-12 * 1.0f64.max(y.abs())
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| trees_close(p, q))
        }
        _ => a == b,
    }
}
