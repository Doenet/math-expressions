//! `default_order`: sort without evaluating (DoenetML `simplify="normalizeOrder"`).
//!
//! The contract is narrow and easy to break by "improving" it: every term the
//! author wrote must survive, spelled as written, and only the *arrangement*
//! may change. A pass that folded `7+4` or dropped `0x²` would look tidier and
//! would silently mark a wrong answer correct.

use math_expressions::{
    default_order, equals_syntactic, EqOptions, Expr, TextToAst, TextToAstOptions,
};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn ordered(s: &str) -> Expr {
    default_order(&parse(s))
}

fn same_form(a: &str, b: &str) -> bool {
    equals_syntactic(&ordered(a), &ordered(b), &EqOptions::default())
}

#[test]
fn the_same_terms_in_any_order_agree() {
    // The DoenetML grading case: one sum, written two ways, with the factors
    // inside a term also swapped (`(x^2)1` against `1x^2`).
    assert!(same_form(
        "1x^2+2-0x^2+3+x^2+3x^2+7+4",
        "4-0x^2 +7+ (x^2)1+3+x^2+2+(x^2)3"
    ));
}

#[test]
fn nothing_is_evaluated_away() {
    // Each of these differs from the target by *something a student did*, and
    // all three must stay distinguishable. This is the whole reason the pass
    // exists rather than reusing `simplify`.
    let target = "1x^2+2-0x^2+3+x^2+3x^2+7+4";
    assert!(!same_form(target, "1x^2+2-0x^2+3+x^2+3x^2+11")); // folded 7+4
    assert!(!same_form(target, "1x^2+2+3+x^2+3x^2+7+4")); // dropped 0x^2
    assert!(!same_form(target, "x^2+2-0x^2+3+x^2+3x^2+7+4")); // dropped the 1
}

#[test]
fn a_negated_literal_and_a_negative_literal_are_one_thing() {
    // `x-1` parses as `["+", x, -1]`; the same expression assembled with the 1
    // substituted in from elsewhere arrives as `["+", x, ["-", 1]]`. They are
    // the same expression and must sort to the same place — otherwise a
    // document that builds its answer from a `<math>` child stops matching the
    // student's typed form.
    let typed = ordered("(x-1)(x+1)");
    let assembled = default_order(&Expr::Mul(vec![
        Expr::Add(vec![
            Expr::sym("x"),
            Expr::Neg(Box::new(Expr::Num(math_expressions::Number::Int(1)))),
        ]),
        Expr::Add(vec![
            Expr::sym("x"),
            Expr::Num(math_expressions::Number::Int(1)),
        ]),
    ]));
    assert!(equals_syntactic(&typed, &assembled, &EqOptions::default()));
}

#[test]
fn comparisons_all_point_one_way() {
    // `>` and `≥` are turned around, so the two spellings of one inequality
    // reach the same tree.
    assert!(same_form("x > 3", "3 < x"));
    assert!(same_form("x >= 3", "3 <= x"));
    // `=` sorts its sides, so an equation matches either way round.
    assert!(same_form("x = 3", "3 = x"));
}

#[test]
fn a_unit_sorts_with_the_value_it_annotates() {
    // JS keys a `["unit", …]` node as its *value's* key with the unit appended
    // to the kind string — `5%` is `[0,"number_%",5]`, `$x` is
    // `[1,"symbol_$","x"]` — so a unit-annotated term sorts among the numbers
    // or the symbols. Keying it as an ordinary operator instead sent it to the
    // `[10, …]` catch-all and put it *last*, reversing these sums.
    //
    // Pinned against the JS library, which returns the unit term first for all
    // three: e.g. `["+",["apply","sqrt","y"],["unit","$","x"]]` sorts to
    // `["+",["unit","$","x"],["apply","sqrt","y"]]`.
    for (written, reversed) in [
        ("$x + sqrt(y)", "sqrt(y) + $x"),
        ("5% + sqrt(y)", "sqrt(y) + 5%"),
        ("30deg + sqrt(y)", "sqrt(y) + 30deg"),
    ] {
        assert!(same_form(written, reversed), "{written} vs {reversed}");
        // The unit term leads, as it does in the JS.
        let Expr::Add(terms) = ordered(reversed) else {
            panic!("expected a sum: {reversed}");
        };
        assert!(
            matches!(&terms[0], Expr::OtherOp(s, _) if s.name() == "unit"),
            "the unit term must sort first: {reversed} -> {:?}",
            terms
        );
    }
}

#[test]
fn idempotent() {
    // Sorting a sorted tree changes nothing — the property every normalizer
    // needs, and the one a comparison-based sort loses if its key is not a
    // total order.
    for s in [
        "1x^2+2-0x^2+3+x^2+3x^2+7+4",
        "(x-1)(x+1)",
        "sin(y)+cos(x)+2",
        "x > 3",
        "a and b and c",
        "$x + sqrt(y)",
        "5% + 3 + sqrt(y)",
    ] {
        let once = ordered(s);
        let twice = default_order(&once);
        assert_eq!(once, twice, "not idempotent: {s}");
    }
}
