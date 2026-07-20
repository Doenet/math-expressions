//! DoenetML-adjacent utilities + assumption-aware simplify. Expected values
//! verified against the JS reference (probed).

use math_expressions::{
    equals_syntactic, get_component, simplify_with, strings_to_subscripts, subscripts_to_strings,
    substitute_component, to_intervals, to_text, Assumptions, EqOptions, Expr, TextToAst,
    TextToAstOptions,
};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn txt(e: &Expr) -> String {
    to_text(e, &Default::default())
}

#[test]
fn simplify_under_assumptions() {
    // JS oracle: sqrt of even powers resolves by sign; abs never rewrites.
    let w = |assume: &str| {
        let mut a = Assumptions::new();
        a.add(&parse(assume));
        a
    };
    let s = |e: &str, a: &Assumptions| txt(&simplify_with(&parse(e), a));
    assert_eq!(s("sqrt(x^2)", &w("x>0")), "x");
    // Deliberate divergence from JS (which stops at |x|): a known sign
    // resolves the abs entirely.
    assert_eq!(s("sqrt(x^2)", &w("x<0")), "-x");
    assert_eq!(s("sqrt(x^2)", &w("x elementof R")), "|x|"); // sign unknown
    assert_eq!(s("sqrt(x^2*y^2)", &w("x>0 and y>0")), "x y");
    assert_eq!(s("sqrt(x^4)", &w("x>0")), "x^2");
    assert_eq!(s("abs(x)", &w("x>0")), "x");
    assert_eq!(s("abs(x)", &w("x<=0")), "-x");
    assert_eq!(s("abs(x)", &w("x != 0")), "|x|"); // nonzero alone: sign unknown
    // No assumptions → unchanged.
    assert_eq!(
        txt(&math_expressions::simplify(&parse("sqrt(x^2)"))),
        "sqrt(x^2)"
    );
}

#[test]
fn components() {
    let t = parse("(a, b, c)");
    assert_eq!(txt(&get_component(&t, 0).unwrap()), "a");
    assert_eq!(txt(&get_component(&t, 1).unwrap()), "b");
    assert!(get_component(&t, 3).is_none());
    assert!(get_component(&parse("x+1"), 0).is_none());
    let replaced = substitute_component(&t, 1, &parse("z")).unwrap();
    assert!(equals_syntactic(&replaced, &parse("(a, z, c)"), &EqOptions::default()));
}

#[test]
fn subscript_string_round_trip() {
    // x_1 + y_a → flat symbols → back to subscripts (numeric index restored
    // as a number).
    let e = parse("x_1 + y_a");
    let flat = subscripts_to_strings(&e);
    assert_eq!(txt(&flat), "x_1 + y_a"); // renders the same, but as flat syms
    let back = strings_to_subscripts(&flat);
    assert_eq!(back, e);
}

#[test]
fn intervals_from_seqs() {
    // (1,2) → open, [1,2] → closed, recursing through unions.
    let open = to_intervals(&parse("(1,2)"));
    assert!(matches!(&open, Expr::Interval { closed, .. } if *closed == (false, false)));
    let closed = to_intervals(&parse("[1,2]"));
    assert!(matches!(&closed, Expr::Interval { closed, .. } if *closed == (true, true)));
    let union = to_intervals(&parse("(1,2) union [3,4]"));
    let Expr::Union(parts) = &union else {
        panic!("expected union, got {union:?}")
    };
    assert!(parts.iter().all(|p| matches!(p, Expr::Interval { .. })));
    // Non-interval shapes untouched.
    assert_eq!(to_intervals(&parse("x+1")), parse("x+1"));
    assert_eq!(to_intervals(&parse("(1,2,3)")), parse("(1,2,3)"));
}
