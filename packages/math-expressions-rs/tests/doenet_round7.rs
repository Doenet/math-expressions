//! The round-7 DoenetML items: scaling-unit folding, mixed-container vector
//! addition, and the indeterminate-form soundness fix.
//!
//! Each `assert` here is a row of an expected-behaviour table DoenetML filed,
//! including the neighbouring rows that must *not* change — those are the
//! point of the test, not padding.

use math_expressions::{expr, simplify, Expr, LatexToAst, TextToAst};

fn t(s: &str) -> Expr {
    TextToAst::new(Default::default()).convert(s).unwrap()
}

fn l(s: &str) -> Expr {
    LatexToAst::new(Default::default()).convert(s).unwrap()
}

fn js(e: &Expr) -> String {
    expr::serde::to_js(e).to_string()
}

fn simplified(s: &str) -> String {
    js(&simplify(&t(s)))
}

// ---- item 13: scaling units --------------------------------------------

#[test]
fn like_units_combine_and_scalars_move_inside() {
    assert_eq!(simplified("$3+$2"), r#"["unit","$",5]"#);
    assert_eq!(simplified("50% * 5"), r#"["unit",250,"%"]"#);
    assert_eq!(simplified("$12 / 4"), r#"["unit","$",3]"#);
    assert_eq!(simplified("360 deg - 270 deg"), r#"["unit",90,"deg"]"#);
    assert_eq!(simplified("x% 3"), r#"["unit",["*",3,"x"],"%"]"#);
}

/// The two boundaries. Crossing either would assert something false: there is
/// no conversion between these units, and a unit is not a bare scalar.
#[test]
fn unlike_units_and_bare_scalars_never_combine() {
    assert_eq!(simplified("$3 + 2"), r#"["+",2,["unit","$",3]]"#);
    assert_eq!(
        simplified("$3 + 2 deg"),
        r#"["+",["unit",2,"deg"],["unit","$",3]]"#
    );
}

/// A lone unit is left exactly as it was — the pass must not churn a tree it
/// has nothing to combine in.
#[test]
fn a_lone_unit_is_untouched() {
    assert_eq!(simplified("250%"), r#"["unit",250,"%"]"#);
    assert_eq!(simplified("$5"), r#"["unit","$",5]"#);
    assert_eq!(simplified("90 deg"), r#"["unit",90,"deg"]"#);
}

// ---- item 13: mixed-container vectors ----------------------------------

fn sum(a: Expr, b: Expr) -> String {
    js(&simplify(&Expr::Add(vec![a, b])))
}

#[test]
fn same_container_vectors_keep_their_container() {
    assert_eq!(
        sum(l(r"\langle a,b\rangle"), l(r"\langle c,d\rangle")),
        r#"["altvector",["+","a","c"],["+","b","d"]]"#
    );
    assert_eq!(
        sum(t("(a,b)"), t("(c,d)")),
        r#"["tuple",["+","a","c"],["+","b","d"]]"#
    );
}

/// `⟨a,b⟩` and `(c,d)` are the same object in two notations, so the sum folds.
///
/// The container is the class's canonical one rather than "the left operand's":
/// `Add` is commutative and canonically sorted, so both orders must produce the
/// same tree. A vector/altvector anywhere makes the result a `vector` — the
/// stronger reading wins over a bare `tuple` (matching the JS oracle's
/// `<vector> + (point)` → vector).
#[test]
fn mixed_container_vectors_fold_order_independently() {
    let expected = r#"["vector",["+","a","c"],["+","b","d"]]"#;
    assert_eq!(sum(l(r"\langle a,b\rangle"), t("(c,d)")), expected);
    assert_eq!(sum(t("(c,d)"), l(r"\langle a,b\rangle")), expected);
}

#[test]
fn different_arities_and_arrays_do_not_fold() {
    // Different arity is not a vector sum at all.
    assert_eq!(
        sum(l(r"\langle a,b\rangle"), t("(c,d,e)")),
        r#"["+",["tuple","c","d","e"],["altvector","a","b"]]"#
    );
    // `[a,b]` is a different container — `createIntervals` reads it as an
    // interval — so it is its own class and does not merge with a tuple.
    assert_eq!(
        sum(t("[a,b]"), t("(c,d)")),
        r#"["+",["tuple","c","d"],["array","a","b"]]"#
    );
}

// ---- soundness: indeterminate forms ------------------------------------

/// Folding an indeterminate form to a value asserts a limit that does not
/// exist. `∞/∞` reached `1` by collecting to `∞^(1−1)` and meeting `x^0 → 1`.
#[test]
fn indeterminate_forms_do_not_fold_to_a_value() {
    for s in [
        "0^0",
        "Infinity^0",
        "Infinity/Infinity",
        "1^Infinity",
        "0*Infinity",
        "Infinity-Infinity",
        "-Infinity+Infinity",
        "0/0",
    ] {
        assert_eq!(simplified(s), r#"{"$":"NaN"}"#, "{s} should be NaN");
    }
}

/// The neighbours of those two rules, which must keep folding.
#[test]
fn determinate_powers_still_fold() {
    assert_eq!(simplified("2^0"), "1");
    assert_eq!(simplified("5^0"), "1");
    assert_eq!(simplified("x^0"), "1");
    assert_eq!(simplified("1^x"), "1");
    assert_eq!(simplified("1^5"), "1");
    assert_eq!(simplified("0^3"), "0");
    assert_eq!(simplified("Infinity+Infinity"), r#"{"$":"Inf"}"#);
    assert_eq!(simplified("1/0"), r#"{"$":"Inf"}"#);
    assert_eq!(simplified("-1/0"), r#"{"$":"-Inf"}"#);
}
