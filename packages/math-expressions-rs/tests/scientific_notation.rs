//! DOENET_INTEGRATION item 3 — the magnitude threshold at which a float
//! renders as `mantissa × 10^exponent`.
//!
//! Legacy had no threshold of its own: it called `Number.prototype.toString()`
//! and switched whenever the result contained an `e`. That puts the switch at
//! the ECMAScript rule — positional over `0.000001 ..< 1e21`, exponential
//! outside — which is what DoenetML's `avoidScientificNotation` attribute is
//! written against. These pin the threshold, the two spellings, and the
//! padding interaction, which is the part that is easy to get subtly wrong.

use math_expressions::num::Number;
use math_expressions::{to_latex, to_text, Expr, LatexOpts, TextOpts};

fn num(v: f64) -> Expr {
    Expr::Num(Number::from_f64(v))
}

fn text(v: f64) -> String {
    to_text(&num(v), &TextOpts::default())
}

fn latex(v: f64) -> String {
    to_latex(&num(v), &LatexOpts::default())
}

/// Text and LaTeX with the padding options set, so the two spellings and the
/// padding rules are exercised together.
fn text_padded(v: f64, digits: Option<u32>, decimals: Option<u32>) -> String {
    to_text(
        &num(v),
        &TextOpts {
            pad_to_digits: digits,
            pad_to_decimals: decimals,
            ..Default::default()
        },
    )
}

#[test]
fn the_threshold_is_javascripts() {
    // Just inside: positional, however long.
    assert_eq!(text(1e20), "100000000000000000000");
    assert_eq!(text(1e-6), "0.000001");
    // Just outside: exponential.
    assert_eq!(text(1e21), "1 * 10^21");
    assert_eq!(text(1e-7), "1 * 10^(-7)");
}

#[test]
fn the_two_printers_spell_it_differently() {
    assert_eq!(text(1.23e22), "1.23 * 10^22");
    assert_eq!(latex(1.23e22), "1.23 \\cdot 10^{22}");
    // A negative exponent needs parens in text (`10^-11` is not text grammar)
    // but not in LaTeX, where the braces already delimit it.
    assert_eq!(text(1.23e-11), "1.23 * 10^(-11)");
    assert_eq!(latex(1.23e-11), "1.23 \\cdot 10^{-11}");
}

/// The rendered form is a product, so it parenthesises as a power's base and a
/// negative one carries its sign outside — the same precedence any product gets.
#[test]
fn it_binds_like_the_product_it_is_spelled_as() {
    let p = Expr::Pow(Box::new(num(1.23e-11)), Box::new(Expr::int(5)));
    assert_eq!(to_text(&p, &TextOpts::default()), "(1.23 * 10^(-11))^5");
    assert_eq!(
        to_latex(&p, &LatexOpts::default()),
        "\\left(1.23 \\cdot 10^{-11}\\right)^{5}"
    );
    assert_eq!(text(-1.23e-11), "-1.23 * 10^(-11)");
}

#[test]
fn avoiding_it_expands_the_number_in_full() {
    let opts = TextOpts {
        avoid_scientific_notation: true,
        ..Default::default()
    };
    assert_eq!(
        to_text(&num(1.23e30), &opts),
        "1230000000000000000000000000000"
    );
    assert_eq!(to_text(&num(1.23e-12), &opts), "0.00000000000123");
    let opts = LatexOpts {
        avoid_scientific_notation: true,
        ..Default::default()
    };
    assert_eq!(to_latex(&num(1.23e-12), &opts), "0.00000000000123");
}

/// `padToDigits` counts significant characters of the *mantissa* — padding
/// `1.23e30` to five digits is `1.2300 * 10^30`, not thirty-one characters.
#[test]
fn pad_to_digits_pads_the_mantissa() {
    assert_eq!(text_padded(1.23e30, Some(5), None), "1.2300 * 10^30");
    assert_eq!(text_padded(1.23e-12, Some(5), None), "1.2300 * 10^(-12)");
    // Below the threshold it is the ordinary positional padding.
    assert_eq!(text_padded(1.23e16, Some(5), None), "12300000000000000");
    assert_eq!(text_padded(1.23, Some(5), None), "1.2300");
}

/// `padToDecimals` asks for decimal places of the *value*. Legacy resolved the
/// mismatch two ways, and both matter:
#[test]
fn pad_to_decimals_shifts_by_the_exponent_or_gives_up_on_scientific() {
    // A positive exponent: scientific notation saves no zeros once decimals
    // are being padded on, so the whole number reverts to positional.
    assert_eq!(
        text_padded(1.23e30, None, Some(5)),
        "1230000000000000000000000000000.00000"
    );
    // A negative exponent: the request shifts by the exponent, so asking for 5
    // decimals of 1.23e-12 asks for -7 of the mantissa — that is, nothing.
    assert_eq!(text_padded(1.23e-12, None, Some(5)), "1.23 * 10^(-12)");
    // Both together, where the digits bound is what actually bites.
    assert_eq!(
        text_padded(1.23e-12, Some(6), Some(2)),
        "1.23000 * 10^(-12)"
    );
    assert_eq!(
        text_padded(1.23e30, Some(6), Some(2)),
        "1230000000000000000000000000000.00"
    );
}

/// Scientific notation written out by hand — `123 * 10^28` — must not have its
/// power padded into `10.000^28.000`. Legacy carved out `integer^integer` for
/// exactly this, and it is the one place padding does not reach.
#[test]
fn an_integer_power_is_never_padded() {
    let e = Expr::Mul(vec![
        Expr::int(123),
        Expr::Pow(Box::new(Expr::int(10)), Box::new(Expr::int(28))),
    ]);
    let opts = TextOpts {
        pad_to_digits: Some(5),
        ..Default::default()
    };
    assert_eq!(to_text(&e, &opts), "123.00 * 10^28");
    let opts = LatexOpts {
        pad_to_decimals: Some(5),
        ..Default::default()
    };
    assert_eq!(to_latex(&e, &opts), "123.00000 \\cdot 10^{28}");
    // A non-integer power still pads, since it is an ordinary decimal display.
    let e = Expr::Pow(Box::new(num(1.5)), Box::new(Expr::int(2)));
    let opts = TextOpts {
        pad_to_decimals: Some(3),
        ..Default::default()
    };
    assert_eq!(to_text(&e, &opts), "1.500^2.000");
}

/// A non-finite float is a word, not a number, and padding it produced
/// `NaN.00`. Padding now skips it.
#[test]
fn a_non_finite_float_is_never_padded() {
    assert_eq!(text_padded(f64::NAN, Some(5), Some(5)), "NaN");
    assert_eq!(text_padded(f64::INFINITY, Some(5), Some(5)), "Infinity");
}

/// Exact integers and rationals take the threshold too — it is a *display*
/// rule, not a float rule.
///
/// This test used to assert the opposite, on the reasoning that the threshold
/// belongs to float rendering. That held only while DoenetML's values reached
/// the printer as floats: they crossed through a JSON AST, which has one
/// numeric type. They no longer do, and the old rule broke the very attribute
/// this file is written against — `avoidScientificNotation` exists to turn the
/// threshold *off*, so an exact value that never reached it made the attribute
/// a no-op. DoenetML's own test pins both directions and both ends:
/// `<math>2000000000000000000000 x^2</math>` renders `2 \cdot 10^{21} x^{2}`,
/// and the same math under the attribute renders every digit.
///
/// Exactness is not what changed — `2 * 10^21` and `1.23 * 10^30` are the exact
/// values, spelled differently. What a `Float` cannot do, and this still does
/// not do, is invent digits it never had.
#[test]
fn exact_numbers_take_the_threshold_too() {
    let parse = |s: &str| {
        math_expressions::TextToAst::new(Default::default())
            .convert(s)
            .unwrap()
    };
    let big = parse("1230000000000000000000000000000");
    assert_eq!(to_text(&big, &TextOpts::default()), "1.23 * 10^30");
    assert_eq!(to_latex(&big, &LatexOpts::default()), "1.23 \\cdot 10^{30}");

    // `avoidScientificNotation` puts every digit back, exactly as authored.
    assert_eq!(
        to_text(
            &big,
            &TextOpts {
                avoid_scientific_notation: true,
                ..Default::default()
            }
        ),
        "1230000000000000000000000000000"
    );

    // Inside the threshold nothing changes, and an exact fraction is not a
    // decimal display at all — it has no exponent to take.
    assert_eq!(
        to_text(&parse("1E20"), &TextOpts::default()),
        "1".to_string() + &"0".repeat(20)
    );
    assert_eq!(to_text(&parse("2/3"), &TextOpts::default()), "2/3");
}
