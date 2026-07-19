//! Expansion (`me.expand()`), PORTING_PLAN.md §7e-adjacent.
//!
//! Distributes multiplication, division-numerator, and negation over sums, and
//! multinomial-expands non-negative integer powers of sums — recursively, into
//! function arguments and everywhere else. Denominators and non-integer /
//! negative powers are left intact (`(x+1)/(x+2)` → `x/(x+2)+1/(x+2)`, and the
//! denominator is NOT itself expanded — a factored `1/((x+1)(x+2))` keeps its
//! factored denominator, matching mathjs). Matches mathjs `expand`, which
//! `me.expand()` delegates to.
//!
//! Built on the canonical smart constructors, so the result is a canonical,
//! like-terms-combined expanded form (`(x+1)(x+2)` → `x²+3x+2`).
//!
//! **Bounded on adversarial input**: like terms are combined after every
//! distributed factor (so `(a+b)^64` stays at ≤65 live terms instead of 2⁶⁴
//! clones), and a hard term-count cap makes genuinely huge expansions (many
//! distinct monomials, e.g. a product of 40 distinct binomials) bail out and
//! return the unexpanded product instead of exhausting memory.

use crate::expr::Expr;
use crate::num::Number;

use super::syntactic::map_children;
use super::{add, mul, pow};

// Caps (limits::current().max_expand_power / max_expand_terms): the exponent
// bound on multinomial expansion, and the raw term-count bound per
// distribution step beyond which the node is left unexpanded. Classroom
// polynomials are far below both; they exist so a pasted product of dozens of
// sums cannot exhaust memory (the bug that once froze this dev container).

/// Fully expand `e`, in display form (`norm::present`). Internal callers
/// that pattern-match on canonical shapes use [`expand_core`].
pub fn expand(e: &Expr) -> Expr {
    super::present(&expand_core(e))
}

/// [`expand`] without the final presentation pass: the result is canonical.
pub(crate) fn expand_core(e: &Expr) -> Expr {
    match e {
        // Sum: expand each term (the smart `add` flattens and combines).
        Expr::Add(ts) => add(ts.iter().map(expand_core).collect()),

        // Negation is multiplication by −1, so it distributes over a sum.
        // (Two factors, one of them a constant: cannot hit the cap on its own.)
        Expr::Neg(a) => {
            let factors = vec![Expr::int(-1), expand_core(a)];
            try_distribute(&factors).unwrap_or_else(|| mul(factors))
        }

        // Product: distribute the (already-expanded) factors; on cap overflow
        // fall back to the unexpanded (canonical) product.
        Expr::Mul(fs) => {
            let factors: Vec<Expr> = fs.iter().map(expand_core).collect();
            try_distribute(&factors).unwrap_or_else(|| mul(factors))
        }

        // Division distributes its numerator over the denominator, which is left
        // as-is (neither expanded nor distributed into): (a+b)/d → a/d + b/d.
        Expr::Div(a, b) => {
            let num = expand_core(a);
            let inv = pow((**b).clone(), Expr::int(-1));
            let factors = vec![num, inv];
            try_distribute(&factors).unwrap_or_else(|| mul(factors))
        }

        Expr::Pow(base, exp) => {
            let base = expand_core(base);
            let exp = expand_core(exp);
            // Multinomial-expand a non-negative integer power of a sum.
            if let Expr::Num(Number::Int(n)) = &exp {
                let is_sum = matches!(&base, Expr::Add(ts) if ts.len() > 1);
                if (1..=crate::limits::current().max_expand_power).contains(n) && is_sum {
                    let factors = vec![base.clone(); *n as usize];
                    if let Some(r) = try_distribute(&factors) {
                        return r;
                    }
                }
            }
            pow(base, exp)
        }

        // Everything else (function applications, sequences, relations, leaves):
        // recurse into children but do not distribute across this node.
        _ => map_children(e, expand_core),
    }
}

/// The additive terms of `e`: the operands of an `Add`, or `e` itself.
fn terms_of(e: Expr) -> Vec<Expr> {
    match e {
        Expr::Add(ts) => ts,
        other => vec![other],
    }
}

/// Multiply out a list of (already-expanded) factors into a single expanded
/// sum. Like terms are combined after each factor (via the smart `add`), so the
/// live term count tracks the *combined* size, not the raw Cartesian product.
/// Returns `None` when a step would exceed the raw-term cap
/// (`limits::current().max_expand_terms`) — the
/// caller keeps the node unexpanded.
fn try_distribute(factors: &[Expr]) -> Option<Expr> {
    let mut acc = vec![Expr::int(1)];
    for f in factors {
        let f_terms = terms_of(f.clone());
        if acc.len().saturating_mul(f_terms.len()) > crate::limits::current().max_expand_terms {
            return None;
        }
        let mut next = Vec::with_capacity(acc.len() * f_terms.len());
        for a in &acc {
            for b in &f_terms {
                next.push(mul(vec![a.clone(), b.clone()]));
            }
        }
        // Combine like terms now: keeps acc multinomially bounded for powers
        // of the same sum ((a+b)^n stays at n+1 terms, not 2^n).
        acc = terms_of(add(next));
    }
    Some(add(acc))
}
