//! Output-formatter correctness via round-trip: for every expression the
//! parsers produce, rendering it and re-parsing must yield a structurally
//! equal expression. This is the oracle for the clean-slate formatters (the
//! JS output strings are no longer the spec).
//!
//! The corpus is every input in the parser tree-fixtures — realistic
//! expressions by construction — so we never hand-author expected output.

use math_expressions::print::{latex, text};
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

/// Whether a re-parse that is not *structurally* identical is nonetheless an
/// acceptable round-trip.
///
/// Exactly one rendering choice has this property, and it is deliberate:
/// scientific notation. Both printers spell a number below `0.000001` as
/// `3 * 10^(-12)` / `3 \cdot 10^{-12}`, because a wall of leading zeros is
/// unreadable and because DoenetML's `avoidScientificNotation` attribute exists
/// to switch that off — an attribute that would do nothing if the threshold
/// were never applied. Parsing does not evaluate, so the spelling comes back as
/// the product it is written as rather than as a single number.
///
/// This is not new with exact decimals — `to_text(Float(3e-12))` has always had
/// it — it was simply unreachable from this corpus, in which every number is
/// exact because that is what the parsers produce. Making it explicit here is
/// better than letting a future exact/float change silently trip a test whose
/// message would point at the formatter.
///
/// The two conditions together are what keep this from being a blanket excuse:
/// the value must be unchanged, and rendering the re-parse must reproduce the
/// *identical* string — so the printer is a fixpoint and nothing drifts on a
/// second trip. A formatter bug fails one or the other.
fn is_display_only_divergence(
    expr: &Expr,
    reparsed: &Expr,
    rendered: &str,
    render: impl Fn(&Expr) -> String,
) -> bool {
    math_expressions::equals(expr, reparsed, &math_expressions::EqOptions::default())
        && render(reparsed) == rendered
}

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
            Ok(reparsed)
                if is_display_only_divergence(&expr, &reparsed, &rendered, |e| {
                    text::convert(e, &opts)
                }) => {}
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
            Ok(reparsed)
                if is_display_only_divergence(&expr, &reparsed, &rendered, |e| {
                    latex::convert(e, &opts)
                }) => {}
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
/// infinity in tight positions, tiny decimals in every syntactic position that
/// could mis-bind, and huge exact rationals in tight positions.
///
/// The tiny decimals used to be here to pin *positional* rendering: rendering
/// them exponentially was avoided precisely so they would re-parse. They now
/// render exponentially — see [`is_display_only_divergence`] for why that
/// changed — so what they pin is the position handling: a `3 * 10^(-12)` in a
/// sum, a product, or an exponent must still come back binding the same way,
/// which is the part a formatter can plausibly get wrong.
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
        let t_back = parse_text(&t).unwrap_or_else(|e| panic!("text {t:?} did not parse: {e}"));
        assert!(
            t_back == expr
                || is_display_only_divergence(&expr, &t_back, &t, |e| text::convert(e, &topts)),
            "text round-trip via {t:?}\n  expr    {expr:?}\n  reparse {t_back:?}"
        );
        let l = latex::convert(&expr, &lopts);
        let l_back = parse_latex(&l).unwrap_or_else(|e| panic!("latex {l:?} did not parse: {e}"));
        assert!(
            l_back == expr
                || is_display_only_divergence(&expr, &l_back, &l, |e| latex::convert(e, &lopts)),
            "latex round-trip via {l:?}\n  expr    {expr:?}\n  reparse {l_back:?}"
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

    // "Overflow" literal is exact, not Infinity. It *displays* in scientific
    // notation (past 1e21, the JS threshold that exact values honour too), so
    // exactness is checked against the digits themselves rather than against a
    // positional rendering — and `avoidScientificNotation` shows all thirty.
    let big = parse_text("1E30").unwrap();
    assert_eq!(text::convert(&big, &text::TextOpts::default()), "1 * 10^30");
    assert_eq!(
        text::convert(
            &big,
            &text::TextOpts {
                avoid_scientific_notation: true,
                ..Default::default()
            }
        ),
        "1".to_string() + &"0".repeat(30)
    );
}
