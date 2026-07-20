//! The full equality corpus ported from the JS `slow_math-expressions.spec.js`
//! (824 pairs). Each pair asserts the JS verdict:
//!
//! - `equivalences` / `nonequivalences` → `equals()` is true / false;
//! - `symbolic_equivalences` / `symbolic_nonequivalences` →
//!   `equals_syntactic()` is true / false.
//!
//! Not all pass yet: many need deferred features (heuristic `simplify`,
//! assumptions, units, more `eval_complex` functions) or are intentional
//! divergences. Our canonical form is deliberately stronger than the JS
//! `equalsViaSyntax`: it folds constants (`3+2` = `5`) and normalizes converse
//! relations (`A⊂B` = `B⊃A`), so a chunk of the JS "symbolic non-equivalences"
//! are — correctly — equal for us. The currently-failing pairs are snapshotted in
//! `fixtures/equality-known-failures.json`; the test asserts *no new
//! regressions* (current failures ⊆ snapshot). As the implementation improves,
//! shrink the snapshot.
//!
//! Regenerate the snapshot after an intended change:
//!   UPDATE_KNOWN_FAILURES=1 cargo test --test equality_corpus

use math_expressions::{equals, equals_syntactic, EqOptions, Expr, TextToAst, TextToAstOptions};
use std::collections::BTreeSet;

fn parse(s: &str) -> Option<Expr> {
    TextToAst::new(TextToAstOptions::default()).convert(s).ok()
}

/// The verdict for one pair, or `None` if it could not even be parsed.
fn verdict(kind: Kind, lhs: &str, rhs: &str) -> Option<bool> {
    let opts = EqOptions::default();
    let (a, b) = (parse(lhs)?, parse(rhs)?);
    let f = match kind {
        Kind::Numeric => equals,
        Kind::Syntactic => equals_syntactic,
    };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&a, &b, &opts))).ok()
}

#[derive(Clone, Copy)]
enum Kind {
    Numeric,
    Syntactic,
}

const CORPUS: &str = include_str!("fixtures/equality-corpus.json");
const KNOWN_FAILURES: &str = include_str!("fixtures/equality-known-failures.json");

#[derive(serde::Deserialize)]
struct Corpus {
    equivalences: Vec<[String; 2]>,
    nonequivalences: Vec<[String; 2]>,
    symbolic_equivalences: Vec<[String; 2]>,
    symbolic_nonequivalences: Vec<[String; 2]>,
}

/// A stable key for a pair, used to snapshot failures.
fn key(group: &str, lhs: &str, rhs: &str) -> String {
    format!("{group}\u{1}{lhs}\u{1}{rhs}")
}

fn collect_failures() -> BTreeSet<String> {
    let corpus: Corpus = serde_json::from_str(CORPUS).unwrap();
    let mut failures = BTreeSet::new();

    let mut run = |group: &str, pairs: &[[String; 2]], kind: Kind, expect: bool| {
        for p in pairs {
            let ok = verdict(kind, &p[0], &p[1]) == Some(expect);
            if !ok {
                failures.insert(key(group, &p[0], &p[1]));
            }
        }
    };

    run("equivalences", &corpus.equivalences, Kind::Numeric, true);
    run(
        "nonequivalences",
        &corpus.nonequivalences,
        Kind::Numeric,
        false,
    );
    run(
        "symbolic_equivalences",
        &corpus.symbolic_equivalences,
        Kind::Syntactic,
        true,
    );
    run(
        "symbolic_nonequivalences",
        &corpus.symbolic_nonequivalences,
        Kind::Syntactic,
        false,
    );
    failures
}

#[test]
fn corpus_no_regressions() {
    std::panic::set_hook(Box::new(|_| {}));
    let failures = collect_failures();

    // Snapshot-update mode: write the current failures and pass.
    if std::env::var("UPDATE_KNOWN_FAILURES").is_ok() {
        let list: Vec<&String> = failures.iter().collect();
        let json = serde_json::to_string_pretty(&list).unwrap();
        std::fs::write(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/equality-known-failures.json"
            ),
            json,
        )
        .unwrap();
        eprintln!("updated snapshot: {} known failures", failures.len());
        return;
    }

    let known: BTreeSet<String> = serde_json::from_str::<Vec<String>>(KNOWN_FAILURES)
        .unwrap()
        .into_iter()
        .collect();

    let new: Vec<&String> = failures.difference(&known).collect();
    let fixed: Vec<&String> = known.difference(&failures).collect();

    if !new.is_empty() {
        eprintln!("{} NEW regressions:", new.len());
        for k in &new {
            eprintln!("  {}", k.replace('\u{1}', " | "));
        }
    }

    // Report progress: pairs that now pass but are still listed as known
    // failures should be pruned from the snapshot.
    if !fixed.is_empty() {
        eprintln!(
            "{} known failures now PASS — prune them (UPDATE_KNOWN_FAILURES=1):",
            fixed.len()
        );
        for k in fixed.iter().take(20) {
            eprintln!("  {}", k.replace('\u{1}', " | "));
        }
    }

    assert!(
        new.is_empty(),
        "{} NEW equality regressions (not in snapshot):\n{}",
        new.len(),
        new.iter()
            .take(40)
            .map(|k| format!("  {}", k.replace('\u{1}', " | ")))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// A headline count so progress is visible in test output.
#[test]
fn corpus_pass_rate() {
    std::panic::set_hook(Box::new(|_| {}));
    let corpus: Corpus = serde_json::from_str(CORPUS).unwrap();
    let total = corpus.equivalences.len()
        + corpus.nonequivalences.len()
        + corpus.symbolic_equivalences.len()
        + corpus.symbolic_nonequivalences.len();
    let failures = collect_failures().len();
    eprintln!(
        "equality corpus: {}/{} pass ({} failing)",
        total - failures,
        total,
        failures
    );
}
