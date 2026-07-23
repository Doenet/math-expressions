//! Comparison relations (`=`, `<`, `≤`) compared up to proportionality of their
//! standard forms, and the proportionality test the `±` path also reuses.

use super::EqOptions;
use crate::eval::{eval_complex, free_symbols, Env};
use crate::expr::{Expr, RelOp};
use crate::norm::canonicalize;
use num_complex::Complex64;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// A two-operand comparison relation, reduced to `(lhs, rhs, op)` with `op` one
/// of the three arithmetic comparisons `=`, `<`, `≤` (JS `equals` handles
/// exactly `["=", ">", "<", "ge", "le"]`). `>`/`≥` are folded to `<`/`≤` by
/// swapping operands — canonicalization already does this, so it is only a
/// safety net here. `≠`, set relations, and chained relations do not qualify.
pub(super) struct Comparison {
    pub(super) lhs: Expr,
    pub(super) rhs: Expr,
    pub(super) op: RelOp,
}

pub(super) fn as_comparison(e: &Expr) -> Option<Comparison> {
    let Expr::Relation { operands, ops } = e else {
        return None;
    };
    let ([l, r], [op]) = (operands.as_slice(), ops.as_slice()) else {
        return None;
    };
    let (lhs, rhs, op) = match op {
        RelOp::Eq => (l.clone(), r.clone(), RelOp::Eq),
        RelOp::Lt => (l.clone(), r.clone(), RelOp::Lt),
        RelOp::Le => (l.clone(), r.clone(), RelOp::Le),
        RelOp::Gt => (r.clone(), l.clone(), RelOp::Lt),
        RelOp::Ge => (r.clone(), l.clone(), RelOp::Le),
        _ => return None,
    };
    Some(Comparison { lhs, rhs, op })
}

/// Two comparisons are equal iff they share an operator (after `>`/`≥` folding)
/// and their standard forms `lhs - rhs` are numerically proportional. `=` allows
/// any nonzero (even complex) constant of proportionality; `<`/`≤` require a
/// positive real one, since a negative factor reverses the inequality.
pub(super) fn relations_equal(a: Comparison, b: Comparison, opts: &EqOptions) -> bool {
    if a.op != b.op {
        return false;
    }
    let std_form =
        |c: Comparison| canonicalize(&Expr::Add(vec![c.lhs, Expr::Neg(Box::new(c.rhs))]));
    let require_positive = a.op != RelOp::Eq;
    let da = std_form(a);
    let db = std_form(b);
    proportional(&da, &db, require_positive, opts)
}

/// Are `a` and `b` proportional — `a ≈ k·b` for one constant `k` across all
/// sample points? Mirrors JS `component_equals` with `allow_proportional`: the
/// factor is fixed at the first jointly-nonzero point (and rejected there if
/// `require_positive` but `k` is not a positive real), then verified at every
/// other point.
pub(super) fn proportional(a: &Expr, b: &Expr, require_positive: bool, opts: &EqOptions) -> bool {
    let mut vars = std::collections::BTreeSet::new();
    free_symbols(a, &mut vars);
    free_symbols(b, &mut vars);
    let vars: Vec<String> = vars.into_iter().collect();

    // Distinct seed from `equals_numerical` so the two stages don't share a
    // sample sequence (harmless, but keeps their behaviours independent).
    let mut rng = SmallRng::seed_from_u64(0x5EED_1234_ABCD_0002);
    let mut factor: Option<Complex64> = None;
    let mut agreements = 0;
    let mut attempts = 0;
    while agreements < opts.num_samples && attempts < opts.num_samples * 4 {
        attempts += 1;
        let env: Env = vars
            .iter()
            .map(|v| {
                let re = rng.random_range(-2.0..2.0) + 0.3;
                let im = rng.random_range(-2.0..2.0) + 0.2;
                (v.clone(), Complex64::new(re, im))
            })
            .collect();

        let (Some(va), Some(vb)) = (eval_complex(a, &env), eval_complex(b, &env)) else {
            // Unevaluable here (domain error / opaque miss): resample rather than
            // declare non-equal, matching JS `component_equals`, whose
            // `try { find_equality_region } catch { continue }` (numerical.js)
            // swallows a mathjs evaluation throw as a skipped point, and the
            // sibling pole branch just below. `attempts` is already bumped and
            // bounded, so exhausting only-unevaluable points still terminates in
            // `agreements > 0 == false` (the correct "cannot prove" outcome).
            continue;
        };
        if !va.re.is_finite() || !va.im.is_finite() || !vb.re.is_finite() || !vb.im.is_finite() {
            continue; // pole; resample
        }

        match factor {
            None => {
                let za = va.norm() <= opts.tolerance_for_zero;
                let zb = vb.norm() <= opts.tolerance_for_zero;
                if za && zb {
                    // Both vanish here: consistent, but reveals nothing about
                    // the factor. Count it and keep looking for a live point.
                    agreements += 1;
                    continue;
                }
                if za != zb {
                    return false; // one side zero, the other not
                }
                let k = va / vb;
                if require_positive && !(k.im.abs() <= 1e-9 && k.re > 0.0) {
                    return false;
                }
                factor = Some(k);
                agreements += 1;
            }
            Some(k) => {
                if close(va, k * vb, opts) {
                    agreements += 1;
                } else {
                    return false;
                }
            }
        }
    }
    // Enough consistent points and no contradiction. (If every point was jointly
    // zero, both standard forms are the zero function — still equal.)
    agreements > 0
}

fn close(a: Complex64, b: Complex64, opts: &EqOptions) -> bool {
    let diff = (a - b).norm();
    let scale = a.norm().max(b.norm());
    if scale <= opts.tolerance_for_zero {
        return diff <= opts.tolerance_for_zero;
    }
    diff <= opts.absolute_tolerance + opts.relative_tolerance * scale
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(s: &str) -> Expr {
        canonicalize(&crate::TextToAst::new(Default::default()).convert(s).unwrap())
    }

    /// `proportional` fixes the constant factor at the first live point and
    /// verifies it elsewhere. `2(x+y)` is `+2·(x+y)`; a negative factor is
    /// accepted for `=`-style comparison but rejected once positivity is required
    /// (an inequality would flip).
    #[test]
    fn proportional_detects_scalar_multiple() {
        let opts = EqOptions::default();
        assert!(proportional(&e("2x + 2y"), &e("x + y"), false, &opts));
        assert!(proportional(&e("2x + 2y"), &e("x + y"), true, &opts));
        assert!(proportional(&e("-2x - 2y"), &e("x + y"), false, &opts));
        assert!(!proportional(&e("-2x - 2y"), &e("x + y"), true, &opts));
    }

    #[test]
    fn proportional_rejects_unrelated() {
        let opts = EqOptions::default();
        assert!(!proportional(&e("x + y"), &e("x - y"), false, &opts));
    }

    /// An unevaluable point must *resample*, not short-circuit to non-equal
    /// (JS `component_equals` swallows a mathjs throw as a skip). `floor` is
    /// real-domain-only, so `eval_complex` returns `None` at every complex
    /// sample here — the fixed `continue` path must exhaust its attempts and
    /// terminate cleanly (return `false`) rather than abort or hang.
    #[test]
    fn proportional_unevaluable_samples_terminate_cleanly() {
        let opts = EqOptions::default();
        assert!(!proportional(&e("floor(x)"), &e("floor(x)"), false, &opts));
    }
}
