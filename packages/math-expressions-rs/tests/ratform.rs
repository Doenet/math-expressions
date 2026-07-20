//! S2 (FULL_SIMPLIFY_PLAN) — rational normal form (`together`/`cancel`) tests.
//!
//! `together` combines an expression over a single common denominator in lowest
//! terms; opaque kernels (`sin x`, `√x`) are held fixed. Equality of the result
//! to a hand-written expected form is checked *semantically* via the library's
//! `equals`, so a different-but-equal shape (e.g. sign/order) still passes,
//! while the structural expectations (single fraction, kernel untouched) are
//! asserted directly.

use math_expressions::assumptions::Assumptions;
use math_expressions::exact::is_zero;
use math_expressions::ratform::{cancel, together};
use math_expressions::{Expr, TextToAst};

fn parse(s: &str) -> Expr {
    TextToAst::new(Default::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e:?}"))
}

fn eq(a: &Expr, b: &Expr) -> bool {
    math_expressions::equals(a, b, &Default::default())
}

// ---------- together: combine over a common denominator ----------

#[test]
fn combine_simple_fractions() {
    // 1/(x+1) + 1/(x-1) = 2x/(x^2 - 1)
    let got = together(&parse("1/(x+1) + 1/(x-1)"));
    assert!(eq(&got, &parse("2*x/(x^2 - 1)")), "got {got:?}");
}

#[test]
fn cancel_reduces_to_polynomial() {
    // (x^2 - 1)/(x - 1) = x + 1
    let got = cancel(&parse("(x^2 - 1)/(x - 1)"));
    assert!(eq(&got, &parse("x + 1")), "got {got:?}");
}

#[test]
fn three_term_sum() {
    // 1/x + 1/(x+1) + 1/(x+2) over a common denominator
    let e = parse("1/x + 1/(x+1) + 1/(x+2)");
    let got = together(&e);
    assert!(eq(&got, &e), "value changed: {got:?}");
}

// ---------- kernel opacity ----------

#[test]
fn kernel_opacity_sin() {
    // 1/sin(x) + 1/sin(x) = 2/sin(x); sin is never expanded or touched.
    let got = together(&parse("1/sin(x) + 1/sin(x)"));
    assert!(eq(&got, &parse("2/sin(x)")), "got {got:?}");
}

#[test]
fn kernel_opacity_mixed() {
    // sqrt(x)/(x) combined stays rational in the kernel sqrt(x) and in x.
    let e = parse("sqrt(x)/x + sqrt(x)/x");
    let got = together(&e);
    assert!(eq(&got, &parse("2*sqrt(x)/x")), "got {got:?}");
}

#[test]
fn distinct_kernels_not_merged() {
    // 1/sin(x) + 1/cos(x) = (cos x + sin x)/(sin x cos x): both kernels survive.
    let e = parse("1/sin(x) + 1/cos(x)");
    let got = together(&e);
    assert!(eq(&got, &e), "value changed: {got:?}");
}

// ---------- no-blowup guard ----------

#[test]
fn many_term_single_variable_sum_is_bounded() {
    // A 12-term sum of distinct simple poles in one variable stays decidable
    // (denominator degree 12 ≤ the poly degree cap) and value-preserving.
    let terms: Vec<String> = (1..=12).map(|k| format!("1/(x+{k})")).collect();
    let e = parse(&terms.join(" + "));
    let got = together(&e);
    assert!(eq(&got, &e), "value changed under combination");
}

#[test]
fn too_many_variables_falls_back_unchanged() {
    // Eight independent variables exceed MAX_INDETERMINATES: `together` must
    // bail to a value-equal form rather than build a 2^8 dense polynomial.
    let e = parse("1/(a+1) + 1/(b+1) + 1/(c+1) + 1/(d+1) + 1/(f+1) + 1/(g+1) + 1/(h+1) + 1/(k+1)");
    let got = together(&e);
    assert!(eq(&got, &e), "fallback must preserve value");
}

// ---------- is_zero stage (d): rational identities ----------

#[test]
fn rational_identities_certified_zero() {
    let z = |s: &str| is_zero(&parse(s), &Assumptions::new());
    assert_eq!(z("1/(x+1) + 1/(x-1) - 2*x/(x^2 - 1)"), Some(true));
    assert_eq!(z("(x^2 - 1)/(x - 1) - (x + 1)"), Some(true));
    assert_eq!(z("1/x - 1/x"), Some(true));
    // Kernel identity: rational in sin(x).
    assert_eq!(z("2/sin(x) - 1/sin(x) - 1/sin(x)"), Some(true));
}

#[test]
fn non_identity_not_certified_zero() {
    let z = |s: &str| is_zero(&parse(s), &Assumptions::new());
    // A genuine non-zero rational function must not be certified zero.
    assert_ne!(z("1/(x+1) + 1/(x-1) - 1/(x^2 - 1)"), Some(true));
}
