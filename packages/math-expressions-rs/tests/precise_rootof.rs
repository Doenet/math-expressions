//! Arbitrary-precision evaluation of `RootOf` (ARBITRARY_PERCISION_PLAN §2d
//! completion): certified dyadic-Newton refinement for real roots, CFix
//! Newton with the rigorous n·|p(z)/p′(z)| bound for complex ones.

use math_expressions::precise::{evaluate_to_precision, Precise};
use math_expressions::{Expr, TextToAst, TextToAstOptions};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn digits_of(p: &Precise, d: usize) -> String {
    let s = p
        .to_decimal_string(d)
        .unwrap_or_else(|| panic!("expected digits from {p:?}"));
    s.chars().filter(|c| c.is_ascii_digit()).take(d).collect()
}

/// The reference strings are truncated; the implementation rounds its last
/// digit — accept exact or +1 in the last place (with carry).
fn assert_digits_eq(got: &str, want_full: &str, d: usize) {
    let want: String = want_full.chars().take(d).collect();
    let mut bumped: Vec<u8> = want.bytes().map(|b| b - b'0').collect();
    for x in bumped.iter_mut().rev() {
        if *x == 9 {
            *x = 0;
        } else {
            *x += 1;
            break;
        }
    }
    let bumped: String = bumped.iter().map(|x| (b'0' + x) as char).collect();
    assert!(
        got == want || got == bumped,
        "digits mismatch:\n got {got}\nwant {want} (or {bumped})"
    );
}

// OEIS A060006 (plastic number, the real root of t³ − t − 1), 70 digits.
const PLASTIC: &str = "1324717957244746025960908854478097340734404056901733364534015050302827";
// OEIS A002193.
const SQRT2: &str = "14142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727350138462309";

#[test]
fn real_rootof_to_60_digits() {
    let p = evaluate_to_precision(&parse("rootof(t^3 - t - 1, 0)"), 60);
    assert_digits_eq(&digits_of(&p, 60), PLASTIC, 60);
}

#[test]
fn rootof_matches_closed_form_sqrt() {
    let p = evaluate_to_precision(&parse("rootof(t^2 - 2, 1)"), 100);
    assert_digits_eq(&digits_of(&p, 100), SQRT2, 100);
    // Negative branch: same digits, negative value.
    let n = evaluate_to_precision(&parse("rootof(t^2 - 2, 0)"), 50);
    assert_digits_eq(&digits_of(&n, 50), SQRT2, 50);
    assert!(n.to_f64().unwrap() < 0.0);
}

#[test]
fn rootof_inside_arithmetic() {
    // ρ² + ρ evaluated through the tape (Root op under Add/Mul/Pow).
    let e = parse("rootof(t^3 - t - 1, 0)^2 + rootof(t^3 - t - 1, 0)");
    let a = digits_of(&evaluate_to_precision(&e, 30), 29);
    let b = digits_of(&evaluate_to_precision(&e, 60), 29);
    assert_eq!(a, b, "self-consistency across precisions");
    // Sanity vs f64: ρ² + ρ ≈ 1.754877666…
    let v = evaluate_to_precision(&e, 15).to_f64().expect("finite");
    let rho = 1.324_717_957_244_746;
    assert!((v - (rho * rho + rho)).abs() < 1e-12);
}

#[test]
fn complex_rootof_components() {
    // t⁴ − 2t² + 3 has no real roots; index 0 is the (re < 0, im < 0) one.
    let e = parse("rootof(t^4 - 2t^2 + 3, 0)");
    let p = evaluate_to_precision(&e, 40);
    let Precise::Complex { re, im } = &p else {
        panic!("expected complex, got {p:?}")
    };
    // Self-consistency: 40- vs 80-digit runs agree on the first 39.
    let p2 = evaluate_to_precision(&e, 80);
    let Precise::Complex { re: re2, im: im2 } = &p2 else {
        panic!("expected complex")
    };
    let take = |m: &math_expressions::precise::fix::MpFix, d: usize| -> String {
        m.to_decimal_string(d)
            .chars()
            .filter(|c| c.is_ascii_digit())
            .take(d)
            .collect()
    };
    assert_eq!(take(re, 39), take(re2, 39));
    assert_eq!(take(im, 39), take(im2, 39));
    // And both components match the f64 seed to ~1e-12.
    let (rf, if_) = p.to_complex_f64().expect("finite");
    use math_expressions::eval::{eval_complex, Env};
    let z = eval_complex(&math_expressions::canonicalize(&e), &Env::new()).expect("numeric");
    assert!((rf - z.re).abs() < 1e-12 && (if_ - z.im).abs() < 1e-12);
    assert!(rf < 0.0 && if_ < 0.0, "index 0 is the (−,−) root");
}

#[test]
fn conjugate_pair_sum_is_real() {
    // The two complex roots of t³ − t − 1 sum to −ρ (the t² coefficient is
    // zero). The complex tier must resolve the sum as real and match the
    // plastic-number digits.
    let e = parse("rootof(t^3 - t - 1, 1) + rootof(t^3 - t - 1, 2)");
    let p = evaluate_to_precision(&e, 40);
    assert!(
        matches!(p, Precise::Bounded(_)),
        "conjugate sum resolves to a real value, got {p:?}"
    );
    assert_digits_eq(&digits_of(&p, 40), PLASTIC, 40);
    assert!(p.to_f64().unwrap() < 0.0);
}

#[test]
fn eigenvalue_end_to_end_precision() {
    use math_expressions::{eigenvalues, Assumptions};
    // Companion of t³ − t − 1 → its real eigenvalue is the plastic number.
    let a = Expr::Matrix {
        rows: 3,
        cols: 3,
        entries: ["0", "0", "1", "1", "0", "1", "0", "1", "0"]
            .iter()
            .map(|s| parse(s))
            .collect(),
    };
    let vals = eigenvalues(&a, &Assumptions::new()).expect("eigenvalues");
    let p = evaluate_to_precision(&vals[0].0, 50);
    assert_digits_eq(&digits_of(&p, 50), PLASTIC, 50);
}

#[test]
fn rootof_precision_refusals_are_honest() {
    // Far beyond the precision cap: Unknown, not a hang.
    let p = evaluate_to_precision(&parse("rootof(t^3 - t - 1, 0)"), 1_000_000);
    assert!(matches!(p, Precise::Unknown(_)));
}
