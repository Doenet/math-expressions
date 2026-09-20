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
//!   `math_expressions::simplify` over `math_expressions::normalize::simplify`.
//! - **API namespaces** — modules used qualified, by design: [`eval_exact`]
//!   (certified zero-equivalence), [`eval_numeric::certified_digits`]
//!   (arbitrary-precision eval / quadrature), [`mathjs_compat`] (the `me.math`
//!   f64 shims Doenet needs — graduating out as native features land),
//!   [`eval_numeric::complex`] (the equality tester's complex sampler),
//!   [`expr::serde`] (`Expr` ⇄ JS `Tree` JSON codec), [`ops::pm`], [`notation`],
//!   [`resource_limits`], [`mod@print`], [`parse`]. Each module's import path is its
//!   physical path — no re-export aliasing.
//! - **Everything else** (`normalize`, `equality*`, `special_functions`, `ops`,
//!   `matrix`, `calculus`, `polynomials`, …) is `pub` for the integration-test
//!   suite, not a stability surface; new external callers should go through the
//!   tiers above.

// The barrel modules above document their private submodules by name
// (`[`canonicalize`]`, `[`table`]`, …). Those links resolve only under
// `--document-private-items`, which is how this crate's internals are meant to
// be read; the lint would otherwise bury the *real* broken-link warnings.
#![allow(rustdoc::private_intra_doc_links)]

pub mod assumptions;
pub mod calculus;
pub mod constant_policy;
pub mod equality;
pub mod equality_structural;
pub mod eval_exact;
pub mod eval_numeric;
pub mod expr;
pub mod grade;
pub mod mathjs_compat;
pub mod matrix;
pub mod normalize;
pub mod notation;
pub mod num;
pub mod ops;
pub mod parse;
pub mod polynomials;
pub mod print;
pub mod resource_limits;
pub mod special_functions;

pub use assumptions::{
    expand_relations, is_complex, is_integer, is_negative, is_nonnegative, is_nonpositive,
    is_nonzero, is_positive, is_real, Assumptions, TreeStore,
};
pub use calculus::critical::critical_points;
pub use calculus::diff::derivative;
pub use calculus::integrate::integrate;
pub use constant_policy::ConstantPolicy;
pub use equality::discrete_infinite::{
    create_discrete_infinite_set, equals_discrete_infinite_sets, match_discrete_infinite,
};
pub use equality::{
    contains_blank, equals, equals_syntactic, equals_via_real, finite_field_evaluate, EqOptions,
};
pub use equality_structural::{
    check_structural_comparison, structural_equality, StructuralComparison,
    StructuralComparisonResult,
};
pub use eval_numeric::certified_digits::{
    evaluate_to_precision, integrate_analyzed, integrate_to_precision, IntegralVerdict, Precise,
    SingularPoint,
};
pub use expr::sym::{interner_len, Sym};
pub use expr::tear_down;
pub use expr::{Expr, Mat, MathConst, RelOp};
pub use grade::{
    equal_specified_sign_errors, equal_with_sign_errors, evaluate_membership, linear_decomposition,
    solve_linear,
};
pub use mathjs_compat::ode::{solve_ode_exprs, solve_ode_with, OdeSolution};
pub use matrix::{
    char_poly, cross_prod, det, dot_prod, eigenvalues, eigenvectors, matmul, matrix_inverse,
    nullspace, rank, rref, scalar_mul, trace, transpose, vector_add, vector_sub, EigenPair,
};
pub use normalize::{
    canonicalize, cmp_default_order, default_order, desugar_units, expand, flatten_logical,
    normalize_applied_functions, normalize_negative_numbers, push_not, simplify, simplify_logical,
    simplify_with,
};
pub use notation::{Digits, Grouping, NumberNotation};
pub use num::Number;
pub use ops::pm::{contains_pm, count_pm, expand_pm_signs, PmOverflow, MAX_PM_COUNT};
pub use ops::{
    add_unit, altvectors_to_vectors, constants_to_floats, evaluate_fast_f64, evaluate_many,
    evaluate_numbers, evaluate_numbers_evaluate_functions,
    evaluate_numbers_evaluate_functions_with_digits, evaluate_numbers_preserve_order,
    evaluate_numbers_preserve_order_with_digits, evaluate_numbers_with_digits,
    evaluate_to_constant, functions, get_component, is_analytic, normalize_function_names,
    operators, perform_vector_matrix_additions_scalar_multiplications, reduce_rational,
    remove_scaling_units, remove_units, round_numbers_to_decimals, round_numbers_to_precision,
    round_numbers_to_precision_plus_decimals, set_small_zero, strings_to_subscripts,
    subscripts_to_strings, subscripts_to_strings_with, substitute, substitute_component,
    to_intervals, tuples_to_vectors, variables, AnalyticOpts, MaxDigits,
};
pub use parse::latex::{LatexToAst, LatexToAstOptions};
pub use parse::text::{TextToAst, TextToAstOptions};
pub use parse::ParseError;
pub use polynomials::factor::{factor, factor_terms};
pub use polynomials::ratform::{cancel, together};
pub use print::{to_latex, to_text, LatexOpts, TextOpts};
