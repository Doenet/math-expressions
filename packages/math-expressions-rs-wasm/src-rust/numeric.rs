//! f64 numeric utilities — the `me.math` replacements (see `src/numeric.rs`):
//! statistics, gcd/lcm, and the mathjs `lusolve`/`eigs` drop-ins.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn math_mod(x: f64, y: f64) -> f64 {
    math_expressions::numeric::math_mod(x, y)
}
#[wasm_bindgen]
pub fn gcd(x: f64, y: f64) -> f64 {
    math_expressions::numeric::gcd_f64(x, y)
}
#[wasm_bindgen]
pub fn lcm(x: f64, y: f64) -> f64 {
    math_expressions::numeric::lcm_f64(x, y)
}
#[wasm_bindgen]
pub fn mean(data: Vec<f64>) -> f64 {
    math_expressions::numeric::mean(&data)
}
#[wasm_bindgen]
pub fn median(data: Vec<f64>) -> f64 {
    math_expressions::numeric::median(&data)
}
/// Unbiased sample variance (mathjs default).
#[wasm_bindgen]
pub fn variance(data: Vec<f64>) -> f64 {
    math_expressions::numeric::variance(&data)
}
#[wasm_bindgen]
pub fn std(data: Vec<f64>) -> f64 {
    math_expressions::numeric::std_dev(&data)
}
/// mathjs `quantileSeq` with linear interpolation.
#[wasm_bindgen]
pub fn quantile_seq(data: Vec<f64>, prob: f64) -> f64 {
    math_expressions::numeric::quantile_seq(&data, prob)
}

/// Solve `A·x = b` for an n×n row-major matrix — the mathjs `lusolve`
/// replacement. `undefined` if singular or mis-sized.
#[wasm_bindgen]
pub fn lusolve(a: Vec<f64>, b: Vec<f64>, n: usize) -> Option<Vec<f64>> {
    math_expressions::numeric::lusolve(&a, &b, n)
}

/// Numeric eigendecomposition of a real n×n row-major matrix — the mathjs
/// `eigs` replacement. Returns JSON in the mathjs result shape Doenet reads:
/// `{"values": [num | {"re","im"}...], "eigenvectors": [{"value": ...,
/// "vector": [...]}]}`. `undefined` when iteration fails to converge.
#[wasm_bindgen]
pub fn eigs(a: Vec<f64>, n: usize) -> Option<String> {
    let pairs = math_expressions::numeric::eigs(&a, n)?;
    fn num(c: num_complex::Complex64) -> serde_json::Value {
        if c.im == 0.0 {
            serde_json::json!(c.re)
        } else {
            serde_json::json!({"re": c.re, "im": c.im})
        }
    }
    let values: Vec<_> = pairs.iter().map(|p| num(p.value)).collect();
    let eigenvectors: Vec<_> = pairs
        .iter()
        .map(|p| {
            serde_json::json!({
                "value": num(p.value),
                "vector": p.vector.iter().map(|&v| num(v)).collect::<Vec<_>>(),
            })
        })
        .collect();
    Some(serde_json::json!({"values": values, "eigenvectors": eigenvectors}).to_string())
}
