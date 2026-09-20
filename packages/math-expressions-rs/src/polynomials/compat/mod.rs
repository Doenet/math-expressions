//! The polynomial engine behind the JavaScript compatibility layer.
//!
//! This is a second polynomial implementation, and it exists because it answers
//! a different question from [`multivariate`](super::multivariate). That one is
//! recursive-dense over ℚ in named variables, and is the right tool for
//! cancelling a rational function. This one is *sparse*, its variables are
//! arbitrary expression trees, and its coefficients are expressions rather than
//! rationals — `t^1000000000`, `sin(x)` as a variable and `−π` as a coefficient
//! are all cases the compat API is asked about directly.
//!
//! It is also written to a fixed AST spelling — `["polynomial", v, [[d, c], …]]`
//! — which callers compare structurally, so the *shape* of a result is part of
//! the contract, not only its value. [`wire`] holds that spelling.
//!
//! - [`rep`] — the representation and its coefficient arithmetic
//! - [`arith`] — ring operations, and conversion back to an expression
//! - [`mono`] — monomials and the lexicographic order
//! - [`divide`] — division with remainder, and inter-reduction
//! - [`groebner`] — Buchberger, and the gcd / lcm / rational reduction it backs
//! - [`convert`] — reading an expression as a polynomial
//! - [`wire`] — the AST codec

mod arith;
mod convert;
mod divide;
mod groebner;
mod mono;
mod rep;
mod wire;

pub use arith::{
    polynomial_add, polynomial_mul, polynomial_neg, polynomial_pow, polynomial_sub,
    polynomial_to_expression,
};
pub use convert::expression_to_polynomial;
pub use divide::{poly_div, reduce, reduce_ith};
pub use groebner::{poly_gcd, poly_lcm, reduce_rational_expression, reduced_grobner};
pub use mono::{
    initial_term, max_div_init, mono_div, mono_gcd, mono_is_div, mono_less_than, mono_to_poly,
};
pub use rep::{Mono, Poly};
pub use wire::{
    mono_from_json, mono_to_json, monos_from_json, poly_from_json, poly_to_json, polys_from_json,
    polys_to_json,
};
