//! The parsers must only build trees the JS AST can express.
//!
//! `to_js` is the contract with every consumer: DoenetML stores `.tree`, and
//! `me.fromAst` reads it back through `try_from_js`. Any distinction the Rust
//! tree carries that the JSON cannot is therefore lost at the first save, so
//! an expression stops being equal to itself — and stops *printing* the same —
//! across a round trip through its own serialization.
//!
//! The shape that broke this was a lone tuple argument: `f((x, y))` parsed to
//! `Apply(f, [Seq(Tuple, [x, y])])` and `f(x, y)` to `Apply(f, [x, y])`, and
//! `to_js` writes `["apply", "f", ["tuple", "x", "y"]]` for both. See
//! `parse::common::apply`.

use math_expressions::expr::serde::{to_js, try_from_js};
use math_expressions::print::{latex::LatexOpts, text::TextOpts, to_latex, to_text};
use math_expressions::{Expr, LatexToAst, LatexToAstOptions, TextToAst, TextToAstOptions};

fn text(e: &Expr) -> String {
    to_text(e, &TextOpts::default())
}

fn latex(e: &Expr) -> String {
    to_latex(e, &LatexOpts::default())
}

fn parse_text(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn parse_latex(s: &str) -> Expr {
    LatexToAst::new(LatexToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse latex {s:?}: {e}"))
}

/// `try_from_js(to_js(e))` must return `e` unchanged.
fn assert_survives_js(e: &Expr, what: &str) {
    let js = to_js(e);
    let back = try_from_js(&js).unwrap_or_else(|err| panic!("{what}: try_from_js: {err}"));
    assert_eq!(
        &back, e,
        "{what}: tree changed across to_js/try_from_js (json {js})"
    );
}

/// The weaker property that must hold for *every* expression: what the
/// consumer sees is a function of the JSON it stored, so re-reading a saved
/// tree must not change how it renders. (The tree itself may legitimately
/// change shape — `to_js` flattens a nested `Add`/`Mul`, and `try_from_js`
/// returns the flat form, which prints identically.)
fn assert_renders_the_same_after_js(e: &Expr, what: &str) {
    let js = to_js(e);
    let back = try_from_js(&js).unwrap_or_else(|err| panic!("{what}: try_from_js: {err}"));
    assert_eq!(
        text(&back),
        text(e),
        "{what}: text rendering changed across a save/reload (json {js})"
    );
    assert_eq!(
        latex(&back),
        latex(e),
        "{what}: latex rendering changed across a save/reload (json {js})"
    );
}

#[derive(serde::Deserialize)]
struct TreeCase {
    input: String,
}

fn fixture_inputs(files: &[&str]) -> Vec<String> {
    files
        .iter()
        .flat_map(|f| {
            serde_json::from_str::<Vec<TreeCase>>(f)
                .unwrap()
                .into_iter()
                .map(|c| c.input)
        })
        .collect()
}

/// A lone parenthesized list as the argument of an application is the
/// multi-argument spelling, in both parsers and in every notation that builds
/// an `Apply`.
#[test]
fn lone_tuple_argument_is_the_argument_list() {
    // (text spelling, latex spelling)
    let pairs = [
        ("sin((x,y))", "sin(x,y)"),
        ("f((x,y))", "f(x,y)"),
        ("floor((x,y))", "floor(x,y)"),
        ("nPr((n,k))", "nPr(n,k)"),
        ("mod((7,3))", "mod(7,3)"),
        ("f((x,y,z))", "f(x,y,z)"),
        // the head need not be a bare symbol
        ("f_1((x,y))", "f_1(x,y)"),
        ("f'((x,y))", "f'(x,y)"),
    ];
    for (nested, flat) in pairs {
        let a = parse_text(nested);
        let b = parse_text(flat);
        assert_eq!(a, b, "text: {nested} vs {flat}");
        assert_survives_js(&a, nested);
        assert_eq!(text(&a), text(&b), "text rendering: {nested} vs {flat}");

        let a = parse_latex(nested);
        let b = parse_latex(flat);
        assert_eq!(a, b, "latex: {nested} vs {flat}");
        assert_survives_js(&a, nested);
        assert_eq!(latex(&a), latex(&b), "latex rendering: {nested} vs {flat}");
    }
}

/// The other notations that build a one-argument `Apply` around a group the
/// user can fill with a comma-separated list.
#[test]
fn lone_tuple_argument_in_the_other_apply_notations() {
    // `|(x,y)|` and `(x,y)!` in both parsers; the LaTeX-only bracket forms.
    for s in ["|(x,y)|", "(x,y)!"] {
        assert_survives_js(&parse_text(s), s);
        assert_survives_js(&parse_latex(s), s);
    }
    for s in [
        r"\lfloor (x,y) \rfloor",
        r"\lceil (x,y) \rceil",
        r"\sqrt{(x,y)}",
        r"\sin\left(\left(x,y\right)\right)",
    ] {
        assert_survives_js(&parse_latex(s), s);
    }
    // `abs((x,y))` and `|(x,y)|` are the same application either way.
    assert_eq!(parse_text("|(x,y)|"), parse_text("abs(x,y)"));
    assert_eq!(
        parse_latex(r"\lfloor (x,y) \rfloor"),
        parse_text("floor(x,y)")
    );
}

/// An inner tuple that is *not* the whole argument list survives the JS round
/// trip on its own, so parsing must leave it alone.
#[test]
fn a_tuple_among_several_arguments_is_preserved() {
    let e = parse_text("f((x,y),z)");
    assert_survives_js(&e, "f((x,y),z)");
    assert_ne!(e, parse_text("f(x,y,z)"));
    // ...and a tuple that is not an argument at all is untouched.
    let e = parse_text("(x,y)");
    assert_survives_js(&e, "(x,y)");
    assert_eq!(text(&e), "(x, y)");
}

/// The fixture files are machine-generated ("do not edit by hand"), and none
/// of them happens to spell a lone tuple argument, so the corpus sweep below
/// would pass with the fix reverted. These carry the defect into it.
const HAND_WRITTEN: &[&str] = &[
    "sin((x,y))",
    "floor((x,y))",
    "|(x,y)|",
    "(x,y)!",
    "f((x,y))",
    "f((x,y),z)",
];

/// Over the parser fixture corpus, rendering is a function of the saved JSON.
/// This is the property the `f((a, b))` defect violated — the same stored tree
/// rendered `\sin\left(\left( x, y \right)\right)` before a save/restore and
/// `\sin\left( x, y \right)` after — and running it over every realistic input
/// we have keeps any other shape from acquiring it.
#[test]
fn the_corpus_renders_the_same_after_a_save_and_reload() {
    let hand = || HAND_WRITTEN.iter().map(|s| s.to_string());
    for input in fixture_inputs(&[
        include_str!("fixtures/text-to-ast.json"),
        include_str!("fixtures/text-to-ast-edge.json"),
    ])
    .into_iter()
    .chain(hand())
    {
        assert_renders_the_same_after_js(&parse_text(&input), &input);
    }
    for input in fixture_inputs(&[
        include_str!("fixtures/latex-to-ast.json"),
        include_str!("fixtures/latex-to-ast-edge.json"),
    ])
    .into_iter()
    .chain(hand())
    {
        assert_renders_the_same_after_js(&parse_latex(&input), &input);
    }
}
