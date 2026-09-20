//! The display-rounding passes on `Float` values — DoenetML upstream request 08,
//! "display rounding loses precision at large magnitudes".
//!
//! `<number>` defaults to `displayDigits = 3, displayDecimals = 2`, so
//! `round_numbers_to_precision_plus_decimals(v, 3, 2)` is the path every number
//! a student sees goes through. It asked for 2 decimal places of `2e21` — a
//! no-op on a value that is already an integer — and returned
//! `1.9999999999999997e21`, because the implementation computed
//! `(v · 10^d).round() / 10^d` and neither `2e23` nor the quotient is
//! representable. Rounding now goes through the float's exact binary value.

use math_expressions::num::Number;
use math_expressions::{
    round_numbers_to_decimals, round_numbers_to_precision,
    round_numbers_to_precision_plus_decimals, Expr, TextOpts,
};

fn f(v: f64) -> Expr {
    Expr::Num(Number::from_f64(v))
}

fn show(e: &Expr) -> String {
    math_expressions::to_text(e, &TextOpts::default())
}

/// The reported case, and the neighbouring magnitudes on both sides of the
/// point where `v · 100` stops being representable (`2^53 / 100 ≈ 9e13`).
#[test]
fn rounding_a_large_float_does_not_perturb_it() {
    for v in [2e21, 2e14, 1.5e16, 9.87e30, 1e21, 6.02e23, -2e21] {
        assert_eq!(
            show(&round_numbers_to_precision_plus_decimals(&f(v), 3.0, 2.0)),
            show(&f(v)),
            "{v:e} is an integer; rounding it to 2 decimals must be identity"
        );
        assert_eq!(show(&round_numbers_to_decimals(&f(v), 2)), show(&f(v)));
    }
}

/// Asking for *more* digits used to be the workaround — it returned the exact
/// answer where 3 did not. Both must agree now, and with the precision-only
/// pass, which was never affected because its decimal count came out negative.
#[test]
fn every_way_of_asking_agrees() {
    let v = f(2e21);
    let expected = "2 * 10^21";
    assert_eq!(show(&round_numbers_to_precision(&v, 3)), expected);
    assert_eq!(
        show(&round_numbers_to_precision_plus_decimals(&v, 3.0, 2.0)),
        expected
    );
    assert_eq!(
        show(&round_numbers_to_precision_plus_decimals(&v, 15.0, 2.0)),
        expected
    );
    assert_eq!(show(&round_numbers_to_decimals(&v, 2)), expected);
}

/// Rounding that has something to do still does it, at the significant figure
/// and at the decimal place, with the larger of the two winning.
#[test]
fn rounding_that_should_change_the_value_still_does() {
    assert_eq!(show(&round_numbers_to_decimals(&f(2.345), 2)), "2.35");
    assert_eq!(show(&round_numbers_to_decimals(&f(-2.345), 2)), "-2.35");
    assert_eq!(
        show(&round_numbers_to_precision(&f(12345.6789), 3)),
        "12300"
    );
    // digits alone would give 12300; decimals raises it to the full value.
    assert_eq!(
        show(&round_numbers_to_precision_plus_decimals(
            &f(12345.6789),
            3.0,
            2.0
        )),
        "12345.68"
    );
    // Below 1, the significant-figure count is the one that bites: 2 decimals
    // of 0.00123456 would be 0, and the point of the pass is that it is not.
    assert_eq!(
        show(&round_numbers_to_precision_plus_decimals(
            &f(0.00123456),
            3.0,
            2.0
        )),
        "0.00123"
    );
}

/// Rounding reads the float's **shortest decimal spelling**, which is what a
/// reader sees and what legacy rounded: mathjs's
/// `format(v, {notation:"fixed", precision:n})` generates digits from that
/// spelling, so `2.675` → `2.68` and `1.005` → `1.01`. (`toFixed` would give
/// `2.67` and `1.00` by reading the stored `2.67499999999999982…`; this test
/// asserted those until it was checked against mathjs itself.) A tie in the
/// spelling rounds away from zero.
#[test]
fn ties_are_resolved_against_the_shortest_spelling() {
    assert_eq!(show(&round_numbers_to_decimals(&f(2.675), 2)), "2.68");
    assert_eq!(show(&round_numbers_to_decimals(&f(1.005), 2)), "1.01");
    // Exactly representable halves: away from zero, both signs.
    assert_eq!(show(&round_numbers_to_decimals(&f(0.125), 2)), "0.13");
    assert_eq!(show(&round_numbers_to_decimals(&f(-0.125), 2)), "-0.13");
    assert_eq!(show(&round_numbers_to_decimals(&f(2.5), 0)), "3");
    assert_eq!(show(&round_numbers_to_decimals(&f(-2.5), 0)), "-3");
}

/// A non-finite float has no decimal expansion to round; it comes back
/// untouched rather than as `NaN` or a trap.
#[test]
fn non_finite_floats_pass_through() {
    for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let rounded = round_numbers_to_precision_plus_decimals(&f(v), 3.0, 2.0);
        assert_eq!(show(&rounded), show(&f(v)), "{v} must be unchanged");
    }
}

/// Exact values are rounded exactly, as before — the float path is the only
/// one that changed.
#[test]
fn exact_values_are_unaffected() {
    use math_expressions::TextToAst;
    let p = |s: &str| TextToAst::new(Default::default()).convert(s).unwrap();
    assert_eq!(show(&round_numbers_to_decimals(&p("2.345"), 2)), "2.35");
    // A fraction stays a fraction whether or not it has been folded into a
    // single `Rat`. Before, `simplify` decided the display: an unevaluated
    // `1/3` is two whole integers and survived, a folded one became `0.33`.
    // Display rounding now leaves a fraction-spelled rational alone either way.
    let third = math_expressions::simplify(&p("1/3"));
    assert_eq!(show(&round_numbers_to_decimals(&third, 2)), "1/3");
    assert_eq!(show(&round_numbers_to_decimals(&p("1/3"), 2)), "1/3");
    // A *decimal*-spelled rational still rounds — that is what the spelling is
    // for, and it keeps `<round>0.5</round>` working.
    assert_eq!(show(&round_numbers_to_decimals(&p("0.5"), 0)), "1");
    // A large exact integer is not perturbed by rounding — the whole point of
    // request 08. It *displays* in scientific notation (past 1e21, the JS
    // threshold, which exact values honour the same way floats do), so the
    // check is that the spelling is exact rather than that it is positional:
    // `2 * 10^21` is the value, `1.9999999999999997e21` was the bug.
    assert_eq!(
        show(&round_numbers_to_precision_plus_decimals(
            &p("2000000000000000000000"),
            3.0,
            2.0
        )),
        "2 * 10^21"
    );
    let opts = TextOpts {
        avoid_scientific_notation: true,
        ..Default::default()
    };
    assert_eq!(
        math_expressions::to_text(
            &round_numbers_to_precision_plus_decimals(&p("2000000000000000000000"), 3.0, 2.0),
            &opts
        ),
        "2000000000000000000000"
    );
}

/// An *exact* tiny decimal takes the same notation threshold a float does.
///
/// Decimals parse to exact rationals, so a typed `5.252E-13` is a `Rat`, not a
/// `Float`, and used to render as `0.0000000000005252` — thirteen leading
/// zeros where legacy (which had only floats) showed `5.252 * 10^(-13)`. The
/// switch is not cosmetic: DoenetML's `avoidScientificNotation` attribute
/// exists to *turn the threshold off*, so if exact values never reached it the
/// attribute would have nothing to do.
#[test]
fn exact_tiny_decimals_use_scientific_notation() {
    use math_expressions::{to_latex, LatexOpts, TextToAst};
    let p = |s: &str| TextToAst::new(Default::default()).convert(s).unwrap();

    assert_eq!(show(&p("5.252E-13")), "5.252 * 10^(-13)");
    assert_eq!(show(&p("0.0000000000005252")), "5.252 * 10^(-13)");
    assert_eq!(show(&p("6E-21")), "6 * 10^(-21)");
    assert_eq!(show(&p("-3E-12")), "-3 * 10^(-12)");
    assert_eq!(
        to_latex(&p("5.252E-13"), &LatexOpts::default()),
        "5.252 \\cdot 10^{-13}"
    );

    // The threshold itself is JS's: positional from `0.000001` up.
    assert_eq!(show(&p("0.000001")), "0.000001");
    assert_eq!(show(&p("0.0000001")), "1 * 10^(-7)");

    // `avoidScientificNotation` puts every digit back.
    let opts = TextOpts {
        avoid_scientific_notation: true,
        ..Default::default()
    };
    assert_eq!(
        math_expressions::to_text(&p("5.252E-13"), &opts),
        "0.0000000000005252"
    );

    // The large side is symmetric — DoenetML's `avoidScientificNotation` test
    // pins `2000000000000000000000 x^2` as `2 \cdot 10^{21} x^{2}` by default
    // and positional under the attribute.
    assert_eq!(show(&p("2E21")), "2 * 10^21");
    assert_eq!(show(&p("1E30")), "1 * 10^30");
    assert_eq!(
        math_expressions::to_text(&p("2E21"), &opts),
        "2".to_string() + &"0".repeat(21)
    );
    // The threshold's upper edge: 1e21 is exponential, 1e20 is not.
    assert_eq!(show(&p("1E20")), "1".to_string() + &"0".repeat(20));
}
