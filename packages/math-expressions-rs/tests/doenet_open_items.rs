//! Regressions for the three DoenetML open items filed against the printers
//! and display-rounding (items 8, 9, 10 in the upstream comment thread):
//!
//! - **8. Display rounding must not turn an exact rational into a decimal when
//!   rounding changes nothing.** `round_numbers_to_precision(5/2, 3)` is `5/2`
//!   (legacy kept `\frac{5}{2}`), because `5/2 = 2.5` exactly and 3 significant
//!   figures leave it untouched — where `1/3 → 0.333` still decimalizes because
//!   rounding *does* change the value. `displayDigits` defaults to 3, so every
//!   rational a student sees goes through this path.
//!
//! - **10a. A negative leading coefficient prints as a subtraction, not a
//!   parenthesised negative.** `["+","a",["*",-3,"b"]]` is `a - 3 b`, not
//!   `a + (-3) b`. Same family: a fraction with a negative numerator pulls the
//!   sign out front (`["/",-2,3]` → `-2/3`), matching legacy.
//!
//! - **10b. The integral head keeps its `∫` glyph and drops the spurious
//!   parentheses around the integrand.** `\int_a^b f(x) dx` renders as
//!   `∫_a^b f(x) dx`, not `int_a^b(f(x) dx)`.

use math_expressions::num::Number;
use math_expressions::{
    round_numbers_to_decimals, round_numbers_to_precision,
    round_numbers_to_precision_plus_decimals, to_latex, to_text, Expr, LatexOpts, LatexToAst,
    TextOpts,
};

fn text(e: &Expr) -> String {
    to_text(e, &TextOpts::default())
}
fn latex(e: &Expr) -> String {
    to_latex(e, &LatexOpts::default())
}
fn ast(s: &str) -> Expr {
    let v: serde_json::Value = serde_json::from_str(s).unwrap();
    math_expressions::expr::serde::try_from_js(&v).unwrap()
}
fn pl(s: &str) -> Expr {
    LatexToAst::new(Default::default()).convert(s).unwrap()
}

// ---- item 8: display rounding preserves exact rationals -------------------

/// `5/2` is `2.5` exactly, so rounding to any number of places is a no-op on
/// the value — and a no-op must not change the *spelling* from a fraction to a
/// decimal. Every display-rounding entry point agrees.
#[test]
fn rounding_that_changes_nothing_keeps_the_fraction() {
    let five_halves = Expr::Num(Number::rat(5, 2));
    for got in [
        round_numbers_to_precision(&five_halves, 3),
        round_numbers_to_precision_plus_decimals(&five_halves, 3.0, 2.0),
        round_numbers_to_decimals(&five_halves, 3),
    ] {
        assert_eq!(text(&got), "5/2");
        assert_eq!(latex(&got), "\\frac{5}{2}");
    }
}

/// A student-typed decimal keeps its decimal spelling through a no-op round —
/// the rule is "preserve whatever spelling it had", not "always prefer a
/// fraction". (`0.5` is `Rat(1,2)` spelled `Decimal`.)
#[test]
fn rounding_that_changes_nothing_keeps_a_decimal_decimal() {
    let half_decimal = Expr::Num(Number::from_decimal_str("0.5"));
    let rounded = round_numbers_to_precision(&half_decimal, 3);
    assert_eq!(text(&rounded), "0.5");
}

/// A fraction is never decimalized by *display* rounding, however few digits
/// are asked for.
///
/// This test previously asserted the opposite (`1/3` → `0.333`) on the reading
/// that rounding decimalizes whenever it would change the value. Legacy could
/// not do that — it had no rational type, so `1/3` was `["/", 1, 3]` and
/// rounding mapped over two whole integers — and the value-changed rule made
/// the display depend on whether anything had called `simplify`:
/// `<math>2/3</math>` showed the fraction and `<point>(2/3,3)</point>` showed
/// `0.67`, in the same document.
#[test]
fn display_rounding_never_decimalizes_a_fraction() {
    let third = Expr::Num(Number::rat(1, 3));
    assert_eq!(text(&round_numbers_to_precision(&third, 3)), "1/3");
    assert_eq!(text(&round_numbers_to_precision(&third, 1)), "1/3");
    assert_eq!(
        latex(&round_numbers_to_precision_plus_decimals(&third, 3.0, 2.0)),
        "\\frac{1}{3}"
    );
    // A decimal quantity is unaffected by the rule and still rounds.
    let half = Expr::Num(Number::from_decimal_str("0.5"));
    assert_eq!(text(&round_numbers_to_precision(&half, 3)), "0.5");
    assert_eq!(text(&round_numbers_to_decimals(&half, 0)), "1");
}

// ---- item 10a: negative coefficients ---------------------------------------

/// The reported case, both formatters.
#[test]
fn negative_leading_coefficient_is_a_subtraction() {
    let e = ast(r#"["+","a",["*",-3,"b"]]"#);
    assert_eq!(text(&e), "a - 3 b");
    assert_eq!(latex(&e), "a - 3 b");
}

/// A bare product with a negative leading coefficient pulls the sign to the
/// front rather than parenthesising it.
#[test]
fn negative_leading_coefficient_standalone() {
    let e = ast(r#"["*",-3,"b"]"#);
    assert_eq!(text(&e), "-3 b");
    assert_eq!(latex(&e), "-3 b");
}

/// A fraction with a negative numerator shows the sign out front (same family:
/// the sign belongs on the whole term, not inside a paren).
#[test]
fn negative_fraction_shows_sign_out_front() {
    assert_eq!(text(&ast(r#"["/",-2,3]"#)), "-2/3");
    assert_eq!(latex(&ast(r#"["/",-2,3]"#)), "-\\frac{2}{3}");
    // and as a sum term the sign folds into the connective
    assert_eq!(text(&ast(r#"["+","z",["/",-2,3]]"#)), "z - 2/3");
    assert_eq!(latex(&ast(r#"["+","z",["/",-2,3]]"#)), "z - \\frac{2}{3}");
}

// ---- item 9 (partial): integer powers of the imaginary unit fold -----------

/// `i^n` folds to `{1, i, -1, -i}` — the unambiguous half of item 9. `equals`
/// already certified `i^2 == -1` numerically (so grading was never affected);
/// this is the `simplify`/`.tree` half that left `["^","i",2]` symbolic. The
/// *root* cases in item 9 (`cbrt(x^3)`, `sqrt(16x²y⁴)`) are deliberately not
/// here: they turn on a branch-cut / domain-assumption choice (`4|x|y²` vs
/// `4xy²`) that needs DoenetML's promised corpus to pin down.
#[test]
fn integer_powers_of_i_fold() {
    use math_expressions::{simplify, TextToAst};
    let p = |s: &str| TextToAst::new(Default::default()).convert(s).unwrap();
    let tree = |e: &Expr| math_expressions::expr::serde::to_js(e).to_string();
    assert_eq!(tree(&simplify(&p("i^2"))), "-1");
    assert_eq!(tree(&simplify(&p("i^3"))), r#"["-","i"]"#); // -i
    assert_eq!(tree(&simplify(&p("i^4"))), "1");
    assert_eq!(tree(&simplify(&p("i*i"))), "-1");
    assert_eq!(tree(&simplify(&p("2i*3i"))), "-6");
    // A negative power too: i^(-1) = -i.
    assert_eq!(tree(&simplify(&p("i^(-1)"))), r#"["-","i"]"#);
}

// ---- item 9 (roots): numeric radicands fold, prefer real else principal ----
//
// Confirmed convention (from the maintainer): a *number* under a root folds,
// preferring a real root when one exists and otherwise the correct principal
// complex root; a *variable* radicand never folds. Square roots of negatives
// are the concrete gap — they are exactly `sqrt(|r|)·i`. Higher even roots of
// negatives need the exact surd form and stay symbolic for now.

fn simp(s: &str) -> Expr {
    use math_expressions::{simplify, TextToAst};
    simplify(&TextToAst::new(Default::default()).convert(s).unwrap())
}
fn tree(e: &Expr) -> String {
    math_expressions::expr::serde::to_js(e).to_string()
}

#[test]
fn sqrt_of_negative_folds_to_principal_imaginary() {
    // `i` surfaces only when the whole radicand is a perfect square; otherwise
    // the perfect-square factor comes out and the sign stays under the root
    // (matching the JS oracle: `sqrt(-810) → 9·sqrt(-10)`). Still numerically
    // equal to the imaginary form, as the `equals` checks below confirm.
    assert_eq!(tree(&simp("sqrt(-1)")), r#""i""#);
    assert_eq!(tree(&simp("sqrt(-4)")), r#"["*",2,"i"]"#); // 2i
    assert_eq!(tree(&simp("sqrt(-2)")), r#"["apply","sqrt",-2]"#); // √(−2)
    assert_eq!(tree(&simp("sqrt(-8)")), r#"["*",2,["apply","sqrt",-2]]"#); // 2√(−2)
                                                                           // the `^(1/2)` power form agrees with the `sqrt` application
    assert_eq!(tree(&simp("(-4)^(1/2)")), r#"["*",2,"i"]"#);
    // whatever it folds to must equal the value the numeric evaluator gives
    use math_expressions::equals;
    let o = Default::default();
    let p = |s: &str| {
        math_expressions::TextToAst::new(Default::default())
            .convert(s)
            .unwrap()
    };
    assert!(equals(&simp("sqrt(-4)"), &p("2i"), &o));
    assert!(equals(&simp("sqrt(-8)"), &p("2i sqrt(2)"), &o));
}

#[test]
fn prefer_a_real_root_when_one_exists() {
    // Odd roots of negatives have a real value, so it wins over the complex
    // principal root — unchanged from before.
    assert_eq!(tree(&simp("cbrt(-8)")), "-2");
    assert_eq!(tree(&simp("nthroot(-8,3)")), "-2");
    assert_eq!(tree(&simp("(-8)^(1/3)")), "-2");
    assert_eq!(tree(&simp("nthroot(-32,5)")), "-2");
}

#[test]
fn a_variable_radicand_pulls_its_numeric_square() {
    // The *positive* perfect-square factor pulls out regardless of the unknown
    // sign of `y` (`sqrt(-4y) = 2·sqrt(-y)`, valid on either branch); the sign
    // and the variable stay under the root. Matches the JS oracle, which pulls
    // `16 → 4` out of `sqrt(-16x⁵)` with no assumptions on `x`.
    assert_eq!(
        tree(&simp("sqrt(-4 y)")),
        r#"["*",2,["apply","sqrt",["-","y"]]]"#
    );
}

#[test]
fn higher_even_root_of_a_negative_stays_symbolic_for_now() {
    // `(-16)^(1/4)` is exactly `√2 (1 + i)`, but that needs the surd lattice we
    // do not yet reach for roots; left symbolic rather than approximated.
    assert!(matches!(simp("nthroot(-16,4)"), Expr::Apply(..)));
}

// ---- item 10b: the integral glyph and parentheses --------------------------

/// `∫_a^b f(x) dx`, not `int_a^b(f(x) dx)`.
#[test]
fn integral_keeps_glyph_and_drops_parentheses() {
    let e = pl(r"\int_{a}^{b} f(x) dx");
    assert_eq!(text(&e), "∫_a^b f(x) dx");
    assert_eq!(latex(&e), "\\int_{a}^{b} f\\left(x\\right) dx");
}
