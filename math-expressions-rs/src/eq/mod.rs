//! Equality testing (PORTING_PLAN.md §10, redesign note §3.5). The staged
//! algorithm: blank guard → canonical structural compare (exact, the §3a
//! payoff) → numerical sampling at random complex points. Finite-field
//! rejection (stage 2) and discrete-infinite-set (stage 4) are deferred; the
//! remaining stages already decide the large majority of cases.

use crate::eval::{eval_complex, free_symbols, Env};
use crate::expr::{Expr, RelOp, SeqKind};
use crate::norm::{canonicalize, desugar_units};
use num_complex::Complex64;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// Options mirroring the JS `equals` parameters (PORTING_PLAN.md §10). Only the
/// tolerances and coercion flags affect this first-cut implementation;
/// `allowed_error_in_numbers` fuzzy matching is a documented follow-up.
#[derive(Debug, Clone)]
pub struct EqOptions {
    pub relative_tolerance: f64,
    pub absolute_tolerance: f64,
    pub tolerance_for_zero: f64,
    pub allow_blanks: bool,
    pub coerce_tuples_arrays: bool,
    pub coerce_vectors: bool,
    /// Number of random complex sample points for the numerical stage.
    pub num_samples: usize,
}

impl Default for EqOptions {
    fn default() -> Self {
        EqOptions {
            relative_tolerance: 1e-12,
            absolute_tolerance: 0.0,
            tolerance_for_zero: 1e-15,
            allow_blanks: false,
            coerce_tuples_arrays: true,
            coerce_vectors: true,
            num_samples: 20,
        }
    }
}

/// Are `a` and `b` mathematically equal?
pub fn equals(a: &Expr, b: &Expr, opts: &EqOptions) -> bool {
    // Stage 0: a blank (missing operand) makes equality undefined.
    if !opts.allow_blanks && (contains_blank(a) || contains_blank(b)) {
        return false;
    }

    // Scaling units (`%`, `deg`, `$`) are arithmetic for full equality: desugar
    // them (`50% → 50/100`, `180 deg → 180·pi/180`, `$n → $·n`) before
    // canonicalizing. So `50% == 1/2` and `$3+$2 == $5`, while `$5 != 5` because
    // `$` survives as a free factor. `equals_syntactic` deliberately skips this.
    let a = desugar_units(a);
    let b = desugar_units(b);

    // Stage 1: exact structural equality of canonical forms.
    let ca = coerce_seqs(canonicalize(&a), opts);
    let cb = coerce_seqs(canonicalize(&b), opts);
    if ca == cb {
        return true;
    }

    // When both sides fold to a bare exact number, stage 1 is *definitive*:
    // they are unequal, and the numerical stage must not override with f64
    // slop (this is the §3a exactness win — `10^20+1` ≠ `10^20+2`). Structure
    // that did not fully evaluate (roots, functions) still needs sampling.
    if matches!(ca, Expr::Num(_)) && matches!(cb, Expr::Num(_)) {
        return false;
    }

    // Two comparison relations denote the same equation/inequality when their
    // *standard forms* (`lhs - rhs`) are proportional: any nonzero factor for
    // `=`, a positive factor for an inequality (a negative factor would flip the
    // direction). So `5x+2y=3` ≡ `6-4y=10x` and `5q-9z<2u+9z` ≡ `27z-5q>-4u+5q-9z`,
    // while `5q<9z` ≢ `5q>9z` (factor -1). This is full mathematical equivalence
    // and is deliberately absent from `equals_syntactic`, so a teacher grading
    // *form* can still tell `5x+2y=3` from `6-4y=10x`.
    if let (Some(ra), Some(rb)) = (as_comparison(&ca), as_comparison(&cb)) {
        return relations_equal(ra, rb, opts);
    }

    // Stage 3: numerical agreement at random complex points.
    equals_numerical(&ca, &cb, opts)
}

/// Symbolic (syntactic) equality: canonical structural comparison only, no
/// numerical sampling. Mirrors the JS `equalsViaSyntax` slot — though our
/// canonical form folds constants and combines like terms, so it is strictly
/// *more* permissive than the JS's non-folding check (e.g. we call `3+2` and
/// `5` symbolically equal). An intentional divergence per the redesign note's
/// baseline decision.
pub fn equals_syntactic(a: &Expr, b: &Expr, opts: &EqOptions) -> bool {
    if !opts.allow_blanks && (contains_blank(a) || contains_blank(b)) {
        return false;
    }
    let ca = coerce_seqs(canonicalize(a), opts);
    let cb = coerce_seqs(canonicalize(b), opts);
    ca == cb
}

/// Does the tree contain a `Blank` (missing operand)? A variant check, not a
/// magic-symbol scan.
fn contains_blank(e: &Expr) -> bool {
    match e {
        Expr::Blank => true,
        Expr::Neg(x) | Expr::Not(x) | Expr::Prime(x) => contains_blank(x),
        Expr::Pow(a, b) | Expr::Div(a, b) | Expr::Index(a, b) => {
            contains_blank(a) || contains_blank(b)
        }
        Expr::Add(xs)
        | Expr::Mul(xs)
        | Expr::And(xs)
        | Expr::Or(xs)
        | Expr::Union(xs)
        | Expr::Intersect(xs)
        | Expr::Seq(_, xs)
        | Expr::OtherOp(_, xs) => xs.iter().any(contains_blank),
        Expr::Apply(h, xs) => contains_blank(h) || xs.iter().any(contains_blank),
        Expr::Interval { endpoints, .. } => {
            contains_blank(&endpoints.0) || contains_blank(&endpoints.1)
        }
        Expr::Relation { operands, .. } => operands.iter().any(contains_blank),
        Expr::Matrix { entries, .. } => entries.iter().any(contains_blank),
        _ => false,
    }
}

/// Map coerced sequence kinds to a common kind so `(1,2)`, `[1,2]`, and vector
/// forms compare equal when the corresponding flag is set. Recurses through
/// every variant — a tuple nested inside a relation, interval, or matrix must
/// coerce too.
fn coerce_seqs(e: Expr, opts: &EqOptions) -> Expr {
    fn recur(e: Expr, opts: &EqOptions) -> Expr {
        let map_kind = |k: SeqKind| match k {
            SeqKind::Array if opts.coerce_tuples_arrays => SeqKind::Tuple,
            SeqKind::Vector | SeqKind::AltVector if opts.coerce_vectors => SeqKind::Tuple,
            other => other,
        };
        let each = |xs: Vec<Expr>, opts: &EqOptions| -> Vec<Expr> {
            xs.into_iter().map(|x| recur(x, opts)).collect()
        };
        match e {
            Expr::Seq(k, xs) => Expr::Seq(map_kind(k), each(xs, opts)),
            Expr::Add(xs) => Expr::Add(each(xs, opts)),
            Expr::Mul(xs) => Expr::Mul(each(xs, opts)),
            Expr::And(xs) => Expr::And(each(xs, opts)),
            Expr::Or(xs) => Expr::Or(each(xs, opts)),
            Expr::Union(xs) => Expr::Union(each(xs, opts)),
            Expr::Intersect(xs) => Expr::Intersect(each(xs, opts)),
            Expr::Pow(a, b) => Expr::Pow(Box::new(recur(*a, opts)), Box::new(recur(*b, opts))),
            Expr::Div(a, b) => Expr::Div(Box::new(recur(*a, opts)), Box::new(recur(*b, opts))),
            Expr::Index(a, b) => Expr::Index(Box::new(recur(*a, opts)), Box::new(recur(*b, opts))),
            Expr::Neg(x) => Expr::Neg(Box::new(recur(*x, opts))),
            Expr::Not(x) => Expr::Not(Box::new(recur(*x, opts))),
            Expr::Prime(x) => Expr::Prime(Box::new(recur(*x, opts))),
            Expr::Apply(h, xs) => Expr::Apply(Box::new(recur(*h, opts)), each(xs, opts)),
            Expr::Interval { endpoints, closed } => {
                let (a, b) = *endpoints;
                Expr::Interval {
                    endpoints: Box::new((recur(a, opts), recur(b, opts))),
                    closed,
                }
            }
            Expr::Relation { operands, ops } => Expr::Relation {
                operands: each(operands, opts),
                ops,
            },
            Expr::Matrix {
                rows,
                cols,
                entries,
            } => Expr::Matrix {
                rows,
                cols,
                entries: each(entries, opts),
            },
            Expr::OtherOp(name, xs) => Expr::OtherOp(name, each(xs, opts)),
            leaf @ (Expr::Num(_) | Expr::Sym(_) | Expr::Const(_) | Expr::Blank | Expr::Ldots) => {
                leaf
            }
        }
    }
    recur(e, opts)
}

/// Sample both expressions at random complex points; accept if they agree
/// within tolerance everywhere they can both be evaluated. A fixed seed keeps
/// results reproducible (and avoids OS entropy, which is unavailable on wasm).
fn equals_numerical(a: &Expr, b: &Expr, opts: &EqOptions) -> bool {
    let mut vars = std::collections::BTreeSet::new();
    free_symbols(a, &mut vars);
    free_symbols(b, &mut vars);
    let vars: Vec<String> = vars.into_iter().collect();

    let mut rng = SmallRng::seed_from_u64(0x5EED_1234_ABCD_0001);
    let mut agreements = 0;
    let mut attempts = 0;
    // Keep sampling until we have enough successful (pole-free) comparisons or
    // run out of attempts; a single disagreement is a definitive reject.
    while agreements < opts.num_samples && attempts < opts.num_samples * 4 {
        attempts += 1;
        let env: Env = vars
            .iter()
            .map(|v| {
                // Sample in a modest box, offset from the origin to dodge the
                // most common poles/branch points.
                let re = rng.random_range(-2.0..2.0) + 0.3;
                let im = rng.random_range(-2.0..2.0) + 0.2;
                (v.clone(), Complex64::new(re, im))
            })
            .collect();

        let (Some(va), Some(vb)) = (eval_complex(a, &env), eval_complex(b, &env)) else {
            // Not numerically evaluable here: can't confirm equality.
            return false;
        };
        if !va.re.is_finite() || !va.im.is_finite() || !vb.re.is_finite() || !vb.im.is_finite() {
            continue; // hit a pole; resample
        }
        if close(va, vb, opts) {
            agreements += 1;
        } else {
            return false;
        }
    }
    agreements > 0
}

/// A two-operand comparison relation, reduced to `(lhs, rhs, op)` with `op` one
/// of the three arithmetic comparisons `=`, `<`, `≤` (JS `equals` handles
/// exactly `["=", ">", "<", "ge", "le"]`). `>`/`≥` are folded to `<`/`≤` by
/// swapping operands — canonicalization already does this, so it is only a
/// safety net here. `≠`, set relations, and chained relations do not qualify.
struct Comparison {
    lhs: Expr,
    rhs: Expr,
    op: RelOp,
}

fn as_comparison(e: &Expr) -> Option<Comparison> {
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
fn relations_equal(a: Comparison, b: Comparison, opts: &EqOptions) -> bool {
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
fn proportional(a: &Expr, b: &Expr, require_positive: bool, opts: &EqOptions) -> bool {
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
            return false;
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
