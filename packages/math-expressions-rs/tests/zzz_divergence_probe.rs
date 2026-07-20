//! TEMPORARY probe: enumerate every JS-fixture case whose Rust output differs.
//! Not a real test — writes a report and always passes.

use math_expressions::js_tree::from_js;
use math_expressions::{to_latex, to_text, LatexOpts, TextOpts};
use serde_json::Value;
use std::fmt::Write as _;

#[derive(serde::Deserialize)]
struct OutCase {
    ast: Value,
    out: String,
}

fn probe(name: &str, fixture: &str, render: impl Fn(&Value) -> Result<String, String>) -> String {
    let cases: Vec<OutCase> = serde_json::from_str(fixture).unwrap();
    let mut report = String::new();
    let mut diffs = 0usize;
    let mut panics = 0usize;
    for c in &cases {
        match render(&c.ast) {
            Ok(got) => {
                if got != c.out {
                    diffs += 1;
                    let _ = write!(
                        report,
                        "- ast: `{}`\n  - js:   `{}`\n  - rust: `{}`\n",
                        c.ast, c.out, got
                    );
                }
            }
            Err(_) => {
                panics += 1;
                let _ = write!(report, "- ast: `{}`\n  - js:   `{}`\n  - rust: PANIC/ERR\n", c.ast, c.out);
            }
        }
    }
    format!(
        "## {name}\n\n{diffs}/{} differ ({panics} panicked)\n\n{report}\n",
        cases.len()
    )
}

#[test]
fn probe_ast_output_divergences() {
    let latex = probe(
        "ast-to-latex",
        include_str!("fixtures/ast-to-latex.json"),
        |v| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                to_latex(&from_js(v), &LatexOpts::default())
            }))
            .map_err(|_| "panic".to_string())
        },
    );
    let text = probe(
        "ast-to-text",
        include_str!("fixtures/ast-to-text.json"),
        |v| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                to_text(&from_js(v), &TextOpts::default())
            }))
            .map_err(|_| "panic".to_string())
        },
    );
    let out = format!("{latex}\n{text}");
    std::fs::write(
        "/tmp/claude-1000/-workspaces-math-expressions/575c72cf-ae26-4f16-87e1-796ac5aedf11/scratchpad/ast-output-divergences.md",
        &out,
    )
    .unwrap();
    // print summary lines
    for line in out.lines().filter(|l| l.contains("differ")) {
        eprintln!("{line}");
    }
}
