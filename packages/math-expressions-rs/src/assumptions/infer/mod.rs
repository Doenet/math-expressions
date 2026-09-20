//! Bottom-up fact inference over a canonical expression.
//!
//! A clean-slate recursive pass: [`facts`] dispatches on the node, deriving a
//! [`Facts`] from its children via the [`combine`] rules, the function-domain
//! table in [`apply`], and the stored assumptions read by [`vars`].
//! Deliberately mirrors JS conservatisms: an unassumed variable is fully
//! unknown (no default-real), odd powers of negatives get no sign, and sums do
//! no interval arithmetic.
//!
//! Two cross-cutting steps bracket the dispatch, both ported from
//! `element_of_sets.js`:
//!
//! * **Constant folding first.** Every JS predicate calls
//!   `evaluate_to_constant` before looking at structure, so `sin(0)` is the
//!   integer `0` and `-2.2/(5-5)` is `-∞` — no structural rule ever sees them.
//! * **Sign normalization last.** JS defines `is_negative` as
//!   `is_real && !is_nonnegative` (and `is_nonpositive` as the dual), with all
//!   four sign predicates short-circuiting on `is_real`. Re-imposing that at
//!   every node ([`Facts::normalize`]) means a rule only has to derive
//!   `nonneg`/`positive`.

mod apply;
mod combine;
mod vars;

use super::facts::Facts;
use super::Assumptions;
use crate::eval_numeric::complex::eval_complex;
use crate::expr::{Expr, MathConst};

/// Bottom-up fact inference over a canonical expression.
pub(super) fn facts(e: &Expr, a: &Assumptions) -> Facts {
    let mut out = facts_of_node(e, a);
    out.normalize();
    out
}

fn facts_of_node(e: &Expr, a: &Assumptions) -> Facts {
    match e {
        Expr::Num(n) => Facts::of_number(n),
        Expr::Const(MathConst::Pi | MathConst::E) => Facts::positive_transcendental(),
        Expr::Const(MathConst::I) => Facts::imaginary_unit(),
        Expr::Const(MathConst::Inf | MathConst::NegInf) => Facts::infinite(),
        // Canonicalization folds `0/0` and `0^0` to this leaf, which is where
        // the JS answers for those literals come from.
        Expr::Const(MathConst::NaN) => Facts::nan(),
        Expr::Const(_) => Facts::unknown(),

        // A *declared* `pi`/`e`/`i` carries the constant's facts; an undeclared
        // one is an ordinary variable and knows only what has been assumed
        // about it — which is the point of declaring, since `π > 0` is not
        // something to believe about a coordinate that happens to be called
        // `pi`.
        Expr::Sym(s) => match s.name().as_str() {
            name @ ("pi" | "e") if crate::expr::sym::is_constant_symbol(name) => {
                Facts::positive_transcendental()
            }
            "i" if crate::expr::sym::is_constant_symbol("i") => Facts::imaginary_unit(),
            name => vars::variable_facts(name, a),
        },

        // Numeric heads: a closed subtree is answered by evaluation, exactly
        // as JS's `evaluate_to_constant` short-circuit does, and only an
        // expression that still mentions a variable reaches the rules.
        Expr::Add(_) | Expr::Mul(_) | Expr::Pow(..) | Expr::Apply(..) => {
            if let Some(f) = constant_facts(e) {
                return f;
            }
            match e {
                Expr::Add(ts) => combine::add(&child_facts(ts, a)),
                Expr::Mul(fs) => combine::mul(fs, &child_facts(fs, a), a),
                Expr::Pow(b, x) => combine::pow(&facts(b, a), &facts(x, a), x, a),
                Expr::Apply(head, args) => apply::apply_facts(head, args, a),
                _ => unreachable!("outer match restricted the node kind"),
            }
        }

        // A relation between constants decides to a boolean; one that still
        // mentions a variable is just another non-numeric head.
        Expr::Relation { operands, .. } => {
            if operands.iter().all(|o| constant_facts(o).is_some()) {
                Facts::boolean()
            } else {
                Facts::non_numeric()
            }
        }

        // Heads that do not denote a number at all.
        Expr::Bool(_)
        | Expr::Seq(..)
        | Expr::Interval { .. }
        | Expr::Matrix(_)
        | Expr::And(_)
        | Expr::Or(_)
        | Expr::Not(_)
        | Expr::Union(_)
        | Expr::Intersect(_) => Facts::non_numeric(),

        _ => Facts::unknown(),
    }
}

fn child_facts(xs: &[Expr], a: &Assumptions) -> Vec<Facts> {
    xs.iter().map(|x| facts(x, a)).collect()
}

/// The value of a subexpression that mentions no free variable, as JS's
/// `evaluate_to_constant` would report it. `None` when anything in the tree is
/// symbolic or outside the numeric slice — the signal to fall back on the
/// structural rules.
fn constant_facts(e: &Expr) -> Option<Facts> {
    Some(Facts::of_constant(eval_complex(e, &Default::default())?))
}
