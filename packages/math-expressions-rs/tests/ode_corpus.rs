//! Differential ODE corpus (ODE_PLAN.md §3.2): the vendored JS library's
//! `numeric.dopri` (the very function Doenet uses today) sampled at fixed
//! abscissae; the Rust solver — driven through the expression-RHS front end,
//! the API Doenet migrates to — must agree at a mutual tolerance.

use math_expressions::{solve_ode_exprs, Expr, TextToAst, TextToAstOptions};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

#[test]
fn ode_corpus_agrees_with_js_dopri() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ode-corpus.json"
    ))
    .unwrap();
    let rows: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut systems = 0;
    for row in rows.as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let rhs: Vec<Expr> = row["rhs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| parse(v.as_str().unwrap()))
            .collect();
        let vars: Vec<String> = row["vars"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        let y0: Vec<f64> = row["y0"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let t1 = row["t1"].as_f64().unwrap();
        let sol = solve_ode_exprs(&rhs, "t", &vars, 0.0, t1, &y0, 1e-8, 10_000)
            .unwrap_or_else(|| panic!("{name}: not constructible"));
        assert!(!sol.terminated_early, "{name}: terminated early");
        // Scale for the mutual tolerance: the trajectory's max magnitude.
        let scale = row["samples"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|s| s["y"].as_array().unwrap())
            .fold(1.0f64, |m, v| m.max(v.as_f64().unwrap().abs()));
        for sample in row["samples"].as_array().unwrap() {
            let t = sample["t"].as_f64().unwrap();
            let want: Vec<f64> = sample["y"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            let got = sol.at(t);
            for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
                assert!(
                    (g - w).abs() < 1e-4 * scale,
                    "{name} @ t={t} component {i}: rust {g} vs js {w}"
                );
            }
        }
        systems += 1;
    }
    assert!(systems >= 10, "corpus coverage: {systems}");
}
