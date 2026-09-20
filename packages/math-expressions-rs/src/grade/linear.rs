//! Reading a tree as a linear function of named variables: isolating one of
//! them in a relation ([`solve_linear`]) and splitting an expression into
//! `b + Σ aᵢ·vᵢ` ([`linear_decomposition`]).
//!
//! Both rest on [`linear_terms`], which is where the "is this actually linear"
//! judgement lives — a variable under a power, inside a function, or multiplied
//! by another of the named variables is a rejection, not a coefficient.

use crate::assumptions::{is_negative, is_nonzero, is_positive, Assumptions};
use crate::expr::{Expr, RelOp};
use crate::normalize::{canonicalize, simplify_with};

/// Solve a relation that is linear in `var` (port of `solve_linear`): simplify
/// under the assumptions, move everything to one side, extract `a·var + b`
/// with `a`, `b` free of `var`, and return `var <op> −b/a`. `None` when the
/// relation is not linear in `var`, `a` is not provably nonzero, or an
/// inequality's coefficient sign is unknown (inequalities flip direction for a
/// provably negative `a`).
pub fn solve_linear(e: &Expr, var: &str, assumptions: &Assumptions) -> Option<Expr> {
    let canon = simplify_with(e, assumptions);
    let Expr::Relation { operands, ops } = &canon else {
        return None;
    };
    let ([lhs, rhs], [op]) = (operands.as_slice(), ops.as_slice()) else {
        return None;
    };
    if !matches!(
        op,
        RelOp::Eq | RelOp::Ne | RelOp::Lt | RelOp::Le | RelOp::Gt | RelOp::Ge
    ) {
        return None;
    }

    // lhs − rhs = 0 form, expanded so the negation distributes over sums
    // (canonicalize alone keeps `−(4+2x)` as a product, which would defeat
    // the linear-term extraction below).
    let zeroed = crate::normalize::expand_core(&Expr::Add(vec![
        lhs.clone(),
        Expr::Neg(Box::new(rhs.clone())),
    ]));

    let (mut a_parts, b_parts) = linear_terms(&zeroed, &[var])?;
    let a_parts = a_parts.pop().expect("one variable in, one coefficient out");
    if a_parts.is_empty() {
        return None; // var does not occur: nothing to solve for
    }
    let a = canonicalize(&Expr::Add(a_parts));
    let b = canonicalize(&Expr::Add(b_parts));

    if is_nonzero(&a, assumptions) != Some(true) {
        return None;
    }

    // var <op'> −b/a, flipping strict/loose inequalities for negative a.
    //
    // Handed to `simplify_with` as one quotient, deliberately — and it is now
    // safe to do so. Folding the numerator first instead
    // (`Div(simplify(Neg(b)), a)`) was once tempting as a way to land
    // `-3y - v <= 2xz + r` on `-((2xz+r+v)/3)` rather than `(-2xz-r-v)/3`, but
    // it only traded that placement for the opposite one on `2uv-v = 3u+q`.
    // Both spellings were `simplify` fixpoints, so which one came out depended
    // on what was handed in. `simplify` now picks one (see
    // `rule_factor_sign_out_of_sum`), so there is nothing left to compensate for
    // here and no reason to pre-fold.
    let solution = simplify_with(
        &Expr::Div(Box::new(Expr::Neg(Box::new(b))), Box::new(a.clone())),
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

/// Decompose `e` as `b + Σ aᵢ·vᵢ` over `vars`, every coefficient and the
/// constant term a plain number *as the JS tree spells one*. Returns the
/// coefficients in `vars` order together with `b`, or `None` when `e` is not of
/// that form — which includes the case where it mentions a variable outside
/// `vars`, since that would leave a non-numeric `b`.
///
/// A variable of `vars` that does not occur gets the coefficient `0`, matching
/// the JS caller's expectation that the answer covers every name it asked
/// about.
pub fn linear_decomposition(e: &Expr, vars: &[String]) -> Option<(Vec<Expr>, Expr)> {
    let names: Vec<&str> = vars.iter().map(String::as_str).collect();
    // Expanded for the same reason as in `solve_linear`: the extraction reads a
    // flat sum of products, and `−(a+b)` is a product until it is expanded.
    let expanded = crate::normalize::expand_core(&crate::normalize::simplify(e));
    let (a_parts, b_parts) = linear_terms(&expanded, &names)?;

    let numeric = |parts: Vec<Expr>| {
        let e = canonicalize(&Expr::Add(parts));
        is_js_number_literal(&e).then_some(e)
    };
    let coefficients: Vec<Expr> = a_parts
        .into_iter()
        .map(numeric)
        .collect::<Option<Vec<_>>>()?;
    Some((coefficients, numeric(b_parts)?))
}

/// JS `is_number`, which asks `typeof tree === "number"` (or `["-", n]`) and so
/// admits only what the tree spells as a bare literal: `3`, `-3`, `0.5`.
///
/// An exact fraction spells as `["/", -4, 3]` there and is **rejected** — not an
/// oversight worth fixing. `get_assumptions` falls back to reporting the facts
/// on each variable separately when the decomposition fails, and that fallback
/// is what recovers the transitive consequences (`3a+4b > 2c+6d` together with
/// `c+3d > 0` also tells you about `c` and `d`). Accepting `−4/3` here takes the
/// other branch and silently loses them.
fn is_js_number_literal(e: &Expr) -> bool {
    matches!(e, Expr::Num(_)) && crate::expr::serde::to_js(e).is_number()
}

/// Split a sum into the factors multiplying each of `vars` and the terms free
/// of all of them. `None` when some term is not linear in `vars`.
///
/// The returned coefficient lists are the raw summands, not yet combined: only
/// the caller knows whether an empty list means "absent, so zero"
/// ([`linear_decomposition`]) or "absent, so there is nothing to solve for"
/// ([`solve_linear`]).
fn linear_terms(sum: &Expr, vars: &[&str]) -> Option<(Vec<Vec<Expr>>, Vec<Expr>)> {
    let mentions =
        |t: &Expr, v: &str| t.any_subexpr(&|c| matches!(c, Expr::Sym(s) if s.name() == v));
    let mut a_parts: Vec<Vec<Expr>> = vec![Vec::new(); vars.len()];
    let mut b_parts: Vec<Expr> = Vec::new();
    let terms: Vec<Expr> = match sum {
        Expr::Add(ts) => ts.clone(),
        other => vec![other.clone()],
    };
    for t in &terms {
        let Some(i) = vars.iter().position(|v| mentions(t, v)) else {
            b_parts.push(t.clone());
            continue;
        };
        let var = vars[i];
        // The term must be the variable itself or a product with it as a bare
        // factor and every other factor free of *all* the named variables — so
        // `x·y` is a rejection even though each factor alone would be linear.
        match t {
            Expr::Sym(_) => a_parts[i].push(Expr::int(1)),
            Expr::Mul(fs) => {
                let mut rest = Vec::new();
                let mut var_count = 0;
                for f in fs {
                    if matches!(f, Expr::Sym(s) if s.name() == var) {
                        var_count += 1;
                    } else if vars.iter().any(|v| mentions(f, v)) {
                        return None; // a named variable inside a nonlinear factor
                    } else {
                        rest.push(f.clone());
                    }
                }
                if var_count != 1 {
                    return None; // var², or missing after all
                }
                a_parts[i].push(crate::normalize::mul(rest));
            }
            _ => return None, // var under a power/function: not linear
        }
    }
    Some((a_parts, b_parts))
}
