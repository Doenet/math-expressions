//! Normalization (PORTING_PLAN.md §7, redesign note): a pure
//! faithful-layer → canonical-layer transform. Canonical form eliminates the
//! display-only variants (`Div`, `Neg`), flattens and sorts commutative
//! operators, folds constants *exactly* (the §3a payoff), and combines like
//! terms and like powers — so two equal canonical expressions are identical
//! trees and structural equality is tree comparison.
//!
//! `canonicalize` is confluent, cheap, and assumption-free. Heuristic
//! simplification (root pulling, trig/log identities) is a separate, deferred
//! layer that needs the assumptions system.

pub(crate) mod order;

use crate::expr::{Expr, SeqKind};
use crate::num::Number;

pub(crate) use order::cmp;

/// Bottom-up canonicalization: canonicalize children, then apply the smart
/// constructor for the node.
pub fn canonicalize(e: &Expr) -> Expr {
    match e {
        Expr::Num(_) | Expr::Sym(_) | Expr::Const(_) | Expr::Blank | Expr::Ldots => e.clone(),

        Expr::Add(ts) => add(ts.iter().map(canonicalize).collect()),
        Expr::Mul(fs) => mul(fs.iter().map(canonicalize).collect()),
        // Display-only variants rewrite into the algebraic core.
        Expr::Div(a, b) => mul(vec![
            canonicalize(a),
            pow(canonicalize(b), Expr::Num(Number::Int(-1))),
        ]),
        Expr::Neg(x) => mul(vec![Expr::Num(Number::Int(-1)), canonicalize(x)]),
        Expr::Pow(b, e) => pow(canonicalize(b), canonicalize(e)),

        Expr::And(xs) => assoc_sorted(Variant::And, xs),
        Expr::Or(xs) => assoc_sorted(Variant::Or, xs),
        Expr::Union(xs) => assoc_sorted(Variant::Union, xs),
        Expr::Intersect(xs) => assoc_sorted(Variant::Intersect, xs),
        Expr::Not(x) => Expr::Not(Box::new(canonicalize(x))),

        Expr::Apply(head, args) => Expr::Apply(
            Box::new(normalize_head(canonicalize(head))),
            args.iter().map(canonicalize).collect(),
        ),
        Expr::Prime(x) => Expr::Prime(Box::new(canonicalize(x))),
        Expr::Index(a, b) => Expr::Index(Box::new(canonicalize(a)), Box::new(canonicalize(b))),

        Expr::Seq(kind, xs) => {
            let mut v: Vec<Expr> = xs.iter().map(canonicalize).collect();
            // A set is unordered; sort so `{a, b}` and `{b, a}` canonicalize
            // alike. Ordered sequences keep their positions.
            if *kind == SeqKind::Set {
                v.sort_by(cmp);
            }
            Expr::Seq(*kind, v)
        }
        Expr::Interval { endpoints, closed } => Expr::Interval {
            endpoints: Box::new((canonicalize(&endpoints.0), canonicalize(&endpoints.1))),
            closed: *closed,
        },
        Expr::Relation { operands, ops } => Expr::Relation {
            operands: operands.iter().map(canonicalize).collect(),
            ops: ops.clone(),
        },
        Expr::Matrix {
            rows,
            cols,
            entries,
        } => Expr::Matrix {
            rows: *rows,
            cols: *cols,
            entries: entries.iter().map(canonicalize).collect(),
        },
        Expr::OtherOp(name, args) => Expr::OtherOp(*name, args.iter().map(canonicalize).collect()),
    }
}

// ---- Smart constructors (assume children are already canonical) ----

/// Build a canonical sum from canonical terms: flatten, fold the numeric part
/// exactly, combine like terms (`3x + 2x → 5x`), drop zeros, sort.
pub(crate) fn add(terms: Vec<Expr>) -> Expr {
    let mut flat = Vec::with_capacity(terms.len());
    for t in terms {
        match t {
            Expr::Add(xs) => flat.extend(xs),
            other => flat.push(other),
        }
    }

    let mut constant = Number::zero();
    // (rest, coefficient) for each distinct non-constant term.
    let mut parts: Vec<(Expr, Number)> = Vec::new();
    for t in flat {
        let (coeff, rest) = split_coeff(t);
        match rest {
            None => constant = constant.add(&coeff),
            Some(r) => match parts.iter_mut().find(|(k, _)| *k == r) {
                Some(slot) => slot.1 = slot.1.add(&coeff),
                None => parts.push((r, coeff)),
            },
        }
    }

    let mut out = Vec::with_capacity(parts.len() + 1);
    if !constant.is_zero() {
        out.push(Expr::Num(constant));
    }
    for (rest, coeff) in parts {
        if coeff.is_zero() {
            continue;
        }
        out.push(mul(vec![Expr::Num(coeff), rest]));
    }
    out.sort_by(cmp);
    match out.len() {
        0 => Expr::Num(Number::zero()),
        1 => out.pop().unwrap(),
        _ => Expr::Add(out),
    }
}

/// Build a canonical product from canonical factors: flatten, fold the numeric
/// coefficient exactly, annihilate on zero, combine like powers
/// (`x² · x³ → x⁵`), drop ones, sort.
pub(crate) fn mul(factors: Vec<Expr>) -> Expr {
    let mut flat = Vec::with_capacity(factors.len());
    for f in factors {
        match f {
            Expr::Mul(xs) => flat.extend(xs),
            other => flat.push(other),
        }
    }

    let mut coeff = Number::one();
    // (base, summed exponent) for each distinct base.
    let mut parts: Vec<(Expr, Expr)> = Vec::new();
    for f in flat {
        if let Expr::Num(n) = &f {
            coeff = coeff.mul(n);
            continue;
        }
        let (base, exp) = split_pow(f);
        match parts.iter_mut().find(|(b, _)| *b == base) {
            Some(slot) => slot.1 = add(vec![std::mem::replace(&mut slot.1, Expr::Blank), exp]),
            None => parts.push((base, exp)),
        }
    }

    if coeff.is_zero() {
        return Expr::Num(Number::zero());
    }

    let mut out = Vec::with_capacity(parts.len() + 1);
    for (base, exp) in parts {
        match pow(base, exp) {
            // A folded power may collapse to a number (e.g. exponent 0 → 1).
            Expr::Num(n) => coeff = coeff.mul(&n),
            other => out.push(other),
        }
    }
    if coeff.is_zero() {
        return Expr::Num(Number::zero());
    }
    out.sort_by(cmp);
    if out.is_empty() {
        return Expr::Num(coeff);
    }
    // The numeric coefficient sorts first (Num has the lowest rank).
    if !coeff.is_one() {
        out.insert(0, Expr::Num(coeff));
    }
    if out.len() == 1 {
        out.pop().unwrap()
    } else {
        Expr::Mul(out)
    }
}

/// Build a canonical power, applying the identities and constant folding that
/// hold without assumptions. `0` to a negative power is left unfolded (an
/// exact division by zero — §3.6 of the redesign note).
pub(crate) fn pow(base: Expr, exp: Expr) -> Expr {
    if let Expr::Num(e) = &exp {
        if e.is_zero() {
            return Expr::Num(Number::one()); // x^0 = 1, including 0^0
        }
        if e.is_one() {
            return base;
        }
    }
    if let Expr::Num(b) = &base {
        if b.is_one() {
            return Expr::Num(Number::one()); // 1^x = 1
        }
        // `as_int` matches only an integer exponent, the case we fold.
        if let Some(k) = as_int(&exp) {
            if let Some(v) = b.checked_pow_int(k) {
                return Expr::Num(v);
            }
            // 0^negative: fall through, stays a Pow node.
        }
    }
    Expr::Pow(Box::new(base), Box::new(exp))
}

// ---- helpers ----

/// A term split into (coefficient, remaining factor). `None` remainder means
/// the term is a pure number.
fn split_coeff(t: Expr) -> (Number, Option<Expr>) {
    match t {
        Expr::Num(n) => (n, None),
        Expr::Mul(xs) => {
            if let Some(Expr::Num(n)) = xs.first() {
                let n = n.clone();
                let rest = mul(xs[1..].to_vec());
                (n, Some(rest))
            } else {
                (Number::one(), Some(Expr::Mul(xs)))
            }
        }
        other => (Number::one(), Some(other)),
    }
}

/// A factor split into (base, exponent).
fn split_pow(f: Expr) -> (Expr, Expr) {
    match f {
        Expr::Pow(b, e) => (*b, *e),
        other => (other, Expr::Num(Number::one())),
    }
}

fn as_int(n: &Expr) -> Option<i64> {
    match n {
        Expr::Num(Number::Int(i)) => Some(*i),
        _ => None,
    }
}

enum Variant {
    And,
    Or,
    Union,
    Intersect,
}

/// Flatten same-variant children and sort (for commutative boolean/set ops).
fn assoc_sorted(variant: Variant, xs: &[Expr]) -> Expr {
    let mut flat: Vec<Expr> = Vec::with_capacity(xs.len());
    for x in xs.iter().map(canonicalize) {
        match (&variant, &x) {
            (Variant::And, Expr::And(v))
            | (Variant::Or, Expr::Or(v))
            | (Variant::Union, Expr::Union(v))
            | (Variant::Intersect, Expr::Intersect(v)) => flat.extend(v.iter().cloned()),
            _ => flat.push(x),
        }
    }
    flat.sort_by(cmp);
    match variant {
        Variant::And => Expr::And(flat),
        Variant::Or => Expr::Or(flat),
        Variant::Union => Expr::Union(flat),
        Variant::Intersect => Expr::Intersect(flat),
    }
}

/// Canonicalize a function head's name (`arcsin → asin`, `ln → log`, …). The
/// head is otherwise already canonical; only a bare function symbol is renamed.
fn normalize_head(head: Expr) -> Expr {
    if let Expr::Sym(s) = &head {
        if let Some(canonical) = normalize_function_name(&s.name()) {
            return Expr::sym(canonical);
        }
    }
    head
}

/// The function-name normalization table (lib/expression/normalization/
/// standard_form.js `function_normalizations`).
fn normalize_function_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "ln" => "log",
        "arccos" => "acos",
        "arccosh" => "acosh",
        "arcsin" => "asin",
        "arcsinh" => "asinh",
        "arctan" => "atan",
        "arctanh" => "atanh",
        "arcsec" => "asec",
        "arcsech" => "asech",
        "arccsc" => "acsc",
        "arccsch" => "acsch",
        "arccot" => "acot",
        "arccoth" => "acoth",
        "cosec" => "csc",
        _ => return None,
    })
}
