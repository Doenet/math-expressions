//! Rewriting the relations that carry more than one fact into an explicit
//! `and`/`or` of two-sided comparisons.
//!
//! The store keeps one fact per variable, so a chained inequality (`a < b < c`),
//! an interval membership (`x ∈ (a,b]`) or an interval containment
//! (`(a,b) ⊂ [c,d)`) has to be broken into the comparisons it stands for before
//! it can be filed. That expansion is also what the store hands back, which is
//! why the bracket kinds map onto strict/non-strict exactly.

use crate::expr::{map_children, Expr, RelOp, SeqKind};

/// Expand every multi-fact relation in `e`, bottom-up.
///
/// Port of the legacy `lib/expression/transformation.js` `expand_relations`.
/// Operands are expanded first, so a relation nested inside an `and` is already
/// expanded by the time the `and` is looked at.
pub fn expand_relations(e: &Expr) -> Expr {
    let e = map_children(e, expand_relations);
    let Expr::Relation { operands, ops } = &e else {
        return e;
    };

    // `a = b = c` is the two equalities it abbreviates.
    if ops.len() >= 2 && ops.iter().all(|o| *o == RelOp::Eq) {
        return Expr::And(
            operands
                .windows(2)
                .map(|w| rel(RelOp::Eq, w[0].clone(), w[1].clone()))
                .collect(),
        );
    }

    // A chained inequality, in either direction. Mixed directions never reach
    // here: the AST spells them as a conjunction already.
    let one_way = ops.iter().all(|o| matches!(o, RelOp::Lt | RelOp::Le))
        || ops.iter().all(|o| matches!(o, RelOp::Gt | RelOp::Ge));
    if ops.len() >= 2 && one_way {
        // Left-nested rather than n-ary, because the ordering pass runs before
        // the flattening one and so sees this shape.
        let mut comparisons = ops
            .iter()
            .enumerate()
            .map(|(i, op)| rel(*op, operands[i].clone(), operands[i + 1].clone()));
        let first = comparisons.next().expect("ops.len() >= 2");
        let second = comparisons.next().expect("ops.len() >= 2");
        return comparisons.fold(Expr::And(vec![first, second]), |acc, c| {
            Expr::And(vec![acc, c])
        });
    }

    let [op] = ops.as_slice() else { return e };

    if let RelOp::In | RelOp::NotIn | RelOp::Ni | RelOp::NotNi = op {
        let negate = matches!(op, RelOp::NotIn | RelOp::NotNi);
        let (x, set) = match op {
            RelOp::In | RelOp::NotIn => (&operands[0], &operands[1]),
            _ => (&operands[1], &operands[0]),
        };
        // Membership in a set that is not an interval (`x ∈ ℝ`) stays as it is
        // — there is nothing to expand it into.
        let Some((a, b, closed_a, closed_b)) = as_interval(set) else {
            return e;
        };
        let lower = match (closed_a, negate) {
            (true, false) => rel(RelOp::Ge, x.clone(), a),
            (true, true) => rel(RelOp::Lt, x.clone(), a),
            (false, false) => rel(RelOp::Gt, x.clone(), a),
            (false, true) => rel(RelOp::Le, x.clone(), a),
        };
        let upper = match (closed_b, negate) {
            (true, false) => rel(RelOp::Le, x.clone(), b),
            (true, true) => rel(RelOp::Gt, x.clone(), b),
            (false, false) => rel(RelOp::Lt, x.clone(), b),
            (false, true) => rel(RelOp::Ge, x.clone(), b),
        };
        return join(negate, lower, upper);
    }

    if let RelOp::Subset | RelOp::NotSubset | RelOp::Superset | RelOp::NotSuperset = op {
        let negate = matches!(op, RelOp::NotSubset | RelOp::NotSuperset);
        let (small, big) = match op {
            RelOp::Subset | RelOp::NotSubset => (&operands[0], &operands[1]),
            _ => (&operands[1], &operands[0]),
        };
        // Containment between things that are not both intervals carries no
        // comparison to expand.
        let (Some((sa, sb, s_closed_a, s_closed_b)), Some((ba, bb, b_closed_a, b_closed_b))) =
            (as_interval(small), as_interval(big))
        else {
            return e;
        };
        // A closed small end inside an open big end is the one case that needs
        // a strict comparison: `[a,b] ⊂ (c,d)` requires `a > c`, while every
        // other combination is satisfied by `a ≥ c`.
        let lower = match (s_closed_a && !b_closed_a, negate) {
            (true, false) => rel(RelOp::Gt, sa, ba),
            (true, true) => rel(RelOp::Le, sa, ba),
            (false, false) => rel(RelOp::Ge, sa, ba),
            (false, true) => rel(RelOp::Lt, sa, ba),
        };
        let upper = match (s_closed_b && !b_closed_b, negate) {
            (true, false) => rel(RelOp::Lt, sb, bb),
            (true, true) => rel(RelOp::Ge, sb, bb),
            (false, false) => rel(RelOp::Le, sb, bb),
            (false, true) => rel(RelOp::Gt, sb, bb),
        };
        return join(negate, lower, upper);
    }

    e
}

fn rel(op: RelOp, a: Expr, b: Expr) -> Expr {
    Expr::Relation {
        operands: vec![a, b],
        ops: vec![op],
    }
}

/// Negating a conjunction gives a disjunction: `x ∉ (a,b)` is `x ≤ a or x ≥ b`,
/// not an `and`.
fn join(negate: bool, lower: Expr, upper: Expr) -> Expr {
    if negate {
        Expr::Or(vec![lower, upper])
    } else {
        Expr::And(vec![lower, upper])
    }
}

/// The endpoints and closure of an interval, where a two-entry tuple/array
/// sitting on either side of a containment spells one: `(a,b)` is open,
/// `[a,b]` closed. Half-open forms already parse as an interval.
///
/// Deliberately not [`crate::ops::to_intervals`], which recurses: only the top
/// level is an interval here, since a nested tuple inside an endpoint is an
/// endpoint and not an interval of its own.
fn as_interval(e: &Expr) -> Option<(Expr, Expr, bool, bool)> {
    match e {
        Expr::Interval { endpoints, closed } => {
            Some((endpoints.0.clone(), endpoints.1.clone(), closed.0, closed.1))
        }
        Expr::Seq(kind, xs) if xs.len() == 2 && matches!(kind, SeqKind::Tuple | SeqKind::Array) => {
            let closed = matches!(kind, SeqKind::Array);
            Some((xs[0].clone(), xs[1].clone(), closed, closed))
        }
        _ => None,
    }
}
