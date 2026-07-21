//! Matrix operations (MATRIX_PLAN.md). The arithmetic itself (entrywise sums,
//! segmented non-commutative products, powers) lives in the canonical layer's
//! smart constructors (`norm::add`/`mul`/`pow`); the functions here are the
//! eager eponymous operations, which evaluate on literal matrices and return
//! an opaque `OtherOp` on anything else (same policy as the derivative
//! catch-all: never a wrong answer, always a renderable residual).
//!
//! Barrel module — the implementation lives in the submodules below:
//!
//! - [`ops`]        — Layer 1 eager ops: transpose, trace, matmul
//! - [`vector`]     — vector arithmetic: add/sub/dot/cross
//! - [`linalg`]     — det / inverse / rref / rank / nullspace
//! - [`kernels`]    — shared numeric elimination + cofactor/Bareiss kernels
//! - [`eigen`]      — char poly, eigenvalues
//! - [`eigenvectors`] — eigenvectors over the quotient ring ℚ[t]/(f)

mod eigen;
mod eigenvectors;
mod kernels;
mod linalg;
mod ops;
mod vector;

pub use eigen::{char_poly, eigenvalues};
pub use eigenvectors::{eigenvectors, EigenPair};
pub use linalg::{det, matrix_inverse, nullspace, rank, rref};
pub use ops::{matmul, trace, transpose};
pub use vector::{cross_prod, dot_prod, vector_add, vector_sub};

// Used by the canonical `pow` to fold `A^(-k)` (see `norm::pow`).
pub(crate) use linalg::invert_rational_literal;
