//! S3 (FULL_SIMPLIFY_PLAN) — trig/exp/log special-value folding + parity.
//!
//! `fold_special_values` is an unconditionally-sound rewrite. Value equalities
//! are checked semantically (`equals`); structural expectations (parity sign
//! pulled out, gated rules NOT fired) are asserted directly.

use math_expressions::normalize::{canonicalize, fold_special_values};
use math_expressions::{equals, Expr, TextToAst};

fn parse(s: &str) -> Expr {
    TextToAst::new(Default::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e:?}"))
}

fn fold(s: &str) -> Expr {
    fold_special_values(&parse(s))
}

fn eq(a: &Expr, b: &str) -> bool {
    equals(a, &parse(b), &Default::default())
}

// ---------- lattice values, all six functions ----------

#[test]
fn lattice_values() {
    assert!(eq(&fold("sin(pi/6)"), "1/2"));
    assert!(eq(&fold("cos(pi/3)"), "1/2"));
    assert!(eq(&fold("tan(pi/4)"), "1"));
    assert!(eq(&fold("cot(pi/4)"), "1"));
    assert!(eq(&fold("sec(pi/3)"), "2"));
    assert!(eq(&fold("csc(pi/6)"), "2"));
    assert!(eq(&fold("sin(pi/4)"), "sqrt(2)/2"));
    assert!(eq(&fold("cos(pi/6)"), "sqrt(3)/2"));
    assert!(eq(&fold("tan(pi/6)"), "sqrt(3)/3"));
    assert!(eq(&fold("sec(pi/4)"), "sqrt(2)"));
    assert!(eq(&fold("cot(pi/6)"), "sqrt(3)"));
}

#[test]
fn sin_two_pi_is_zero() {
    // The case that motivated the chunk.
    assert!(eq(&fold("sin(2*pi)"), "0"));
    assert!(eq(&fold("cos(pi/2)"), "0"));
    assert!(eq(&fold("tan(pi)"), "0"));
}

#[test]
fn periodicity_reduction_of_constants() {
    assert!(eq(&fold("sin(101*pi/6)"), "1/2"));
    assert!(eq(&fold("cos(7*pi/3)"), "1/2"));
    assert!(eq(&fold("sin(13*pi/6)"), "1/2"));
}

// ---------- parity ----------

#[test]
fn parity() {
    assert!(eq(&fold("sin(-x)"), "-sin(x)"));
    assert!(eq(&fold("cos(-x)"), "cos(x)"));
    assert!(eq(&fold("tan(-x)"), "-tan(x)"));
    assert!(eq(&fold("csc(-x)"), "-csc(x)"));
    assert!(eq(&fold("sec(-x)"), "sec(x)"));
    assert!(eq(&fold("cot(-x)"), "-cot(x)"));
}

// ---------- π-shift (integer multiples) ----------

#[test]
fn pi_shift() {
    assert!(eq(&fold("sin(x + 2*pi)"), "sin(x)"));
    assert!(eq(&fold("sin(x + pi)"), "-sin(x)"));
    assert!(eq(&fold("cos(x + pi)"), "-cos(x)"));
    assert!(eq(&fold("tan(x + pi)"), "tan(x)"));
    assert!(eq(&fold("cos(x + 3*pi)"), "-cos(x)"));
}

// ---------- exp / log inverses ----------

#[test]
fn exp_log_inverses() {
    assert!(eq(&fold("exp(log(x))"), "x"));
    assert!(eq(&fold("e^(log(x))"), "x"));
    assert!(eq(&fold("log(exp(5))"), "5"));
    assert!(eq(&fold("ln(e)"), "1"));
    assert!(eq(&fold("ln(1)"), "0"));
    assert!(eq(&fold("exp(0)"), "1"));
}

#[test]
fn log_of_exp_of_variable_is_gated() {
    // ln(exp(x)) is only sound for real x; without S5 we do NOT fold it, so
    // the result still contains the log/exp structure (unchanged, canonical).
    let e = parse("log(exp(x))");
    assert_eq!(fold_special_values(&e), canonicalize(&e));
}

// ---------- idempotence ----------

#[test]
fn idempotent() {
    for s in [
        "sin(pi/6) + cos(-x) + tan(x + pi)",
        "sin(101*pi/6)",
        "exp(log(x)) + ln(1)",
        "sec(pi/4) - csc(pi/6)",
        "cos(pi/5) + sec(pi/5)",
        "asin(-x) + acos((1+sqrt(5))/4)",
        "cos(asin(x)) + tan(atan(y))",
        "asin(x) + acos(x) + asin(y)",
    ] {
        let once = fold(s);
        let twice = fold_special_values(&once);
        assert_eq!(once, twice, "not idempotent on {s:?}");
    }
}

// ---------- the pentagonal (π/10) lattice ----------

/// The second lattice. `cos 36° = (1+√5)/4` and its relatives are in the
/// surd ring, so they fold alongside the twelfths.
#[test]
fn pentagonal_lattice_values() {
    assert!(eq(&fold("cos(pi/5)"), "(1+sqrt(5))/4"));
    assert!(eq(&fold("sin(pi/10)"), "(sqrt(5)-1)/4"));
    assert!(eq(&fold("sin(3pi/10)"), "(1+sqrt(5))/4"));
    assert!(eq(&fold("cos(2pi/5)"), "(sqrt(5)-1)/4"));
    assert!(eq(&fold("cos(4pi/5)"), "-(1+sqrt(5))/4"));
    assert!(eq(&fold("sin(-pi/10)"), "-(sqrt(5)-1)/4"));
    // Periodicity still applies on this lattice.
    assert!(eq(&fold("cos(11pi/5)"), "(1+sqrt(5))/4"));
    // Reciprocals: sec(π/5) = 4/(1+√5) = √5 − 1, which needs the conjugate
    // rationalization in `Exact::inverse` — a two-term denominator.
    assert!(eq(&fold("sec(pi/5)"), "sqrt(5)-1"));
    assert!(eq(&fold("csc(pi/10)"), "1+sqrt(5)"));
}

/// The half of the pentagonal lattice that is *not* in the ring: `sin 36°` is
/// `√(10−2√5)/4`, a radical nested one level deeper than the `√r` basis holds.
/// Declining is the honest answer — an approximation is not one.
///
/// This is also why tangent never fires here: it needs the sine and the cosine
/// of the same angle, and on this lattice exactly one of the two is nested.
#[test]
fn pentagonal_lattice_declines_where_the_radical_nests() {
    for s in [
        "sin(pi/5)",
        "cos(pi/10)",
        "tan(pi/5)",
        "tan(pi/10)",
        "cot(pi/5)",
    ] {
        let e = parse(s);
        assert_eq!(
            fold_special_values(&e),
            canonicalize(&e),
            "{s:?} should not fold"
        );
    }
}

// ---------- inverse trig ----------

/// Inverting the forward tables covers both lattices at once, so the
/// pentagonal values come back as angles too.
#[test]
fn inverse_trig_on_both_lattices() {
    assert!(eq(&fold("asin(1)"), "pi/2"));
    assert!(eq(&fold("acos(sqrt(3)/2)"), "pi/6"));
    assert!(eq(&fold("atan(1)"), "pi/4"));
    assert!(eq(&fold("acos((1+sqrt(5))/4)"), "pi/5"));
    assert!(eq(&fold("asin((sqrt(5)-1)/4)"), "pi/10"));
    assert!(eq(&fold("acos((sqrt(5)-1)/4)"), "2pi/5"));
    // Reciprocal branches, again through the general inverse.
    assert!(eq(&fold("asec(sqrt(5)-1)"), "pi/5"));
    assert!(eq(&fold("acot(2+sqrt(3))"), "pi/12"));
    assert!(eq(&fold("asec(sqrt(6)-sqrt(2))"), "pi/12"));
}

/// asin, atan, acsc and acot are odd; acos and asec reflect through π/2. The
/// reciprocal three are odd *because* this library defines them at `1/z` —
/// under the `(0, π)` convention for acot the last line would be wrong.
#[test]
fn inverse_trig_parity() {
    assert!(eq(&fold("asin(-x)"), "-asin(x)"));
    assert!(eq(&fold("atan(-x)"), "-atan(x)"));
    assert!(eq(&fold("acsc(-x)"), "-acsc(x)"));
    assert!(eq(&fold("acot(-x)"), "-acot(x)"));
    assert!(eq(&fold("acos(-x)"), "pi - acos(x)"));
    assert!(eq(&fold("asec(-x)"), "pi - asec(x)"));
    // Off the lattice the sign still comes out, but no angle is invented.
    assert!(eq(&fold("asin(-3)"), "-asin(3)"));
}

/// `f(f⁻¹(u)) = u` on every principal branch, with no domain condition —
/// each inverse is *defined* as a right inverse.
#[test]
fn trig_of_its_own_inverse_is_the_argument() {
    for (f, g) in [
        ("sin", "asin"),
        ("cos", "acos"),
        ("tan", "atan"),
        ("sec", "asec"),
        ("csc", "acsc"),
        ("cot", "acot"),
    ] {
        assert_eq!(fold(&format!("{f}({g}(x))")), parse("x"), "{f}∘{g}");
    }
}

/// The mixed pairs close in radicals: `cos(asin u) = √(1−u²)` because asin's
/// range has cosine nonnegative, `sec(atan u) = √(1+u²)` likewise.
#[test]
fn trig_of_a_different_inverse_closes_in_radicals() {
    assert!(eq(&fold("cos(asin(x))"), "sqrt(1-x^2)"));
    assert!(eq(&fold("sin(acos(x))"), "sqrt(1-x^2)"));
    assert!(eq(&fold("tan(asin(x))"), "x/sqrt(1-x^2)"));
    assert!(eq(&fold("sec(asin(x))"), "1/sqrt(1-x^2)"));
    assert!(eq(&fold("csc(asin(x))"), "1/x"));
    assert!(eq(&fold("sin(atan(x))"), "x/sqrt(1+x^2)"));
    assert!(eq(&fold("cos(atan(x))"), "1/sqrt(1+x^2)"));
    assert!(eq(&fold("sec(atan(x))"), "sqrt(1+x^2)"));
    assert!(eq(&fold("cot(atan(x))"), "1/x"));
    assert!(eq(&fold("tan(acos(x))"), "sqrt(1-x^2)/x"));
    assert!(eq(&fold("sin(acsc(x))"), "1/x"));
    assert!(eq(&fold("cos(asec(x))"), "1/x"));
    assert!(eq(&fold("tan(acot(x))"), "1/x"));
}

/// The composition in the other direction is true only on the principal
/// branch — `asin(sin 3) = π − 3` — so it is not folded. It needs a range
/// assumption on `x`, not an unconditional rewrite.
#[test]
fn inverse_of_a_trig_function_is_not_folded() {
    for s in [
        "asin(sin(x))",
        "acos(cos(x))",
        "atan(tan(x))",
        "asec(sec(x))",
    ] {
        let e = parse(s);
        assert_eq!(
            fold_special_values(&e),
            canonicalize(&e),
            "{s:?} should not fold"
        );
    }
}

/// `asin u + acos u = π/2` for every `u`, and the same at `1/u` for the
/// reciprocal pair.
#[test]
fn complementary_inverse_pairs_sum_to_a_right_angle() {
    assert!(eq(&fold("asin(x)+acos(x)"), "pi/2"));
    assert!(eq(&fold("acsc(x)+asec(x)"), "pi/2"));
    assert!(eq(&fold("3asin(x)+3acos(x)"), "3pi/2"));
    assert!(eq(&fold("y*asin(x)+y*acos(x)"), "pi*y/2"));
    assert!(eq(&fold("1+asin(x)+acos(x)+z"), "1+z+pi/2"));
    assert!(eq(&fold("asin(x^2+1)+acos(x^2+1)"), "pi/2"));
}

/// `atan u + acot u` is *not* in that set. With `acot u` defined as
/// `atan(1/u)` the sum is `π/2` for positive `u` and `−π/2` for negative `u`,
/// so there is no unconditional value to fold to. Nor do mismatched
/// coefficients or arguments pair up.
#[test]
fn sums_without_an_unconditional_value_are_left_alone() {
    for s in [
        "atan(x)+acot(x)",
        "asin(x)+acos(y)",
        "2asin(x)+acos(x)",
        "asin(x)-acos(x)",
    ] {
        let e = parse(s);
        assert_eq!(
            fold_special_values(&e),
            canonicalize(&e),
            "{s:?} should not fold"
        );
    }
    // A product is not a sum.
    assert!(eq(&fold("asin(x)*acos(x)"), "asin(x)*acos(x)"));
}

#[test]
fn cot_at_its_zeros_folds_to_zero() {
    // cot = cos/sin, so cot folds to 0 at the odd multiples of π/2 (tan's poles,
    // where the old 1/tan route gave None). Non-zero lattice values unchanged.
    assert!(eq(&fold("cot(pi/2)"), "0"));
    assert!(eq(&fold("cot(3*pi/2)"), "0"));
    assert!(eq(&fold("cot(-pi/2)"), "0"));
    assert!(eq(&fold("cot(pi/4)"), "1"));
    assert!(eq(&fold("cot(pi/6)"), "sqrt(3)"));
    assert!(eq(&fold("cot(pi/3)"), "sqrt(3)/3"));
}
