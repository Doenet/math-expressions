//! WebAssembly / JavaScript bindings for the `math-expressions` core
//! (PORTING_PLAN.md §13).
//!
//! This crate is the JS boundary and nothing else: it is a thin `wasm-bindgen`
//! adapter over the public API of the `math-expressions` crate (its path
//! dependency), compiled to a `cdylib` for the `wasm32-unknown-unknown` target.
//! It holds no math logic of its own.
//!
//! A single opaque `Expression` handle owns a parsed tree; JS calls parse once
//! and then invokes methods. Read-only methods take `&self`; transforming
//! methods return a fresh `Expression`. Only primitives and strings cross the
//! boundary, so there is no tree-serialisation overhead.
//!
//! Barrel crate. The central [`Expression`] handle is defined here (every
//! submodule adds `#[wasm_bindgen] impl Expression` blocks to it); the bindings
//! themselves are grouped by feature in the submodules:
//!
//! - [`parse`]       — text/LaTeX parsing (plain and with JS-style options)
//! - [`core_ops`]    — render, equality, algebra, evaluation, arithmetic builders
//! - [`grading`]     — equality variants, structural comparison, analyticity
//! - [`transform`]   — simplification / units / normalization passes
//! - [`matrix_ops`]  — matrix & vector operations, eigen-decomposition
//! - [`calculus`]    — integration and arbitrary-precision evaluation
//! - [`ode`]         — ODE solving (numeric and expression-RHS)
//! - [`numeric`]     — f64 numeric utilities (the `me.math` replacements)
//! - [`interop`]     — JS-tree AST boundary (Doenet interop)
//! - [`tree_ops`]    — JS-tree operations the compat layer needs without an
//!   `Expression`'s normalization (default order, not-pushdown, linear solving)
//! - [`poly_ops`]    — the compat polynomial / Gröbner engine (JSON in, JSON out)
//! - [`js_match`]    — the JS-tree template-match / flatten-unflatten engine
//!   backing [`interop`] (JS-shape only, so it lives here rather than in the
//!   core crate)
//! - [`assumptions`] — the mutable `Assumptions` handle and related builders

// Like the core crate: the barrel docs above name this crate's private
// submodules, which resolve only under `--document-private-items`. Silencing
// the lint keeps any *real* broken-link warning visible.
#![allow(rustdoc::private_intra_doc_links)]

use math_expressions::Expr;
use wasm_bindgen::prelude::*;

mod assumptions;
mod calculus;
mod constants;
mod core_ops;
mod grading;
mod interop;
mod js_match;
mod limits;
mod matrix_ops;
mod numeric;
mod ode;
mod parse;
mod poly_ops;
mod transform;
mod tree_ops;

/// An opaque handle to a parsed math expression.
///
/// The handle **carries the notation it was parsed with** (field 1): every
/// render — `to_text`/`to_latex` and the JSON payloads of matrix/calculus
/// endpoints — defaults to that notation, and derived expressions
/// (`simplify()`, arithmetic builders, …) inherit it. The `*_with_options`
/// render methods override it per call. This is what keeps a comma-decimal
/// session comma-decimal end-to-end without re-supplying the notation at
/// every call site.
#[wasm_bindgen]
pub struct Expression(Expr, math_expressions::NumberNotation);

impl Expression {
    /// Wrap a derived expression, inheriting this handle's notation.
    fn derive(&self, expr: Expr) -> Expression {
        Expression(expr, self.1.clone())
    }

    /// Wrap an expression that has no parse provenance (default notation).
    fn with_default_notation(expr: Expr) -> Expression {
        Expression(expr, Default::default())
    }

    /// Render an arbitrary expression in this handle's notation (for JSON
    /// payloads that embed rendered components).
    fn text_of(&self, e: &Expr) -> String {
        math_expressions::to_text(
            e,
            &math_expressions::TextOpts {
                notation: self.1.clone(),
                ..Default::default()
            },
        )
    }
}

/// Report a Rust panic to the JS console before the module traps
/// (DOENET_INTEGRATION item 2).
///
/// A panic in wasm reaches the browser as `RuntimeError: unreachable executed`
/// and nothing else, which makes a failing `assert_eq!` unreadable — you can
/// see *that* something tripped, never *which* thing or with what values. That
/// was long blamed on `panic = "abort"` in the release profile. It is not the
/// cause: std runs the panic hook before aborting, and the payload survives the
/// size-oriented profile intact. What was missing is a hook at all — the
/// default one writes to stderr, which on `wasm32-unknown-unknown` goes
/// nowhere.
///
/// So this is unconditional rather than gated behind a diagnostic build: it
/// measured 1,958 bytes (0.14%) on the shipped binary, which is not a price
/// worth making anyone opt into. It does not change *behaviour* — the trap
/// still happens, right after — only whether the trap says anything. Written
/// against `wasm-bindgen` directly rather than pulling in
/// `console_error_panic_hook`, to leave the dependency set alone.
mod panic_report {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console)]
        fn error(msg: String);
    }

    /// Installed automatically at module instantiation.
    #[wasm_bindgen(start)]
    pub fn install() {
        // `PanicHookInfo`'s Display already carries the location and the
        // payload — the same text the native runtime prints — so this only
        // labels which module it came from.
        std::panic::set_hook(Box::new(|info| {
            error(format!("[math-expressions wasm] {info}"));
        }));
    }

    /// Panic on purpose, to check that a harness actually surfaces the message.
    /// Worth running first when a trap reports nothing: that looks identical
    /// whether the hook is missing or the console output is being swallowed
    /// somewhere upstream, and this tells the two apart before anyone starts
    /// hunting a real panic.
    ///
    /// Behind the `debug-panics` feature (`build-wasm.sh --debug`) so a call
    /// that kills the worker cannot be made against a shipped build.
    #[cfg(feature = "debug-panics")]
    #[wasm_bindgen]
    pub fn debug_panic_selftest() {
        assert_eq!(2 + 2, 5, "the panic hook reports assertion messages");
    }
}

/// Free the handle's tree iteratively (STACK_SAFETY_PLAN item 21). A handle can
/// hold an adversarially deep tree — `((((…))))` from student input — whose
/// ordinary recursive `Drop` would blow the ~1 MB wasm shadow stack and, under
/// `panic = "abort"`, kill the worker. `tear_down` dismantles it with a heap
/// worklist first, leaving `self.0` a shallow shell for the ordinary drop.
impl Drop for Expression {
    fn drop(&mut self) {
        math_expressions::tear_down(&mut self.0);
    }
}

/// The number of distinct symbol names interned this session — a memory gauge
/// for the long-lived worker (item 8). The interner is append-only (a `Sym` is
/// a raw index into it), so this only grows; it lets the host measure symbol
/// growth before committing to the generational-`Sym` redesign true eviction
/// would need.
#[wasm_bindgen]
pub fn interner_size() -> usize {
    math_expressions::interner_len()
}

#[cfg(test)]
mod notation_carry_tests {
    /// The notation an expression was parsed with must follow it through
    /// rendering and derived results without being re-supplied.
    #[test]
    fn expression_carries_parse_notation_through_derives() {
        let json = r#"{"notation":{"decimalSeparator":","}}"#;
        let e = crate::parse::parse_text_with_options("1,5 + x", json).expect("parse");
        assert!(
            e.to_text().contains("1,5"),
            "render must use the parse notation: {}",
            e.to_text()
        );
        // Derived expressions inherit the notation.
        let s = e.simplify();
        assert!(
            s.to_text().contains("1,5"),
            "derived render must inherit notation: {}",
            s.to_text()
        );
        // Plain parses stay period-decimal.
        let p = crate::parse::parse_text("1.5 + x").expect("parse");
        assert!(p.to_text().contains("1.5"), "{}", p.to_text());
    }
}
