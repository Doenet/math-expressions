//! The arithmetic rules: what a sum, product or power inherits from its parts.
//!
//! Ported from the `+` / `*` / `^` branches of `element_of_sets.js`, including
//! its conservatisms — sums do no interval arithmetic, and an odd power of a
//! possibly-negative base gets no sign. Only `nonneg`/`positive` are derived
//! here; [`Facts::normalize`](super::super::facts::Facts::normalize) fills in
//! `negative`/`nonpos`. Sums and products live here; the `^` branch is long
//! enough to warrant its own module ([`pow`]).

mod pow;

pub(super) use pow::pow;

use super::super::facts::Facts;
use super::super::{Assumptions, MaybeBool};
use super::facts;
use crate::expr::Expr;

/// Could this factor be `±∞` (or `NaN`)? Only a power can: `b^e` blows up when
/// `b` may be zero while `e` may be zero or negative.
fn may_be_infinite(e: &Expr, a: &Assumptions) -> bool {
    let Expr::Pow(base, exp) = e else {
        return false;
    };
    facts(base, a).nonzero != Some(true) && facts(exp, a).positive != Some(true)
}

/// Three-valued "all of them": T if every entry is T, F never inferred here,
/// U otherwise.
fn all(fs: &[Facts], get: impl Fn(&Facts) -> MaybeBool) -> MaybeBool {
    if fs.iter().all(|f| get(f) == Some(true)) {
        Some(true)
    } else {
        None
    }
}

pub(super) fn add(fs: &[Facts]) -> Facts {
    // A term known to be zero contributes nothing. JS gets this from
    // `simplify`, which drops the term before any predicate runs, so `x + 0`
    // is answered as plain `x` — including `is_nonzero`, which the rules below
    // could not recover.
    let terms: Vec<Facts> = fs
        .iter()
        .filter(|f| f.nonzero != Some(false))
        .copied()
        .collect();
    match terms.as_slice() {
        [] => return Facts::of_f64(0.0),
        [only] => return *only,
        _ => {}
    }
    let fs = terms.as_slice();

    let mut out = Facts::unknown();
    out.integer = all(fs, |f| f.integer);
    // Exactly one definitely-non-integer term among integers → not integer
    // (`3 + π`); two or more non-integers could cancel, so stay unknown.
    if out.integer.is_none()
        && fs.iter().filter(|f| f.integer == Some(false)).count() == 1
        && fs.iter().all(|f| f.integer.is_some())
    {
        out.integer = Some(false);
    }
    out.real = all(fs, |f| f.real);
    out.complex = all(fs, |f| f.complex);

    // Sign: only from uniform term signs (no interval arithmetic, like JS).
    let all_nonneg = fs.iter().all(|f| f.nonneg == Some(true));
    let all_nonpos = fs.iter().all(|f| f.nonpos == Some(true));
    let any_pos = fs.iter().any(|f| f.positive == Some(true));
    let any_neg = fs.iter().any(|f| f.negative == Some(true));
    if all_nonneg {
        out.nonneg = Some(true);
        if any_pos {
            out.positive = Some(true);
            out.nonzero = Some(true);
        }
    }
    if all_nonpos {
        out.positive = Some(false);
        if any_neg {
            out.nonneg = Some(false);
            out.nonzero = Some(true);
        }
    }
    out
}

pub(super) fn mul(factors: &[Expr], fs: &[Facts], a: &Assumptions) -> Facts {
    // "One confirmed zero factor makes product zero" — JS's rule verbatim, and
    // it holds even for a wholly unknown other factor. It stops at a division,
    // though: JS keeps `a/b` as its own node and refuses to answer at all
    // unless the denominator is known nonzero, since `0/0` is `NaN`, not `0`.
    // Canonicalization has rewritten every `a/b` to `a·b⁻¹`, so that guard has
    // to be recovered from the shape here.
    if fs.iter().any(|f| f.nonzero == Some(false)) && !factors.iter().any(|x| may_be_infinite(x, a))
    {
        return Facts::of_f64(0.0);
    }

    let mut out = Facts::unknown();
    out.integer = all(fs, |f| f.integer);
    out.real = all(fs, |f| f.real);
    out.complex = all(fs, |f| f.complex);
    // Nonzero is multiplicative in any field — no realness required
    // (`x ≠ 0` alone makes `2x`, `x·x` nonzero).
    if fs.iter().all(|f| f.nonzero == Some(true)) {
        out.nonzero = Some(true);
    }

    // Definite sign only when every factor is real with a definite strict-or-
    // zero-allowed sign.
    if out.real == Some(true) {
        let mut sign_known = true;
        let mut negatives = 0usize;
        let mut may_be_zero = false;
        for f in fs {
            if f.positive == Some(true) {
                // positive factor: no change
            } else if f.negative == Some(true) {
                negatives += 1;
            } else if f.nonneg == Some(true) {
                may_be_zero = true;
            } else if f.nonpos == Some(true) {
                negatives += 1;
                may_be_zero = true;
            } else {
                sign_known = false;
                break;
            }
        }
        if sign_known {
            let positive_product = negatives.is_multiple_of(2);
            if positive_product {
                out.nonneg = Some(true);
                if !may_be_zero {
                    out.positive = Some(true);
                    out.nonzero = Some(true);
                }
            } else {
                out.positive = Some(false);
                if !may_be_zero {
                    out.nonneg = Some(false);
                    out.nonzero = Some(true);
                }
            }
        }
    }
    out
}
