//! Radical predicates. Every radical check goes through one decomposition,
//! [`root_of`], so the many syntactic spellings of a root — `sqrt(x)`,
//! `cbrt(x)`, `nthroot(x, m)`, `x^(1/m)` (a `Div` exponent), `x^0.5` (a `Rat`
//! exponent), and their negative (in-denominator) variants — are recognized in
//! exactly one place. Also home to the negative-exponent scan.

use super::helpers::{as_int, is_negative_sign, is_power_free_int};
use crate::expr::Expr;

/// A root node decomposed into its radicand, index (`m ≥ 2`), and whether the
/// exponent is negative (so it sits under a fraction bar). `None` if `e` is not
/// a root. Only *unit-fraction* powers `x^(±1/m)` count — `x^(2/3)` and
/// integer-valued exponents like `x^(4/2)` are deliberately excluded.
struct Root<'a> {
    radicand: &'a Expr,
    index: u32,
    in_denominator: bool,
}

fn root_of(e: &Expr) -> Option<Root<'_>> {
    match e {
        Expr::Apply(h, args) => {
            let Expr::Sym(s) = &**h else { return None };
            let (index, radicand) = match s.name().as_str() {
                "sqrt" => (2, args.first()?),
                "cbrt" => (3, args.first()?),
                // `nthroot(x, m)` is the m-th root; single-arg is a square root.
                "nthroot" => match args.get(1).and_then(as_int) {
                    Some(m) => (valid_index(m)?, args.first()?),
                    None if args.len() == 1 => (2, args.first()?),
                    _ => return None,
                },
                _ => return None,
            };
            Some(Root { radicand, index, in_denominator: false })
        }
        Expr::Pow(b, exp) => {
            let (index, negative) = unit_root_exponent(exp)?;
            Some(Root { radicand: b, index, in_denominator: negative })
        }
        _ => None,
    }
}

/// A unit-fraction exponent `±1/m` → `(m, negative)`, across the `Div` (written
/// `1/m`), `Rat` (decimal `0.5`), and `Neg`-wrapped spellings.
fn unit_root_exponent(exp: &Expr) -> Option<(u32, bool)> {
    match exp {
        Expr::Neg(x) => unit_root_exponent(x).map(|(m, neg)| (m, !neg)),
        Expr::Num(crate::num::Number::Rat(n, d)) => unit_from(*n, *d),
        Expr::Div(a, b) => unit_from(as_int(a)?, as_int(b)?),
        _ => None,
    }
}

/// `n/d` as a root: numerator ±1, denominator the index `m ≥ 2`.
fn unit_from(n: i64, d: i64) -> Option<(u32, bool)> {
    if n.unsigned_abs() != 1 || d == 0 {
        return None;
    }
    let index = u32::try_from(d.unsigned_abs()).ok().filter(|&k| k >= 2)?;
    Some((index, (n < 0) ^ (d < 0)))
}

/// A valid root index `m ≥ 2` (guards the i64→u32 conversion against absurd
/// indices like `nthroot(x, 10^12)`).
fn valid_index(m: i64) -> Option<u32> {
    u32::try_from(m).ok().filter(|&k| k >= 2)
}

fn contains_radical(e: &Expr) -> bool {
    e.any_subexpr(&|n| root_of(n).is_some())
}

pub(super) fn denom_has_radical(e: &Expr) -> bool {
    // A root written with a negative exponent (`x^(-1/2)` = 1/√x) already sits
    // in a denominator — checked here so every exponent spelling is caught.
    if root_of(e).is_some_and(|r| r.in_denominator) {
        return true;
    }
    match e {
        Expr::Div(_, d) => contains_radical(d) || denom_has_radical(d),
        // `base^(negative)` = 1/base^|·|: a surd there if the base has one.
        Expr::Pow(b, exp) if is_negative_sign(exp) => {
            contains_radical(b) || denom_has_radical(b)
        }
        _ => e.children().iter().any(|c| denom_has_radical(c)),
    }
}

pub(super) fn is_radical_simplified(e: &Expr) -> bool {
    if denom_has_radical(e) {
        return false;
    }
    // Every integer radicand must be free of a perfect power matching its index
    // (`sqrt` → square-free, `cbrt` → cube-free, `x^(1/m)` → m-th-power free);
    // otherwise a factor could be pulled out.
    fn radicands_ok(e: &Expr) -> bool {
        let here = match root_of(e) {
            Some(r) => as_int(r.radicand).is_none_or(|n| is_power_free_int(n, r.index)),
            None => true,
        };
        here && e.children().iter().all(|c| radicands_ok(c))
    }
    radicands_ok(e)
}

pub(super) fn has_negative_exponent(e: &Expr) -> bool {
    e.any_subexpr(&|n| matches!(n, Expr::Pow(_, exp) if is_negative_sign(exp)))
}
