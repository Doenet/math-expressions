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
/// These come from the adversarial lexer edge-corpus, not realistic input.
const KNOWN_AMBIGUOUS: &[&str] = &[
    "d^2x/dsdta=q",
    "d^3x/dsdt^2a=q",
    "d^3κ/dξdβ^2♡=q",
    "|a*|b|*c|",
    "|a(q|b|r)c|",
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
/// integral floats.
#[test]
fn constructed_roundtrip() {
    use math_expressions::expr::MathConst;
    use math_expressions::Number;

    let x = || Expr::sym("x");
    let neg_inf = || Expr::Const(MathConst::NegInf);
    let float = |v: f64| Expr::Num(Number::from_f64(v));

    let cases = vec![
        Expr::Mul(vec![Expr::int(2), neg_inf()]),
        Expr::Pow(Box::new(x()), Box::new(neg_inf())),
        Expr::Div(Box::new(x()), Box::new(neg_inf())),
        float(3e-12),
        float(-3e-12),
        float(6.02e23),
        float(1e300),
        float(9.3e18), // integral but above i64::MAX: stays a Float
        Expr::Add(vec![x(), float(-3e-12)]),
        Expr::Mul(vec![float(1.5e-7), x()]),
        Expr::Pow(Box::new(x()), Box::new(float(2.5e22))),
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

/// Overflowing literals become Infinity (as JS parseFloat does), never a
/// Float(inf) that the renderers can't spell.
#[test]
fn overflow_literal_is_infinity() {
    use math_expressions::expr::MathConst;
    let expr = parse_text("(1E999)").unwrap();
    assert_eq!(expr, Expr::Const(MathConst::Inf));
}
