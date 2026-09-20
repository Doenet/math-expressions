//! `simplify` reaches **one** fixpoint per value, not one per spelling.
//!
//! Idempotence (`simplify(simplify(e)) == simplify(e)`) was never the problem —
//! it is asserted over the corpus in `simplify_corpus.rs` and it always held.
//! The problem was confluence: `−(a+b)/3` and `(−a−b)/3` are the same value,
//! both were stable, and they were stable at *different* trees, so a caller
//! comparing two correct answers structurally saw them disagree.
//!
//! The cause was that sign placement was decided by a rule that only ever
//! pushed a sign **into** a sum. A sum that arrived already distributed was a
//! fixpoint by default, so the preference was never applied to it. The fix is
//! the converse rewrite (`rule_factor_sign_out_of_sum`) with a threshold that
//! complements the existing one exactly — see its docs for why precisely one of
//! the two spellings is stable for every sum.
//!
//! Each group below is one value written several ways. The assertion is that
//! the group collapses to a single tree; which tree it collapses to is the
//! subject of `doenet_sign_distribution.rs`, not this file.

use math_expressions::{equals, expr, simplify, EqOptions, Expr, TextToAst};
use std::collections::BTreeSet;

fn t(s: &str) -> Expr {
    TextToAst::new(Default::default()).convert(s).unwrap()
}

fn simplified(s: &str) -> String {
    expr::serde::to_js(&simplify(&t(s))).to_string()
}

/// Spellings of one value. A group must simplify to exactly one tree.
const GROUPS: &[&[&str]] = &[
    // The case this was found on: `solve_linear` handing `simplify` a quotient
    // whose sign could sit in either place.
    &[
        "-(2xz+r+v)/3",
        "(-2xz-r-v)/3",
        "-((2xz+r+v)/3)",
        "(-(-2xz-r-v))/(-3)",
    ],
    &["-(a+b)/3", "(-a-b)/3", "-((a+b)/3)", "(a+b)/(-3)"],
    &["-a/b", "a/(-b)", "(-a)/b", "-(a/b)"],
    &["-2x/3", "2x/(-3)", "(-2x)/3", "-(2x/3)"],
    &["-(a+b)", "-a-b", "(-1)(a+b)"],
    // Sum denominators: no numeric coefficient for a sign rule to grab, so
    // these only converge once the sign can be factored out of the sums.
    &["-(x+1)/(y+1)", "(-x-1)/(y+1)", "(x+1)/(-(y+1))"],
    &["-(q+v)/(3-2v)", "(q+v)/(2v-3)", "(-q-v)/(3-2v)"],
    // Ties in sign count: `k` odd with `2n = k + 1`, so both spellings cost the
    // same. These are exactly the cases that stay double-stable if either
    // threshold is off by one, and they are the reason the tie in
    // `rule_distribute_sign` has to resolve toward the pushed-in form.
    &["-x-y+z", "-(x+y-z)"],
    &["-a-b-c+d", "-(a+b+c-d)"],
    &["-a-b-c+d+e", "-(a+b+c-d-e)"],
    &["-(x+y-z)^3", "(-x-y+z)^3"],
    // Strict wins in each direction.
    &["a-b-c", "-b+a-c", "-(-a+b+c)"],
    &["-a-b-c", "-(a+b+c)"],
];

#[test]
fn each_value_has_exactly_one_fixpoint() {
    for g in GROUPS {
        let trees: BTreeSet<String> = g.iter().map(|s| simplified(s)).collect();
        assert_eq!(
            trees.len(),
            1,
            "{g:?} simplified to {} different trees: {trees:#?}",
            trees.len()
        );
    }
}

#[test]
fn every_spelling_still_settles() {
    // Confluence is not idempotence, and adding a converse rewrite is exactly
    // how a rule pair starts to ping-pong. The thresholds are disjoint by
    // construction; this is the check that they stayed that way.
    for s in GROUPS.iter().flat_map(|g| g.iter()) {
        let once = simplify(&t(s));
        let twice = simplify(&once);
        assert_eq!(
            expr::serde::to_js(&once).to_string(),
            expr::serde::to_js(&twice).to_string(),
            "{s} did not settle"
        );
    }
}

#[test]
fn the_value_is_unchanged() {
    // Sign bookkeeping, not arithmetic.
    for s in GROUPS.iter().flat_map(|g| g.iter()) {
        let before = t(s);
        let after = simplify(&before);
        assert!(
            equals(&before, &after, &EqOptions::default()),
            "{s} changed value: {}",
            expr::serde::to_js(&after)
        );
    }
}

#[test]
fn a_double_negative_over_the_bar_cancels() {
    // Fell out of the pull-out rule rather than being aimed at: both sums shed
    // their −1, the two meet in the enclosing product, and they cancel. Neither
    // sign rule could see this before, because there was no numeric coefficient
    // anywhere for either of them to act on — so the fraction simply stayed as
    // written, in this engine and in alpha94 both.
    assert_eq!(simplified("(-a-b)/(-c-d)"), simplified("(a+b)/(c+d)"));
    assert_eq!(simplified("(-x-1)/(-y-1)"), simplified("(x+1)/(y+1)"));
}

#[test]
fn factoring_a_sign_is_not_factoring_a_coefficient() {
    // The sign is a unit and moves freely; a magnitude does not move, because
    // that is `expand`'s decision and it changes the term count. These two must
    // stay distinct — collapsing them would make `simplify` an expander.
    assert_ne!(simplified("2(a+b)"), simplified("2a+2b"));
    assert_eq!(simplified("2(a+b)"), r#"["*",2,["+","a","b"]]"#);
    assert_eq!(simplified("2a+2b"), r#"["+",["*",2,"a"],["*",2,"b"]]"#);
}
