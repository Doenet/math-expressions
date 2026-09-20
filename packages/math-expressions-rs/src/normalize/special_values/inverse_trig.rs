//! The inverse direction: asin/acos/atan/asec/acsc/acot at a value the forward
//! tables take, their parity, their compositions with the forward functions,
//! and the complementary sums.

use super::util::{apply, canon, negate, strip_negation, INVERSE_TRIG};
use crate::expr::Expr;
use crate::normalize::{add, mul, pow};

pub(super) fn fold_inverse_trig(name: &str, arg: &Expr) -> Option<Expr> {
    crate::eval_exact::inverse_trig_special_value(name, arg)
        .or_else(|| inverse_trig_parity(name, arg))
}

/// `asin(−u) → −asin u` and its five siblings.
///
/// asin, atan, acsc and acot are odd; acos and asec reflect through π/2:
/// `acos(−u) = π − acos u`. The reciprocal three follow from the other three
/// because this library *defines* them that way — `acsc z = asin(1/z)`,
/// `asec z = acos(1/z)`, `acot z = atan(1/z)` (see
/// `special_functions::trig_inverse`) — and `1/(−u) = −(1/u)`. Note what that
/// makes acot: **odd**, with range `(−π/2, π/2)`. Under the other common
/// convention, range `(0, π)`, it would instead satisfy `acot(−u) = π − acot u`;
/// the rule here follows the definition the rest of the engine evaluates.
fn inverse_trig_parity(name: &str, arg: &Expr) -> Option<Expr> {
    let u = strip_negation(&canon(arg))?;
    Some(match name {
        "asin" | "atan" | "acsc" | "acot" => negate(&apply(name, u)),
        "acos" | "asec" => canon(&add(vec![
            Expr::sym("pi"),
            mul(vec![Expr::int(-1), apply(name, u)]),
        ])),
        _ => return None,
    })
}

/// `asin u + acos u → π/2`, and likewise `acsc u + asec u`.
///
/// Both hold for every `u`, with no domain condition: the principal branches
/// are defined so that `acos = π/2 − asin`, and the reciprocal pair is that
/// same identity at `1/u`.
///
/// `atan + acot` is deliberately *not* here. With `acot u` defined as
/// `atan(1/u)`, `atan u + acot u` is `π/2` for positive `u` and `−π/2` for
/// negative `u`, so there is no unconditional value to fold to.
///
/// The two terms must carry identical remaining factors, so `3 asin x + 3 acos x`
/// folds to `3π/2` while `2 asin x + acos x` is left alone — splitting the
/// latter would not shorten it anyway.
pub(super) fn fold_complementary(ts: &[Expr]) -> Option<Expr> {
    const PAIRS: [(&str, &str); 2] = [("asin", "acos"), ("acsc", "asec")];
    let parts: Vec<Option<(Vec<Expr>, String, Expr)>> = ts.iter().map(split_inverse_term).collect();
    // The scan below is quadratic in the number of summands, and every `Add`
    // node in the tree reaches it. Almost none of them contain two inverse trig
    // terms, so settle that in one linear pass first.
    if parts.iter().flatten().count() < 2 {
        return None;
    }
    for (i, pi) in parts.iter().enumerate() {
        for (j, pj) in parts.iter().enumerate() {
            let (Some((ri, ni, ui)), Some((rj, nj, uj))) = (pi, pj) else {
                continue;
            };
            if i == j || ri != rj || ui != uj || !PAIRS.contains(&(ni.as_str(), nj.as_str())) {
                continue;
            }
            let mut out: Vec<Expr> = ts
                .iter()
                .enumerate()
                .filter(|(k, _)| *k != i && *k != j)
                .map(|(_, t)| t.clone())
                .collect();
            let mut half_pi = vec![Expr::Num(crate::num::Number::rat(1, 2)), Expr::sym("pi")];
            half_pi.extend(ri.iter().cloned());
            out.push(mul(half_pi));
            return Some(canon(&add(out)));
        }
    }
    None
}

/// A term of the shape `rest · f(u)` with `f` inverse trig, split into
/// `(rest, f, u)`. Only the first such factor is taken, so `asin(x)·acos(x)`
/// (a product, not a sum) never looks like a complementary pair.
fn split_inverse_term(t: &Expr) -> Option<(Vec<Expr>, String, Expr)> {
    let factors: Vec<Expr> = match t {
        Expr::Mul(fs) => fs.clone(),
        other => vec![other.clone()],
    };
    let mut rest = Vec::new();
    let mut found = None;
    for f in factors {
        if found.is_none() {
            if let Some(hit) = inverse_trig_apply(&f) {
                found = Some(hit);
                continue;
            }
        }
        rest.push(f);
    }
    let (name, u) = found?;
    Some((rest, name, u))
}

/// `f(g⁻¹(u))` in closed form, for every trig / inverse-trig pair.
///
/// Each `g⁻¹(u)` is turned into the pair `(sin θ, cos θ)` for `θ = g⁻¹(u)`, and
/// the outer function is then read off that pair. The direct pairs come out of
/// the same machinery as everything else: `sec(asec u)` is `1/cos(acos(1/u))`
/// is `1/(1/u)` is `u`.
///
/// Every one of these is exact on the principal branches — no `|u| ≤ 1`
/// condition. `cos(asin u) = √(1−u²)` because asin's range is `[−π/2, π/2]`,
/// where cosine is nonnegative, and the principal square root continues that
/// agreement off the real interval; `cos(atan u) = 1/√(1+u²)` likewise, atan's
/// range having positive cosine throughout.
pub(super) fn compose_with_inverse(outer: &str, arg: &Expr) -> Option<Expr> {
    let (inner, u) = inverse_trig_apply(arg)?;
    let (sin, cos) = sin_cos_of_inverse(&inner, &u)?;
    Some(canon(&match outer {
        "sin" => sin,
        "cos" => cos,
        "tan" => mul(vec![sin, pow(cos, Expr::int(-1))]),
        "cot" => mul(vec![cos, pow(sin, Expr::int(-1))]),
        "sec" => pow(cos, Expr::int(-1)),
        "csc" => pow(sin, Expr::int(-1)),
        _ => return None,
    }))
}

/// `(sin θ, cos θ)` for `θ = name(u)`, as canonical expressions.
fn sin_cos_of_inverse(name: &str, u: &Expr) -> Option<(Expr, Expr)> {
    // The reciprocal three are their partner at 1/u, by definition.
    let (base, u) = match name {
        "asin" | "acos" | "atan" => (name, canon(u)),
        "acsc" => ("asin", reciprocal(u)),
        "asec" => ("acos", reciprocal(u)),
        "acot" => ("atan", reciprocal(u)),
        _ => return None,
    };
    // √(1 − u²) for asin/acos; for atan, sin and cos share the factor 1/√(1+u²).
    Some(match base {
        "asin" => (u.clone(), sqrt_of(&add(vec![Expr::int(1), neg_square(&u)]))),
        "acos" => (sqrt_of(&add(vec![Expr::int(1), neg_square(&u)])), u.clone()),
        _ => {
            let w = reciprocal(&sqrt_of(&add(vec![
                Expr::int(1),
                pow(u.clone(), Expr::int(2)),
            ])));
            (canon(&mul(vec![u, w.clone()])), w)
        }
    })
}

fn sqrt_of(u: &Expr) -> Expr {
    canon(&apply("sqrt", canon(u)))
}

fn reciprocal(u: &Expr) -> Expr {
    canon(&pow(u.clone(), Expr::int(-1)))
}

fn neg_square(u: &Expr) -> Expr {
    mul(vec![Expr::int(-1), pow(u.clone(), Expr::int(2))])
}

/// `f(u)` split into `(f, u)` when `f` is one of the inverse trig functions.
fn inverse_trig_apply(e: &Expr) -> Option<(String, Expr)> {
    if let Expr::Apply(head, args) = e {
        if let (Expr::Sym(s), [u]) = (&**head, args.as_slice()) {
            let name = s.name();
            if INVERSE_TRIG.contains(&name.as_str()) {
                return Some((name, u.clone()));
            }
        }
    }
    None
}
