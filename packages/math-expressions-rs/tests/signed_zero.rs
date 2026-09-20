//! Signed-zero (`Number::NegZero`) behaviour: a division-by-zero pole reports a
//! *signed* infinity, with the sign flowing through products and negation, while
//! a bare `−0` still reads as plain `0` everywhere else.

use math_expressions::{evaluate_numbers_preserve_order, expr, simplify, TextToAst};

/// Parse `s`, canonicalise/simplify, and spell the result as its JS tree.
fn run(s: &str) -> String {
    let e = TextToAst::new(Default::default()).convert(s).unwrap();
    expr::serde::to_js(&simplify(&e)).to_string()
}

/// The same, folded along the **`skip_ordering`** path instead — the separate
/// pass behind DoenetML's `simplify="numberspreserveorder"`. It shares no code
/// with `simplify`'s canonical layer (it keeps trees unflattened and un-peeled
/// to preserve operand order), so it decides the indeterminate forms with its
/// own predicate and has to be asserted on separately.
fn preserve_order(s: &str) -> String {
    let e = TextToAst::new(Default::default()).convert(s).unwrap();
    expr::serde::to_js(&evaluate_numbers_preserve_order(&e)).to_string()
}

#[test]
fn pole_sign_from_literal_negative_zero() {
    // 1/(−0) is −∞; 1/0 stays +∞.
    assert_eq!(run("1/0"), r#"{"$":"Inf"}"#);
    assert_eq!(run("1/-0"), r#"{"$":"-Inf"}"#);
    // The numerator's own sign composes with the pole's.
    assert_eq!(run("6/-0"), r#"{"$":"-Inf"}"#);
    assert_eq!(run("-6/-0"), r#"{"$":"Inf"}"#);
}

#[test]
fn sign_flows_through_a_product_into_the_zero() {
    // The zero acquires its sign from a negative factor before the reciprocal.
    assert_eq!(run("1/((-1)*0)"), r#"{"$":"-Inf"}"#);
    assert_eq!(run("1/((-1)(0))"), r#"{"$":"-Inf"}"#);
    // Two negatives cancel: the zero is +0 again.
    assert_eq!(run("1/((-1)*(-1)*0)"), r#"{"$":"Inf"}"#);
    assert_eq!(run("1/(2*(-3)*0)"), r#"{"$":"-Inf"}"#);
}

#[test]
fn bare_negative_zero_reads_as_plain_zero() {
    // `−0` is value-equal to `0`: it prints as `0` and annihilates as usual.
    assert_eq!(run("-0"), "0");
    assert_eq!(run("(-1)*0"), "0");
    assert_eq!(run("(-1)*0+x"), r#""x""#);
    // `0/0` is still the indeterminate NaN, sign or no sign.
    assert_eq!(run("0/0"), r#"{"$":"NaN"}"#);
}

/// The indeterminate forms the `∞`/`NaN` cluster in `normalize::simplify`
/// documents. Named there as the pin for that comment, so the two stay honest
/// about which behaviour is current.
///
/// The load-bearing half is the `skip_ordering` path: `simplify` decides these
/// in `constructors::mul`, *after* `peel_nonzero_scaling` has flattened the
/// product and split every factor into a `(base, exponent)` pair, so it only
/// ever sees a bare leaf. `evaluate_numbers_preserve_order` cannot do that
/// without losing the order it exists to preserve, so it walks the wrappers
/// itself (`ops::preserve_order::is_non_finite`) — an entirely separate
/// decision, which the `simplify` assertions below do not touch. When it got
/// this wrong the answer was a wrong *number* (`0`) rather than an error, on
/// the path DoenetML's equality checking, `MathOperators` and `Parabola` run.
#[test]
fn indeterminate_forms_do_not_annihilate() {
    // The four spellings that reach the pass with the infinity still wrapped:
    // under an exponent, under a fraction bar, inside a sum, and as a pole.
    assert_eq!(preserve_order("0*infinity^2"), r#"{"$":"NaN"}"#);
    assert_eq!(preserve_order("0*(infinity/2)"), r#"{"$":"NaN"}"#);
    assert_eq!(preserve_order("0*(infinity+1)"), r#"{"$":"NaN"}"#);
    assert_eq!(preserve_order("0*(0^(-1))"), r#"{"$":"NaN"}"#);
    // And the bare ones, plus `0^0`, which has no infinity in it at all.
    assert_eq!(preserve_order("0*infinity"), r#"{"$":"NaN"}"#);
    assert_eq!(preserve_order("0*(-infinity)"), r#"{"$":"NaN"}"#);
    assert_eq!(preserve_order("0*(1/0)"), r#"{"$":"NaN"}"#);
    assert_eq!(preserve_order("x*0*infinity"), r#"{"$":"NaN"}"#);
    assert_eq!(preserve_order("0^0"), r#"{"$":"NaN"}"#);
    // Nothing provably non-finite: the zero still annihilates. `1/x` and a
    // bare symbol are *unknown*, not infinite, and `1/∞` is a plain zero.
    assert_eq!(preserve_order("0*x"), "0");
    assert_eq!(preserve_order("0*(x+1)"), "0");
    assert_eq!(preserve_order("0*(1/x)"), "0");
    assert_eq!(preserve_order("0*(1/infinity)"), "0");

    // The canonical layer agrees on every one of them, which is the property
    // that matters: one expression must not get two answers depending on which
    // pass DoenetML ran.
    assert_eq!(run("0*infinity^2"), r#"{"$":"NaN"}"#);
    assert_eq!(run("0*(infinity/2)"), r#"{"$":"NaN"}"#);
    assert_eq!(run("0*(infinity+1)"), r#"{"$":"NaN"}"#);
    assert_eq!(run("0*(0^(-1))"), r#"{"$":"NaN"}"#);
    assert_eq!(run("0*infinity"), r#"{"$":"NaN"}"#);
    assert_eq!(run("0*(-infinity)"), r#"{"$":"NaN"}"#);
    assert_eq!(run("0*(1/0)"), r#"{"$":"NaN"}"#);
    assert_eq!(run("x*0*infinity"), r#"{"$":"NaN"}"#);
    // `0^0` under the default strict `pow` policy; JS answers `1` here, and so
    // does this engine with `constant_policy::pow_strict` cleared.
    assert_eq!(run("0^0"), r#"{"$":"NaN"}"#);
    // An ordinary zero product is still zero: nothing here is provably
    // non-finite.
    assert_eq!(run("0*x"), "0");
}
