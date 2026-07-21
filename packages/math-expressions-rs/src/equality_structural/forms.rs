//! Whole-shape predicates: expanded, completely factored, combined like terms,
//! completed square, and the integration-constant check.

use super::helpers::{as_int, strip_neg, symbol_occurrences};
use crate::expr::{Expr, SeqKind};
use crate::norm::canonicalize;

fn is_sum(e: &Expr) -> bool {
    matches!(e, Expr::Add(_))
}

/// Fully distributed: no `Mul` has a sum factor, and no sum is raised to an
/// integer power ≥ 2. (Matches STACK's `Expanded`: `x²-(a+b)x+ab` is expanded,
/// `(x-a)(x-b)` and `(x+1)^2` are not.)
pub(super) fn is_expanded(e: &Expr) -> bool {
    let here = match e {
        Expr::Mul(fs) => !fs.iter().any(is_sum),
        Expr::Pow(b, exp) => !(is_sum(b) && as_int(exp).map(|k| k >= 2).unwrap_or(false)),
        _ => true,
    };
    here && e.children().iter().all(|c| is_expanded(c))
}

/// A product of irreducible factors. Oracle: `factor` produces the fully
/// factored form and `canonicalize` keeps products factored, so an already
/// completely-factored `e` satisfies `canonicalize(e) == canonicalize(factor(e))`.
/// (Univariate over ℚ — `factor` returns multivariate/non-poly unchanged, so
/// those are reported factored; documented limitation.)
pub(super) fn is_factored_completely(e: &Expr) -> bool {
    canonicalize(e) == canonicalize(&crate::factor(e))
}

/// The "monomial key" of an additive term: its canonical form with any leading
/// numeric coefficient stripped. `None` marks a pure constant.
fn term_key(t: &Expr) -> Option<Expr> {
    match canonicalize(t) {
        Expr::Num(_) => None,
        Expr::Mul(mut fs) if matches!(fs.first(), Some(Expr::Num(_))) => {
            fs.remove(0);
            Some(canonicalize(&Expr::Mul(fs)))
        }
        other => Some(other),
    }
}

/// Two summands share a monomial key (combinable), or two summands are pure
/// constants (un-reduced arithmetic), in some `Add` node.
pub(super) fn like_terms_remain(e: &Expr) -> bool {
    if let Expr::Add(terms) = e {
        let mut keys: Vec<Expr> = Vec::new();
        let mut consts = 0;
        for t in terms {
            match term_key(t) {
                None => consts += 1,
                Some(k) => {
                    if keys.contains(&k) {
                        return true;
                    }
                    keys.push(k);
                }
            }
        }
        if consts >= 2 {
            return true;
        }
    }
    e.children().iter().any(|c| like_terms_remain(c))
}

/// `a(x - h)^2 + k`: a sum whose only non-constant term is a (coefficient times
/// a) square of a degree-1 expression in a single variable.
pub(super) fn is_completed_square(e: &Expr) -> bool {
    let Expr::Add(terms) = e else { return false };
    let vars = crate::variables(e);
    if vars.len() != 1 {
        return false;
    }
    let var = &vars[0];
    let mentions = |t: &Expr| t.any_subexpr(&|c| matches!(c, Expr::Sym(s) if &s.name()==var));
    let mut squares = 0;
    for t in terms {
        if !mentions(t) {
            continue; // the constant k
        }
        if is_square_of_linear(t, var) {
            squares += 1;
        } else {
            return false; // a variable term that is not the square
        }
    }
    squares == 1
}

/// `(linear)^2` or `c*(linear)^2`, linear = degree-1 in `var`.
fn is_square_of_linear(t: &Expr, var: &str) -> bool {
    match t {
        Expr::Mul(fs) => {
            // exactly one factor is the square; the rest are var-free.
            let mut sq = 0;
            for f in fs {
                let has_var = f.any_subexpr(&|c| matches!(c, Expr::Sym(s) if s.name()==var));
                if has_var {
                    if is_square_of_linear(f, var) {
                        sq += 1;
                    } else {
                        return false;
                    }
                }
            }
            sq == 1
        }
        Expr::Pow(b, exp) => as_int(exp) == Some(2) && is_linear_in(b, var),
        _ => false,
    }
}

/// Is `e` a degree-1 polynomial in `var` (`a·var + b`, with `a`/`b` free of
/// `var` and at least one `a·var` term)? Every canonical additive term must be
/// var-free or `coeff·var` with `var` a *bare* factor — so `var` inside a power
/// (`x^2`), a function (`sin(x)`), or a denominator (`1/x`) is rejected.
fn is_linear_in(e: &Expr, var: &str) -> bool {
    let terms = match canonicalize(e) {
        Expr::Add(ts) => ts,
        other => vec![other],
    };
    let mut has_var_term = false;
    for t in &terms {
        if symbol_occurrences(t, var) == 0 {
            continue; // a var-free (constant) term
        }
        let linear_term = match t {
            Expr::Sym(s) => s.name() == var,
            Expr::Mul(fs) => {
                let var_factors = fs
                    .iter()
                    .filter(|f| matches!(f, Expr::Sym(s) if s.name() == var))
                    .count();
                // exactly one bare `var` factor, and no `var` hiding elsewhere.
                var_factors == 1
                    && fs
                        .iter()
                        .filter(|f| !matches!(f, Expr::Sym(s) if s.name() == var))
                        .all(|f| symbol_occurrences(f, var) == 0)
            }
            _ => false, // var inside a Pow / Apply / Div / …
        };
        if !linear_term {
            return false;
        }
        has_var_term = true;
    }
    has_var_term
}

pub(super) fn has_integration_constant(e: &Expr, exclude: Option<&str>) -> bool {
    let Expr::Add(terms) = e else { return false };
    // A `+ C` is an *isolated* additive symbol: a bare symbol (not the excluded
    // integration variable) that appears nowhere else. Requiring it appear
    // exactly once rejects a variable that merely happens to be a lone term
    // (`x + x^2`). It cannot distinguish `x + C` from `x + y` — that genuinely
    // needs `exclude` — so pass the integration variable there when known.
    terms.iter().any(|t| match strip_neg(t) {
        Expr::Sym(s) => {
            let name = s.name();
            exclude.map(|v| name != v).unwrap_or(true) && symbol_occurrences(e, &name) == 1
        }
        _ => false,
    })
}

// Kept for a future `MatchesTemplate`/sequence-aware check; silence dead-code
// until then without dropping the intent.
#[allow(dead_code)]
fn is_seq(e: &Expr, kind: SeqKind) -> bool {
    matches!(e, Expr::Seq(k, _) if *k == kind)
}
