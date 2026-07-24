//! Adversarial parser inputs (PARSER_FUEL_PLAN.md, Part B1).
//!
//! Every pathological / malformed string here must **terminate** — return
//! `Ok` or `Err`, never panic, never hang — for BOTH the text and LaTeX
//! parsers. This is the permanent regression guard for the `\begin{bmatrix}`
//! infinite loop and, more generally, for the parse-fuel backstop
//! (`ResourceLimits::max_parse_steps` + `tick()` in every parser loop).
//!
//! Deterministic: a fixed corpus, run in source order, identical every time —
//! no randomness. Each parse runs on a worker thread with a timeout, so a
//! *missing* fuel tick (a future regression) surfaces as a test failure, not a
//! hung CI job.

use math_expressions::resource_limits::{self, ResourceLimits};
use math_expressions::{LatexToAst, LatexToAstOptions, TextToAst, TextToAstOptions};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn text_ok(s: &str) -> bool {
    TextToAst::new(TextToAstOptions::default()).convert(s).is_ok()
}
fn latex_ok(s: &str) -> bool {
    LatexToAst::new(LatexToAstOptions::default()).convert(s).is_ok()
}

/// Run `f` on a worker thread; fail if it panics or does not finish in time.
/// The default parse-step budget bounds every (ticked) loop, so this only
/// times out if a loop was left un-ticked — exactly the regression to catch.
fn must_terminate(label: String, f: impl FnOnce() + Send + 'static) {
    let (tx, rx) = mpsc::channel();
    // Match the main-thread stack the recursion-depth cap (`MAX_PARSE_DEPTH`)
    // is tuned for; the default 2 MiB worker stack overflows on deep nesting
    // that the parser itself handles fine (it errors at the depth cap).
    let handle = thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            let _ = tx.send(r.is_ok());
        })
        .expect("spawn worker thread");
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(true) => {
            let _ = handle.join();
        }
        Ok(false) => panic!("parser PANICKED on {label:?}"),
        Err(_) => panic!("parser HUNG (>10s) on {label:?} — a loop is missing its fuel tick()"),
    }
}

/// Both parsers must terminate on `s`.
fn assert_terminates(s: &str) {
    let (a, b) = (s.to_string(), s.to_string());
    must_terminate(s.to_string(), move || {
        let _ = text_ok(&a);
    });
    must_terminate(s.to_string(), move || {
        let _ = latex_ok(&b);
    });
}

/// Hand-written pathological inputs, grouped by failure mode.
const CORPUS: &[&str] = &[
    // --- unclosed / mismatched environments (the reported freeze) ---
    r"\begin{bmatrix}",
    r"\begin{bmatrix} 1",
    r"\begin{bmatrix} 1 & 2",
    r"\begin{bmatrix} 1 & 2 \\ 3",
    r"\begin{bmatrix}\end{pmatrix}",
    r"\begin{a}\begin{b}\end{b}",
    r"\end{bmatrix}",
    r"\begin{",
    r"\begin",
    r"\begin{bmatrix",
    // --- long unary / postfix chains ---
    "----x",
    "!!!!x",
    "x^^^^",
    "x____",
    "+++++x",
    "x!!!!",
    // --- unbalanced delimiters ---
    "(1",
    "[1",
    "{1",
    "|1",
    r"\left( 1",
    "((((((((((",
    // --- stray environment / structural tokens ---
    "&",
    r"\\",
    "1 & 2",
    r"1 \\ 2",
    "^",
    "_",
    "!",
    // --- junk / lone escapes ---
    r"\",
    r"\frac",
    r"\sqrt",
    r"\frac{",
    r"\sqrt[",
];

#[test]
fn adversarial_corpus_terminates() {
    for &s in CORPUS {
        assert_terminates(s);
    }
    // Bulk / deep variants built here to keep the corpus table readable. These
    // exercise the recursion-depth cap and the loop-fuel backstop; they all
    // return promptly (an error at the depth cap, or a bounded parse).
    assert_terminates(&"(".repeat(5_000)); // deep nesting → depth cap errors
    assert_terminates(&r"\frac{".repeat(2_000)); // deep recursion → depth cap
    assert_terminates(&r"\sqrt{".repeat(2_000));
    assert_terminates(&"!".repeat(20_000)); // postfix run → depth cap errors
    // Unclosed matrix with many entries: without the EOF exit this looped
    // forever; now it breaks at EOF (and fuel would catch it regardless).
    assert_terminates(&(r"\begin{bmatrix}".to_string() + &"1 & ".repeat(20_000)));
    // NOTE: `"^".repeat(N)` for large N is deliberately NOT here — it exposes a
    // SEPARATE, pre-existing bug (deep `Pow` AST from the loop-based caret
    // handler is not counted against the depth cap, so later recursive tree
    // processing overflows the stack ~N≥4000). Captured by the ignored
    // `superscript_nesting_overflows_known_bug` below; unrelated to the loop
    // hang this suite guards.
    assert_terminates(&"^".repeat(200)); // safe depth; still exercises the caret loop
}

/// KNOWN PRE-EXISTING BUG (found by this adversarial suite, not introduced by
/// the parse-fuel work): the caret handler builds an arbitrarily deep `Pow`
/// tree in a loop without charging the recursion-depth budget, so a very deep
/// superscript chain (`^^^^…`) parses "successfully" into a tree whose later
/// recursive processing (Drop / normalize / output) overflows the stack. The
/// fix is to count loop-built nesting against `MAX_PARSE_DEPTH` (or bound AST
/// depth) — a separate change from the loop-fuel backstop. Un-ignore once
/// fixed; it should then return `Err` ("too deeply nested") like `!`×N does.
#[test]
#[ignore = "pre-existing deep-Pow AST overflow; needs depth accounting for loop-built nesting"]
fn superscript_nesting_overflows_known_bug() {
    let deep = "^".repeat(50_000);
    assert!(
        LatexToAst::new(LatexToAstOptions::default())
            .convert(&deep)
            .is_err(),
        "a 50000-deep superscript chain should be refused at the depth cap"
    );
}

/// The exact reported freeze: an opened-but-unclosed matrix environment must be
/// a parse *error*, returned promptly — not an infinite loop.
#[test]
fn begin_bmatrix_is_an_error_not_a_hang() {
    must_terminate(r"\begin{bmatrix}".to_string(), || {
        let r = LatexToAst::new(LatexToAstOptions::default()).convert(r"\begin{bmatrix}");
        assert!(r.is_err(), "unclosed \\begin{{bmatrix}} must be a parse error");
    });
}

/// A *valid* matrix still parses — the fuel ticks did not break normal input.
#[test]
fn valid_matrix_still_parses() {
    assert!(latex_ok(r"\begin{bmatrix} 1 & 2 \\ 3 & 4 \end{bmatrix}"));
    assert!(latex_ok(r"\frac{x}{y} + \sqrt{x}"));
    assert!(text_ok("sin^2(x) + cos^2(x) + 1"));
}

/// The step budget is enforced deterministically: under a tiny cap a
/// long flat sum trips the fuel and returns the same `Err` every run.
#[test]
fn parse_step_budget_is_enforced_deterministically() {
    let tiny = ResourceLimits {
        max_parse_steps: 100,
        ..ResourceLimits::default()
    };
    let input = "1".to_string() + &"+1".repeat(1_000); // ~1000 addition-loop iterations

    let run = || {
        resource_limits::with(tiny, || {
            TextToAst::new(TextToAstOptions::default())
                .convert(&input)
                .is_err()
        })
    };
    assert!(run(), "a 1000-term sum must exceed a 100-step budget");
    assert_eq!(run(), run(), "budget enforcement must be deterministic");

    // The same input parses fine under the default (generous) budget.
    assert!(text_ok(&input));
}
