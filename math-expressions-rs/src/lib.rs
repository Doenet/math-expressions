//! Rust port of math-expressions (see tmp/PORTING_PLAN.md in the JS repo).
//!
//! Current status: Phase 3 — expression tree, text parser, LaTeX parser.

pub mod eq;
pub mod eval;
pub mod expr;
pub mod js_tree;
pub mod norm;
pub mod num;
pub mod output;
pub mod parse;
pub mod sym;

pub use eq::{equals, EqOptions};
pub use expr::Expr;
pub use norm::canonicalize;
pub use num::Number;
pub use output::{to_latex, to_text, LatexOpts, TextOpts};
pub use parse::latex::{LatexToAst, LatexToAstOptions};
pub use parse::text::{TextToAst, TextToAstOptions};
pub use parse::ParseError;
pub use sym::Sym;
