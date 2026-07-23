//! Regressions for the review-cycle fixes (post-2026-07-22 architecture round).
//! Each test pins a specific defect found during the PR review so it cannot
//! silently regress. Under the workspace `panic = "abort"` profile, the
//! panic/abort cases here would be full wasm-worker crashes if they regressed.

use math_expressions::precise::{evaluate_to_precision, Precise};
use math_expressions::{canonicalize, det, Expr, LatexToAst, LatexToAstOptions, Number, NumberNotation};

fn parse_latex(nt: NumberNotation, s: &str) -> Result<Expr, math_expressions::ParseError> {
    LatexToAst::new(LatexToAstOptions {
        notation: nt,
        ..Default::default()
    })
    .convert(s)
}

/// `get_single_digit_as_number` used to hardcode `.` as the decimal separator,
/// so a leading-decimal number under a non-`.` notation (`",5"`, or the 2-byte
/// Arabic `"٫5"`) drove `b'0'` subtraction underflow / a mid-codepoint slice —
/// an abort. It must now fall through cleanly regardless of notation.
#[test]
fn latex_single_digit_decimal_separator_does_not_abort() {
    // Comma-decimal: `x^,5` is `x^0.5`. Previously underflowed `',' - '0'`.
    let comma = parse_latex(NumberNotation::comma(), "x^,5")
        .expect("comma-notation superscript must parse, not abort");
    assert_eq!(
        canonicalize(&comma),
        canonicalize(&parse_latex(NumberNotation::period(), "x^{0.5}").unwrap()),
        "x^,5 (comma) should mean x^0.5"
    );
    // Arabic decimal separator ٫ (U+066B, 2 bytes): previously a hard slice
    // panic on `text[1..]`. Must not abort (Ok or a clean Err are both fine).
    let _ = parse_latex(NumberNotation::from_decimal('\u{066B}'), "x^\u{066B}5");
}

/// An astronomically large integer power must degrade to `Unknown`, never
/// materialize a multi-gigabyte mantissa (an allocation-failure abort). The
/// magnitude guard in `powint_fix`/`cpowint` enforces this.
#[test]
fn astronomical_integer_power_is_unknown_not_abort() {
    let p = evaluate_to_precision(&parse("2^100000000000"), 10);
    assert!(
        matches!(p, Precise::Unknown(_)),
        "2^1e11 must be Unknown, got {p:?}"
    );
    // A merely-large but tractable power still evaluates exactly (guard is not
    // too aggressive).
    let ok = evaluate_to_precision(&parse("2^200 + 1"), 10);
    assert!(matches!(ok, Precise::Exact(_)), "2^200+1 got {ok:?}");
}

/// `ln` of a negative real takes the principal `+iπ` branch (matching
/// `atan2(+0, −x) = +π` and the `eval_complex` reference), not `−iπ`. The
/// high-precision `atan2_fix` path used to return `−π` on the negative real
/// axis; the existing digit-string test could not see the sign.
#[test]
fn ln_negative_real_uses_principal_plus_pi() {
    let p = evaluate_to_precision(&parse("ln(-1)"), 40);
    let (re, im) = p
        .to_complex_f64()
        .unwrap_or_else(|| panic!("ln(-1) should be complex, got {p:?}"));
    assert!(re.abs() < 1e-30, "Re ln(-1) ≈ 0, got {re}");
    assert!(
        (im - std::f64::consts::PI).abs() < 1e-12,
        "Im ln(-1) must be +π (principal branch), got {im}"
    );
}

/// A singular polynomial matrix in the Bareiss tier (n > `max_symbolic_det_dim`
/// = 6) must return determinant `0`, not an unevaluated opaque `det(…)`.
#[test]
fn singular_bareiss_tier_determinant_is_zero() {
    let n = 7; // above the cofactor tier, so det() routes through det_bareiss.
    let mut entries = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            // Column 3 is identically zero ⇒ the matrix is singular; the other
            // entries are distinct polynomials in x so `as_numbers` declines
            // (forcing the symbolic path) without collapsing too early.
            entries.push(if j == 3 {
                Expr::Num(Number::Int(0))
            } else {
                parse(&format!("x + {}", i * n + j + 1))
            });
        }
    }
    let m = Expr::Matrix {
        rows: n as u32,
        cols: n as u32,
        entries,
    };
    assert_eq!(
        canonicalize(&det(&m)),
        Expr::Num(Number::Int(0)),
        "det of a singular Bareiss-tier polynomial matrix must be 0, not opaque"
    );
}

fn parse(s: &str) -> Expr {
    math_expressions::TextToAst::new(Default::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e:?}"))
}
