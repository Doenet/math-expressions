//! `erf` evaluates numerically, and the same way on every entry point.
//!
//! The symbol parsed and printed but had no `eval1`, so every engine-side
//! numeric path answered "no value": `evaluate_to_constant("erf(0.5)")` was
//! `None` (`NaN` across the wasm boundary) and `evaluate_many` sampled `NaN` at
//! every point — while `Expression#f()`, which compiles through math.js, gave
//! the right answer throughout. In DoenetML that meant `<function>erf(x)`
//! plotted a correct curve whose `<number>$$f(0.5)</number>` read `NaN` and
//! whose extrema search found nothing, and it was a regression: the legacy
//! JavaScript library evaluated `erf` from every one of those paths.
//!
//! The implementation is a port of the same W. J. Cody rational-Chebyshev
//! approximation math.js uses, so the two paths agree to the last bit rather
//! than merely to a tolerance. The reference values below are the ones math.js
//! documents plus high-precision values for the other two intervals.

use math_expressions::{
    evaluate_fast_f64, evaluate_many, evaluate_to_constant, Expr, TextToAst, TextToAstOptions,
};
use std::collections::HashMap;

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn constant(s: &str) -> f64 {
    let v = evaluate_to_constant(&parse(s)).unwrap_or_else(|| panic!("{s} has no constant value"));
    assert!(v.im.abs() < 1e-15, "{s} came back complex: {v:?}");
    v.re
}

/// Cody's approximation is exercised over its three intervals: the polynomial
/// branch below 0.46875, the first `erfc` branch up to 4, and the tail beyond.
#[test]
fn erf_matches_reference_values() {
    // The three math.js documents, so a divergence between the engine and the
    // `f()` path would show up here first.
    assert!((constant("erf(0.2)") - 0.22270258921047847).abs() < 1e-15);
    assert!((constant("erf(-0.5)") + 0.5204998778130465).abs() < 1e-15);
    assert!((constant("erf(4)") - 0.9999999845827421).abs() < 1e-15);

    // One value per interval, against high-precision references.
    assert!((constant("erf(1)") - 0.8427007929497149).abs() < 1e-14);
    assert!((constant("erf(2)") - 0.9953222650189527).abs() < 1e-14);
    assert!((constant("erf(4.5)") - 0.9999999998033839).abs() < 1e-14);

    // Odd, and exactly saturated far out.
    assert_eq!(constant("erf(0)"), 0.0);
    assert_eq!(constant("erf(30)"), 1.0);
    assert_eq!(constant("erf(-30)"), -1.0);
}

/// The three numeric entry points DoenetML uses — `evaluate_to_constant` for
/// `<number>$$f(x)</number>`, `evaluate_fast_f64` for per-point evaluation, and
/// `evaluate_many` for the extrema sampler — must not disagree.
#[test]
fn every_numeric_entry_point_agrees() {
    let xs = [-3.0, -0.5, 0.0, 0.25, 1.5, 5.0];
    let expr = parse("erf(x)");
    let batch = evaluate_many(&expr, "x", &xs);

    for (i, &x) in xs.iter().enumerate() {
        let point = evaluate_fast_f64(&expr, &HashMap::from([("x".to_string(), x)]))
            .unwrap_or_else(|| panic!("erf({x}) has no value"));
        let folded = constant(&format!("erf({x})"));
        assert_eq!(point.re, batch[i], "erf({x}): per-point vs batch");
        assert_eq!(point.re, folded, "erf({x}): per-point vs constant folding");
    }
}
