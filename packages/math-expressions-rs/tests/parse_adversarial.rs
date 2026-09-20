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
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .is_ok()
}
fn latex_ok(s: &str) -> bool {
    LatexToAst::new(LatexToAstOptions::default())
        .convert(s)
        .is_ok()
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
    // Deep caret runs: the loop-based caret handler now charges each `Pow`
    // level against `MAX_PARSE_DEPTH`, so these error at the cap instead of
    // building a spine a later recursive pass overflows on. See
    // `superscript_nesting_is_charged_against_depth_cap` for the assertion.
    assert_terminates(&"^".repeat(200));
    assert_terminates(&"^".repeat(50_000));
}

/// Regression: loop-built postfix nesting (`^^^…`, `!!!…`, `'''…`) is charged
/// against `MAX_PARSE_DEPTH`, exactly like the recursive-descent depth cap.
///
/// Before the fix, the caret/factorial/prime handlers wrapped `result` one
/// level deeper per loop iteration WITHOUT touching the depth budget, so a deep
/// chain parsed "successfully" into a `Pow`/`factorial`/`Prime` spine whose
/// later recursive processing (Drop / normalize / output) overflowed the stack
/// — and on wasm32 a stack-overflow trap kills the whole module instance, so
/// one adversarial answer bricks the engine for the session. Now the chain is
/// refused with a clean `ParseError`, in BOTH parsers, like `((…))` already was.
#[test]
fn superscript_nesting_is_charged_against_depth_cap() {
    // A chain far past the cap must be a clean error, never a would-be trap.
    let chains = [
        "^".repeat(50_000),                 // \blank^\blank^… (Pow spine)
        format!("x{}", "!".repeat(50_000)), // x!!!… (factorial spine)
        format!("x{}", "'".repeat(50_000)), // x'''… (prime spine)
    ];
    for deep in &chains {
        assert!(
            LatexToAst::new(LatexToAstOptions::default())
                .convert(deep)
                .is_err(),
            "a 50000-deep postfix chain must be refused at the depth cap (latex): {:?}",
            &deep[..deep.len().min(12)]
        );
        assert!(
            TextToAst::new(TextToAstOptions::default())
                .convert(deep)
                .is_err(),
            "a 50000-deep postfix chain must be refused at the depth cap (text): {:?}",
            &deep[..deep.len().min(12)]
        );
    }

    // Real educational input — a handful of levels — still parses fine.
    assert!(text_ok("sin^2(x) + cos^2(x)"));
    assert!(latex_ok(r"\sin^2 x"));
    assert!(text_ok("x^2^3"));
    assert!(text_ok("f''(x)"));
}

/// The exact reported freeze: an opened-but-unclosed matrix environment must be
/// a parse *error*, returned promptly — not an infinite loop.
#[test]
fn begin_bmatrix_is_an_error_not_a_hang() {
    must_terminate(r"\begin{bmatrix}".to_string(), || {
        let r = LatexToAst::new(LatexToAstOptions::default()).convert(r"\begin{bmatrix}");
        assert!(
            r.is_err(),
            "unclosed \\begin{{bmatrix}} must be a parse error"
        );
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
