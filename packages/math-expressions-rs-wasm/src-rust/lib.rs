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
//! - [`assumptions`] — the mutable `Assumptions` handle and related builders

use math_expressions::Expr;
use wasm_bindgen::prelude::*;

mod assumptions;
mod calculus;
mod core_ops;
mod grading;
mod interop;
mod limits;
mod matrix_ops;
mod numeric;
mod ode;
mod parse;
mod transform;

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
