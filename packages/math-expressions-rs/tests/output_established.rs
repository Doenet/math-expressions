//! Established-output test for the AST → LaTeX / AST → text formatters.
//!
//! `ast-to-latex.json` (265) and `ast-to-text.json` (247) hold the JS reference
//! output for each AST. Rust's formatter is clean-slate, so a fixed set of cases
//! render differently *on purpose* — enumerated in
//! `active-plans/JS_RUST_TEST_DIVERGENCES.md` §2–§3 and snapshotted here in
//! `fixtures/ast-output-known-divergences.json`.
//!
//! This test asserts every fixture case either matches the JS established output or
//! is a snapshotted intentional divergence with the exact same Rust output. Any
//! **new** unlisted divergence, any change to a divergent output, and any stale
//! snapshot entry (a case that now matches, or whose AST left the fixture) all
//! fail — turning the old informational `zzz_divergence_probe` into a guard.
//!
//! After an intentional formatter change, re-bless the snapshot:
//!
//! ```text
//! BLESS=1 cargo test --test output_established
//! ```

use math_expressions::js_tree::try_from_js;
use math_expressions::{to_latex, to_text, LatexOpts, TextOpts};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(serde::Deserialize)]
struct OutCase {
    ast: Value,
    out: String,
}

/// One snapshotted intentional divergence.
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq)]
struct Divergence {
    kind: String,
    ast: String,
    js: String,
    rust: String,
}

const SNAPSHOT: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ast-output-known-divergences.json");

fn render_latex(v: &Value) -> String {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        to_latex(&try_from_js(v).expect("fixture tree"), &LatexOpts::default())
    }))
    .unwrap_or_else(|_| "<PANIC>".to_string())
}

fn render_text(v: &Value) -> String {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        to_text(&try_from_js(v).expect("fixture tree"), &TextOpts::default())
    }))
    .unwrap_or_else(|_| "<PANIC>".to_string())
}

/// Every case whose Rust output differs from the JS established output, keyed by
/// `"kind\0ast"` for a stable, collision-free lookup.
fn current_divergences() -> BTreeMap<String, Divergence> {
    let mut out = BTreeMap::new();
    let mut add = |kind: &str, fixture: &str, render: &dyn Fn(&Value) -> String| {
        let cases: Vec<OutCase> = serde_json::from_str(fixture).unwrap();
        for c in &cases {
            let got = render(&c.ast);
            if got != c.out {
                let ast = c.ast.to_string();
                out.insert(
                    format!("{kind}\0{ast}"),
                    Divergence { kind: kind.to_string(), ast, js: c.out.clone(), rust: got },
                );
            }
        }
    };
    add("latex", include_str!("fixtures/ast-to-latex.json"), &render_latex);
    add("text", include_str!("fixtures/ast-to-text.json"), &render_text);
    out
}

fn load_snapshot() -> BTreeMap<String, Divergence> {
    let raw = std::fs::read_to_string(SNAPSHOT)
        .unwrap_or_else(|e| panic!("read snapshot {SNAPSHOT}: {e} (run with BLESS=1 to create it)"));
    let list: Vec<Divergence> = serde_json::from_str(&raw).unwrap();
    list.into_iter().map(|d| (format!("{}\0{}", d.kind, d.ast), d)).collect()
}

#[test]
fn ast_output_matches_established_modulo_snapshot() {
    let current = current_divergences();

    if std::env::var("BLESS").is_ok() {
        let mut list: Vec<&Divergence> = current.values().collect();
        list.sort_by(|a, b| (&a.kind, &a.ast).cmp(&(&b.kind, &b.ast)));
        std::fs::write(SNAPSHOT, serde_json::to_string_pretty(&list).unwrap() + "\n").unwrap();
        eprintln!("blessed {} intentional divergences into {SNAPSHOT}", list.len());
        return;
    }

    let snapshot = load_snapshot();
    let mut problems = Vec::new();

    for (key, d) in &current {
        match snapshot.get(key) {
            None => problems.push(format!(
                "NEW divergence [{}] ast {}\n    js:   {}\n    rust: {}",
                d.kind, d.ast, d.js, d.rust
            )),
            Some(s) if s.rust != d.rust || s.js != d.js => problems.push(format!(
                "CHANGED divergence [{}] ast {}\n    js:   {} (was {})\n    rust: {} (was {})",
                d.kind, d.ast, d.js, s.js, d.rust, s.rust
            )),
            Some(_) => {}
        }
    }
    for key in snapshot.keys() {
        if !current.contains_key(key) {
            let s = &snapshot[key];
            problems.push(format!(
                "STALE snapshot entry [{}] ast {} — no longer diverges (matches JS now, or left the fixture)",
                s.kind, s.ast
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "{} formatter established-output problem(s) (re-bless with BLESS=1 if intentional):\n\n{}",
        problems.len(),
        problems.join("\n")
    );
}
