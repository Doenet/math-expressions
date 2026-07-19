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
pub mod grade;
pub mod expr;
pub mod js_match;
pub mod js_tree;
pub mod limits;
pub mod matrix;
pub mod norm;
pub mod num;
pub mod numeric;
pub mod ops;
pub mod output;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
pub mod parse;
mod poly;
pub mod sym;

pub use assumptions::{
    is_complex, is_integer, is_negative, is_nonnegative, is_nonpositive, is_nonzero, is_positive,
    is_real, Assumptions,
};
pub use diff::derivative;
pub use eq::discrete_infinite::{create_discrete_infinite_set, match_discrete_infinite};
pub use eq::{contains_blank, equals, equals_syntactic, EqOptions};
pub use grade::{
    equal_specified_sign_errors, equal_with_sign_errors, evaluate_membership, solve_linear,
};
pub use expr::{Expr, MathConst, RelOp};
pub use matrix::{det, matmul, matrix_inverse, nullspace, rank, rref, trace, transpose};
pub use norm::{canonicalize, desugar_units, expand, simplify, simplify_with};
pub use num::Number;
pub use ops::{
    constants_to_floats, evaluate, evaluate_numbers, evaluate_to_constant, functions,
    get_component, operators, reduce_rational, round_numbers_to_decimals, strings_to_subscripts,
    subscripts_to_strings, substitute_component, to_intervals,
    round_numbers_to_precision, round_numbers_to_precision_plus_decimals, substitute, variables,
};
pub use output::{to_latex, to_text, LatexOpts, TextOpts};
pub use parse::latex::{LatexToAst, LatexToAstOptions};
pub use parse::text::{TextToAst, TextToAstOptions};
pub use parse::ParseError;
pub use sym::Sym;
