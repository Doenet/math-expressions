//! Grading helpers beyond plain equality.
//!
//! Barrel module:
//!
//! - [`sign_errors`] — sign-error-tolerant comparison (`equalWithSignErrors`)
//! - [`linear`]      — isolating a variable in a relation, and reading an
//!   expression as an affine combination of named variables
//! - [`membership`]  — finite-set membership evaluated to a truth value

pub mod linear;
pub mod membership;
pub mod sign_errors;

pub use linear::{linear_decomposition, solve_linear};
pub use membership::evaluate_membership;
pub use sign_errors::{equal_specified_sign_errors, equal_with_sign_errors};
