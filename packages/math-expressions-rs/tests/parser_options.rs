//! Option-variant parser tests, hand-ported from the imperative test blocks
//! at the bottom of spec/quick_text-to-ast.spec.js and
//! spec/quick_latex-to-ast.spec.js (the fixture-extraction script only
//! captures the data maps). These exercise code paths no fixture touches:
//! splitSymbols off, custom symbol lists, simplified application disabled,
//! Leibniz parsing disabled, and the non-scientific-notation lexer.

use math_expressions::js_tree::to_js;
use math_expressions::{LatexToAst, LatexToAstOptions, TextToAst, TextToAstOptions};
use serde_json::{json, Value};

fn text(opts: TextToAstOptions, input: &str) -> Value {
    to_js(&TextToAst::new(opts).convert(input).unwrap())
}

fn latex(opts: LatexToAstOptions, input: &str) -> Value {
    to_js(&LatexToAst::new(opts).convert(input).unwrap())
}

fn strings(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| s.to_string()).collect()
}

// ---- text parser --------------------------------------------------------

#[test]
fn text_split_symbols() {
    let default = TextToAstOptions::default();
    let split = TextToAstOptions {
        split_symbols: true,
        ..Default::default()
    };
    let nosplit = TextToAstOptions {
        split_symbols: false,
        ..Default::default()
    };

    assert_eq!(text(default, "xzy"), json!(["*", "x", "z", "y"]));
    assert_eq!(text(split, "xzy"), json!(["*", "x", "z", "y"]));
    assert_eq!(text(nosplit, "xzy"), json!("xzy"));
}

#[test]
fn text_unsplit_symbols() {
    let none = TextToAstOptions {
        unsplit_symbols: vec![],
        ..Default::default()
    };
    assert_eq!(text(none, "3pi"), json!(["*", 3, "p", "i"]));

    let pi = TextToAstOptions {
        unsplit_symbols: strings(&["pi"]),
        ..Default::default()
    };
    assert_eq!(text(pi, "3pi"), json!(["*", 3, "pi"]));
}

#[test]
fn text_function_symbols() {
    let with = |fs: &[&str]| TextToAstOptions {
        function_symbols: strings(fs),
        ..Default::default()
    };

    assert_eq!(
        text(with(&[]), "f(x)+h(y)"),
        json!(["+", ["*", "f", "x"], ["*", "h", "y"]])
    );
    assert_eq!(
        text(with(&["f"]), "f(x)+h(y)"),
        json!(["+", ["apply", "f", "x"], ["*", "h", "y"]])
    );
    assert_eq!(
        text(with(&["f", "h"]), "f(x)+h(y)"),
        json!(["+", ["apply", "f", "x"], ["apply", "h", "y"]])
    );
    assert_eq!(
        text(with(&["f", "h", "x"]), "f(x)+h(y)"),
        json!(["+", ["apply", "f", "x"], ["apply", "h", "y"]])
    );
}

#[test]
fn text_applied_function_symbols() {
    let with = |af: &[&str]| TextToAstOptions {
        applied_function_symbols: strings(af),
        ..Default::default()
    };

    // with no applied functions, "sin" splits into letters
    let split_sin = json!([
        "+",
        ["*", "s", "i", "n", "x"],
        ["*", "c", "u", "s", "t", "o", "m", "y"]
    ]);
    assert_eq!(text(with(&[]), "sin(x) + custom(y)"), split_sin);
    assert_eq!(text(with(&[]), "sin x  + custom y"), split_sin);

    let custom_only = json!(["+", ["*", "s", "i", "n", "x"], ["apply", "custom", "y"]]);
    assert_eq!(text(with(&["custom"]), "sin(x) + custom(y)"), custom_only);
    assert_eq!(text(with(&["custom"]), "sin x  + custom y"), custom_only);

    let both = json!(["+", ["apply", "sin", "x"], ["apply", "custom", "y"]]);
    assert_eq!(text(with(&["custom", "sin"]), "sin(x) + custom(y)"), both);
    assert_eq!(text(with(&["custom", "sin"]), "sin x  + custom y"), both);
}

#[test]
fn text_allow_simplified_function_application() {
    assert_eq!(
        text(TextToAstOptions::default(), "sin x"),
        json!(["apply", "sin", "x"])
    );

    let disallow = TextToAstOptions {
        allow_simplified_function_application: false,
        ..Default::default()
    };
    let err = TextToAst::new(disallow).convert("sin x").unwrap_err();
    assert!(err.message.contains("Expecting ( after function"));

    let allow = TextToAstOptions {
        allow_simplified_function_application: true,
        ..Default::default()
    };
    assert_eq!(text(allow, "sin x"), json!(["apply", "sin", "x"]));
}

#[test]
fn text_parse_leibniz_notation() {
    let leibniz = json!(["derivative_leibniz", "y", ["tuple", "x"]]);
    assert_eq!(text(TextToAstOptions::default(), "dy/dx"), leibniz);

    let off = TextToAstOptions {
        parse_leibniz_notation: false,
        ..Default::default()
    };
    assert_eq!(
        text(off, "dy/dx"),
        json!(["*", ["/", ["*", "d", "y"], "d"], "x"])
    );

    let on = TextToAstOptions {
        parse_leibniz_notation: true,
        ..Default::default()
    };
    assert_eq!(text(on, "dy/dx"), leibniz);
}

#[test]
fn text_parse_scientific_notation() {
    let sci = json!(["+", ["*", 2, ["^", "E", 2]], -300]);
    assert_eq!(text(TextToAstOptions::default(), "2E^2-3E+2"), sci);

    let off = TextToAstOptions {
        parse_scientific_notation: false,
        ..Default::default()
    };
    assert_eq!(
        text(off, "2E^2-3E+2"),
        json!(["+", ["*", 2, ["^", "E", 2]], ["-", ["*", 3, "E"]], 2])
    );

    let on = TextToAstOptions {
        parse_scientific_notation: true,
        ..Default::default()
    };
    assert_eq!(text(on, "2E^2-3E+2"), sci);
}

#[test]
fn text_conditional_probability() {
    let p = || TextToAstOptions {
        function_symbols: strings(&["P"]),
        ..Default::default()
    };

    assert_eq!(text(p(), "P(A|B)"), json!(["apply", "P", ["|", "A", "B"]]));
    assert_eq!(text(p(), "P(A:B)"), json!(["apply", "P", [":", "A", "B"]]));
    assert_eq!(
        text(p(), "P(R=1|X>2)"),
        json!(["apply", "P", ["|", ["=", "R", 1], [">", "X", 2]]])
    );
    assert_eq!(
        text(p(), "P(R=1:X>2)"),
        json!(["apply", "P", [":", ["=", "R", 1], [">", "X", 2]]])
    );
    assert_eq!(
        text(p(), "P( A and B | C or D )"),
        json!(["apply", "P", ["|", ["and", "A", "B"], ["or", "C", "D"]]])
    );
    assert_eq!(
        text(p(), "P( A and B : C or D )"),
        json!(["apply", "P", [":", ["and", "A", "B"], ["or", "C", "D"]]])
    );
}

// ---- latex parser -------------------------------------------------------

#[test]
fn latex_function_symbols() {
    let with = |fs: &[&str]| LatexToAstOptions {
        function_symbols: strings(fs),
        ..Default::default()
    };

    assert_eq!(
        latex(with(&[]), "f(x)+h(y)"),
        json!(["+", ["*", "f", "x"], ["*", "h", "y"]])
    );
    assert_eq!(
        latex(with(&["f"]), "f(x)+h(y)"),
        json!(["+", ["apply", "f", "x"], ["*", "h", "y"]])
    );
    assert_eq!(
        latex(with(&["f", "h"]), "f(x)+h(y)"),
        json!(["+", ["apply", "f", "x"], ["apply", "h", "y"]])
    );
    assert_eq!(
        latex(with(&["f", "h", "x"]), "f(x)+h(y)"),
        json!(["+", ["apply", "f", "x"], ["apply", "h", "y"]])
    );
}

#[test]
fn latex_applied_function_symbols() {
    let with = |af: &[&str]| LatexToAstOptions {
        applied_function_symbols: strings(af),
        allowed_latex_symbols: strings(&["custom", "sin"]),
        ..Default::default()
    };

    let neither = json!(["+", ["*", "sin", "x"], ["*", "custom", "y"]]);
    assert_eq!(latex(with(&[]), "\\sin(x) + \\custom(y)"), neither);
    assert_eq!(latex(with(&[]), "\\sin x  + \\custom y"), neither);

    let custom_only = json!(["+", ["*", "sin", "x"], ["apply", "custom", "y"]]);
    assert_eq!(
        latex(with(&["custom"]), "\\sin(x) + \\custom(y)"),
        custom_only
    );
    assert_eq!(
        latex(with(&["custom"]), "\\sin x  + \\custom y"),
        custom_only
    );

    let both = json!(["+", ["apply", "sin", "x"], ["apply", "custom", "y"]]);
    assert_eq!(
        latex(with(&["custom", "sin"]), "\\sin(x) + \\custom(y)"),
        both
    );
    assert_eq!(
        latex(with(&["custom", "sin"]), "\\sin x  + \\custom y"),
        both
    );
}

#[test]
fn latex_allow_simplified_function_application() {
    assert_eq!(
        latex(LatexToAstOptions::default(), "\\sin x"),
        json!(["apply", "sin", "x"])
    );

    let disallow = LatexToAstOptions {
        allow_simplified_function_application: false,
        ..Default::default()
    };
    let err = LatexToAst::new(disallow).convert("\\sin x").unwrap_err();
    assert!(err.message.contains("Expecting ( after function"));

    let allow = LatexToAstOptions {
        allow_simplified_function_application: true,
        ..Default::default()
    };
    assert_eq!(latex(allow, "\\sin x"), json!(["apply", "sin", "x"]));
}

#[test]
fn latex_parse_leibniz_notation() {
    let leibniz = json!(["derivative_leibniz", "y", ["tuple", "x"]]);
    assert_eq!(
        latex(LatexToAstOptions::default(), "\\frac{dy}{dx}"),
        leibniz
    );

    let off = LatexToAstOptions {
        parse_leibniz_notation: false,
        ..Default::default()
    };
    assert_eq!(
        latex(off, "\\frac{dy}{dx}"),
        json!(["/", ["*", "d", "y"], ["*", "d", "x"]])
    );

    let on = LatexToAstOptions {
        parse_leibniz_notation: true,
        ..Default::default()
    };
    assert_eq!(latex(on, "\\frac{dy}{dx}"), leibniz);
}

#[test]
fn latex_parse_scientific_notation() {
    let sci = json!(["+", ["*", 2, ["^", "E", 2]], -300]);
    assert_eq!(latex(LatexToAstOptions::default(), "2E^2-3E+2"), sci);

    let off = LatexToAstOptions {
        parse_scientific_notation: false,
        ..Default::default()
    };
    assert_eq!(
        latex(off, "2E^2-3E+2"),
        json!(["+", ["*", 2, ["^", "E", 2]], ["-", ["*", 3, "E"]], 2])
    );

    let on = LatexToAstOptions {
        parse_scientific_notation: true,
        ..Default::default()
    };
    assert_eq!(latex(on, "2E^2-3E+2"), sci);
}

#[test]
fn latex_conditional_probability() {
    let p = || LatexToAstOptions {
        function_symbols: strings(&["P"]),
        ..Default::default()
    };

    assert_eq!(latex(p(), "P(A|B)"), json!(["apply", "P", ["|", "A", "B"]]));
    assert_eq!(latex(p(), "P(A:B)"), json!(["apply", "P", [":", "A", "B"]]));
    assert_eq!(
        latex(p(), "P(R=1|X>2)"),
        json!(["apply", "P", ["|", ["=", "R", 1], [">", "X", 2]]])
    );
    assert_eq!(
        latex(p(), "P(R=1:X>2)"),
        json!(["apply", "P", [":", ["=", "R", 1], [">", "X", 2]]])
    );
    assert_eq!(
        latex(p(), "P( A \\land B | C \\lor D )"),
        json!(["apply", "P", ["|", ["and", "A", "B"], ["or", "C", "D"]]])
    );
    assert_eq!(
        latex(p(), "P( A \\land B : C \\lor D )"),
        json!(["apply", "P", [":", ["and", "A", "B"], ["or", "C", "D"]]])
    );
}
