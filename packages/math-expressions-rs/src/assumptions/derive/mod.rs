//! The facts that follow from the stored ones, and the operator algebra behind
//! them.

mod combine;
mod derived;
mod for_expr;
mod relops;

pub(crate) use derived::calculate_derived_assumptions;
pub(crate) use for_expr::get_assumptions_for_expr;
