//! Matrix operations. The arithmetic itself (entrywise sums,
//! segmented non-commutative products, powers) lives in the canonical layer's
//! smart constructors (`normalize::add`/`mul`/`pow`); the functions here are the
//! eager eponymous operations, which evaluate on literal matrices and return
//! an opaque `OtherOp` on anything else (same policy as the derivative
//! catch-all: never a wrong answer, always a renderable residual).
//!
//! Barrel module — the implementation lives in the submodules below:
//!
//! - [`ops`]        — eager ops: transpose, trace, matmul
//! - [`vector`]     — vector arithmetic: add/sub/dot/cross
//! - [`linalg`]     — det / inverse / rref / rank / nullspace
//! - [`elimination`] — shared elimination + cofactor/Bareiss kernels
//! - [`eigen`]      — char poly, eigenvalues
//! - [`eigenvectors`] — eigenvectors over the quotient ring `ℚ[t]/(f)`

mod eigen;
mod eigenvectors;
mod elimination;
mod linalg;
mod ops;
mod vector;

pub use eigen::{char_poly, eigenvalues};
pub use eigenvectors::{eigenvectors, EigenPair};
pub use linalg::{det, matrix_inverse, nullspace, rank, rref};
pub use ops::{matmul, trace, transpose};
pub use vector::{cross_prod, dot_prod, scalar_mul, vector_add, vector_sub};

// Used by the canonical `pow` to fold `A^(-k)` (see `normalize::pow`).
pub(crate) use linalg::invert_rational_literal;

use crate::expr::Expr;

/// The scalar an application of `det`/`trace` to a literal matrix denotes:
/// `det([[1,2],[3,4]])` → `-2`, `trace([[x,2],[3,y]])` → `x + y`. `None` for
/// any other head, for a non-matrix argument, and for a matrix the reducers
/// decline (non-square, or a dimension over `resource_limits`) — those come
/// back as the opaque `OtherOp` residual described in this module's header,
/// which is not a scalar.
///
/// This exists because *two* layers have to agree about which applications have
/// a scalar value: [`normalize::fold_apply`](crate::normalize) folds them, and
/// the equality sampler [`eval_numeric::complex`](crate::eval_numeric) decides
/// from the same question whether to evaluate an application or sample it as an
/// unknown. When they disagreed, `det([[1,2],[3,4]])` simplified to `-2` and
/// compared **unequal** to `-2`: the sampler had never heard of `det`, so it
/// drew a fresh random value for the whole application. The scalar `eval1`
/// kernels on `DET`/`TRACE` cannot serve here — they are mathjs's `det(2) = 2`
/// convention for a *non*-matrix argument and cannot see inside a `Matrix`.
pub(crate) fn scalar_reduction(head: &Expr, args: &[Expr]) -> Option<Expr> {
    let (Expr::Sym(s), [arg @ Expr::Matrix(_)]) = (head, args) else {
        return None;
    };
    let reduced = match s.name().as_str() {
        "det" => det(arg),
        "trace" => trace(arg),
        _ => return None,
    };
    (!matches!(reduced, Expr::OtherOp(..))).then_some(reduced)
}
