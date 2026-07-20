//! Stack-safety tests (PORTING_PLAN.md §6e). Untrusted deep input must yield a
//! clean `ParseError`, never a stack-overflow trap — which on wasm32 (1 MB
//! shadow stack) kills the whole instance.
//!
//! Note on stack sizes: recursive-descent parsing is intrinsically stack-heavy
//! (~40 KB/level in debug, ~15 KB/level in release), so *reaching* the depth
//! cap costs real stack. The cap (~12 bracket levels) is sized for the release
//! wasm target (well under 1 MB); these debug tests therefore run in threads
//! generous enough to reach the cap in debug, and assert the cap fires with an
//! error rather than overflowing. The post-parse passes have small frames and
//! are checked separately in a tiny stack.

use math_expressions::{
    canonicalize, equals, EqOptions, Expr, LatexToAst, LatexToAstOptions, TextToAst,
    TextToAstOptions,
};

fn parse_text(s: &str) -> Result<Expr, String> {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .map_err(|e| e.to_string())
}
fn parse_latex(s: &str) -> Result<Expr, String> {
    LatexToAst::new(LatexToAstOptions::default())
        .convert(s)
        .map_err(|e| e.to_string())
}

/// Run `f` in a thread with a `kb`-KB stack, returning its value (propagating
/// panics). If `f` overflows, the process aborts — which is the failure we are
/// guarding against.
fn in_stack<T: Send + 'static>(kb: usize, f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(kb * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .expect("closure overflowed its stack")
}

/// Deeply nested brackets, prefix-sign chains, NOT chains, absolute values,
/// and LaTeX fractions all error rather than crashing. Run in a 4 MB thread:
/// reaching the ~12-level cap costs real stack in debug, and a *broken* cap
/// would recurse to 100 000 levels and overflow any stack.
#[test]
fn deep_input_errors_not_crashes() {
    in_stack(4096, || {
        let n = 100_000;
        // These bottom out directly on the depth cap.
        for input in [
            "(".repeat(n) + "x" + &")".repeat(n),
            "-".repeat(n) + "x",
            "!".repeat(n) + "x",
        ] {
            let err = parse_text(&input).expect_err("deep input should error, not crash");
            assert!(
                err.contains("deeply nested"),
                "expected depth error, got: {err}"
            );
        }
        // Nested `|…|` is resolved by the bar-fallback to a different (still
        // clean) error — the guarantee is *a* ParseError, never a crash.
        let bars = "|".repeat(n) + "x" + &"|".repeat(n);
        parse_text(&bars).expect_err("deep bars should error, not crash");

        let latex = "\\frac{".repeat(n) + "1" + &"}{2}".repeat(n);
        let err = parse_latex(&latex).expect_err("deep latex should error");
        assert!(err.contains("deeply nested"), "got: {err}");
    });
}

/// Reasonable nesting (well under the cap, far past real usage of ~2) parses —
/// the cap is a safety limit, not a rejection of ordinary input.
#[test]
fn reasonable_nesting_parses() {
    in_stack(4096, || {
        let d = 10;
        let grouped = "(".repeat(d) + "x+1" + &")".repeat(d);
        assert!(
            parse_text(&grouped).is_ok(),
            "10-deep grouping should parse"
        );
        let nested = "f(".repeat(d) + "x" + &")".repeat(d);
        assert!(parse_text(&nested).is_ok(), "10-deep f(...) should parse");
    });
}

/// The post-parse passes (`canonicalize`, `cmp`, `equals`) and the derived
/// `Drop` are stack-cheap: a deep tree flows through the whole pipeline in a
/// 256 KB stack — unlike the parser, whose frames are large. The tree is built
/// iteratively (no parser, no recursion) in the same thread, since the `Sym`
/// interner is thread-local (single-threaded WASM) and cannot cross threads.
#[test]
fn post_parse_passes_fit_tiny_stack() {
    in_stack(256, || {
        // Wrap a shallow leaf in 40 applications: an Expr ~40 levels deep,
        // far past what the parser cap admits, built without recursion.
        let d = 40;
        let wrap = |leaf: Expr| {
            let mut e = leaf;
            for _ in 0..d {
                e = Expr::Apply(Box::new(Expr::sym("f")), vec![e]);
            }
            e
        };
        let a = wrap(parse_text("x+1").unwrap());
        let b = wrap(parse_text("1+x").unwrap());
        let _ = canonicalize(&a); // recurses 40 deep
        assert!(equals(&a, &b, &EqOptions::default()));
        // a/b dropped here — the derived recursive Drop runs in 256 KB too.
    });
}
