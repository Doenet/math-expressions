//! Grading helpers: sign-error equality, solve_linear, set membership, and
//! generic assumptions. Expected values verified against the JS reference.

use math_expressions::{
    equal_specified_sign_errors, equal_with_sign_errors, equals, evaluate_membership,
    is_positive, is_real, solve_linear, Assumptions, EqOptions, Expr, RelOp, TextToAst,
    TextToAstOptions,
};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn o() -> EqOptions {
    EqOptions::default()
}

#[test]
fn sign_errors() {
    // JS-oracle verdicts.
    assert!(equal_specified_sign_errors(&parse("x+y"), &parse("x-y"), &o(), 1));
    assert!(equal_specified_sign_errors(&parse("x+y"), &parse("-x-y"), &o(), 1)); // root flip
    assert!(!equal_specified_sign_errors(&parse("x+y+z"), &parse("x-y-z"), &o(), 1));
    assert!(equal_specified_sign_errors(&parse("x+y+z"), &parse("x-y-z"), &o(), 2));
    assert!(!equal_specified_sign_errors(&parse("x+y"), &parse("x+y"), &o(), 1)); // exact ≠ 1 error

    assert_eq!(equal_with_sign_errors(&parse("x+y"), &parse("x-y"), &o(), 2), Some(1));
    assert_eq!(equal_with_sign_errors(&parse("x+y"), &parse("x+y"), &o(), 2), Some(0));
    assert_eq!(equal_with_sign_errors(&parse("x+y"), &parse("x*y"), &o(), 2), None);
}

#[test]
fn solve_linear_cases() {
    let a = Assumptions::new();
    // JS: ["=","x",-2]
    let sol = solve_linear(&parse("2*x+4=0"), "x", &a).unwrap();
    assert!(equals(&sol, &parse("x = -2"), &o()));
    // JS: x = (3-y)/2
    let sol = solve_linear(&parse("2*x+y=3"), "x", &a).unwrap();
    assert!(equals(&sol, &parse("x = (3-y)/2"), &o()));
    // Inequality with negative coefficient flips: 3-x<5 → x > -2.
    let sol = solve_linear(&parse("3-x<5"), "x", &a).unwrap();
    let Expr::Relation { ops, .. } = &sol else { panic!() };
    assert_eq!(ops, &vec![RelOp::Gt]);
    assert!(equals(&sol, &parse("x > -2"), &o()));
    // Nonlinear → None; symbolic coefficient needs a nonzero assumption.
    assert!(solve_linear(&parse("x^2=4"), "x", &a).is_none());
    assert!(solve_linear(&parse("a*x+b=0"), "x", &a).is_none());
    let mut w = Assumptions::new();
    w.add(&parse("a != 0"));
    let sol = solve_linear(&parse("a*x+b=0"), "x", &w).unwrap();
    assert!(equals(&sol, &parse("x = -b/a"), &o()));
}

#[test]
fn membership() {
    // 3 ∈ {1,2,3} — the DoenetML #1504 case (∋ folds to ∈ in canonical form).
    assert_eq!(evaluate_membership(&parse("3 elementof {1,2,3}"), &o()), Some(true));
    assert_eq!(evaluate_membership(&parse("4 elementof {1,2,3}"), &o()), Some(false));
    assert_eq!(evaluate_membership(&parse("{1,2,3} containselement 3"), &o()), Some(true));
    assert_eq!(evaluate_membership(&parse("4 notelementof {1,2,3}"), &o()), Some(true));
    // Value-level equality, not syntax: 2/2 ∈ {1}.
    assert_eq!(evaluate_membership(&parse("2/2 elementof {1}"), &o()), Some(true));
    // Symbolic non-match is indeterminate; symbolic match is definite.
    assert_eq!(evaluate_membership(&parse("x elementof {1,2}"), &o()), None);
    assert_eq!(evaluate_membership(&parse("x elementof {x, y}"), &o()), Some(true));
    // Not a membership relation.
    assert_eq!(evaluate_membership(&parse("x = 2"), &o()), None);
}

#[test]
fn generic_assumptions() {
    let mut a = Assumptions::new();
    a.add_generic(&parse("x > 0"));
    // Applies to any variable without specific facts…
    assert_eq!(is_positive(&parse("q"), &a), Some(true));
    assert_eq!(is_real(&parse("q + z"), &a), Some(true));
    // …but specific facts win.
    a.add(&parse("y < 0"));
    assert_eq!(is_positive(&parse("y"), &a), Some(false));
    // Removal restores unknowns.
    a.remove_generic(&parse("x > 0"));
    assert_eq!(is_positive(&parse("q"), &a), None);
}
