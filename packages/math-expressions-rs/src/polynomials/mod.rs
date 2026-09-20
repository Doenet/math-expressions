//! Exact polynomial algebra.
//!
//! - [`multivariate`] — recursive dense (SymPy DMP) GCD over ℚ, backing
//!   `reduce_rational` / `cancel`.
//! - [`univariate`] — dense `ℚ[t]` utilities for the `RootOf` pipeline and
//!   quotient-ring elimination.
//! - [`factor`] — univariate factorization over ℚ.
//! - [`rootof`] — the `RootOf` leaf: construction, power reduction, numeric eval.
//! - [`ratform`] — rational-function normal form (`together` / `cancel`).
//! - [`kernel`] — opaque subtrees (`sin x`, `π`) as fresh indeterminates, so a
//!   ring over ℚ can still answer questions about expressions that are not
//!   polynomials in it.
//! - [`compat`] — the sparse, expression-coefficient engine the JavaScript
//!   compatibility layer's polynomial API is written against.

mod multivariate;

pub mod compat;
pub mod factor;
pub(crate) mod kernel;
pub mod ratform;
pub(crate) mod rootof;
pub(crate) mod univariate;

pub(crate) use multivariate::*;
