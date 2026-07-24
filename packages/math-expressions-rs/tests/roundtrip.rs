//! Output-formatter correctness via round-trip: for every expression the
//! parsers produce, rendering it and re-parsing must yield a structurally
//! equal expression. This is the oracle for the clean-slate formatters (the
//! JS output strings are no longer the spec).
//!
//! The corpus is every input in the parser tree-fixtures — realistic
//! expressions by construction — so we never hand-author expected output.

use math_expressions::output::{latex, text};
use math_expressions::{Expr, LatexToAst, LatexToAstOptions, TextToAst, TextToAstOptions};

#[derive(serde::Deserialize)]
struct TreeCase {
    input: String,
}

fn inputs(files: &[&str]) -> Vec<String> {
    let mut v = vec![];
    for f in files {
        let cases: Vec<TreeCase> = serde_json::from_str(f).unwrap();
        v.extend(cases.into_iter().map(|c| c.input));
    }
    v
}

fn text_inputs() -> Vec<String> {
    inputs(&[
        include_str!("fixtures/text-to-ast.json"),
        include_str!("fixtures/text-to-ast-edge.json"),
    ])
}

fn latex_inputs() -> Vec<String> {
    inputs(&[
        include_str!("fixtures/latex-to-ast.json"),
        include_str!("fixtures/latex-to-ast-edge.json"),
    ])
}

fn parse_text(s: &str) -> Result<Expr, String> {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .map_err(|e| e.to_string())
}

fn parse_latex(s: &str) -> Result<Expr, String> {
    LatexToAst::new(LatexToAstOptions::default())
        .convert(s)
        .map_err(|e| e.to_string())
}

/// Inputs whose parsed expression cannot round-trip through *any* text
/// rendering, because the notation itself is ambiguous — not a formatter bug:
///
/// - a raw expression containing the bare symbol `d` inside a fraction
///   re-parses as a Leibniz derivative (`d…/d…` means differentiation), so
///   e.g. `Mul([Div([d^2 x, d]), s, d, t, a])` has no unambiguous text form;
/// - nested `|…|` absolute values are formally ambiguous (the parser itself
///   resolves them by backtracking).
///
/// - a function whose head is itself a *power of a power* applied to arguments
///   (`sin^^` → `Apply(Pow(Pow(sin, ＿), ＿), [＿])`): a correct rendering must
///   parenthesize the inner power (`(sin^＿)^＿`, since bare `x^y^z` is invalid),
///   but a parenthesized head immediately followed by `(…)` re-parses as
///   multiplication rather than application. The two are irreconcilable for this
///   degenerate shape.
///
/// These come from the adversarial lexer edge-corpus, not realistic input.
const KNOWN_AMBIGUOUS: &[&str] = &[
    "d^2x/dsdta=q",
    "d^3x/dsdt^2a=q",
    "d^3κ/dξdβ^2♡=q",
    "|a*|b|*c|",
    "|a(q|b|r)c|",
    "sin^^",
    "\\sin^^",
];

#[test]
fn text_roundtrip() {
    let opts = text::TextOpts::default();
    let mut failures = vec![];
    let mut n = 0;

    for input in text_inputs() {
        if KNOWN_AMBIGUOUS.contains(&input.as_str()) {
            continue;
        }
        let Ok(expr) = parse_text(&input) else {
            continue;
        };
        n += 1;
        let rendered = text::convert(&expr, &opts);
        match parse_text(&rendered) {
            Ok(reparsed) if reparsed == expr => {}
            Ok(reparsed) => failures.push(format!(
                "input   {:?}\n  render  {:?}\n  expr    {:?}\n  reparse {:?}",
                input, rendered, expr, reparsed
            )),
            Err(e) => failures.push(format!(
                "input   {:?}\n  render  {:?}\n  expr    {:?}\n  ERROR   {}",
                input, rendered, expr, e
            )),
        }
    }

    if !failures.is_empty() {
        panic!(
            "{}/{} text round-trips failed:\n\n{}",
            failures.len(),
            n,
            failures
                .iter()
                .take(40)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n\n")
        );
    }
}

#[test]
fn latex_roundtrip() {
    let opts = latex::LatexOpts::default();
    let mut failures = vec![];
    let mut n = 0;

    for input in latex_inputs() {
        if KNOWN_AMBIGUOUS.contains(&input.as_str()) {
            continue;
        }
        let Ok(expr) = parse_latex(&input) else {
            continue;
        };
        n += 1;
        let rendered = latex::convert(&expr, &opts);
        match parse_latex(&rendered) {
            Ok(reparsed) if reparsed == expr => {}
            Ok(reparsed) => failures.push(format!(
                "input   {:?}\n  render  {:?}\n  expr    {:?}\n  reparse {:?}",
                input, rendered, expr, reparsed
            )),
            Err(e) => failures.push(format!(
                "input   {:?}\n  render  {:?}\n  expr    {:?}\n  ERROR   {}",
                input, rendered, expr, e
            )),
        }
    }

    if !failures.is_empty() {
        panic!(
            "{}/{} latex round-trips failed:\n\n{}",
            failures.len(),
            n,
            failures
                .iter()
                .take(40)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n\n")
        );
    }
}

/// Round-trip expressions that the fixture corpus doesn't reach: negative
/// infinity in tight positions, floats whose JS rendering would be
/// exponential (we render positional decimal so they re-parse), and huge
/// exact rationals in tight positions.
#[test]
fn constructed_roundtrip() {
    use math_expressions::expr::MathConst;

    let x = || Expr::sym("x");
    let neg_inf = || Expr::Const(MathConst::NegInf);
    // Exact rationals, the way §3a decimals parse.
    let dec = |s: &str| parse_text(s).unwrap();

    let cases = vec![
        Expr::Mul(vec![Expr::int(2), neg_inf()]),
        Expr::Pow(Box::new(x()), Box::new(neg_inf())),
        Expr::Div(Box::new(x()), Box::new(neg_inf())),
        dec("0.000000000003"),
        dec("-0.000000000003"),
        dec("0.0000001"),
        Expr::Add(vec![x(), dec("-0.000000000003")]),
        Expr::Mul(vec![dec("0.00000015"), x()]),
        Expr::Pow(Box::new(x()), Box::new(dec("0.5"))),
    ];

    let topts = text::TextOpts::default();
    let lopts = latex::LatexOpts::default();
    for expr in cases {
        let t = text::convert(&expr, &topts);
        assert_eq!(
            parse_text(&t).as_ref(),
            Ok(&expr),
            "text round-trip via {:?}",
            t
        );
        let l = latex::convert(&expr, &lopts);
        assert_eq!(
            parse_latex(&l).as_ref(),
            Ok(&expr),
            "latex round-trip via {:?}",
            l
        );
    }
}

/// §3a: decimals parse to exact rationals, never floats. A tiny decimal keeps
/// full precision (no f64 rounding), and an "overflow" literal that JS would
/// round to Infinity becomes an exact big integer.
#[test]
fn decimals_are_exact() {
    use math_expressions::Number;

    // 0.1 + 0.2 == 0.3 structurally (the whole point of exactness).
    let lhs = parse_text("0.1").unwrap();
    let rhs = parse_text("0.2").unwrap();
    let sum = parse_text("0.3").unwrap();
    assert_eq!(lhs, Expr::Num(Number::rat(1, 10)));
    assert_eq!(rhs, Expr::Num(Number::rat(1, 5)));
    assert_eq!(sum, Expr::Num(Number::rat(3, 10)));

    // Half is a rational, not a float.
    assert_eq!(parse_text("0.5").unwrap(), Expr::Num(Number::rat(1, 2)));

    // No float ever appears from parsing.
    assert!(!matches!(
        parse_text("3.14159").unwrap(),
        Expr::Num(Number::Float(_))
    ));

    // "Overflow" literal is exact, not Infinity.
    let big = parse_text("1E30").unwrap();
    assert_eq!(
        text::convert(&big, &text::TextOpts::default()),
        "1".to_string() + &"0".repeat(30)
    );
}
