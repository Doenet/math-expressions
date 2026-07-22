//! Shared numeric/symbolic kernels for the linear-algebra layer: exact
//! elimination over `Number`, cofactor expansion, fraction-free Bareiss, the
//! assumption-gated rref, and the literal-matrix extraction helpers used by
//! both [`super::linalg`] and the eigen modules.

use crate::assumptions::{is_nonzero, Assumptions};
use crate::expr::Expr;
use crate::norm::{add, canonicalize, mul, pow};
use crate::num::Number;
use num_rational::BigRational;

pub(super) fn as_numbers(entries: &[Expr]) -> Option<Vec<Number>> {
    entries
        .iter()
        .map(|e| match e {
            Expr::Num(n) => Some(n.clone()),
            _ => None,
        })
        .collect()
}

pub(super) fn as_rationals(entries: &[Expr]) -> Option<Vec<BigRational>> {
    entries
        .iter()
        .map(|e| match e {
            Expr::Num(n) => n.to_bigrational(),
            _ => None,
        })
        .collect()
}

/// A canonical square literal matrix with its dimension, under the dim cap.
pub(super) fn square_literal(c: &Expr) -> Option<(usize, &[Expr])> {
    let Expr::Matrix {
        rows,
        cols,
        entries,
    } = c
    else {
        return None;
    };
    let n = *rows as usize;
    if rows != cols || n == 0 || n > crate::resource_limits::current().max_matrix_dim {
        return None;
    }
    Some((n, entries))
}

/// Is this entry provably zero? Pivot/rank/discriminant decisions ride on
/// this: a symbolic-but-zero entry (e.g. `sqrt(8) - 2 sqrt(2)`) chosen as a
/// pivot silently corrupts the elimination, so anything that isn't a literal
/// gets the certified exact-zero service (accept-only — no sampling, budget-
/// governed). Undecidable entries stay "nonzero", the conservative direction
/// this code always used.
pub(super) fn is_zero(e: &Expr) -> bool {
    match e {
        Expr::Num(n) => n.is_zero(),
        // Atoms other than numbers are never zero (symbols are indeterminates
        // here; π/e are nonzero constants).
        Expr::Sym(_) | Expr::Const(_) => false,
        _ => crate::exact::certified_zero(e, &crate::assumptions::Assumptions::new()),
    }
}

/// Exact elimination over `Number`: det = sign · ∏ pivots.
pub(super) fn det_rational(mut m: Vec<Number>, n: usize) -> Number {
    let mut sign_neg = false;
    let mut result = Number::Int(1);
    for col in 0..n {
        let Some(pivot_row) = (col..n).find(|&r| !m[r * n + col].is_zero()) else {
            return Number::Int(0);
        };
        if pivot_row != col {
            sign_neg = !sign_neg;
            for k in 0..n {
                m.swap(col * n + k, pivot_row * n + k);
            }
        }
        let p = m[col * n + col].clone();
        result = result.mul(&p);
        for r in col + 1..n {
            if m[r * n + col].is_zero() {
                continue;
            }
            let Some(f) = m[r * n + col].checked_div(&p) else {
                return Number::Int(0); // unreachable: p nonzero
            };
            for k in col..n {
                m[r * n + k] = m[r * n + k].sub(&f.mul(&m[col * n + k]));
            }
        }
    }
    if sign_neg {
        result.neg()
    } else {
        result
    }
}

/// Cofactor expansion along the first row (general symbolic entries; the
/// n ≤ `max_symbolic_det_dim` cap bounds the n! growth).
pub(super) fn det_cofactor(entries: &[Expr], n: usize) -> Expr {
    if n == 1 {
        return entries[0].clone();
    }
    let mut terms = Vec::with_capacity(n);
    for j in 0..n {
        if is_zero(&entries[j]) {
            continue;
        }
        let minor = minor_entries(entries, n, 0, j);
        let mut factors = vec![entries[j].clone(), det_cofactor(&minor, n - 1)];
        if j % 2 == 1 {
            factors.push(Expr::int(-1));
        }
        terms.push(mul(factors));
    }
    add(terms)
}

/// The (i, j) cofactor `(−1)^(i+j)·det(minor)`.
pub(super) fn cofactor(entries: &[Expr], n: usize, i: usize, j: usize) -> Expr {
    let minor = minor_entries(entries, n, i, j);
    let d = det_cofactor(&minor, n - 1);
    if (i + j) % 2 == 1 {
        mul(vec![Expr::int(-1), d])
    } else {
        d
    }
}

fn minor_entries(entries: &[Expr], n: usize, skip_row: usize, skip_col: usize) -> Vec<Expr> {
    let mut out = Vec::with_capacity((n - 1) * (n - 1));
    for r in 0..n {
        if r == skip_row {
            continue;
        }
        for c in 0..n {
            if c == skip_col {
                continue;
            }
            out.push(entries[r * n + c].clone());
        }
    }
    out
}

/// Is `e` a polynomial expression (the Bareiss tier's domain: exact division
/// is decidable there and a canonical zero is a true zero)?
pub(super) fn is_polynomial(e: &Expr) -> bool {
    match e {
        Expr::Num(_) | Expr::Sym(_) => true,
        Expr::Add(ts) | Expr::Mul(ts) => ts.iter().all(is_polynomial),
        Expr::Pow(b, x) => {
            is_polynomial(b) && matches!(&**x, Expr::Num(Number::Int(k)) if *k >= 0)
        }
        _ => false,
    }
}

/// Fraction-free Bareiss over polynomial entries: every division is exact in
/// the polynomial ring; `reduce_rational` performs the cancellation. `None`
/// if a division fails to cancel (non-polynomial residue — conservative).
pub(super) fn det_bareiss(entries: &[Expr], n: usize) -> Option<Expr> {
    let mut m: Vec<Expr> = entries.to_vec();
    let mut prev = Expr::int(1);
    let mut sign_neg = false;
    for k in 0..n - 1 {
        if is_zero(&m[k * n + k]) {
            let swap = (k + 1..n).find(|&r| !is_zero(&m[r * n + k]))?;
            // All-zero column would have returned via the entries being zero:
            // a canonical polynomial is zero iff it is the zero polynomial.
            sign_neg = !sign_neg;
            for c in 0..n {
                m.swap(k * n + c, swap * n + c);
            }
        }
        for i in k + 1..n {
            for j in k + 1..n {
                let num = add(vec![
                    mul(vec![m[i * n + j].clone(), m[k * n + k].clone()]),
                    mul(vec![Expr::int(-1), m[i * n + k].clone(), m[k * n + j].clone()]),
                ]);
                let q = crate::ops::reduce_rational(&Expr::Div(
                    Box::new(num),
                    Box::new(prev.clone()),
                ));
                if !is_polynomial(&canonicalize(&q)) {
                    return None;
                }
                m[i * n + j] = canonicalize(&q);
            }
            m[i * n + k] = Expr::int(0);
        }
        prev = m[k * n + k].clone();
    }
    let d = m[(n - 1) * n + (n - 1)].clone();
    Some(if sign_neg {
        mul(vec![Expr::int(-1), d])
    } else {
        d
    })
}

/// Is this entry certified nonzero? The assumptions engine first (it knows
/// about symbols under hypotheses), then — for variable-free entries only —
/// the exact-constant service, which decides surd/π arithmetic like
/// `sqrt(8) - sqrt(2)` that the assumptions engine can't. Never guesses:
/// variable entries with no assumption answer stay `None`, since sampling
/// would wrongly certify a parameter expression that vanishes at some
/// parameter values.
fn entry_nonzero(e: &Expr, assumptions: &Assumptions) -> Option<bool> {
    if let Some(v) = is_nonzero(e, assumptions) {
        return Some(v);
    }
    let variable_free = crate::ops::variables(e)
        .iter()
        .all(|v| crate::sym::is_constant_symbol(v));
    if variable_free {
        // exact::is_zero never samples on variable-free input.
        return crate::exact::is_zero(e, assumptions).map(|z| !z);
    }
    None
}

/// Gauss–Jordan over `Expr` with tri-state pivot gating. Returns the reduced
/// entries and the pivot columns, or `None` when a pivot decision needs an
/// unavailable assumption.
pub(super) fn rref_core(
    entries: &[Expr],
    rows: usize,
    cols: usize,
    assumptions: &Assumptions,
) -> Option<(Vec<Expr>, Vec<usize>)> {
    let lim = crate::resource_limits::current();
    if rows > lim.max_matrix_dim || cols > lim.max_matrix_dim {
        return None;
    }
    let mut m: Vec<Expr> = entries.to_vec();
    let mut pivots: Vec<usize> = Vec::new();
    let mut row = 0;
    for col in 0..cols {
        if row == rows {
            break;
        }
        // Classify candidates in this column below `row`.
        let mut pivot_row = None;
        let mut undecidable = false;
        for r in row..rows {
            let e = &m[r * cols + col];
            if is_zero(e) {
                continue;
            }
            match entry_nonzero(e, assumptions) {
                Some(true) => {
                    pivot_row = Some(r);
                    break;
                }
                Some(false) => {} // provably zero (e.g. under assumptions)
                None => undecidable = true,
            }
        }
        let Some(p) = pivot_row else {
            if undecidable {
                return None; // §0 decision 4: never guess a pivot
            }
            continue; // genuinely free column
        };
        if p != row {
            for c in 0..cols {
                m.swap(row * cols + c, p * cols + c);
            }
        }
        let scale = pow(m[row * cols + col].clone(), Expr::int(-1));
        for c in 0..cols {
            m[row * cols + c] = mul(vec![scale.clone(), m[row * cols + c].clone()]);
        }
        for r in 0..rows {
            if r == row || is_zero(&m[r * cols + col]) {
                continue;
            }
            let f = m[r * cols + col].clone();
            for c in 0..cols {
                let sub = mul(vec![Expr::int(-1), f.clone(), m[row * cols + c].clone()]);
                m[r * cols + c] = add(vec![m[r * cols + c].clone(), sub]);
            }
        }
        pivots.push(col);
        row += 1;
    }
    Some((m, pivots))
}
