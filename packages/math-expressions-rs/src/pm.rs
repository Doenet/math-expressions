//! Plus-minus (`±`) operator helpers — port of JS `lib/expression/pm.js`.
//!
//! `pm` is unary, analogous to unary `-`: `5 ± 3` is `Add(5, OtherOp("pm",[3]))`
//! and each `["pm", x]` denotes the two-element set `{x, -x}` with an
//! independent sign choice. These helpers detect, count, and sign-expand `pm`
//! nodes; numerical set-equality of `pm`-bearing expressions lives in
//! `equality::pm_equals` (the consumer, mirroring JS `equality/pm-numerical.js`).
//!
//! **Supported containers.** `±` may appear in scalars, relations (equations and
//! inequalities), and sequences — tuples, vectors, altvectors, arrays, lists,
//! and sets — which `equality::pm_equals` compares componentwise. **`±` is not
//! supported inside a matrix entry or an interval endpoint**: those are compared
//! only as opaque wholes, so a `pm`-bearing matrix or interval is equal only to
//! a structurally identical one (`equals` cannot see through the `±` there).
//! Callers should not place `±` inside matrices or intervals.

use crate::expr::Expr;
use crate::norm::syntactic::map_children;

/// Maximum number of `pm` operators allowed for sign-expansion.
/// `expand_pm_signs` produces `2^n` variants, so this caps the work at 1024.
pub const MAX_PM_COUNT: u32 = 10;

/// `expand_pm_signs` refused to expand because the `pm` count exceeds
/// [`MAX_PM_COUNT`] (`2^n` would be too many variants).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PmOverflow {
    pub count: usize,
}

impl std::fmt::Display for PmOverflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "pm: cannot expand {} plus-minus operators (limit is {})",
            self.count, MAX_PM_COUNT
        )
    }
}

/// Is `e` a single `±x` node (`OtherOp("pm", [x])`)?
pub(crate) fn is_pm(e: &Expr) -> bool {
    matches!(e, Expr::OtherOp(name, args) if name.name() == "pm" && args.len() == 1)
}

/// Build a `±inner` node.
pub(crate) fn make_pm(inner: Expr) -> Expr {
    crate::parse::common::other_op("pm", vec![inner])
}

/// Does `e` contain any `pm` operator anywhere in its subtree?
pub fn contains_pm(e: &Expr) -> bool {
    is_pm(e) || e.children().iter().any(|c| contains_pm(c))
}

/// Number of `pm` operators anywhere in `e`.
pub fn count_pm(e: &Expr) -> usize {
    let here = usize::from(is_pm(e));
    here + e.children().iter().map(|c| count_pm(c)).sum::<usize>()
}

/// Enumerate all `2^n` sign assignments for the `pm` operators in `e`. Each
/// `["pm", x]` becomes either `x` (sign `+`) or `Neg(x)` (sign `−`). Returns a
/// single-element `[e]` when there are no `pm` nodes, or [`PmOverflow`] when the
/// count exceeds [`MAX_PM_COUNT`].
pub fn expand_pm_signs(e: &Expr) -> Result<Vec<Expr>, PmOverflow> {
    let n = count_pm(e);
    if n == 0 {
        return Ok(vec![e.clone()]);
    }
    if n > MAX_PM_COUNT as usize {
        return Err(PmOverflow { count: n });
    }
    let total = 1usize << n;
    let mut out = Vec::with_capacity(total);
    for mask in 0..total {
        let mut idx = 0;
        out.push(replace_pm(e, mask, &mut idx));
    }
    Ok(out)
}

/// Replace each `pm` node by its `+` or `−` variant per the corresponding bit of
/// `mask`. `idx` walks the `pm` nodes in the same pre-order as [`count_pm`], so
/// bit `k` of `mask` controls the `k`-th `pm`.
fn replace_pm(e: &Expr, mask: usize, idx: &mut usize) -> Expr {
    if is_pm(e) {
        let Expr::OtherOp(_, args) = e else { unreachable!() };
        let bit = (mask >> *idx) & 1;
        *idx += 1;
        let inner = replace_pm(&args[0], mask, idx);
        return if bit == 0 {
            inner
        } else {
            Expr::Neg(Box::new(inner))
        };
    }
    map_children(e, |c| replace_pm(c, mask, idx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Expr, TextToAst, TextToAstOptions};

    fn parse(s: &str) -> Expr {
        TextToAst::new(TextToAstOptions::default()).convert(s).unwrap()
    }

    #[test]
    fn detects_and_counts() {
        assert!(!contains_pm(&parse("5 + 3")));
        assert!(contains_pm(&parse("5 ± 3")));
        assert_eq!(count_pm(&parse("5 + 3")), 0);
        assert_eq!(count_pm(&parse("5 ± 3")), 1);
        assert_eq!(count_pm(&parse("5 ± 3 ± 4")), 2);
    }

    #[test]
    fn expands_2_to_the_n() {
        assert_eq!(expand_pm_signs(&parse("5")).unwrap().len(), 1);
        assert_eq!(expand_pm_signs(&parse("5 ± 3")).unwrap().len(), 2);
        assert_eq!(expand_pm_signs(&parse("5 ± 3 ± 4")).unwrap().len(), 4);
    }

    #[test]
    fn overflow_beyond_limit() {
        let many = "x ".to_string() + &"± 1 ".repeat((MAX_PM_COUNT + 1) as usize);
        let e = parse(many.trim());
        assert_eq!(count_pm(&e), (MAX_PM_COUNT + 1) as usize);
        assert!(expand_pm_signs(&e).is_err());
    }
}
