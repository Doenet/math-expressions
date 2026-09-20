//! Regressions for `active-plans/DOENET_INTEGRATION.md` §2–§5 — the four
//! engine-level items DoenetML filed after switching permanently to the Rust
//! engine.
//!
//! §1 of that document is not a fifth defect: the two WASM traps it reports are
//! §2 and §5 reached through an `assert_eq!` in DoenetML's own core, which
//! `panic = "abort"` reduces to a bare `unreachable`. Both go away with the
//! tests below, so each carries a note saying which trap it retires.

use math_expressions::{
    evaluate_numbers, simplify, to_latex, to_text, Expr, LatexOpts, TextOpts, TextToAst,
};

fn p(s: &str) -> Expr {
    TextToAst::new(Default::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e:?}"))
}

fn tree(e: &Expr) -> String {
    math_expressions::expr::serde::to_js(e).to_string()
}

fn text(e: &Expr) -> String {
    to_text(e, &TextOpts::default())
}

// ---- §2: `evaluate_numbers` is numeric only ------------------------------

/// `simplify="numbers"` is specified as "fold numeric constants, leave the
/// symbolic structure alone". Collecting like terms is a correct
/// simplification but not a numeric one, and it made the attribute
/// indistinguishable from `simplify="full"`.
///
/// This is also the `simplify_math` trap: DoenetML's core asserts
/// `x+2+x+3+4 → x + x + 9` and the mismatch aborted the worker.
#[test]
fn evaluate_numbers_does_not_collect_like_terms() {
    assert_eq!(text(&evaluate_numbers(&p("x+2+x+3+4"))), "x + x + 9");
    assert_eq!(
        tree(&evaluate_numbers(&p("x^2+3x^2"))),
        r#"["+",["^","x",2],["*",3,["^","x",2]]]"#
    );
    assert_eq!(
        tree(&evaluate_numbers(&p("1x+4x"))),
        r#"["+","x",["*",4,"x"]]"#
    );
    assert_eq!(tree(&evaluate_numbers(&p("x+x"))), r#"["+","x","x"]"#);
    // The numeric part still folds, across intervening symbolic terms — that
    // is what separates this from `evaluate_numbers_preserve_order`.
    assert_eq!(tree(&evaluate_numbers(&p("4+x-2"))), r#"["+","x",2]"#);
    assert_eq!(tree(&evaluate_numbers(&p("3*2*x*4"))), r#"["*",24,"x"]"#);
}

/// Terms that *cancel* still collapse. Suppressing that too would leave
/// `evaluate_numbers` unable to see a zero it is being asked about, which the
/// legacy corpus depends on (`(3x−3x)^0`) — and a vanished term is not a
/// shortened one.
#[test]
fn cancelling_terms_still_reach_zero() {
    assert_eq!(tree(&evaluate_numbers(&p("3x-3x"))), "0");
    assert_eq!(tree(&evaluate_numbers(&p("x-x"))), "0");
    assert_eq!(tree(&evaluate_numbers(&p("(2-1)x"))), r#""x""#);
    // A zero coefficient written out disappears; the rest stay separate.
    assert_eq!(
        tree(&evaluate_numbers(&p("1x^2 + 0x^2 - 2x^2 + 5x^2"))),
        r#"["+",["*",-2,["^","x",2]],["^","x",2],["*",5,["^","x",2]]]"#
    );
}

/// `simplify` is unaffected — it is supposed to be the aggressive one.
#[test]
fn full_simplify_still_collects_like_terms() {
    assert_eq!(tree(&simplify(&p("x^2+3x^2"))), r#"["*",4,["^","x",2]]"#);
    assert_eq!(tree(&simplify(&p("x+2+x+3+4"))), r#"["+",["*",2,"x"],9]"#);
}

// ---- §3: inverse trig at exact values ------------------------------------

/// `sin^(-1)(1)` parses (better than legacy, which read it as `1/sin`) but did
/// not evaluate, so `simplifyOnCompare` could not grade it.
#[test]
fn inverse_trig_folds_at_exact_values() {
    for (input, expected) in [
        ("asin(1)", "pi/2"),
        ("sin^(-1)(1)", "pi/2"),
        ("asin(-1)", "-pi/2"),
        ("asin(0)", "0"),
        ("asin(1/2)", "pi/6"),
        ("acos(1)", "0"),
        ("acos(0)", "pi/2"),
        ("acos(-1/2)", "2pi/3"),
        ("acos(sqrt(3)/2)", "pi/6"),
        ("atan(1)", "pi/4"),
        ("atan(sqrt(3))", "pi/3"),
        ("acot(1)", "pi/4"),
        ("asec(2)", "pi/3"),
        ("acsc(2)", "pi/6"),
    ] {
        assert_eq!(
            simplify(&p(input)),
            simplify(&p(expected)),
            "{input} should simplify to {expected}"
        );
    }
}

/// The fold recognizes a *value*, not a spelling. Comparing canonicalized
/// trees made it depend on whether the radical rules happened to have
/// rationalized the argument first — `asin(√2/2)` folded and `asin(1/√2)`, the
/// same number, did not. Comparison happens in the exact ring instead, whose
/// normal form is zero exactly when the value is.
#[test]
fn inverse_trig_ignores_how_the_argument_is_written() {
    for group in [
        vec![
            "asin(sqrt(2)/2)",
            "asin(1/sqrt(2))",
            "asin(0.5*sqrt(2))",
            "asin(2/(2sqrt(2)))",
        ],
        vec!["atan(sqrt(3)/3)", "atan(1/sqrt(3))"],
        vec!["asin(sqrt(6)/4+sqrt(2)/4)", "asin((sqrt(6)+sqrt(2))/4)"],
        vec!["asin(1/2)", "asin(0.5)", "asin(2/4)"],
    ] {
        let first = simplify(&p(group[0]));
        assert!(
            !matches!(first, Expr::Apply(..)),
            "{:?} should fold at all",
            group[0]
        );
        for s in &group[1..] {
            assert_eq!(
                simplify(&p(s)),
                first,
                "{s:?} spells the same value as {:?}",
                group[0]
            );
        }
    }
}

/// Only values that really are rational multiples of π fold. Everything else
/// stays symbolic rather than becoming a float — the exactness gate the rest
/// of the fold layer keeps.
#[test]
fn inverse_trig_declines_everything_off_the_lattice() {
    for s in ["asin(2)", "asin(x)", "atan(1/3)", "acos(0.3)"] {
        assert!(
            matches!(simplify(&p(s)), Expr::Apply(..)),
            "{s:?} must stay symbolic"
        );
    }
    // Parity pulls the sign out of a negative argument, but it does not invent
    // an angle: the argument is still off the lattice underneath.
    assert_eq!(
        tree(&simplify(&p("asin(-3)"))),
        r#"["-",["apply","asin",3]]"#
    );
}

/// Each branch is inverted on its *principal* range, so the fold is a function
/// and round-trips the way the numeric evaluator does.
#[test]
fn inverse_trig_stays_on_the_principal_branch() {
    // sin(5pi/6) is also 1/2, but asin's range is [-pi/2, pi/2].
    assert_eq!(simplify(&p("asin(sin(5pi/6))")), simplify(&p("pi/6")));
    // cos(-pi/3) is also 1/2; acos's range is [0, pi].
    assert_eq!(simplify(&p("acos(cos(-pi/3))")), simplify(&p("pi/3")));
    assert_eq!(simplify(&p("sin(asin(1))")), simplify(&p("1")));
}

// ---- §4: change of base --------------------------------------------------

/// `log_b(a)` was inert: it combined with nothing, so the difference against
/// `log(a)/log(b)` never reached zero.
#[test]
fn based_logarithms_reduce_by_change_of_base() {
    for (a, b) in [("a", "b"), ("9", "2"), ("x", "2"), ("x", "y")] {
        assert_eq!(
            simplify(&p(&format!("log_{b}({a})"))),
            simplify(&p(&format!("log({a})/log({b})"))),
            "log_{b}({a}) should reduce by change of base"
        );
        assert_eq!(
            tree(&simplify(&p(&format!("log_{b}({a}) - log({a})/log({b})")))),
            "0"
        );
    }
    // A base of e cancels itself once `log e` folds to 1.
    assert_eq!(tree(&simplify(&p("log_e(x)"))), r#"["apply","log","x"]"#);
    assert_eq!(tree(&simplify(&p("log_x(x)"))), "1");
}

/// The rewrite must not pre-empt an exact fold: it runs *before* the numeric
/// application pass in each `full_simplify` round, so it has to decline where
/// that pass would answer.
#[test]
fn an_exact_based_logarithm_still_folds_to_its_value() {
    for (s, expected) in [
        ("log_2(8)", "3"),
        ("log_10(1000)", "3"),
        ("log_7(343)", "3"),
        ("log_2(1024)", "10"),
        ("log_2(1)", "0"),
    ] {
        assert_eq!(tree(&simplify(&p(s))), expected, "{s} should fold exactly");
    }
}

// ---- §5: fractions stay fractions ----------------------------------------

/// `.tree` decimalized every terminating rational, so DoenetML's structural
/// criteria (`ReducedFraction`, `ExactValue`) could not see a fraction that was
/// no longer there. Decimals and fractions parse to the *same* exact rational,
/// so the fix is a spelling carried on the value, not a serializer tweak.
///
/// This is also the `arithmetic_on_math` trap: DoenetML's core asserts
/// `3/6 → 1/2`.
#[test]
fn a_fraction_of_integers_stays_a_fraction() {
    for (input, want_tree, want_text, want_latex) in [
        ("3/6", r#"["/",1,2]"#, "1/2", "\\frac{1}{2}"),
        ("5/2", r#"["/",5,2]"#, "5/2", "\\frac{5}{2}"),
        ("2/4", r#"["/",1,2]"#, "1/2", "\\frac{1}{2}"),
        ("1/3", r#"["/",1,3]"#, "1/3", "\\frac{1}{3}"),
        ("-3/6", r#"["/",-1,2]"#, "-1/2", "-\\frac{1}{2}"),
    ] {
        let e = simplify(&p(input));
        assert_eq!(tree(&e), want_tree, "{input} tree");
        assert_eq!(text(&e), want_text, "{input} text");
        assert_eq!(
            to_latex(&e, &LatexOpts::default()),
            want_latex,
            "{input} latex"
        );
    }
}

/// The other half, and the reason the naive "emit every rational as a
/// fraction" patch is wrong: a typed decimal *is* an exact rational, and must
/// read back as the decimal the student wrote.
#[test]
fn a_typed_decimal_stays_a_decimal() {
    for (input, want) in [
        ("0.5", "0.5"),
        ("19.9", "19.9"),
        ("2.345", "2.345"),
        ("0.1+0.2", "0.3"),
        ("0.5^2", "0.25"),
        ("0.75-0.25", "0.5"),
        ("abs(-3.5)", "3.5"),
    ] {
        assert_eq!(tree(&simplify(&p(input))), want, "{input}");
    }
}

/// Decimal is contagious through arithmetic, exactly as `Float` is: once a
/// decimal quantity is in the computation the result is a decimal quantity.
#[test]
fn a_decimal_operand_makes_the_result_decimal() {
    assert_eq!(tree(&simplify(&p("1/2+1/4"))), r#"["/",3,4]"#);
    assert_eq!(tree(&simplify(&p("1/2+0.25"))), "0.75");
    assert_eq!(tree(&simplify(&p("0.5+1/4"))), "0.75");
    // Through the polynomial layer, which works in spelling-free `BigRational`
    // and so has to have one restored.
    assert_eq!(tree(&simplify(&p("(1.5x+1.5)/(x+1)"))), "1.5");
    assert_eq!(tree(&simplify(&p("(3x+3)/(2x+2)"))), r#"["/",3,2]"#);
}

/// A coefficient keeps its own spelling instead of always being split across a
/// fraction bar. This was the subtle half of §5: `0.5·x` presented as
/// `Div(x, 2)`, and re-canonicalizing that left two integers behind with the
/// decimal origin destroyed — several passes away from anything that looked
/// responsible.
#[test]
fn a_coefficient_is_not_forced_under_a_fraction_bar() {
    assert_eq!(tree(&simplify(&p("0.5x"))), r#"["*",0.5,"x"]"#);
    assert_eq!(tree(&simplify(&p("2.5x"))), r#"["*",2.5,"x"]"#);
    assert_eq!(tree(&simplify(&p("x/2"))), r#"["/","x",2]"#);
    assert_eq!(tree(&simplify(&p("(1/2)x"))), r#"["/","x",2]"#);
    // Same rule in an exponent.
    assert_eq!(tree(&simplify(&p("x^(3/2)"))), r#"["^","x",["/",3,2]]"#);
    assert_eq!(tree(&simplify(&p("x^1.5"))), r#"["^","x",1.5]"#);
}

/// Rounding *to decimal places* imposes the decimal spelling on a decimal
/// quantity — but never on a fraction the author wrote.
///
/// This used to turn on whether the round changed the value, which kept `5/2`
/// and lost `1/3`. That rule assumed the JS library decimalized a
/// non-terminating fraction, and it had no way to: `1/3` was the tree
/// `["/", 1, 3]` and rounding, which maps over *numbers*, found two whole
/// integers. The distinction now is [`Spelling`], which is the same one legacy
/// drew by having no rational type at all.
#[test]
fn rounding_produces_decimals() {
    use math_expressions::{round_numbers_to_decimals, round_numbers_to_precision};
    // A fraction stays a fraction, folded or not — `displayDigits` defaults to
    // 3 and every rational a student sees goes through this path, so in a
    // fractions lesson this is the whole point.
    assert_eq!(
        tree(&round_numbers_to_decimals(&simplify(&p("1/3")), 2)),
        r#"["/",1,3]"#
    );
    assert_eq!(tree(&round_numbers_to_decimals(&p("2.345"), 2)), "2.35");
    // Rounding that changes nothing (`3/6` is `1/2` is `0.5` exactly) leaves the
    // fraction untouched, at both a decimal count and a significant-figure count.
    assert_eq!(
        tree(&round_numbers_to_decimals(&simplify(&p("3/6")), 3)),
        r#"["/",1,2]"#
    );
    assert_eq!(
        tree(&round_numbers_to_precision(&simplify(&p("3/6")), 3)),
        r#"["/",1,2]"#
    );
}

/// The spelling is *not* part of a number's identity: `0.5` and `1/2` are the
/// same value and must stay interchangeable everywhere equality is decided.
/// The hand-written `PartialEq`/`Hash` on `Number` is what guarantees it, and
/// getting this wrong would silently split canonical trees in two.
#[test]
fn spelling_does_not_affect_equality() {
    use math_expressions::equals;
    let opts = Default::default();
    assert!(equals(&p("0.5"), &p("1/2"), &opts));
    assert!(equals(&p("3/6"), &p("0.5"), &opts));
    assert!(equals(&p("0.5x"), &p("x/2"), &opts));
    // Structural, not just numeric: the canonical forms must be one tree.
    assert_eq!(
        math_expressions::canonicalize(&p("0.5")),
        math_expressions::canonicalize(&p("1/2"))
    );
    assert_eq!(
        math_expressions::canonicalize(&p("0.5x+0.5x")),
        math_expressions::canonicalize(&p("x"))
    );
}
