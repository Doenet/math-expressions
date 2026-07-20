//! Grading helpers beyond plain equality: sign-error-tolerant comparison
//! (`equalWithSignErrors`), linear solving (`solve_linear`), and finite-set
//! membership evaluation (§18 / DoenetML #1504).

use crate::assumptions::{is_negative, is_nonzero, is_positive, Assumptions};
use crate::eq::{equals, EqOptions};
use crate::expr::{Expr, RelOp, SeqKind};
use crate::norm::{canonicalize, simplify_with, syntactic::map_children};

/// Does `a` equal `b` after exactly `n` sign flips of subtrees of `a`?
/// `n = 0` is plain [`equals`]. Port of `equalSpecifiedSignErrors`: every
/// single-position negation is tried (root and every operand at every depth),
/// recursively for `n > 1`.
pub fn equal_specified_sign_errors(a: &Expr, b: &Expr, opts: &EqOptions, n: u32) -> bool {
    if n == 0 {
        return equals(a, b, opts);
    }
    single_negations(a)
        .iter()
        .any(|variant| equal_specified_sign_errors(variant, b, opts, n - 1))
}

/// Match with up to `max_sign_errors` sign errors; returns the smallest number
/// of errors that makes the pair equal (`Some(0)` = plainly equal), or `None`.
/// Port of `equalWithSignErrors`.
pub fn equal_with_sign_errors(
    a: &Expr,
    b: &Expr,
    opts: &EqOptions,
    max_sign_errors: u32,
) -> Option<u32> {
    (0..=max_sign_errors).find(|&n| equal_specified_sign_errors(a, b, opts, n))
}

/// Every tree obtained from `e` by negating exactly one subtree (including
/// the whole tree).
fn single_negations(e: &Expr) -> Vec<Expr> {
    let mut out = vec![Expr::Neg(Box::new(e.clone()))];
    let n_children = e.children().len();
    for i in 0..n_children {
        let child = e.children()[i].clone();
        for cv in single_negations(&child) {
            let mut idx = 0usize;
            out.push(map_children(e, |c| {
                let r = if idx == i { cv.clone() } else { c.clone() };
                idx += 1;
                r
            }));
        }
    }
    out
}

/// Solve a relation that is linear in `var` (port of `solve_linear`): simplify
/// under the assumptions, move everything to one side, extract `a·var + b`
/// with `a`, `b` free of `var`, and return `var <op> −b/a`. `None` when the
/// relation is not linear in `var`, `a` is not provably nonzero, or an
/// inequality\'s coefficient sign is unknown (inequalities flip direction for a
/// provably negative `a`).
pub fn solve_linear(e: &Expr, var: &str, assumptions: &Assumptions) -> Option<Expr> {
    let canon = simplify_with(e, assumptions);
    let Expr::Relation { operands, ops } = &canon else {
        return None;
    };
    let ([lhs, rhs], [op]) = (operands.as_slice(), ops.as_slice()) else {
        return None;
    };
    if !matches!(op, RelOp::Eq | RelOp::Ne | RelOp::Lt | RelOp::Le | RelOp::Gt | RelOp::Ge) {
        return None;
    }

    // lhs − rhs = 0 form, expanded so the negation distributes over sums
    // (canonicalize alone keeps `−(4+2x)` as a product, which would defeat
    // the linear-term extraction below).
    let zeroed = crate::norm::expand_core(&Expr::Add(vec![
        lhs.clone(),
        Expr::Neg(Box::new(rhs.clone())),
    ]));

    // Extract the linear coefficients a·var + b from the canonical sum.
    let mentions = |t: &Expr| t.any_subexpr(&|c| matches!(c, Expr::Sym(s) if s.name() == var));
    let mut a_parts: Vec<Expr> = Vec::new();
    let mut b_parts: Vec<Expr> = Vec::new();
    let terms: Vec<Expr> = match &zeroed {
        Expr::Add(ts) => ts.clone(),
        other => vec![other.clone()],
    };
    for t in &terms {
        if !mentions(t) {
            b_parts.push(t.clone());
            continue;
        }
        // The term must be var itself or a product with var as a bare factor
        // and everything else var-free.
        match t {
            Expr::Sym(_) => a_parts.push(Expr::int(1)),
            Expr::Mul(fs) => {
                let mut rest = Vec::new();
                let mut var_count = 0;
                for f in fs {
                    if matches!(f, Expr::Sym(s) if s.name() == var) {
                        var_count += 1;
                    } else if mentions(f) {
                        return None; // var inside a nonlinear factor
                    } else {
                        rest.push(f.clone());
                    }
                }
                if var_count != 1 {
                    return None; // var², or missing after all
                }
                a_parts.push(crate::norm::mul(rest));
            }
            _ => return None, // var under a power/function: not linear
        }
    }
    if a_parts.is_empty() {
        return None;
    }
    let a = canonicalize(&Expr::Add(a_parts));
    let b = canonicalize(&Expr::Add(b_parts));

    if is_nonzero(&a, assumptions) != Some(true) {
        return None;
    }

    // var <op\'> −b/a, flipping strict/loose inequalities for negative a.
    let solution = simplify_with(
        &Expr::Div(
            Box::new(Expr::Neg(Box::new(b))),
            Box::new(a.clone()),
        ),
        assumptions,
    );
    let out_op = match op {
        RelOp::Eq | RelOp::Ne => *op,
        _ => {
            if is_positive(&a, assumptions) == Some(true) {
                *op
            } else if is_negative(&a, assumptions) == Some(true) {
                match op {
                    RelOp::Lt => RelOp::Gt,
                    RelOp::Le => RelOp::Ge,
                    RelOp::Gt => RelOp::Lt,
                    RelOp::Ge => RelOp::Le,
                    _ => unreachable!(),
                }
            } else {
                return None; // inequality with unknown coefficient sign
            }
        }
    };
    Some(Expr::Relation {
        operands: vec![Expr::sym(var), solution],
        ops: vec![out_op],
    })
}

/// Evaluate a finite-set membership relation to a truth value (§18 /
/// DoenetML #1504): `x ∈ {a, b, …}` is `Some(true)` when `x` equals a member,
/// `Some(false)` when every membership comparison is decidably false and the
/// candidate is a closed (constant) expression, and `None` otherwise.
/// `∋`/`∌` orientations are handled by canonicalization; `∉` negates.
pub fn evaluate_membership(e: &Expr, opts: &EqOptions) -> Option<bool> {
    let canon = canonicalize(e);
    let Expr::Relation { operands, ops } = &canon else {
        return None;
    };
    let ([lhs, rhs], [op]) = (operands.as_slice(), ops.as_slice()) else {
        return None;
    };
    let negate = match op {
        RelOp::In => false,
        RelOp::NotIn => true,
        _ => return None,
    };
    let Expr::Seq(SeqKind::Set, members) = rhs else {
        return None;
    };
    if members.iter().any(|m| equals(lhs, m, opts)) {
        return Some(!negate);
    }
    // No member matched: definitive only when everything is closed (constant)
    // — a symbolic candidate might still equal a member.
    let closed = |x: &Expr| {
        crate::ops::variables(x)
            .iter()
            .all(|v| crate::sym::is_constant_symbol(v))
    };
    if closed(lhs) && members.iter().all(closed) {
        return Some(negate);
    }
    None
}
