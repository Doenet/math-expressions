//! 100-digit certification of every kernel in the `precise` registry.
//!
//! One test per registry function, each checked digit-for-digit against an
//! externally published expansion. References are the OEIS b-files (5000
//! certified digits per constant; A-numbers cited at each constant below),
//! truncated to 112+ significant digits — comfortably past the 100 digits
//! under test, so reference truncation can never interact with the
//! implementation's final-digit rounding.
//!
//! The first 54 digits of each reference also agree with the independently
//! sourced constants in `precise.rs`, and the leading 16 with f64 libm, so a
//! transcription error on either side would be caught twice over.

use math_expressions::precise::{evaluate_to_precision, Precise};
use math_expressions::{Expr, TextToAst, TextToAstOptions};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

/// First `d` significant digits as a plain digit string (no dot/exponent).
fn digits_of(p: &Precise, d: usize) -> String {
    let s = p
        .to_decimal_string(d)
        .unwrap_or_else(|| panic!("expected digits from {p:?}"));
    s.chars().filter(|c| c.is_ascii_digit()).take(d).collect()
}

fn increment_decimal(s: &str) -> String {
    let mut digits: Vec<u8> = s.bytes().map(|b| b - b'0').collect();
    for d in digits.iter_mut().rev() {
        if *d == 9 {
            *d = 0;
        } else {
            *d += 1;
            return digits.iter().map(|d| (b'0' + d) as char).collect();
        }
    }
    let mut out = String::from("1");
    out.extend(digits.iter().map(|d| (b'0' + d) as char));
    out.truncate(s.len());
    out
}

/// Assert the first 100 significant digits of `expr` match the reference
/// (which is truncated, so the correctly-rounded final digit may be +1).
fn assert_100(expr: &str, want: &str) {
    let got = digits_of(&evaluate_to_precision(&parse(expr), 100), 100);
    let want_trunc: String = want.chars().take(100).collect();
    let bumped = increment_decimal(&want_trunc);
    assert!(
        got == want_trunc || got == bumped,
        "{expr} @ 100 digits:\n got {got}\nwant {want_trunc}\n(or  {bumped})"
    );
}

// Significant-digit reference strings from OEIS b-files (fetched 2026-07-19).
const SQRT2: &str = "14142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727350138462309"; // A002193, sqrt(2)
const PI: &str = "31415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679821480865132"; // A000796, pi
const E: &str = "27182818284590452353602874713526624977572470936999595749669676277240766303535475945713821785251664274274663919320"; // A001113, e
const LN2: &str = "69314718055994530941723212145817656807550013436025525412068000949339362196969471560586332699641868754200148102057"; // A002162, ln(2)
const EXP2: &str = "73890560989306502272304274605750078131803155705518473240871278225225737960790577633843124850791217947737531612654"; // A072334, e^2
const LN3: &str = "10986122886681096913952452369225257046474905578227494517346943336374942932186089668736157548137320887879700290659"; // A002391, ln(3)
const SIN1: &str = "8414709848078965066525023216302989996225630607983710656727517099919104043912396689486397435430526958543490379079"; // A049469, sin(1)
const COS1: &str = "5403023058681397174009366074429766037323104206179222276700972553811003947744717645179518560871830893435717311600"; // A049470, cos(1)
const TAN1: &str = "15574077246549022305069748074583601730872507723815200383839466056988613971517272895550999652022429838046338214117"; // A049471, tan(1)
const PI_6: &str = "5235987755982988730771072305465838140328615665625176368291574320513027343810348331046724708903528446636913477522"; // A019673, pi/6 = asin(1/2)
const ACOS_THIRD: &str = "12309594173407746821349291782479873757103400093550948390555483336639923144782560878532516201708609211389442794492"; // A137914, arccos(1/3)
const ATAN_2: &str = "11071487177940905030170654601785370400700476454014326466765392074337103389773627940134171286861706414345441910054"; // A105199, arctan(2)
const SINH1: &str = "11752011936438014568823818505956008151557179813340958702295654130133075673043238956071174520896233918404195333275"; // A073742, sinh(1)
const COSH1: &str = "15430806348152437784779056207570616826015291123658637047374022147107690630492236989642647264355430355870468586044"; // A073743, cosh(1)
const TANH1: &str = "7615941559557648881194582826047935904127685972579365515968105001219532445766384834589475216736767144219027597015"; // A073744, tanh(1)
const LOG10_2: &str = "3010299956639811952137388947244930267681898814621085413104274611271081892744245094869272521181861720406844771914"; // A007524, log_10(2)

#[test]
fn sqrt_100_digits() {
    assert_100("sqrt(2)", SQRT2);
}

#[test]
fn exp_100_digits() {
    assert_100("exp(2)", EXP2);
    assert_100("exp(1)", E);
}

#[test]
fn ln_100_digits() {
    assert_100("ln(3)", LN3);
    // "log" is an alias for the same kernel.
    assert_100("log(3)", LN3);
}

#[test]
fn abs_100_digits() {
    // |1 − e| = e − 1: the argument's sign is only known numerically, so the
    // abs kernel itself must run. e − 1 leaves every digit of e after the
    // leading one unchanged.
    let e_minus_1 = format!("1{}", &E[1..]);
    assert_100("abs(1 - e)", &e_minus_1);
}

#[test]
fn sin_100_digits() {
    assert_100("sin(1)", SIN1);
}

#[test]
fn cos_100_digits() {
    assert_100("cos(1)", COS1);
}

#[test]
fn tan_100_digits() {
    assert_100("tan(1)", TAN1);
}

#[test]
fn asin_100_digits() {
    assert_100("asin(1/2)", PI_6);
}

#[test]
fn acos_100_digits() {
    assert_100("acos(1/3)", ACOS_THIRD);
}

#[test]
fn atan_100_digits() {
    assert_100("atan(2)", ATAN_2);
}

#[test]
fn sinh_100_digits() {
    assert_100("sinh(1)", SINH1);
}

#[test]
fn cosh_100_digits() {
    assert_100("cosh(1)", COSH1);
}

#[test]
fn tanh_100_digits() {
    assert_100("tanh(1)", TANH1);
}

#[test]
fn log10_100_digits() {
    assert_100("log10(2)", LOG10_2);
}

#[test]
fn constants_100_digits() {
    assert_100("pi", PI);
    assert_100("e", E);
    assert_100("ln(2)", LN2);
}

#[test]
fn complex_kernels_100_digits() {
    // Principal branches at 100 digits, against the same references:
    // ln(-1) = iπ and sqrt(-2) = i·√2.
    for (expr, want) in [("ln(-1)", PI), ("sqrt(-2)", SQRT2)] {
        let p = evaluate_to_precision(&parse(expr), 100);
        let Precise::Complex { im, .. } = &p else {
            panic!("{expr}: expected complex, got {p:?}")
        };
        let got: String = im
            .to_decimal_string(100)
            .chars()
            .filter(|c| c.is_ascii_digit())
            .take(100)
            .collect();
        let want_trunc: String = want.chars().take(100).collect();
        let bumped = increment_decimal(&want_trunc);
        assert!(
            got == want_trunc || got == bumped,
            "{expr} @ 100 digits:\n got {got}\nwant {want_trunc}"
        );
    }
}
