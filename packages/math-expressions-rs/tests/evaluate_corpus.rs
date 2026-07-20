//! Differential corpus for `evaluate` / `evaluate_to_constant` (PORTING_PLAN.md
//! §15). Random inputs with random real bindings; JS results (encoded as
//! {re, im} or null) are the oracle. Rust must match within tolerance.
//! Divergences are snapshotted. Regenerate:
//!   node scripts/generate-evaluate-corpus.mjs
//!   UPDATE_KNOWN_FAILURES=1 cargo test --test evaluate_corpus

use math_expressions::{evaluate, evaluate_to_constant, Expr, TextToAst, TextToAstOptions};
use num_complex::Complex64;
use std::collections::{BTreeSet, HashMap};

fn parse(s: &str) -> Option<Expr> {
    TextToAst::new(TextToAstOptions::default()).convert(s).ok()
}

fn catch<T>(f: impl FnOnce() -> T) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok()
}

#[derive(serde::Deserialize)]
struct Val {
    re: f64,
    im: f64,
}

#[derive(serde::Deserialize)]
struct Case {
    input: String,
    binds: HashMap<String, f64>,
    evaluated: Option<Val>,
    constant: Option<Val>,
}

const CORPUS: &str = include_str!("fixtures/evaluate-corpus.json");
const KNOWN_FAILURES: &str = include_str!("fixtures/evaluate-known-failures.json");

/// Compare a Rust result to the JS-recorded one. A generous relative tolerance
/// absorbs the tiny float drift between mathjs and `num_complex` (e.g. `powc`
/// on integer exponents). Both `None` agree; a `Some`/`None` mismatch fails.
fn agree(got: Option<Complex64>, want: &Option<Val>) -> bool {
    match (got, want) {
        (None, None) => true,
        (Some(g), Some(w)) => {
            let scale = 1.0 + g.re.abs() + g.im.abs() + w.re.abs() + w.im.abs();
            (g.re - w.re).abs() < 1e-6 * scale && (g.im - w.im).abs() < 1e-6 * scale
        }
        _ => false,
    }
}

fn collect_failures() -> BTreeSet<String> {
    let cases: Vec<Case> = serde_json::from_str(CORPUS).unwrap();
    let mut failures = BTreeSet::new();
    for c in &cases {
        let Some(e) = parse(&c.input) else { continue };
        let ev = catch(|| evaluate(&e, &c.binds)).flatten();
        let ct = catch(|| evaluate_to_constant(&e)).flatten();
        if !agree(ev, &c.evaluated) {
            failures.insert(format!("evaluate {}", c.input));
        }
        if !agree(ct, &c.constant) {
            failures.insert(format!("constant {}", c.input));
        }
    }
    failures
}

#[test]
fn evaluate_corpus_no_regressions() {
    std::panic::set_hook(Box::new(|_| {}));
    let failures = collect_failures();
    if std::env::var("UPDATE_KNOWN_FAILURES").is_ok() {
        let list: Vec<&String> = failures.iter().collect();
        std::fs::write(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/evaluate-known-failures.json"),
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
        "{} NEW evaluate divergences from JS:\n{}",
        new.len(),
        new.iter().take(40).map(|k| format!("  {k}")).collect::<Vec<_>>().join("\n"),
    );
}

#[test]
fn evaluate_corpus_pass_rate() {
    std::panic::set_hook(Box::new(|_| {}));
    let total = serde_json::from_str::<Vec<Case>>(CORPUS).unwrap().len() * 2;
    let failures = collect_failures().len();
    eprintln!(
        "evaluate corpus: {}/{} evaluate+constant checks match JS ({} failing)",
        total - failures,
        total,
        failures
    );
}
