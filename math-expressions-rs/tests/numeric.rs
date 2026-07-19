//! f64 numeric module (`me.math` replacements) — hand-checkable cases.
//! Differential coverage against the JS mathjs oracle lives in
//! `numeric_corpus.rs`.

use math_expressions::numeric::*;

fn assert_close(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() <= tol, "expected {b}, got {a}");
}

#[test]
fn scalar_utilities_match_mathjs_conventions() {
    assert_close(math_mod(7.0, 3.0), 1.0, 0.0);
    assert_close(math_mod(-7.0, 3.0), 2.0, 1e-12); // sign of divisor
    assert_close(math_mod(7.0, -3.0), -2.0, 1e-12);
    assert_close(math_mod(5.0, 0.0), 5.0, 0.0); // mathjs: y=0 -> x
    assert_close(gcd_f64(12.0, 18.0), 6.0, 0.0);
    assert_close(gcd_f64(0.0, 0.0), 0.0, 0.0);
    assert!(gcd_f64(1.5, 3.0).is_nan()); // non-integer
    assert_close(lcm_f64(4.0, 6.0), 12.0, 0.0);
    assert_close(lcm_f64(0.0, 5.0), 0.0, 0.0);
}

#[test]
fn statistics_match_mathjs_defaults() {
    let d = [2.0, 4.0, 6.0, 8.0];
    assert_close(mean(&d), 5.0, 0.0);
    assert_close(median(&d), 5.0, 0.0);
    assert_close(variance(&d), 20.0 / 3.0, 1e-12); // unbiased (n-1)
    assert_close(std_dev(&d), (20.0f64 / 3.0).sqrt(), 1e-12);
    assert_close(quantile_seq(&d, 0.25), 3.5, 1e-12); // linear interpolation
    let odd = [3.0, 1.0, 2.0];
    assert_close(median(&odd), 2.0, 0.0);
}

#[test]
fn lusolve_small_systems() {
    // x + y = 3, x - y = 1 -> (2, 1)
    let x = lusolve(&[1.0, 1.0, 1.0, -1.0], &[3.0, 1.0], 2).unwrap();
    assert_close(x[0], 2.0, 1e-12);
    assert_close(x[1], 1.0, 1e-12);
    // Needs pivoting: first pivot is zero.
    let x = lusolve(&[0.0, 1.0, 1.0, 0.0], &[5.0, 7.0], 2).unwrap();
    assert_close(x[0], 7.0, 1e-12);
    assert_close(x[1], 5.0, 1e-12);
    // Singular -> None.
    assert!(lusolve(&[1.0, 2.0, 2.0, 4.0], &[1.0, 2.0], 2).is_none());
}

#[test]
fn eigs_real_symmetric() {
    // [[2,1],[1,2]] -> eigenvalues 1, 3; vectors (1,-1)/sqrt2, (1,1)/sqrt2.
    let pairs = eigs(&[2.0, 1.0, 1.0, 2.0], 2).unwrap();
    assert_eq!(pairs.len(), 2);
    assert_close(pairs[0].value.re, 1.0, 1e-9);
    assert_close(pairs[0].value.im, 0.0, 1e-9);
    assert_close(pairs[1].value.re, 3.0, 1e-9);
    // Eigenvector residual: ||A v - lambda v|| small.
    for p in &pairs {
        check_residual(&[2.0, 1.0, 1.0, 2.0], 2, p);
    }
}

#[test]
fn eigs_complex_pair() {
    // Rotation matrix [[0,-1],[1,0]] -> +/- i.
    let pairs = eigs(&[0.0, -1.0, 1.0, 0.0], 2).unwrap();
    assert_close(pairs[0].value.re, 0.0, 1e-9);
    assert_close(pairs[0].value.im, -1.0, 1e-9);
    assert_close(pairs[1].value.im, 1.0, 1e-9);
    for p in &pairs {
        check_residual(&[0.0, -1.0, 1.0, 0.0], 2, p);
    }
}

#[test]
fn eigs_companion_and_defective() {
    // Companion of t^3 - t^2 - t - 1 (the "tribonacci" matrix).
    let a = [0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0];
    let pairs = eigs(&a, 3).unwrap();
    assert_eq!(pairs.len(), 3);
    let real_root = pairs
        .iter()
        .find(|p| p.value.im == 0.0)
        .expect("one real root");
    assert_close(real_root.value.re, 1.839_286_755_214_161, 1e-9);
    for p in &pairs {
        check_residual(&a, 3, p);
    }
    // Defective: Jordan block [[1,1],[0,1]] — both eigenvalues 1.
    let pairs = eigs(&[1.0, 1.0, 0.0, 1.0], 2).unwrap();
    assert_close(pairs[0].value.re, 1.0, 1e-7);
    assert_close(pairs[1].value.re, 1.0, 1e-7);
}

fn check_residual(a: &[f64], n: usize, p: &EigenPair) {
    let mut max = 0.0f64;
    for i in 0..n {
        let mut av = num_complex::Complex64::new(0.0, 0.0);
        for j in 0..n {
            av += a[i * n + j] * p.vector[j];
        }
        max = max.max((av - p.value * p.vector[i]).norm());
    }
    assert!(max < 1e-8, "eigen residual {max} too large");
    let mag: f64 = p.vector.iter().map(|v| v.norm_sqr()).sum::<f64>().sqrt();
    assert_close(mag, 1.0, 1e-9);
}

#[test]
fn match_template_default_mode() {
    use math_expressions::js_match::match_template;
    use serde_json::json;

    // ["+", ["*", 2, "x"], 3] against ["+", ["*", "a", "x"], "b"]:
    // wildcards a, x, b (all pattern variables).
    let tree = json!(["+", ["*", 2, "x"], 3]);
    let pat = json!(["+", ["*", "a", "y"], "b"]);
    let m = match_template(&tree, &pat).unwrap();
    assert_eq!(m.get("a").unwrap(), &json!(2));
    assert_eq!(m.get("y").unwrap(), &json!("x"));
    assert_eq!(m.get("b").unwrap(), &json!(3));

    // Grouping: last wildcard absorbs the rest of an associative operator.
    let tree = json!(["+", 1, 2, 3]);
    let m = match_template(&tree, &json!(["+", "u", "v"])).unwrap();
    assert_eq!(m.get("u").unwrap(), &json!(1));
    assert_eq!(m.get("v").unwrap(), &json!(["+", 2, 3]));

    // Repeated wildcard must bind equal subtrees.
    assert!(match_template(&json!(["+", "x", "x"]), &json!(["+", "u", "u"])).is_some());
    assert!(match_template(&json!(["+", "x", "y"]), &json!(["+", "u", "u"])).is_none());

    // Unary minus of product matches a * pattern.
    let tree = json!(["-", ["*", "x", "y"]]);
    let m = match_template(&tree, &json!(["*", "a", "b"])).unwrap();
    assert_eq!(m.get("a").unwrap(), &json!(["-", "x"]));
    assert_eq!(m.get("b").unwrap(), &json!("y"));

    // Operators must match exactly; no match across operators.
    assert!(match_template(&json!(["*", 1, 2]), &json!(["+", "u", "v"])).is_none());
    // Exact variable-free match -> empty bindings.
    assert_eq!(
        match_template(&json!(["+", 1, 2]), &json!(["+", 1, 2]))
            .unwrap()
            .len(),
        0
    );
}
