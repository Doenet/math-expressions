//! `substitute` and `variables` (PORTING_PLAN.md §15). Expected values verified
//! against `me.substitute` / `me.variables`.

use math_expressions::{equals, substitute, variables, EqOptions, Expr, TextToAst, TextToAstOptions};
use std::collections::HashMap;

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn sub(input: &str, map: &[(&str, &str)]) -> Expr {
    let m: HashMap<String, Expr> = map.iter().map(|(k, v)| (k.to_string(), parse(v))).collect();
    substitute(&parse(input), &m)
}

fn eq(a: &Expr, b: &str) -> bool {
    equals(a, &parse(b), &EqOptions::default())
}

#[test]
fn substitute_basic() {
    assert!(eq(&sub("x^2", &[("x", "y+1")]), "(y+1)^2"));
    assert!(eq(&sub("sin(x)", &[("x", "a+b")]), "sin(a+b)"));
    assert!(eq(&sub("n^2", &[("n", "3")]), "3^2"));
    // Multi-char and unaffected variables.
    assert!(eq(&sub("x*y + z", &[("x", "1"), ("y", "2")]), "1*2 + z"));
}

#[test]
fn substitute_is_simultaneous() {
    // {x: y, y: x} swaps rather than collapsing both to one.
    assert!(eq(&sub("x - y", &[("x", "y"), ("y", "x")]), "y - x"));
    // A replacement is not itself re-substituted.
    assert!(eq(&sub("x", &[("x", "x+1")]), "x+1"));
}

#[test]
fn substitute_does_not_simplify() {
    // `x^2 + x` with x → 2 yields `2^2 + 2`, structurally (not folded to 6).
    let got = sub("x^2 + x", &[("x", "2")]);
    let text = math_expressions::to_text(&got, &Default::default());
    assert!(text.contains("2^2"), "expected unsimplified 2^2, got {text:?}");
    // ...but it is still numerically 6.
    assert!(eq(&got, "6"));
}

#[test]
fn variables_order_and_membership() {
    assert_eq!(variables(&parse("x^2 + y*z")), vec!["x", "y", "z"]);
    assert_eq!(variables(&parse("2*a*b + f(x)")), vec!["a", "b", "x"]); // f excluded
    // pi/e/i count as variables (they are ordinary symbols here).
    assert_eq!(variables(&parse("sin(x) + pi")), vec!["x", "pi"]);
    assert_eq!(variables(&parse("pi + e + x")), vec!["pi", "e", "x"]);
    // First-appearance order, de-duplicated.
    assert_eq!(variables(&parse("y + x + y + x")), vec!["y", "x"]);
    assert_eq!(variables(&parse("5")), Vec::<String>::new());
}

#[test]
fn variables_skip_compound_apply_heads() {
    // JS drops the apply head wholesale (tree.slice(2)) — even compound heads.
    // The derivative of an unknown f produces Apply(Prime(f), [x]): its
    // variables are [x], never [f, x].
    use math_expressions::derivative;
    let d = derivative(&parse("f(x)"), "x");
    assert_eq!(variables(&d), vec!["x"]);
    // sin^2(x) in the faithful layer has head Pow(sin, 2): still just [x].
    assert_eq!(variables(&parse("sin^2(x)")), vec!["x"]);
}

#[test]
fn functions_and_operators() {
    use math_expressions::{functions, operators};
    // JS-oracle values (probed): first-appearance order, de-duplicated.
    assert_eq!(functions(&parse("sin(x)+f(y)*g(x)")), vec!["sin", "f", "g"]);
    assert_eq!(functions(&parse("f(x)+f(y)")), vec!["f"]);
    assert_eq!(functions(&parse("x+y*z")), Vec::<String>::new());
    assert_eq!(functions(&parse("sin(x)^2+cos(x)")), vec!["sin", "cos"]);
    assert_eq!(operators(&parse("sin(x)+f(y)*g(x)")), vec!["+", "*"]);
    assert_eq!(operators(&parse("x/y - z^2")), vec!["+", "/", "-", "^"]);
}

#[test]
fn evaluate_numbers_folds_exactly() {
    use math_expressions::{canonicalize, evaluate_numbers};
    // Exact fold, returned in display form (canonically the same tree).
    let e = parse("4 + x - 2");
    assert_eq!(canonicalize(&evaluate_numbers(&e)), canonicalize(&parse("x + 2")));
    // §3a payoff: the fold is exact AND still renders as a decimal, not 3/10.
    let e = parse("0.1 + 0.2");
    assert_eq!(evaluate_numbers(&e), canonicalize(&parse("0.3")));
}
