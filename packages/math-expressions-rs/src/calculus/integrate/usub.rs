//! Derivative-divides u-substitution: for each composite candidate `u`, test
//! whether `f / u′` rewrites as a function of `u` alone; if so, integrate that
//! in a fresh variable and substitute back (Rubi's substitution meta-rule; the
//! SymPy `manualintegrate` workhorse). [`usub`] re-enters [`super::integ`] on
//! the substituted integrand, sharing the caller's fuel budget.

use super::integ;
use super::util::{depends_on, int};
use crate::expr::Expr;
use crate::normalize::{canonicalize, pow};
use crate::num::Number;

/// Derivative-divides: for each composite candidate `u`, test whether
/// `f / u′` rewrites as a function of `u` alone; if so, integrate that in a
/// fresh variable and substitute back.
pub(super) fn usub(e: &Expr, x: &str, fuel: &mut i64) -> Option<Expr> {
    const U: &str = "_usub";
    let mut candidates: Vec<Expr> = Vec::new();
    collect_candidates(e, &mut candidates);
    // Order-preserving dedup: `collect_candidates` yields the same subtree from
    // several places (an `Apply`'s argument, then the `Apply` itself, then again
    // from a sibling), and those repeats are rarely *adjacent* — `Vec::dedup`
    // would leave them in, and each one would then eat one of the
    // `max_integration_candidates` slots that bound the search.
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|u| depends_on(u, x) && !matches!(u, Expr::Sym(_)) && seen.insert(u.clone()));
    candidates.truncate(crate::resource_limits::current().max_integration_candidates);
    for u in candidates {
        let du = canonicalize(&crate::calculus::diff::derivative(&u, x));
        if matches!(&du, Expr::Num(n) if n.is_zero()) {
            continue;
        }
        // f/u′, aggressively cancelled.
        let q = crate::normalize::simplify_core(&crate::ops::reduce_rational(&Expr::Div(
            Box::new(e.clone()),
            Box::new(du.clone()),
        )));
        let replaced = replace_subtree(&q, &u, &Expr::sym(U));
        if depends_on(&replaced, x) {
            continue;
        }
        // The substituted integrand is x-free but may still be unintegrable
        // (`u = x²` turns `x·sin(x²)·cos(x²)` into `sin(u)cos(u)/2`, which then
        // needs its own u-sub). Move on to the next candidate rather than
        // abandoning the whole stage — the shared `fuel` still bounds the total
        // work, and `integ` refuses immediately once it runs out.
        let Some(inner) = integ(&canonicalize(&replaced), U, fuel) else {
            continue;
        };
        let subs = std::collections::HashMap::from([(U.to_string(), u.clone())]);
        return Some(canonicalize(&crate::ops::substitute(&inner, &subs)));
    }
    None
}

/// Composite subtrees worth trying as `u`: application arguments, the
/// applications themselves, and non-atomic power bases.
fn collect_candidates(e: &Expr, out: &mut Vec<Expr>) {
    match e {
        Expr::Apply(_, args) => {
            for a in args {
                out.push(a.clone());
                collect_candidates(a, out);
            }
            out.push(e.clone());
        }
        Expr::Pow(b, ex) => {
            out.push((**b).clone());
            // x⁴ hides x² (and x⁶ hides x³): propose divisor powers so
            // u = x² can match inside 1 − x⁴ (replace_subtree understands
            // the power-multiple rewrite).
            if let Expr::Num(Number::Int(k)) = &**ex {
                for d in 2..*k {
                    if *k % d == 0 {
                        out.push(pow((**b).clone(), int(d)));
                    }
                }
            }
            collect_candidates(b, out);
            collect_candidates(ex, out);
        }
        _ => {
            for c in e.children() {
                collect_candidates(c, out);
            }
        }
    }
}

/// Structural replacement of every occurrence of `target` by `to`, plus the
/// power-multiple rewrite: with target `b^k`, an occurrence `b^(k·j)`
/// becomes `to^j` (this is what lets u = x² act inside x⁴).
fn replace_subtree(e: &Expr, target: &Expr, to: &Expr) -> Expr {
    if e == target {
        return to.clone();
    }
    if let (Expr::Pow(tb, tk), Expr::Pow(eb, ek)) = (target, e) {
        if tb == eb {
            if let (Expr::Num(Number::Int(tk)), Expr::Num(Number::Int(ek))) = (&**tk, &**ek) {
                if *tk >= 2 && *ek % *tk == 0 && *ek != *tk {
                    return Expr::Pow(Box::new(to.clone()), Box::new(int(*ek / *tk)));
                }
            }
        }
    }
    crate::expr::map_children(e, |c| replace_subtree(c, target, to))
}
