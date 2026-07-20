//! Shared fixture-test harness for the parser integration tests.

use math_expressions::js_tree::to_js;
use math_expressions::{Expr, ParseError};
use serde_json::Value;

#[derive(serde::Deserialize)]
pub struct TreeCase {
    pub input: String,
    pub tree: Value,
}

#[derive(serde::Deserialize)]
pub struct ErrorCase {
    pub input: String,
    pub error: String,
}

/// Run every tree case through `parse`, comparing the js_tree encoding of the
/// result with the fixture. Panics with a report of all failures.
pub fn run_tree_cases(fixture_json: &str, mut parse: impl FnMut(&str) -> Result<Expr, ParseError>) {
    let cases: Vec<TreeCase> = serde_json::from_str(fixture_json).unwrap();
    assert!(!cases.is_empty(), "fixture file is empty");
    let mut failures = vec![];

    for case in &cases {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse(&case.input))) {
            Ok(Ok(expr)) => {
                let got = to_js(&expr);
                if got != case.tree {
                    failures.push(format!(
                        "{:?}\n  expected: {}\n  got:      {}",
                        case.input, case.tree, got
                    ));
                }
            }
            Ok(Err(e)) => failures.push(format!("{:?}\n  parse error: {}", case.input, e)),
            Err(_) => failures.push(format!("{:?}\n  PANICKED", case.input)),
        }
    }

    if !failures.is_empty() {
        panic!(
            "{}/{} fixture cases failed:\n\n{}",
            failures.len(),
            cases.len(),
            failures
                .iter()
                .take(40)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Run every error case through `parse`, expecting a ParseError whose message
/// contains the fixture text (vitest's toThrow(msg) checks containment).
pub fn run_error_cases(
    fixture_json: &str,
    mut parse: impl FnMut(&str) -> Result<Expr, ParseError>,
) {
    let cases: Vec<ErrorCase> = serde_json::from_str(fixture_json).unwrap();
    assert!(!cases.is_empty(), "fixture file is empty");
    let mut failures = vec![];

    for case in &cases {
        match parse(&case.input) {
            Ok(expr) => failures.push(format!(
                "{:?}: expected error {:?}, parsed as {}",
                case.input,
                case.error,
                to_js(&expr)
            )),
            Err(e) => {
                if !e.message.contains(&case.error) {
                    failures.push(format!(
                        "{:?}: expected error containing {:?}, got {:?}",
                        case.input, case.error, e.message
                    ));
                }
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "{}/{} error cases failed:\n{}",
            failures.len(),
            cases.len(),
            failures.join("\n")
        );
    }
}
