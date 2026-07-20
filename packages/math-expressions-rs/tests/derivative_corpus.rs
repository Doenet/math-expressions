//! Differential corpus for symbolic differentiation (PORTING_PLAN.md §15 Phase
//! 8). `fixtures/derivative-corpus.json` holds random differentiable inputs with
//! the tree JS `.derivative('x')` (mathjs) produces. Rust's `derivative` must be
//! mathematically equal (via `equals`) to JS's — not tree-identical.
//! Regenerate: `node scripts/generate-derivative-corpus.mjs [count] [seed]`.
//!
//! Divergences are snapshotted (`derivative-known-failures.json`) to guard
//! against regression and shrink over time. Regenerate the snapshot:
//!   UPDATE_KNOWN_FAILURES=1 cargo test --test derivative_corpus

use math_expressions::{derivative, equals, js_tree, EqOptions, Expr, TextToAst, TextToAstOptions};
use serde_json::Value;
use std::collections::BTreeSet;

fn parse(s: &str) -> Option<Expr> {
    TextToAst::new(TextToAstOptions::default()).convert(s).ok()
}

fn catch<T>(f: impl FnOnce() -> T) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok()
}

#[derive(serde::Deserialize)]
struct Case {
    input: String,
    deriv: Value,
}

const CORPUS: &str = include_str!("fixtures/derivative-corpus.json");
const KNOWN_FAILURES: &str = include_str!("fixtures/derivative-known-failures.json");

fn collect_failures() -> BTreeSet<String> {
    let cases: Vec<Case> = serde_json::from_str(CORPUS).unwrap();
    let opts = EqOptions::default();
    let mut failures = BTreeSet::new();
    for c in &cases {
        let ok = catch(|| {
            let input = parse(&c.input)?;
            let got = derivative(&input, "x");
            let want = js_tree::from_js(&c.deriv);
            Some(equals(&got, &want, &opts))
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
fn derivative_corpus_no_regressions() {
    std::panic::set_hook(Box::new(|_| {}));
    let failures = collect_failures();

    if std::env::var("UPDATE_KNOWN_FAILURES").is_ok() {
        let list: Vec<&String> = failures.iter().collect();
        std::fs::write(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/derivative-known-failures.json"
            ),
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
    let fixed: Vec<&String> = known.difference(&failures).collect();

    if !fixed.is_empty() {
        eprintln!(
            "{} known failures now PASS — prune (UPDATE_KNOWN_FAILURES=1):",
            fixed.len()
        );
    }
    assert!(
        new.is_empty(),
        "{} NEW derivative divergences from JS:\n{}",
        new.len(),
        new.iter().take(40).map(|k| format!("  {k}")).collect::<Vec<_>>().join("\n"),
    );
}

#[test]
fn derivative_corpus_pass_rate() {
    std::panic::set_hook(Box::new(|_| {}));
    let total = serde_json::from_str::<Vec<Case>>(CORPUS).unwrap().len();
    let failures = collect_failures().len();
    eprintln!(
        "derivative corpus: {}/{} match JS .derivative() ({} failing)",
        total - failures,
        total,
        failures
    );
}
