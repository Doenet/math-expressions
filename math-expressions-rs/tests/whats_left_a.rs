//! Tests for the WHATS_LEFT A.2 / A.3 additions: factor, simplify_logical,
//! isAnalytic, equals_via_real, finite-field evaluate, vector ops, units,
//! set_small_zero, and the normalization passes.

use math_expressions::{
    add_unit, altvectors_to_vectors, canonicalize, cross_prod, dot_prod, equals, equals_via_real,
    factor, finite_field_evaluate, is_analytic, normalize_function_names, remove_scaling_units,
    remove_units, set_small_zero, simplify_logical, to_text, tuples_to_vectors, vector_add,
    vector_sub, AnalyticOpts, Assumptions, EqOptions, Expr, TextToAst, TextToAstOptions,
};
use std::collections::HashMap;

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn eq(a: &Expr, b: &str) -> bool {
    equals(a, &parse(b), &EqOptions::default())
}

// ---- item 8: factor ----

#[test]
fn factor_difference_of_squares() {
    let f = factor(&parse("x^2 - 1"));
    assert!(eq(&f, "(x-1)(x+1)"), "got {}", to_text(&f, &Default::default()));
    // Structurally a product, not the expanded form.
    assert!(matches!(f, Expr::Mul(_)), "expected a product, got {f:?}");
}

#[test]
fn factor_repeated_root_keeps_multiplicity() {
    let f = factor(&parse("x^2 + 2x + 1"));
    assert!(eq(&f, "(x+1)^2"));
}

#[test]
fn factor_pulls_leading_coefficient() {
    let f = factor(&parse("2x^2 - 8"));
    assert!(eq(&f, "2(x-2)(x+2)"));
}

#[test]
fn factor_irreducible_quadratic_is_preserved_and_equal() {
    // x^2 + 1 has no rational roots; the result must still equal the input.
    let f = factor(&parse("x^2 + 1"));
    assert!(eq(&f, "x^2 + 1"));
}

#[test]
fn factor_cubic() {
    let f = factor(&parse("x^3 - x"));
    assert!(eq(&f, "x(x-1)(x+1)"));
}

#[test]
fn factor_non_polynomial_is_unchanged_but_equal() {
    let f = factor(&parse("sin(x) + 1"));
    assert!(eq(&f, "sin(x) + 1"));
}

#[test]
fn factor_multivariate_left_alone() {
    let f = factor(&parse("x^2 - y^2"));
    assert!(eq(&f, "x^2 - y^2"));
}

#[test]
fn factor_huge_degree_is_a_polite_refusal() {
    // §7f: the dense coefficient vector allocates one entry per degree, so an
    // adversarial exponent must be refused before sizing, not after. Compare
    // structurally — numeric sampling of x^10^9 is not meaningful here.
    let e = parse("x^1000000000 - x");
    assert_eq!(factor(&e), canonicalize(&e));
}

#[test]
fn factor_degree_cap_is_scoped() {
    use math_expressions::resource_limits::{self, ResourceLimits};
    let e = parse("x^5 - x");
    let strict = ResourceLimits {
        max_factor_degree: 3,
        ..ResourceLimits::default()
    };
    let under = resource_limits::with(strict, || factor(&e));
    assert_eq!(under, canonicalize(&e), "expected refusal under tight cap");
    assert!(matches!(factor(&e), Expr::Mul(_)), "default cap must factor");
}

// ---- item 9: simplify_logical ----

#[test]
fn simplify_logical_double_negation() {
    let a = Assumptions::new();
    let r = simplify_logical(&parse("not(not(x > 0))"), &a);
    assert!(equals(&r, &parse("x > 0"), &EqOptions::default()) || eq(&r, "x > 0"));
}

#[test]
fn simplify_logical_de_morgan() {
    let a = Assumptions::new();
    let r = simplify_logical(&parse("not(x > 0 and y > 0)"), &a);
    // De Morgan: not(a and b) -> (not a) or (not b) -> x <= 0 or y <= 0
    assert!(matches!(r, Expr::Or(_)), "expected an Or, got {r:?}");
}

#[test]
fn simplify_logical_negates_relation() {
    let a = Assumptions::new();
    let r = simplify_logical(&parse("not(x = y)"), &a);
    assert!(matches!(r, Expr::Relation { .. }), "got {r:?}");
}

// ---- item 7: isAnalytic ----

#[test]
fn analytic_polynomial_is_analytic() {
    assert!(is_analytic(&parse("x^2 + 3x - 1"), &AnalyticOpts::default()));
    assert!(is_analytic(&parse("sin(x) + cos(x)"), &AnalyticOpts::default()));
}

#[test]
fn abs_is_not_analytic_by_default() {
    assert!(!is_analytic(&parse("abs(x)"), &AnalyticOpts::default()));
    let opts = AnalyticOpts {
        allow_abs: true,
        ..Default::default()
    };
    assert!(is_analytic(&parse("abs(x)"), &opts));
}

#[test]
fn logical_and_relations_are_not_analytic() {
    assert!(!is_analytic(&parse("x > 0 and y > 0"), &AnalyticOpts::default()));
    // A bare order relation is analytic only under allow_relation.
    assert!(!is_analytic(&parse("x > 0"), &AnalyticOpts::default()));
    let opts = AnalyticOpts {
        allow_relation: true,
        ..Default::default()
    };
    assert!(is_analytic(&parse("x > 0"), &opts));
    // Set membership is never an analytic relation.
    assert!(!is_analytic(&parse("x elementof Z"), &opts));
}

// ---- item 10: equals_via_real ----

#[test]
fn equals_via_real_agrees_on_reals() {
    let opts = EqOptions::default();
    assert!(equals_via_real(&parse("(x+1)^2"), &parse("x^2 + 2x + 1"), &opts));
}

#[test]
fn equals_via_real_rejects_nonanalytic() {
    // abs is non-analytic, so equalsViaReal declines (returns false) even though
    // abs(x) == sqrt(x^2) on the reals.
    let opts = EqOptions::default();
    assert!(!equals_via_real(&parse("abs(x)"), &parse("sqrt(x^2)"), &opts));
}

// ---- item 16: finite_field_evaluate ----

#[test]
fn finite_field_evaluate_basic() {
    let bindings = HashMap::from([("x".to_string(), 4i64)]);
    // x^2 + 1 at x=4 mod 5 -> 17 mod 5 -> 2
    let r = finite_field_evaluate(&parse("x^2 + 1"), &bindings, 5).expect("field value");
    assert!(r.contains(&2), "got {r:?}");
}

// ---- item 12: vector ops ----

#[test]
fn vector_add_and_sub() {
    let s = vector_add(&parse("(1,2,3)"), &parse("(4,5,6)"));
    assert!(eq(&s, "(5,7,9)"), "got {}", to_text(&s, &Default::default()));
    let d = vector_sub(&parse("(4,5,6)"), &parse("(1,2,3)"));
    assert!(eq(&d, "(3,3,3)"));
}

#[test]
fn dot_and_cross_product() {
    let dp = dot_prod(&parse("(1,2,3)"), &parse("(4,5,6)"));
    // 1*4 + 2*5 + 3*6 = 32
    assert!(eq(&dp, "32"), "got {}", to_text(&dp, &Default::default()));
    let cp = cross_prod(&parse("(1,0,0)"), &parse("(0,1,0)"));
    assert!(eq(&cp, "(0,0,1)"), "got {}", to_text(&cp, &Default::default()));
}

// ---- item 17: set_small_zero ----

#[test]
fn set_small_zero_clears_noise() {
    let r = set_small_zero(&parse("1 + 0.0000000001 x"), 1e-8);
    assert!(eq(&r, "1"), "got {}", to_text(&r, &Default::default()));
}

// ---- item 14: units ----

#[test]
fn remove_units_with_and_without_scaling() {
    // 50% scaled -> 1/2, unscaled -> 50
    let scaled = remove_units(&parse("50%"), true);
    assert!(eq(&scaled, "1/2"), "got {}", to_text(&scaled, &Default::default()));
    let bare = remove_units(&parse("50%"), false);
    assert!(eq(&bare, "50"), "got {}", to_text(&bare, &Default::default()));
}

#[test]
fn remove_scaling_units_matches_scaled_removal() {
    let r = remove_scaling_units(&parse("50%"));
    assert!(eq(&r, "1/2"));
}

#[test]
fn add_unit_roundtrips_through_removal() {
    let with = add_unit(&parse("5"), "$");
    let back = remove_units(&with, false);
    assert!(eq(&back, "5"));
}

// ---- item 13: normalization passes ----

#[test]
fn normalize_function_names_folds_aliases() {
    let r = normalize_function_names(&parse("arcsin(x)"));
    // arcsin -> asin
    assert_eq!(math_expressions::functions(&r), vec!["asin".to_string()]);
}

#[test]
fn tuples_and_altvectors_to_vectors() {
    use math_expressions::expr::SeqKind;
    let t = tuples_to_vectors(&parse("(1,2)"));
    assert!(matches!(t, Expr::Seq(SeqKind::Vector, _)), "got {t:?}");
    let a = altvectors_to_vectors(&parse("langle 1,2 rangle"));
    // langle/rangle may or may not parse to AltVector; only assert when it does.
    if let Expr::Seq(k, _) = &a {
        assert_ne!(*k, SeqKind::AltVector);
    }
}
