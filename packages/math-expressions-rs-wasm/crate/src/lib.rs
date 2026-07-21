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
mod matrix_ops;
mod numeric;
mod ode;
mod parse;
mod transform;

/// An opaque handle to a parsed math expression.
#[wasm_bindgen]
pub struct Expression(Expr);
