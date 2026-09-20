//! Tiered [`Number`] type.
//!
//! The type and its exact/float arithmetic live in [`number`]; decimal parsing
//! and rendering in [`decimal`]; integer GCD in [`gcd`].

mod decimal;
mod gcd;
mod number;

pub use number::{BigNumber, Number, Spelling, F64};

// Used by the printer's f64 shortest-round-trip rendering.
pub(crate) use decimal::{
    js_exponential_parts, js_f64_to_string, positional_from_digits, shortest_digits,
};
