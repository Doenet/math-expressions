//! Presentation layer: canonical tree → display tree.
//!
//! `canonicalize`'s output is optimized for *equality testing*, not reading:
//! `Div` becomes `Mul·Pow⁻¹`, `Neg` becomes a `−1` coefficient, and
//! commutative operands sort by `norm::order`'s variant-rank order, which
//! prints `x^2 + 2x + 1` as `1 + x^2 + 2 x`. This pass converts a canonical
//! tree into the equivalent faithful tree a calculus student would write:
//!
//! - negative exponents become division (`x^(−1) → 1/x`, `3 x^(−2) → 3/x²`),
//!   with rational coefficients joining the fraction (`(2/3)·x⁻¹ → 2/(3 x)`);
//! - negative leading coefficients become `Neg`, so sums render with `−`;
//! - `Add` terms sort like a polynomial: descending total degree, ties broken
//!   graded-lexicographically on the variables — so constants come last and
//!   `x² + x y + y²` reads in the conventional order;
//! - `Mul` factors sort coefficient first, then alphabetically by base.
//!
//! The pass is display-only and meaning-preserving: `canonicalize(present(e))
//! == e` for canonical `e`, and `present` is idempotent (it understands the
//! `Div`/`Neg` shapes it produces, so re-presenting is a no-op).

use crate::expr::Expr;
use crate::num::{BigNumber, Number};
use std::cmp::Ordering;

use super::syntactic::map_children;

/// Convert a canonical tree to its display form (see module docs).
pub(crate) fn present(e: &Expr) -> Expr {
    match e {
        Expr::Add(ts) => present_add(ts),
        Expr::Mul(fs) => present_mul(fs),
        Expr::Pow(b, x) => present_pow(b, x),
        _ => map_children(e, present),
    }
}

/// Present an exponent. A non-integer exact rational exponent displays as a
/// fraction — `x^(3/2)`, not the renderer's terminating-decimal `x^1.5`.
/// Only exponents get this: elsewhere a rational either joins a fraction bar
/// (`present_mul`) or stays a plain number so exact decimal folds still
/// render as decimals (`0.1 + 0.2 → 0.3`, the §3a round-trip).
fn present_exponent(x: &Expr) -> Expr {
    if let Expr::Num(n) = x {
        let (neg, num, den) = split_number(n);
        if !den.is_one() {
            let frac = Expr::Div(Box::new(Expr::Num(num)), Box::new(Expr::Num(den)));
            return if neg { Expr::Neg(Box::new(frac)) } else { frac };
        }
    }
    present(x)
}

/// `b^x` with a negative exponent displays as `1/b^(−x)`.
fn present_pow(b: &Expr, x: &Expr) -> Expr {
    // A matrix-valued base never moves under a fraction bar: `A^(-1)` is the
    // inverse, not the scalar `1/A`.
    if matches!(b, Expr::Matrix { .. }) {
        return Expr::Pow(Box::new(present(b)), Box::new(present_exponent(x)));
    }
    if let Some(pos) = negated_exponent(x) {
        return Expr::Div(
            Box::new(Expr::int(1)),
            Box::new(pow_display(present(b), present_exponent(&pos))),
        );
    }
    Expr::Pow(Box::new(present(b)), Box::new(present_exponent(x)))
}

/// `base^exp` for display, collapsing `base^1` (which arises when a `x^(−1)`
/// factor moves to a denominator) to `base`.
fn pow_display(base: Expr, exp: Expr) -> Expr {
    if matches!(&exp, Expr::Num(n) if n.is_one()) {
        base
    } else {
        Expr::Pow(Box::new(base), Box::new(exp))
    }
}

/// If `x` is a definitely-negative canonical exponent, return its negation:
/// a negative number, or a `Mul` whose leading numeric coefficient is
/// negative (`Mul(−1, n) → n`). `None` for anything sign-ambiguous.
fn negated_exponent(x: &Expr) -> Option<Expr> {
    match x {
        Expr::Num(n) if n.is_negative() => Some(Expr::Num(n.neg())),
        Expr::Mul(fs) => match fs.first() {
            Some(Expr::Num(n)) if n.is_negative() => {
                let m = n.neg();
                let rest = &fs[1..];
                if m.is_one() && rest.len() == 1 {
                    Some(rest[0].clone())
                } else if m.is_one() {
                    Some(Expr::Mul(rest.to_vec()))
                } else {
                    let mut out = vec![Expr::Num(m)];
                    out.extend(rest.iter().cloned());
                    Some(Expr::Mul(out))
                }
            }
            _ => None,
        },
        _ => None,
    }
}

/// Split a canonical `Mul` into sign, numerator, and denominator, and
/// reassemble as `[Neg] num`, `[Neg] num/den`. The numeric coefficient's
/// numerator/denominator split across the fraction bar (`(2/3)·x⁻¹ →
/// 2/(3 x)`, `(1/2)·x → x/2`).
fn present_mul(fs: &[Expr]) -> Expr {
    let mut negative = false;
    let mut coeff_num = Number::Int(1);
    let mut coeff_den = Number::Int(1);
    let mut num_factors: Vec<Expr> = Vec::new();
    let mut den_factors: Vec<Expr> = Vec::new();

    for f in fs {
        match f {
            Expr::Num(n) => {
                let (neg, num, den) = split_number(n);
                negative ^= neg;
                coeff_num = num;
                coeff_den = den;
            }
            Expr::Pow(b, x) => {
                let neg_exp = if matches!(**b, Expr::Matrix { .. }) {
                    None // A^(-1) is an inverse, not a fraction (MATRIX_PLAN §1a)
                } else {
                    negated_exponent(x)
                };
                if let Some(pos) = neg_exp {
                    den_factors.push(pow_display(present(b), present_exponent(&pos)));
                } else {
                    num_factors.push(present(f));
                }
            }
            _ => num_factors.push(present(f)),
        }
    }

    sort_factors(&mut num_factors);
    sort_factors(&mut den_factors);

    let num = assemble(coeff_num, num_factors);
    let out = if den_factors.is_empty() && coeff_den.is_one() {
        num
    } else {
        Expr::Div(Box::new(num), Box::new(assemble(coeff_den, den_factors)))
    };
    if negative {
        Expr::Neg(Box::new(out))
    } else {
        out
    }
}

/// One side of a fraction bar: the coefficient (dropped when it is a
/// redundant 1) followed by the factors.
fn assemble(coeff: Number, factors: Vec<Expr>) -> Expr {
    let mut items = Vec::with_capacity(factors.len() + 1);
    if !coeff.is_one() || factors.is_empty() {
        items.push(Expr::Num(coeff));
    }
    items.extend(factors);
    if items.len() == 1 {
        items.pop().unwrap()
    } else {
        Expr::Mul(items)
    }
}

/// `n` as (is_negative, |numerator|, denominator). Floats and integers have
/// denominator 1; exact rationals split across the fraction bar.
fn split_number(n: &Number) -> (bool, Number, Number) {
    let neg = n.is_negative();
    let a = n.abs();
    match &a {
        Number::Rat(p, q) => (neg, Number::Int(*p), Number::Int(*q)),
        Number::Big(b) => match &**b {
            BigNumber::Rat(r) => (
                neg,
                Number::from_bigint(r.numer().clone()),
                Number::from_bigint(r.denom().clone()),
            ),
            BigNumber::Int(_) => (neg, a, Number::Int(1)),
        },
        _ => (neg, a, Number::Int(1)),
    }
}

/// Sort multiplicands alphabetically by their base symbol (`x² y`, not
/// `y x²`); constants (`π`) come first, factors with no symbol/constant base
/// (functions, sums) keep their canonical order at the end.
fn sort_factors(factors: &mut [Expr]) {
    fn key(f: &Expr) -> (u8, String) {
        let base = if let Expr::Pow(b, _) = f { &**b } else { f };
        match base {
            Expr::Const(_) => (0, String::new()),
            Expr::Sym(s) => (1, s.name().to_string()),
            _ => (2, String::new()),
        }
    }
    factors.sort_by_cached_key(key); // stable: ties keep canonical order
}

/// Order `Add` terms like a polynomial: descending total degree, then
/// graded-lexicographic on the (alphabetized) variables. The sort is stable,
/// so equal keys keep their canonical order.
fn present_add(ts: &[Expr]) -> Expr {
    let mut items: Vec<(DegKey, Expr)> = ts.iter().map(|t| (deg_key(t), present(t))).collect();
    items.sort_by(|a, b| key_order(&a.0, &b.0));
    Expr::Add(items.into_iter().map(|p| p.1).collect())
}

/// A term's monomial signature: total degree plus per-variable exponents
/// (name-sorted). Non-monomial parts (function applications, unexpanded
/// powers of sums, symbolic exponents) contribute degree 0, so they sort
/// with the constants.
struct DegKey {
    total: f64,
    vars: Vec<(String, f64)>,
}

fn deg_key(t: &Expr) -> DegKey {
    let mut vars: Vec<(String, f64)> = Vec::new();
    collect_deg(t, 1.0, &mut vars);
    vars.sort_by(|a, b| a.0.cmp(&b.0));
    // Merge duplicate names (e.g. from a presented `Div` with x on both sides).
    vars.dedup_by(|next, prev| {
        if prev.0 == next.0 {
            prev.1 += next.1;
            true
        } else {
            false
        }
    });
    vars.retain(|(_, d)| *d != 0.0);
    let total = vars.iter().map(|(_, d)| d).sum();
    DegKey { total, vars }
}

fn collect_deg(t: &Expr, mult: f64, vars: &mut Vec<(String, f64)>) {
    match t {
        Expr::Sym(s) if !crate::sym::is_constant_symbol(&s.name()) => {
            vars.push((s.name().to_string(), mult));
        }
        Expr::Pow(b, x) => {
            if let Expr::Num(n) = &**x {
                collect_deg(b, mult * n.to_f64(), vars);
            }
        }
        Expr::Mul(fs) => {
            for f in fs {
                collect_deg(f, mult, vars);
            }
        }
        // Presented shapes, so re-presenting (idempotence) sees the same keys.
        Expr::Neg(a) => collect_deg(a, mult, vars),
        Expr::Div(a, b) => {
            collect_deg(a, mult, vars);
            collect_deg(b, -mult, vars);
        }
        _ => {}
    }
}

/// `Less` ⇔ `a` displays before `b`: higher total degree first, then the
/// first alphabetical variable where the exponents differ, higher first.
fn key_order(a: &DegKey, b: &DegKey) -> Ordering {
    match b.total.partial_cmp(&a.total) {
        Some(Ordering::Equal) | None => {}
        Some(o) => return o,
    }
    let (mut i, mut j) = (0, 0);
    while i < a.vars.len() || j < b.vars.len() {
        let (an, ad) = a
            .vars
            .get(i)
            .map(|(n, d)| (n.as_str(), *d))
            .unwrap_or(("\u{10FFFF}", 0.0));
        let (bn, bd) = b
            .vars
            .get(j)
            .map(|(n, d)| (n.as_str(), *d))
            .unwrap_or(("\u{10FFFF}", 0.0));
        match an.cmp(bn) {
            Ordering::Equal => {
                match bd.partial_cmp(&ad) {
                    Some(Ordering::Equal) | None => {}
                    Some(o) => return o,
                }
                i += 1;
                j += 1;
            }
            // One term has an (alphabetically earlier) variable the other
            // lacks: the term that has it displays first (x·y before z²).
            Ordering::Less => return Ordering::Less,
            Ordering::Greater => return Ordering::Greater,
        }
    }
    Ordering::Equal
}
