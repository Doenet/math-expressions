//! Predicates read the *derived* assumption store with sound boolean semantics.
//!
//! `is_real`/`is_negative`/… used to answer from a flat list of facts filed
//! verbatim under each variable: no chaining, no equalities, no disjunctions.
//! The transitive closure the store already computed (equality following,
//! bound arithmetic, the `or`-entails-what-every-branch-entails rule) sat in a
//! second half of the store that the predicates never consulted. They now read
//! that half and walk its boolean structure: `and` is a meet (both hold), `or`
//! a join (a value is known only when every disjunct agrees).
//!
//! One place this **diverges from the legacy engine, deliberately**: legacy
//! reads `and` as `left || right` (the first branch that answers wins), so it
//! reports `is_real(x)` *true* for the contradiction `x ∈ R and x ∉ R`. That is
//! unsound — no `x` satisfies both. We answer `undefined` instead. See
//! `a_contradiction_is_not_a_licence_to_guess`.
//!
//! These cases run against the *tree* store (`trees_mut().add_assumption`),
//! which is the half js-compat feeds and the half that holds the closure; the
//! flat `Assumptions::add` path is exercised by `assumptions.rs`.

use math_expressions::{
    is_integer, is_negative, is_nonzero, is_positive, is_real, Assumptions, Expr, TextToAst,
    TextToAstOptions,
};

type MaybeBool = Option<bool>;

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

/// Assumptions filed into the derived store, the way the JS API feeds it.
fn assume(facts: &[&str]) -> Assumptions {
    let mut a = Assumptions::new();
    for f in facts {
        a.trees_mut().add_assumption(&parse(f), false);
    }
    a
}

fn real_of(facts: &[&str], q: &str) -> MaybeBool {
    is_real(&parse(q), &assume(facts))
}
fn neg_of(facts: &[&str], q: &str) -> MaybeBool {
    is_negative(&parse(q), &assume(facts))
}

#[test]
fn an_equality_carries_facts_between_its_sides() {
    assert_eq!(real_of(&["y elementof R", "x = y"], "x"), Some(true));
    assert_eq!(neg_of(&["x < 0", "y = x"], "y"), Some(true));
    assert_eq!(
        is_integer(&parse("y"), &assume(&["x elementof Z", "y = x"])),
        Some(true)
    );
    // A doubly-negated inequality *is* an equality, and carries the fact; a
    // plain `!=` does not.
    assert_eq!(neg_of(&["x < 0", "not(y != x)"], "y"), Some(true));
    assert_eq!(neg_of(&["x < 0", "y != x"], "y"), None);
    assert_eq!(real_of(&["y elementof R", "x != y"], "x"), None);
}

#[test]
fn a_bound_chains_through_the_variables_between_it_and_zero() {
    // x ≤ y < −1 ⇒ x < 0, and u < x ⇒ u < 0.
    let a = assume(&["y < -1", "y >= x", "u < x"]);
    assert_eq!(is_negative(&parse("x"), &a), Some(true));
    assert_eq!(is_negative(&parse("u"), &a), Some(true));
    // u < y − x with y < −1 and x > 1 ⇒ u < −2: the bound is arithmetic, not
    // just a rename.
    let a = assume(&["y < -1", "x > 1", "u < y - x"]);
    assert_eq!(is_negative(&parse("u"), &a), Some(true));
    // y + 1 < 0 has to be solved for y before it reads as a bound.
    assert_eq!(neg_of(&["y + 1 < 0"], "y"), Some(true));
}

#[test]
fn a_disjunction_entails_only_what_every_branch_does() {
    // Both branches miss zero, so the union does too.
    assert_eq!(
        is_nonzero(&parse("x"), &assume(&["x < 0 or x > 0"])),
        Some(true)
    );
    // Both branches are real; neither pins the sign.
    let a = assume(&["x < 0 or x > 5"]);
    assert_eq!(is_real(&parse("x"), &a), Some(true));
    assert_eq!(is_nonzero(&parse("x"), &a), Some(true));
    assert_eq!(is_positive(&parse("x"), &a), None);
    // One branch is positive, the other only nonzero: the sign does not survive
    // the join, but nonzero and real do.
    let a = assume(&["x > 0 or (x elementof (4,8))"]);
    assert_eq!(is_positive(&parse("x"), &a), Some(true));
    // An interval on both sides of an `and`/`or` reduces to bounds.
    let a = assume(&["x > 2 and (x < 7 or x > 8)"]);
    assert_eq!(is_positive(&parse("x"), &a), Some(true));
}

#[test]
fn a_contradiction_is_not_a_licence_to_guess() {
    // The deliberate divergence from alpha94: it answers `is_real` true here by
    // taking the first conjunct; no value is both real and non-real, so we
    // answer unknown. (`is_complex` is still known: every branch agrees on it.)
    let a = assume(&["x elementof R and x notelementof R"]);
    assert_eq!(is_real(&parse("x"), &a), None);
}

#[test]
fn following_an_equality_cycle_terminates() {
    // `x = x` and `x = y = x` must not spin: the equality is filed with a
    // symbol on the other side, which contributes no bound, rather than being
    // chased back to itself.
    assert_eq!(real_of(&["x = x"], "x"), None);
    assert_eq!(real_of(&["x = y = x"], "x"), None);
}
