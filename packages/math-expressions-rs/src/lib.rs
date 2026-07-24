//! A Rust port of the `math-expressions` computer-algebra library.
//!
//! The crate parses mathematical expressions from text and LaTeX into an
//! [`Expr`] tree, then offers canonical normalization, simplification and
//! expansion, a staged test of mathematical equality (syntactic comparison,
//! finite-field probing, complex-number sampling, and discrete infinite sets),
//! symbolic differentiation and integration, arbitrary-precision evaluation,
//! symbolic matrix algebra, an ODE solver, and formatting back to text and
//! LaTeX. An assumptions system supplies variable facts, and every operation is
//! bounded by configurable [resource limits](resource_limits).
//!
//! Fidelity to the original library is guarded by differential test corpora:
//! mathematical equality passes all 824 reference cases, and the simplify,
//! derivative, expand, evaluate, and assumptions corpora pass with a small,
//! documented set of snapshotted divergences.
//!
//! The wasm-bindgen JavaScript bindings live in a separate crate, a thin
//! adapter over this crate's public API.
//!
//! # Public surface (facade tiers)
//!
//! - **Root re-exports** (below) are the primary API: prefer
//!   `math_expressions::simplify` over `math_expressions::norm::simplify`.
//! - **API namespaces** — modules used qualified, by design: [`exact`]
//!   (certified zero-equivalence), [`precise`] (arbitrary-precision eval /
//!   quadrature), [`numeric`] (mathjs-compatible f64 kernels), [`js_tree`] /
//!   [`js_match`] (JS `Tree` interop), [`pm`], [`ode`], [`notation`],
//!   [`resource_limits`], [`output`], [`parse`].
//! - **Everything else** (`norm`, `eval`, `equality*`, `functions`, `ops`,
//!   `matrix`, …) is `pub` for the integration-test suite, not a stability
//!   surface; new external callers should go through the tiers above.

pub mod assumptions;
pub mod diff;
pub mod equality;
pub mod eval;
pub mod exact;
pub mod factor;
pub mod ratform;
pub mod grade;
pub mod expr;
pub mod functions;
pub mod equality_structural;
pub mod integrate;
pub mod js_match;
pub mod js_tree;
pub mod resource_limits;
pub mod matrix;
pub mod norm;
pub mod notation;
pub mod num;
pub mod numeric;
pub mod ode;
pub mod ops;
pub mod output;
pub mod parse;
pub mod pm;
mod poly;
pub mod precise;
pub(crate) mod rootof;
pub mod sym;
pub(crate) mod upoly;

pub use assumptions::{
    is_complex, is_integer, is_negative, is_nonnegative, is_nonpositive, is_nonzero, is_positive,
    is_real, Assumptions,
};
pub use diff::derivative;
pub use equality::discrete_infinite::{create_discrete_infinite_set, match_discrete_infinite};
pub use equality::{
    contains_blank, equals, equals_syntactic, equals_via_real, finite_field_evaluate, EqOptions,
};
pub use factor::{factor, factor_terms};
pub use integrate::integrate;
pub use grade::{
    equal_specified_sign_errors, equal_with_sign_errors, evaluate_membership, solve_linear,
};
pub use expr::{Expr, MathConst, RelOp};
pub use equality_structural::{
    check_structural_comparison, structural_equality, StructuralComparison,
    StructuralComparisonResult,
};
pub use matrix::{
    char_poly, cross_prod, det, dot_prod, eigenvalues, eigenvectors, matmul, matrix_inverse,
    nullspace, rank, rref, trace, transpose, vector_add, vector_sub, EigenPair,
};
pub use norm::{canonicalize, desugar_units, expand, simplify, simplify_logical, simplify_with};
pub use num::Number;
pub use ode::{solve_ode_exprs, solve_ode_with, OdeSolution};
pub use ops::{
    add_unit, altvectors_to_vectors, constants_to_floats, evaluate, evaluate_numbers,
    evaluate_to_constant, functions, get_component, is_analytic, normalize_function_names,
    operators, reduce_rational, remove_scaling_units, remove_units, round_numbers_to_decimals,
    round_numbers_to_precision, round_numbers_to_precision_plus_decimals, set_small_zero,
    strings_to_subscripts, subscripts_to_strings, substitute, substitute_component,
    to_intervals, tuples_to_vectors, variables, AnalyticOpts,
};
pub use notation::{Digits, Grouping, NumberNotation};
pub use output::{to_latex, to_text, LatexOpts, TextOpts};
pub use parse::latex::{LatexToAst, LatexToAstOptions};
pub use parse::text::{TextToAst, TextToAstOptions};
pub use parse::ParseError;
pub use pm::{contains_pm, count_pm, expand_pm_signs, PmOverflow, MAX_PM_COUNT};
pub use ratform::{cancel, together};
pub use precise::{
    evaluate_to_precision, integrate_analyzed, integrate_to_precision, IntegralVerdict, Precise,
    SingularPoint,
};
pub use sym::Sym;
