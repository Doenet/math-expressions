//! Fixture-driven LaTeX parser tests. Fixtures are auto-extracted from the JS
//! spec data maps by scripts/extract-fixtures.mjs — do not edit them by hand.

use math_expressions::js_tree::to_js;
use math_expressions::{LatexToAst, LatexToAstOptions};
use serde_json::Value;

#[derive(serde::Deserialize)]
struct TreeCase {
    input: String,
    tree: Value,
}

#[derive(serde::Deserialize)]
struct ErrorCase {
    input: String,
    error: String,
}

#[test]
fn latex_to_ast_fixtures() {
    let cases: Vec<TreeCase> =
        serde_json::from_str(include_str!("fixtures/latex-to-ast.json")).unwrap();
    let mut failures = vec![];

    for case in &cases {
        let mut converter = LatexToAst::new(LatexToAstOptions::default());
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            converter.convert(&case.input)
        })) {
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

#[test]
fn latex_to_ast_error_fixtures() {
    let cases: Vec<ErrorCase> =
        serde_json::from_str(include_str!("fixtures/latex-to-ast-errors.json")).unwrap();
    let mut failures = vec![];

    for case in &cases {
        let mut converter = LatexToAst::new(LatexToAstOptions::default());
        match converter.convert(&case.input) {
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
