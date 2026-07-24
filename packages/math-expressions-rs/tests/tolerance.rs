//! Port of `spec/slow_check-equality-numerical-errors.spec.js` — the
//! `allowed_error_in_numbers` grading matrix. For each `original` and its
//! grading options (relative vs absolute error, exponent inclusion), the JS
//! spec asserts every `correct_answer` compares equal and at least one
//! `at_least_one_incorrect_answer` does not. Driven through Rust `equals`, whose
//! `EqOptions` mirror the JS option object.
//!
//! `equals` grades by random numeric sampling, so a handful of the hardest
//! cases (exponent-tolerance on fractional powers, deeply nested exp) land
//! differently than JS. Following the repo's corpus idiom, this is a
//! **no-regressions** test: the accepted divergences are snapshotted in
//! `fixtures/tolerance-known-failures.json` and only a NEW divergence fails.
//! Re-bless after an intentional change:  `BLESS=1 cargo test --test tolerance`.
//!
//! Fixture: `fixtures/tolerance-corpus.json`, from
//! `scripts/generate-tolerance-corpus.mjs`.
//!
//! NOTE: the companion `slow_check-symbolic-equality-numerical-errors.spec.js`
//! drives `equalsViaSyntax` with number tolerance; Rust's `equals_syntactic` is
//! exact and does not apply `allowed_error_in_numbers`, so that matrix is a
//! documented behavioral divergence (see JS_TEST_COVERAGE_AUDIT.md), not ported.

use math_expressions::{equals, EqOptions, Expr, TextToAst, TextToAstOptions};
use std::collections::BTreeSet;

#[derive(serde::Deserialize)]
struct ToleranceCase {
    original: String,
    allowed_error: f64,
    absolute_error: bool,
    include_exponents: bool,
    correct_answers: Vec<String>,
    at_least_one_incorrect_answer: Vec<String>,
}

const SNAPSHOT: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tolerance-known-failures.json");

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

/// Every case that grades differently from the JS spec, as stable one-line keys.
fn current_failures(cases: &[ToleranceCase]) -> BTreeSet<String> {
    let mut failures = BTreeSet::new();
    for c in cases {
        let opts = EqOptions {
            allowed_error_in_numbers: c.allowed_error,
            include_error_in_number_exponents: c.include_exponents,
            allowed_error_is_absolute: c.absolute_error,
            ..EqOptions::default()
        };
        let orig = parse(&c.original);
        for ans in &c.correct_answers {
            if !equals(&orig, &parse(ans), &opts) {
                failures.insert(format!("[{}] correct rejected: {ans}", c.original));
            }
        }
        let any_rejected = c
            .at_least_one_incorrect_answer
            .iter()
            .any(|ans| !equals(&orig, &parse(ans), &opts));
        if !any_rejected {
            failures.insert(format!("[{}] no incorrect rejected", c.original));
        }
    }
    failures
}

#[test]
fn tolerance_grading_matrix_no_regressions() {
    let cases: Vec<ToleranceCase> =
        serde_json::from_str(include_str!("fixtures/tolerance-corpus.json")).unwrap();
    assert!(!cases.is_empty(), "fixture is empty");
    let current = current_failures(&cases);

    if std::env::var("BLESS").is_ok() {
        let mut list: Vec<&String> = current.iter().collect();
        list.sort();
        std::fs::write(SNAPSHOT, serde_json::to_string_pretty(&list).unwrap() + "\n").unwrap();
        eprintln!("blessed {} known tolerance divergences", list.len());
        return;
    }

    let raw = std::fs::read_to_string(SNAPSHOT)
        .unwrap_or_else(|e| panic!("read {SNAPSHOT}: {e} (BLESS=1 to create)"));
    let known: BTreeSet<String> = serde_json::from_str(&raw).unwrap();

    let new: Vec<&String> = current.difference(&known).collect();
    assert!(
        new.is_empty(),
        "{} NEW tolerance grading divergence(s) (BLESS=1 if intentional):\n{}",
        new.len(),
        new.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
    );

    // A shrinking snapshot is fine (Rust improved); surface it so the snapshot
    // can be re-blessed to stay tight, without failing the build.
    let fixed = known.difference(&current).count();
    if fixed > 0 {
        eprintln!("note: {fixed} snapshotted divergence(s) now pass — re-bless to tighten");
    }
}
