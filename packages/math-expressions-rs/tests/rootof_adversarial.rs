//! Adversarial root finding: ill-conditioned, clustered, high-degree, and
//! huge-coefficient polynomials. The contract under stress is the §7f one:
//! every answer is *certified* (exact Sturm counts + certified refinement)
//! or an honest refusal — never a wrong value or a wrong index order.

use math_expressions::eval::{eval_complex, Env};
use math_expressions::precise::{evaluate_to_precision, Precise};
use math_expressions::{canonicalize, Expr, TextToAst, TextToAstOptions};
use num_complex::Complex64;

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn digits_of(p: &Precise, d: usize) -> String {
    let s = p
        .to_decimal_string(d)
        .unwrap_or_else(|| panic!("expected digits from {p:?}"));
    s.chars().filter(|c| c.is_ascii_digit()).take(d).collect()
}

/// `rootof(<poly>, k)` text from dense i128 coefficients (low → high).
fn rootof_text(coeffs: &[i128], k: usize) -> String {
    let terms: Vec<String> = coeffs
        .iter()
        .enumerate()
        .filter(|(_, c)| **c != 0)
        .map(|(i, c)| match i {
            0 => format!("({c})"),
            1 => format!("({c})*t"),
            _ => format!("({c})*t^{i}"),
        })
        .collect();
    format!("rootof({}, {k})", terms.join(" + "))
}

fn root_value(coeffs: &[i128], k: usize) -> Option<Complex64> {
    let e = canonicalize(&parse(&rootof_text(coeffs, k)));
    assert!(
        matches!(e, Expr::RootOf { .. }),
        "polynomial must canonicalize to a RootOf leaf"
    );
    eval_complex(&e, &Env::new())
}

/// ∏ (x − k) for k = 1..=n, exact i128 convolution.
fn wilkinson(n: i128) -> Vec<i128> {
    let mut p: Vec<i128> = vec![1];
    for k in 1..=n {
        let mut next = vec![0i128; p.len() + 1];
        for (i, c) in p.iter().enumerate() {
            next[i + 1] += c;
            next[i] -= k * c;
        }
        p = next;
    }
    p
}

#[test]
fn wilkinson_20_all_roots_exact() {
    // The classic ill-conditioned case: 20 real roots 1..20, coefficients
    // up to ~10¹⁹. Coefficient-space conditioning is terrible for floating
    // point — but isolation here is *exact* Sturm arithmetic, so every root
    // must come back at its integer, in ascending index order.
    let w = wilkinson(20);
    for k in 0..20usize {
        let z = root_value(&w, k).unwrap_or_else(|| panic!("W20 root {k} unevaluable"));
        assert_eq!(z.im, 0.0, "W20 roots are real");
        let expect = (k + 1) as f64;
        assert!(
            (z.re - expect).abs() < 1e-6,
            "W20 root {k}: got {}, want {expect}",
            z.re
        );
    }
    // And through the certified arbitrary-precision path: the 5th root IS 5.
    let p = evaluate_to_precision(&parse(&rootof_text(&w, 4)), 30);
    assert!(
        digits_of(&p, 30).starts_with("5000000000"),
        "certified refinement of an exact integer root: {p:?}"
    );
}

#[test]
fn mignotte_close_real_pair() {
    // Mignotte's family: t¹⁰ − 2(10t − 1)² = t¹⁰ − 200t² + 40t − 2
    // (irreducible by Eisenstein at 2) has two real roots ~1.4·10⁻⁶ apart
    // near 0.1. Exact isolation must separate them; refinement must give
    // 40 digits of each; ordering must be ascending.
    let coeffs: Vec<i128> = {
        let mut c = vec![0i128; 11];
        c[10] = 1;
        c[2] = -200;
        c[1] = 40;
        c[0] = -2;
        c
    };
    let mut reals: Vec<(usize, f64)> = Vec::new();
    for k in 0..10 {
        if let Some(z) = root_value(&coeffs, k) {
            if z.im == 0.0 {
                reals.push((k, z.re));
            }
        }
    }
    let near: Vec<&(usize, f64)> = reals.iter().filter(|(_, v)| (v - 0.1).abs() < 1e-3).collect();
    assert_eq!(near.len(), 2, "two roots near 1/10: {reals:?}");
    let (k0, v0) = *near[0];
    let (k1, v1) = *near[1];
    assert!(k0 + 1 == k1 && v0 < v1, "adjacent ascending indices");
    let sep = v1 - v0;
    assert!(
        (1e-7..1e-5).contains(&sep),
        "separation ≈ 1.4e-6, got {sep:e}"
    );
    // 40-digit certified values: distinct, and consistent with the f64
    // separation (the pair straddles 1/10, so the significant-digit strings
    // differ from the first digit — 0.099999… vs 0.100000…).
    let d0 = digits_of(
        &evaluate_to_precision(&parse(&rootof_text(&coeffs, k0)), 40),
        40,
    );
    let d1 = digits_of(
        &evaluate_to_precision(&parse(&rootof_text(&coeffs, k1)), 40),
        40,
    );
    assert_ne!(d0, d1, "certified digits distinguish the pair");
    assert!(d0.starts_with("9999") && d1.starts_with("1000"), "{d0} / {d1}");
}

#[test]
fn clustered_conjugate_pairs_certify_or_refuse() {
    // (t² + 1)(t² + c): two conjugate pairs on the imaginary axis.
    // Separation 5e-3 → must certify and order by |im|.
    // Separation 1e-9 → below the f64 ordering certificate → honest refusal.
    let resolvable: Vec<i128> = vec![10201, 0, 20201, 0, 10000]; // c = (101/100)²
    for k in 0..4 {
        let z = root_value(&resolvable, k).unwrap_or_else(|| panic!("root {k}"));
        assert!(z.im != 0.0);
    }
    let z0 = root_value(&resolvable, 0).unwrap();
    let z2 = root_value(&resolvable, 2).unwrap();
    // Pairs ordered by |im|: first pair is ±i, second ±1.01i, negative first.
    assert!(z0.im < 0.0 && (z0.im.abs() - 1.0).abs() < 1e-9);
    assert!(z2.im < 0.0 && (z2.im.abs() - 1.01).abs() < 1e-9);

    // c = (1 + 10⁻⁹)²: scaled to integers, pairs 1e-9 apart.
    let e9: i128 = 1_000_000_000;
    let c_num = (e9 + 1) * (e9 + 1); // (10⁹+1)²  over 10¹⁸
    let scale = e9 * e9;
    let unresolvable: Vec<i128> = vec![c_num, 0, scale + c_num, 0, scale];
    for k in 0..4 {
        assert!(
            root_value(&unresolvable, k).is_none(),
            "1e-9 pair separation is below the ordering certificate — must refuse"
        );
    }
    let p = evaluate_to_precision(&parse(&rootof_text(&unresolvable, 0)), 10);
    assert!(
        matches!(p, Precise::Unknown(_)),
        "precise path refuses too: {p:?}"
    );
}

#[test]
fn high_degree_sparse() {
    // t⁵⁰ − t − 1: 2 real roots, 48 complex, ~uniformly spread near |t| = 1.
    let mut coeffs = vec![0i128; 51];
    coeffs[50] = 1;
    coeffs[1] = -1;
    coeffs[0] = -1;
    let mut n_real = 0;
    let mut prev_key: Option<(u8, f64, f64, f64)> = None;
    for k in 0..50 {
        let z = root_value(&coeffs, k).unwrap_or_else(|| panic!("deg-50 root {k}"));
        // Residual sanity via f64 Horner (coefficients are tiny here).
        let mut p = Complex64::ZERO;
        for c in coeffs.iter().rev() {
            p = p * z + Complex64::new(*c as f64, 0.0);
        }
        assert!(
            p.norm() < 1e-8 * 50.0 * z.norm().powi(49).max(1.0),
            "root {k} residual {p:?}"
        );
        if z.im == 0.0 {
            n_real += 1;
        }
        // Canonical order: reals first ascending, then pairs by (re,|im|).
        let key = (u8::from(z.im != 0.0), z.re, z.im.abs(), z.im);
        if let Some(prev) = prev_key {
            assert!(prev <= key, "canonical index order violated at {k}");
        }
        // Conjugate adjacency: even complex positions are the (im < 0) mate.
        prev_key = Some(key);
    }
    assert_eq!(n_real, 2, "t⁵⁰ − t − 1 has exactly two real roots");
    // Certified digits at depth on the largest real root, cross-checked
    // against a 2× precision run.
    let e = parse(&rootof_text(&coeffs, 1));
    let a = digits_of(&evaluate_to_precision(&e, 30), 29);
    let b = digits_of(&evaluate_to_precision(&e, 60), 29);
    assert_eq!(a, b);
}

#[test]
fn huge_coefficients() {
    // t³ − 2·10³⁰: the real root is 2^(1/3)·10¹⁰ — the Cauchy bound is
    // astronomically wide, so isolation stress-tests the bisection budget.
    let mut coeffs = vec![0i128; 4];
    coeffs[3] = 1;
    coeffs[0] = -2_000_000_000_000_000_000_000_000_000_000;
    let e = parse(&rootof_text(&coeffs, 0));
    let via_root = digits_of(&evaluate_to_precision(&e, 30), 30);
    let via_pow = digits_of(&evaluate_to_precision(&parse("2^(1/3)"), 40), 30);
    assert_eq!(
        via_root, via_pow,
        "2^(1/3)·10¹⁰ digits must match the kernel path"
    );
    let z = eval_complex(&canonicalize(&e), &Env::new()).expect("numeric");
    assert!((z.re - 1.259921049894873e10).abs() < 1.0);
}

#[test]
fn degree_cap_refuses() {
    // Degree 65 exceeds max_rootof_degree: stays an unevaluated application,
    // and the precise path answers Unknown.
    let mut coeffs = vec![0i128; 66];
    coeffs[65] = 1;
    coeffs[1] = -1;
    coeffs[0] = -1;
    let c = canonicalize(&parse(&rootof_text(&coeffs, 0)));
    assert!(
        !matches!(c, Expr::RootOf { .. }),
        "above the degree cap the application must stay opaque"
    );
    let p = evaluate_to_precision(&parse(&rootof_text(&coeffs, 0)), 10);
    assert!(matches!(p, Precise::Unknown(_)));
}
