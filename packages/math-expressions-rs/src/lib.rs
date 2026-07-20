//! Rust port of math-expressions (see tmp/PORTING_PLAN.md in the JS repo).
//!
//! All planned subsystems are implemented: parsers (text/LaTeX), canonical
//! normalization, simplify/expand, the full staged `equals` (syntactic,
//! finite-field, complex sampling, discrete infinite sets), differentiation,
//! evaluation, output formatting, expression utilities, the polynomial layer
//! (`reduce_rational`), the assumptions core, resource limits (§7f), and the
//! wasm-bindgen JS surface (`src/wasm.rs`, built by `scripts/build-wasm.sh`).
//! Fidelity is guarded by differential corpora against the JS reference
//! (equality 824/824; simplify/derivative/expand/evaluate/assumptions corpora
//! with snapshotted, documented divergences).

pub mod assumptions;
pub mod diff;
pub mod eq;
pub mod eval;
pub mod exact;
pub mod factor;
pub mod grade;
pub mod expr;
pub mod functions;
pub mod integrate;
pub mod js_match;
pub mod js_tree;
pub mod resource_limits;
pub mod matrix;
pub mod norm;
pub mod num;
pub mod numeric;
pub mod ode;
pub mod ops;
pub mod output;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
pub mod parse;
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
pub use eq::discrete_infinite::{create_discrete_infinite_set, match_discrete_infinite};
pub use eq::{
    contains_blank, equals, equals_syntactic, equals_via_real, finite_field_evaluate, EqOptions,
};
pub use factor::factor;
pub use integrate::integrate;
pub use grade::{
    equal_specified_sign_errors, equal_with_sign_errors, evaluate_membership, solve_linear,
};
pub use expr::{Expr, MathConst, RelOp};
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
pub use output::{to_latex, to_text, LatexOpts, TextOpts};
pub use parse::latex::{LatexToAst, LatexToAstOptions};
pub use parse::text::{TextToAst, TextToAstOptions};
pub use parse::ParseError;
pub use precise::{
    evaluate_to_precision, integrate_analyzed, integrate_to_precision, IntegralVerdict, Precise,
    SingularPoint,
};
pub use sym::Sym;
