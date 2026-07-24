//! I18N_MATH_NOTATION_PLAN conformance suite.
//!
//! Drives the language-neutral fixture (`fixtures/notation/phase1.json`, the
//! cross-implementation handoff — JS math-expressions must pass the same rows)
//! and checks the two structural laws:
//!   * round-trip (A6): `parse(print(ast, N), N) == ast`;
//!   * notation independence: `parse(t, comma) == parse(equiv_t, period)`.

use math_expressions::{
    js_tree, to_latex, to_text, Expr, LatexOpts, LatexToAst, LatexToAstOptions, NumberNotation,
    ParseError, TextOpts, TextToAst, TextToAstOptions,
};
use serde_json::Value;

#[derive(serde::Deserialize)]
struct Fixture {
    version: u32,
    cases: Vec<Case>,
    failing: Vec<Failing>,
}

#[derive(serde::Deserialize)]
struct NotationSpec {
    #[serde(rename = "decimalSeparator")]
    decimal: String,
    #[serde(rename = "argumentSeparator")]
    argument: String,
    #[serde(rename = "alsoAcceptDecimal", default)]
    also: Option<Vec<String>>,
}

#[derive(serde::Deserialize)]
struct Case {
    name: String,
    notation: NotationSpec,
    #[serde(default)]
    input_text: Option<String>,
    #[serde(default)]
    input_latex: Option<String>,
    expected_ast: Value,
    #[serde(default)]
    expected_text_out: Option<String>,
    #[serde(default)]
    expected_latex_out: Option<String>,
}

#[derive(serde::Deserialize)]
struct Failing {
    notation: NotationSpec,
    #[serde(default)]
    input_text: Option<String>,
    #[serde(default)]
    input_latex: Option<String>,
}

fn one_char(s: &str) -> char {
    s.chars().next().expect("separator must be one char")
}

fn notation(n: &NotationSpec) -> NumberNotation {
    NumberNotation {
        decimal_separator: one_char(&n.decimal),
        argument_separator: one_char(&n.argument),
        also_accept_decimal: n
            .also
            .as_ref()
            .map(|v| v.iter().map(|s| one_char(s)).collect()),
        ..NumberNotation::default()
    }
}

fn parse_text(nt: &NumberNotation, s: &str) -> Result<Expr, ParseError> {
    TextToAst::new(TextToAstOptions {
        notation: nt.clone(),
        ..Default::default()
    })
    .convert(s)
}

fn parse_latex(nt: &NumberNotation, s: &str) -> Result<Expr, ParseError> {
    LatexToAst::new(LatexToAstOptions {
        notation: nt.clone(),
        ..Default::default()
    })
    .convert(s)
}

fn text_out(nt: &NumberNotation, e: &Expr) -> String {
    to_text(
        e,
        &TextOpts {
            notation: nt.clone(),
            ..Default::default()
        },
    )
}

fn latex_out(nt: &NumberNotation, e: &Expr) -> String {
    to_latex(
        e,
        &LatexOpts {
            notation: nt.clone(),
        },
    )
}

const FIXTURE: &str = include_str!("fixtures/notation/phase1.json");

fn fixture() -> Fixture {
    serde_json::from_str(FIXTURE).expect("fixture parses")
}

#[test]
fn conformance_cases() {
    let fx = fixture();
    assert_eq!(fx.version, 1, "fixture schema version");
    for c in &fx.cases {
        let nt = notation(&c.notation);
        if let Some(t) = &c.input_text {
            let e = parse_text(&nt, t)
                .unwrap_or_else(|err| panic!("{}: text parse {t:?}: {err:?}", c.name));
            assert_eq!(js_tree::to_js(&e), c.expected_ast, "{}: text AST", c.name);
            if let Some(exp) = &c.expected_text_out {
                assert_eq!(&text_out(&nt, &e), exp, "{}: text output", c.name);
            }
        }
        if let Some(l) = &c.input_latex {
            let e = parse_latex(&nt, l)
                .unwrap_or_else(|err| panic!("{}: latex parse {l:?}: {err:?}", c.name));
            assert_eq!(js_tree::to_js(&e), c.expected_ast, "{}: latex AST", c.name);
            if let Some(exp) = &c.expected_latex_out {
                assert_eq!(&latex_out(&nt, &e), exp, "{}: latex output", c.name);
            }
        }
    }
}

#[test]
fn failing_rows_must_error() {
    let fx = fixture();
    for f in &fx.failing {
        let nt = notation(&f.notation);
        if let Some(t) = &f.input_text {
            assert!(
                parse_text(&nt, t).is_err(),
                "expected a text parse error for {t:?} under {nt:?}"
            );
        }
        if let Some(l) = &f.input_latex {
            assert!(
                parse_latex(&nt, l).is_err(),
                "expected a latex parse error for {l:?} under {nt:?}"
            );
        }
    }
}

#[test]
fn round_trip_law() {
    // A6: parse(print(ast, N), N) == ast, for both text and latex.
    let fx = fixture();
    for c in &fx.cases {
        let nt = notation(&c.notation);
        if let Some(t) = &c.input_text {
            let e = parse_text(&nt, t).unwrap();
            let printed = text_out(&nt, &e);
            let back = parse_text(&nt, &printed)
                .unwrap_or_else(|err| panic!("{}: reparse text {printed:?}: {err:?}", c.name));
            assert_eq!(
                js_tree::to_js(&e),
                js_tree::to_js(&back),
                "{}: text round-trip via {printed:?}",
                c.name
            );
        }
        if let Some(l) = &c.input_latex {
            let e = parse_latex(&nt, l).unwrap();
            let printed = latex_out(&nt, &e);
            let back = parse_latex(&nt, &printed)
                .unwrap_or_else(|err| panic!("{}: reparse latex {printed:?}: {err:?}", c.name));
            assert_eq!(
                js_tree::to_js(&e),
                js_tree::to_js(&back),
                "{}: latex round-trip via {printed:?}",
                c.name
            );
        }
    }
}

#[test]
fn notation_independence() {
    // parse(t, comma) == parse(equiv_t, period) for equivalent inputs — the
    // AST is notation-independent (stored as exact rationals, A6).
    let comma = NumberNotation::comma();
    let period = NumberNotation::period();
    let pairs = [
        ("3,14", "3.14"),
        ("f(x;y)", "f(x,y)"),
        ("(1;2)", "(1,2)"),
        ("1;2;3", "1,2,3"),
        ("1,5;2,5", "1.5,2.5"),
        ("sin(x) + 2,5", "sin(x) + 2.5"),
        ("2,5*x^2 + 1,25", "2.5*x^2 + 1.25"),
        // multi-argument function with decimal args exercises the args-join and
        // decimal-scan paths simultaneously
        ("f(1,5; 2,5)", "f(1.5, 2.5)"),
    ];
    for (c, p) in pairs {
        let ec = parse_text(&comma, c).unwrap();
        let ep = parse_text(&period, p).unwrap();
        assert_eq!(
            js_tree::to_js(&ec),
            js_tree::to_js(&ep),
            "independence: {c:?} (comma) vs {p:?} (period)"
        );
    }
}

#[test]
fn a2_defaults_are_period_notation() {
    // The default notation must behave exactly like period notation, so all
    // pre-i18n behavior is preserved.
    assert_eq!(NumberNotation::default(), NumberNotation::period());
    let def = NumberNotation::default();
    let e = parse_text(&def, "f(1.5, 2)").unwrap();
    assert_eq!(js_tree::to_js(&e), serde_json::json!(["apply", "f", ["tuple", 1.5, 2]]));
    assert_eq!(text_out(&def, &e), "f(1.5, 2)");
}

#[test]
fn decimal_pairs_autofill_argument_separator() {
    // The two special-cased pairs: from the decimal alone.
    assert_eq!(NumberNotation::paired_argument_separator('.'), Some(','));
    assert_eq!(NumberNotation::paired_argument_separator(','), Some(';'));
    assert_eq!(NumberNotation::paired_argument_separator('\u{066B}'), None); // Arabic ٫

    assert_eq!(NumberNotation::from_decimal('.'), NumberNotation::period());
    assert_eq!(NumberNotation::from_decimal(','), NumberNotation::comma());
    // from_decimal produces a coherent (validatable) notation for the pairs.
    assert!(NumberNotation::from_decimal(',').validate().is_ok());
    assert_eq!(
        NumberNotation::from_decimal(',').argument_separator,
        ';'
    );
}

#[test]
fn validate_rejects_incoherent_notations() {
    // The dangerous partial spec: decimal set to ',' but argument left at the
    // default ',' — both become ',' and parsing is ambiguous.
    let collide = NumberNotation {
        decimal_separator: ',',
        ..NumberNotation::default()
    };
    assert!(collide.validate().is_err(), "decimal == argument must be rejected");

    // Coherent notations pass, including a valid partial spec (only the
    // argument separator changed; decimal stays the default '.').
    assert!(NumberNotation::period().validate().is_ok());
    assert!(NumberNotation::comma().validate().is_ok());
    assert!(NumberNotation {
        argument_separator: ';',
        ..NumberNotation::default()
    }
    .validate()
    .is_ok());

    // Digit / letter separators would break number scanning.
    assert!(NumberNotation {
        decimal_separator: '5',
        argument_separator: ';',
        ..NumberNotation::default()
    }
    .validate()
    .is_err());

    // Leniency set may not reclaim the argument separator as a decimal.
    let mut lenient = NumberNotation::comma();
    lenient.also_accept_decimal = Some(vec![';']);
    assert!(lenient.validate().is_err());

    // Group separator must be distinct too.
    let mut grouped = NumberNotation::comma();
    grouped.group_separator = Some(';');
    assert!(grouped.validate().is_err());

    // Separators colliding with operator/bracket glyphs the lexer already
    // uses must be rejected (they'd silently shadow those tokens).
    for bad_arg in [':', '|', '(', ')', '[', '{', '+', '\\'] {
        let n = NumberNotation {
            argument_separator: bad_arg,
            ..NumberNotation::default()
        };
        assert!(n.validate().is_err(), "argument {bad_arg:?} must be rejected");
    }
    // decimal '-' would lex `1-2` as the number 1.2.
    let n = NumberNotation {
        decimal_separator: '-',
        argument_separator: ';',
        ..NumberNotation::default()
    };
    assert!(n.validate().is_err(), "decimal '-' must be rejected");
    // '.' as ARGUMENT separator conflicts with `...` and leading decimals.
    let n = NumberNotation {
        decimal_separator: ',',
        argument_separator: '.',
        ..NumberNotation::default()
    };
    assert!(n.validate().is_err(), "argument '.' must be rejected");
}

#[test]
fn scientific_notation_uses_argument_separator_delimiter() {
    // The post-exponent delimiter follows the argument separator: under comma
    // notation a list `1E2;3` closes the exponent at ';'.
    let comma = NumberNotation::comma();
    let e = parse_text(&comma, "1E2;3").unwrap();
    assert_eq!(js_tree::to_js(&e), serde_json::json!(["list", 100, 3]));
}
