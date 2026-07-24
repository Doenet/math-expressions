//! S4 (FULL_SIMPLIFY_PLAN) — full factorization over ℚ + `factor_terms`.
//!
//! `factor` now splits the no-rational-root remainder into irreducibles
//! (Kronecker), so `x⁶−1` factors completely. We assert this *structurally*
//! (the degrees of the irreducible factors) since a value-only check is
//! trivially satisfied by the `equals` gate inside `factor`.

use math_expressions::num::Number;
use math_expressions::{factor, factor_terms, Expr, TextToAst};

fn parse(s: &str) -> Expr {
    TextToAst::new(Default::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e:?}"))
}

fn eq(a: &Expr, b: &str) -> bool {
    math_expressions::equals(a, &parse(b), &Default::default())
}

/// Degree of `e` in `var` (0 for constants).
fn deg(e: &Expr, var: &str) -> i64 {
    match e {
        Expr::Sym(s) if s.name() == var => 1,
        Expr::Num(_) | Expr::Sym(_) => 0,
        Expr::Pow(b, x) => match &**x {
            Expr::Num(Number::Int(k)) => deg(b, var) * k,
            _ => 0,
        },
        Expr::Mul(fs) => fs.iter().map(|f| deg(f, var)).sum(),
        Expr::Add(ts) => ts.iter().map(|t| deg(t, var)).max().unwrap_or(0),
        Expr::Neg(a) => deg(a, var),
        _ => 0,
    }
}

/// Sorted degrees of the top-level irreducible factors (constants dropped).
fn factor_degrees(e: &Expr, var: &str) -> Vec<i64> {
    let factors: Vec<Expr> = match e {
        Expr::Mul(fs) => fs.clone(),
        other => vec![other.clone()],
    };
    let mut ds: Vec<i64> = factors
        .iter()
        .map(|f| deg(f, var))
        .filter(|d| *d > 0)
        .collect();
    ds.sort_unstable();
    ds
}

// ---------- full irreducible splitting ----------

#[test]
fn x6_minus_1_fully_splits() {
    let f = factor(&parse("x^6 - 1"));
    // (x−1)(x+1)(x²+x+1)(x²−x+1)
    assert_eq!(factor_degrees(&f, "x"), vec![1, 1, 2, 2]);
    assert!(eq(&f, "x^6 - 1"), "value must be preserved");
}

#[test]
fn x4_plus_x2_plus_1_splits_into_two_quadratics() {
    let f = factor(&parse("x^4 + x^2 + 1"));
    assert_eq!(factor_degrees(&f, "x"), vec![2, 2]);
    assert!(eq(&f, "(x^2 + x + 1)*(x^2 - x + 1)"));
}

#[test]
fn x4_minus_1_splits() {
    let f = factor(&parse("x^4 - 1"));
    // (x−1)(x+1)(x²+1)
    assert_eq!(factor_degrees(&f, "x"), vec![1, 1, 2]);
}

// ---------- irreducibles are left intact ----------

#[test]
fn irreducibles_stay_put() {
    // x²+1 and x⁴+1 have no factorization over ℚ.
    assert_eq!(factor_degrees(&factor(&parse("x^2 + 1")), "x"), vec![2]);
    assert_eq!(factor_degrees(&factor(&parse("x^4 + 1")), "x"), vec![4]);
    // x⁸−1: the x⁴+1 piece must remain irreducible.
    assert_eq!(factor_degrees(&factor(&parse("x^8 - 1")), "x"), vec![1, 1, 2, 4]);
}

// ---------- factor_terms ----------

/// The numeric factor pulled to the front of a `Mul`, if any.
fn front_numeric(e: &Expr) -> Option<i64> {
    if let Expr::Mul(fs) = e {
        if let Some(Expr::Num(Number::Int(k))) = fs.first() {
            return Some(*k);
        }
    }
    None
}

#[test]
fn factor_terms_pulls_content_and_monomial() {
    let f = factor_terms(&parse("6*x^2 + 9*x"));
    assert_eq!(front_numeric(&f), Some(3), "should pull numeric content 3");
    assert!(eq(&f, "6*x^2 + 9*x"));
}

#[test]
fn factor_terms_multivariate() {
    let f = factor_terms(&parse("2*x*y + 4*x"));
    assert_eq!(front_numeric(&f), Some(2));
    assert!(eq(&f, "2*x*y + 4*x"));
}

#[test]
fn factor_terms_kernel_aware() {
    // sin(x) is a common opaque factor.
    let f = factor_terms(&parse("sin(x)*a + sin(x)*b"));
    assert!(eq(&f, "sin(x)*(a + b)"));
    assert!(matches!(&f, Expr::Mul(_)), "should factor, got {f:?}");
}

#[test]
fn factor_terms_nothing_common_unchanged() {
    // x + y has no common factor: returned unchanged (not a product).
    let f = factor_terms(&parse("x + y"));
    assert!(!matches!(&f, Expr::Mul(_)), "must not invent a factor: {f:?}");
}
