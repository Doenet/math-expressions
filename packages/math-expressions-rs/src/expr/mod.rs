//! Expression tree: the core [`Expr`] enum ([`tree`]) and its interned symbol
//! leaves ([`sym`]), plus read-only traversal and n-ary flattening ([`visit`])
//! and the JS `Tree` JSON codec ([`serde`], `Expr` ⇄ the shape JavaScript
//! consumes).

mod matrix;
mod teardown;
mod tree;
mod visit;

pub mod serde;
pub mod sym;

pub use matrix::Mat;
pub use teardown::tear_down;
pub use tree::{Expr, MathConst, RelOp, SeqKind};
pub use visit::flatten;
pub(crate) use visit::map_children;
