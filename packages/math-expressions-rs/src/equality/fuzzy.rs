//! Number-error-tolerant comparison: fuzzy structural equality (`trees/basic.js
//! equal`) and the first-order sensitivity tolerance (`tolerance_function`) that
//! the sampling stages add to their per-point comparisons.

use super::EqOptions;
use crate::eval::{eval_complex, Env};
use crate::expr::Expr;
use num_complex::Complex64;

/// Structural equality with number leaves compared within the allowed error
/// (canonical trees; port of `trees/basic.js equal`). Exponents of `Pow` are
/// compared exactly unless `include_error_in_number_exponents`.
pub(super) fn fuzzy_tree_eq(a: &Expr, b: &Expr, opts: &EqOptions) -> bool {
    match (a, b) {
        (Expr::Num(x), Expr::Num(y)) => fuzzy_number_eq(x, y, opts),
        (Expr::Pow(b1, e1), Expr::Pow(b2, e2)) => {
            let base_ok = fuzzy_tree_eq(b1, b2, opts);
            let exp_ok = if opts.include_error_in_number_exponents {
                fuzzy_tree_eq(e1, e2, opts)
            } else {
                e1 == e2
            };
            base_ok && exp_ok
        }
        _ => {
            if std::mem::discriminant(a) != std::mem::discriminant(b) {
                return false;
            }
            // Same variant: compare non-Expr structure via a cheap projection,
            // then children pairwise. Kind/name/op mismatches show up either
            // in the discriminant or in the skeleton compare below.
            if !same_skeleton(a, b) {
                return false;
            }
            let (ca, cb) = (a.children(), b.children());
            ca.len() == cb.len()
                && ca
                    .iter()
                    .zip(cb.iter())
                    .all(|(x, y)| fuzzy_tree_eq(x, y, opts))
        }
    }
}

/// Non-child structure equal (symbol names, seq kinds, relation ops, matrix
/// shape, interval closure)?
fn same_skeleton(a: &Expr, b: &Expr) -> bool {
    match (a, b) {
        (Expr::Sym(x), Expr::Sym(y)) => x == y,
        (Expr::Const(x), Expr::Const(y)) => x == y,
        (Expr::Seq(k1, _), Expr::Seq(k2, _)) => k1 == k2,
        (Expr::OtherOp(n1, _), Expr::OtherOp(n2, _)) => n1 == n2,
        (
            Expr::Relation { ops: o1, .. },
            Expr::Relation { ops: o2, .. },
        ) => o1 == o2,
        (
            Expr::Matrix { rows: r1, cols: c1, .. },
            Expr::Matrix { rows: r2, cols: c2, .. },
        ) => r1 == r2 && c1 == c2,
        (Expr::Interval { closed: cl1, .. }, Expr::Interval { closed: cl2, .. }) => cl1 == cl2,
        _ => true,
    }
}

/// JS `trees/basic.js` number comparison: relative mode uses
/// `max(1e-14, allowed)·min(|l|,|r|)`; absolute mode `max(1e-14·min, allowed)`.
fn fuzzy_number_eq(x: &crate::num::Number, y: &crate::num::Number, opts: &EqOptions) -> bool {
    let (l, r) = (x.to_f64(), y.to_f64());
    if !l.is_finite() || !r.is_finite() {
        return x == y;
    }
    let min_abs = l.abs().min(r.abs());
    let tol = if opts.allowed_error_is_absolute {
        (1e-14 * min_abs).max(opts.allowed_error_in_numbers)
    } else {
        1e-14f64.max(opts.allowed_error_in_numbers) * min_abs
    };
    (l - r).abs() <= tol
}

/// The per-sample-point extra tolerance from the allowed number error: a
/// first-order sensitivity bound. Numbers in `expr` are replaced by
/// parameters; the tolerance expression is
/// `allowed_error · Σᵢ ∂f/∂pᵢ · (valᵢ if relative)` and is evaluated at each
/// sample point (port of the JS `tolerance_function`).
pub(super) struct FuzzyTol {
    tolerance_expr: Expr,
    /// Parameter name → its numeric value, added to every evaluation env.
    params: Vec<(String, f64)>,
}

pub(super) fn build_fuzzy_tol(expr: &Expr, vars: &[String], opts: &EqOptions) -> Option<FuzzyTol> {
    let mut params: Vec<(String, f64)> = Vec::new();
    let with_params = replace_numbers(expr, vars, opts.include_error_in_number_exponents, &mut params);
    if params.is_empty() {
        return None;
    }
    let mut terms = Vec::new();
    for (name, val) in &params {
        let d = crate::diff::derivative(&with_params, name);
        let term = if opts.allowed_error_is_absolute {
            d
        } else {
            Expr::Mul(vec![d, Expr::Num(crate::num::Number::from_f64(*val))])
        };
        terms.push(term);
    }
    let tolerance_expr = Expr::Mul(vec![
        Expr::Num(crate::num::Number::from_f64(opts.allowed_error_in_numbers)),
        Expr::Add(terms),
    ]);
    Some(FuzzyTol {
        tolerance_expr,
        params,
    })
}

/// Replace each nonzero number literal (and the constants pi/e) with a fresh
/// parameter symbol, recording its value. `Pow` exponents are left untouched
/// unless `include_exponents`.
fn replace_numbers(
    e: &Expr,
    vars: &[String],
    include_exponents: bool,
    params: &mut Vec<(String, f64)>,
) -> Expr {
    let fresh = |val: f64, params: &mut Vec<(String, f64)>| -> Expr {
        let mut n = params.len() + 1;
        let mut name = format!("par{n}");
        while vars.contains(&name) {
            n += 1;
            name = format!("par{n}");
        }
        params.push((name.clone(), val));
        Expr::sym(&name)
    };
    match e {
        Expr::Num(n) => {
            let v = n.to_f64();
            if v == 0.0 || !v.is_finite() {
                e.clone()
            } else {
                fresh(v, params)
            }
        }
        Expr::Sym(s) if s.name() == "pi" => fresh(std::f64::consts::PI, params),
        Expr::Sym(s) if s.name() == "e" => fresh(std::f64::consts::E, params),
        Expr::Pow(b, x) if !include_exponents => Expr::Pow(
            Box::new(replace_numbers(b, vars, include_exponents, params)),
            x.clone(),
        ),
        _ => crate::norm::syntactic::map_children(e, |c| {
            replace_numbers(c, vars, include_exponents, params)
        }),
    }
}

impl FuzzyTol {
    /// |tolerance| at a sample point; `None` when it cannot be evaluated
    /// (treated as a disagreeing point, like the JS).
    pub(super) fn at(&self, bindings: &Env) -> Option<f64> {
        let mut env = bindings.clone();
        for (name, val) in &self.params {
            env.insert(name.clone(), Complex64::new(*val, 0.0));
        }
        let v = eval_complex(&self.tolerance_expr, &env)?;
        let t = v.norm();
        t.is_finite().then_some(t)
    }
}
