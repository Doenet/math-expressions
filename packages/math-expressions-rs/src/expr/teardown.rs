//! Iterative teardown of a deep [`Expr`] (STACK_SAFETY_PLAN item 21).
//!
//! Dropping an `Expr` recurses through its `Box`/`Vec` children, so a
//! sufficiently deep tree — `((((…))))` from adversarial student input — blows
//! the ~1 MB wasm shadow stack *on free*, and `panic = "abort"` turns that into
//! a dead worker. [`tear_down`] dismantles the tree with an explicit heap
//! worklist instead, so the recursion depth is constant.
//!
//! This is a free function rather than `impl Drop for Expr` on purpose: `Expr`
//! derives `Clone` and is destructured by value across the whole crate
//! (`match e { Expr::Add(xs) => … }`), and a `Drop` impl would make every such
//! move a borrow-check error (E0509). Ownership sinks that can hold an untrusted
//! deep tree — the wasm `Expression` handle — call this from *their* `Drop`
//! instead; it leaves `root` as a shell whose remaining children are leaves, so
//! the subsequent ordinary drop of `root` is shallow.

use super::Expr;

/// Dismantle `root`'s descendants iteratively, leaving `root` a shallow shell
/// (its direct children replaced by leaves). After this returns, dropping
/// `root` the ordinary way recurses at most one level.
pub fn tear_down(root: &mut Expr) {
    let mut stack: Vec<Expr> = Vec::new();
    drain_children(root, &mut stack);
    // Each popped node's children have already been moved onto `stack`; when the
    // node itself drops at the end of the iteration its children are leaves, so
    // its ordinary recursive `Drop` bottoms out immediately.
    while let Some(mut node) = stack.pop() {
        drain_children(&mut node, &mut stack);
    }
}

/// Move every direct child of `e` onto `stack`, replacing it in place with a
/// cheap leaf. Uses `mem::replace`/`mem::take` (a swap, not a move-out) so `e`
/// stays a fully-valid `Expr` throughout — no `Drop`-impl requirement.
fn drain_children(e: &mut Expr, stack: &mut Vec<Expr>) {
    use std::mem::{replace, take};
    let push_box = |b: &mut Box<Expr>, stack: &mut Vec<Expr>| {
        stack.push(replace(b.as_mut(), Expr::Blank));
    };
    match e {
        // Leaves — nothing to drain.
        Expr::Num(_)
        | Expr::Sym(_)
        | Expr::Const(_)
        | Expr::Bool(_)
        | Expr::RootOf { .. }
        | Expr::Blank
        | Expr::Ldots => {}

        // Single boxed child.
        Expr::Neg(a) | Expr::Not(a) | Expr::Prime(a) => push_box(a, stack),

        // Two boxed children.
        Expr::Div(a, b) | Expr::Pow(a, b) | Expr::Index(a, b) => {
            push_box(a, stack);
            push_box(b, stack);
        }

        // n-ary `Vec` children.
        Expr::Add(xs)
        | Expr::Mul(xs)
        | Expr::And(xs)
        | Expr::Or(xs)
        | Expr::Union(xs)
        | Expr::Intersect(xs)
        | Expr::Seq(_, xs)
        | Expr::Relation { operands: xs, .. }
        | Expr::OtherOp(_, xs) => stack.extend(take(xs)),

        // Not part of the or-pattern above: the entries live behind `Mat`'s
        // private fields, which hand them over only alongside a dimension
        // reset (see `Mat::take_entries`).
        Expr::Matrix(m) => stack.extend(m.take_entries()),

        // Head plus argument list.
        Expr::Apply(head, args) => {
            push_box(head, stack);
            stack.extend(take(args));
        }

        // Endpoints live behind a single `Box<(Expr, Expr)>`.
        Expr::Interval { endpoints, .. } => {
            let (a, b) = endpoints.as_mut();
            stack.push(replace(a, Expr::Blank));
            stack.push(replace(b, Expr::Blank));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A right-nested tower `Neg(Neg(…Blank…))` of the given depth.
    fn deep_tower(depth: usize) -> Expr {
        let mut e = Expr::Blank;
        for _ in 0..depth {
            e = Expr::Neg(Box::new(e));
        }
        e
    }

    #[test]
    fn tear_down_leaves_a_shallow_shell() {
        let mut e = deep_tower(5);
        tear_down(&mut e);
        // Root is still a `Neg`, but its child is now a leaf.
        match e {
            Expr::Neg(inner) => assert!(matches!(*inner, Expr::Blank)),
            other => panic!("expected Neg shell, got {other:?}"),
        }
    }

    /// The point of the whole module: a tree deep enough to overflow a small
    /// stack on ordinary recursive drop tears down without one. Run in a
    /// 128 KiB thread so a recursive free would actually trap.
    #[test]
    fn deep_tree_tears_down_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let mut e = deep_tower(200_000);
                tear_down(&mut e);
                // `e` (a shallow shell) drops here without recursing.
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
