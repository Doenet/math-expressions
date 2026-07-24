//! Differential corpus for `expand` (PORTING_PLAN.md §15). Random products /
//! powers of sums with the tree JS `.expand()` (mathjs) produces; Rust's
//! `expand` must be mathematically equal (via `equals`). Divergences are
//! snapshotted. Regenerate: `node scripts/generate-expand-corpus.mjs`.
//!   UPDATE_KNOWN_FAILURES=1 cargo test --test expand_corpus

use math_expressions::{equals, expand, js_tree, EqOptions, Expr, TextToAst, TextToAstOptions};
use serde_json::Value;
use std::collections::BTreeSet;

fn parse(s: &str) -> Option<Expr> {
    TextToAst::new(TextToAstOptions::default()).convert(s).ok()
}

fn catch<T>(f: impl FnOnce() -> T) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok()
}

#[derive(serde::Deserialize)]
struct Corpus {
    cases: Vec<Case>,
    #[serde(rename = "jsHangs")]
    js_hangs: Vec<String>,
}

#[derive(serde::Deserialize)]
struct Case {
    input: String,
    expanded: Value,
}

const CORPUS: &str = include_str!("fixtures/expand-corpus.json");
const KNOWN_FAILURES: &str = include_str!("fixtures/expand-known-failures.json");

fn corpus() -> Corpus {
    serde_json::from_str(CORPUS).unwrap()
}

fn collect_failures() -> BTreeSet<String> {
    let cases = corpus().cases;
    let opts = EqOptions::default();
    let mut failures = BTreeSet::new();
    for c in &cases {
        let ok = catch(|| {
            let got = expand(&parse(&c.input)?);
            Some(equals(&got, &js_tree::try_from_js(&c.expanded).expect("fixture tree"), &opts))
        })
        .flatten()
        .unwrap_or(false);
        if !ok {
            failures.insert(c.input.clone());
        }
    }
    failures
}

#[test]
fn expand_corpus_no_regressions() {
    std::panic::set_hook(Box::new(|_| {}));
    let failures = collect_failures();
    if std::env::var("UPDATE_KNOWN_FAILURES").is_ok() {
        let list: Vec<&String> = failures.iter().collect();
        std::fs::write(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/expand-known-failures.json"),
            serde_json::to_string_pretty(&list).unwrap() + "\n",
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
    assert!(
        new.is_empty(),
        "{} NEW expand divergences from JS:\n{}",
        new.len(),
        new.iter().take(40).map(|k| format!("  {k}")).collect::<Vec<_>>().join("\n"),
    );
}

#[test]
fn expand_corpus_pass_rate() {
    std::panic::set_hook(Box::new(|_| {}));
    let total = corpus().cases.len();
    let failures = collect_failures().len();
    eprintln!(
        "expand corpus: {}/{} match JS .expand() ({} failing)",
        total - failures,
        total,
        failures
    );
}

/// Inputs on which mathjs `expand` hangs must expand promptly in Rust (the
/// `MAX_EXPAND_POWER` cap keeps it bounded). Another robustness win over the
/// reference, surfaced by the differential harness.
#[test]
fn does_not_hang_where_mathjs_does() {
    use std::time::{Duration, Instant};
    for input in corpus().js_hangs {
        let Some(e) = parse(&input) else { continue };
        let t = Instant::now();
        let _ = expand(&e);
        assert!(
            t.elapsed() < Duration::from_secs(2),
            "expand({input:?}) took {:?} — mathjs hangs here; Rust must stay bounded",
            t.elapsed(),
        );
    }
}
