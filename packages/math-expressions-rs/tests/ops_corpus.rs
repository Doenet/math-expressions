//! Differential corpus for `variables` and `substitute` (PORTING_PLAN.md §15).
//! `variables` must match JS's array *exactly* (order included); `substitute`
//! must match JS's result via `equals`. Regenerate:
//!   node scripts/generate-ops-corpus.mjs

use math_expressions::{
    equals, js_tree, substitute, variables, EqOptions, Expr, TextToAst, TextToAstOptions,
};
use serde_json::Value;
use std::collections::HashMap;

fn parse(s: &str) -> Option<Expr> {
    TextToAst::new(TextToAstOptions::default()).convert(s).ok()
}

fn catch<T>(f: impl FnOnce() -> T) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok()
}

#[derive(serde::Deserialize)]
struct Case {
    input: String,
    vars: Vec<String>,
    sub: Option<Sub>,
}

#[derive(serde::Deserialize)]
struct Sub {
    var: String,
    repl: String,
    tree: Value,
}

const CORPUS: &str = include_str!("fixtures/ops-corpus.json");

#[test]
fn variables_match_js_exactly() {
    std::panic::set_hook(Box::new(|_| {}));
    let cases: Vec<Case> = serde_json::from_str(CORPUS).unwrap();
    let mut diffs = Vec::new();
    for c in &cases {
        let Some(e) = parse(&c.input) else { continue };
        let got = catch(|| variables(&e));
        if got.as_ref() != Some(&c.vars) {
            diffs.push(format!("  {:?}: JS {:?} vs Rust {:?}", c.input, c.vars, got));
        }
    }
    assert!(
        diffs.is_empty(),
        "{} variables divergence(s):\n{}",
        diffs.len(),
        diffs.iter().take(30).cloned().collect::<Vec<_>>().join("\n"),
    );
}

#[test]
fn substitute_matches_js() {
    std::panic::set_hook(Box::new(|_| {}));
    let cases: Vec<Case> = serde_json::from_str(CORPUS).unwrap();
    let opts = EqOptions::default();
    let mut diffs = Vec::new();
    for c in &cases {
        let (Some(sub), Some(e)) = (&c.sub, parse(&c.input)) else {
            continue;
        };
        let Some(repl) = parse(&sub.repl) else { continue };
        let map = HashMap::from([(sub.var.clone(), repl)]);
        let want = js_tree::try_from_js(&sub.tree).expect("fixture tree");
        let ok = catch(|| equals(&substitute(&e, &map), &want, &opts)).unwrap_or(false);
        if !ok {
            diffs.push(format!("  {:?} [{}->{}]", c.input, sub.var, sub.repl));
        }
    }
    assert!(
        diffs.is_empty(),
        "{} substitute divergence(s):\n{}",
        diffs.len(),
        diffs.iter().take(30).cloned().collect::<Vec<_>>().join("\n"),
    );
}
