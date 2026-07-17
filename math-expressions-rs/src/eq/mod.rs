//! Equality testing (PORTING_PLAN.md §10, redesign note §3.5). The staged
//! algorithm: blank guard → canonical structural compare (exact, the §3a
//! payoff) → numerical sampling at random complex points. Finite-field
//! rejection (stage 2) and discrete-infinite-set (stage 4) are deferred; the
//! remaining stages already decide the large majority of cases.

mod finite_field;

use crate::eval::{eval_complex, free_symbols, Env};
use crate::expr::{Expr, RelOp, SeqKind};
use crate::norm::{canonicalize, desugar_units, normalize_syntactic, simplify_canonical};
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

    // Sequence-kind coercion runs BEFORE simplification so the tuple/vector
    // rewrite clusters see unified kinds: `[1,2]+(3,4)` must combine
    // componentwise when `coerce_tuples_arrays` is set, which requires the
    // Array to already be a Tuple when the grouping rule fires. (simplify
    // never introduces new sequence kinds, so no post-coercion is needed.)
    let ca = canonicalize(&coerce_seqs(a, opts));
    let cb = canonicalize(&coerce_seqs(b, opts));

    // Stage 1a: fast path — most equal pairs already agree canonically, without
    // paying for the rewrite clusters.
    if ca == cb {
        return true;
    }

    // Stage 1b: exact structural equality of the *simplified* canonical forms.
    // `simplify_canonical` adds the heuristic rewrite clusters (§7e: radical,
    // tuple/vector, ∞/NaN, and trig identities), run to a fixpoint — so
    // real-domain equalities like `sin²x+cos²x == 1` and `cbrt(-x²) == -cbrt(x²)`
    // are caught structurally here rather than left to numerical sampling (which
    // rejects the branch-cut cases). Matches the JS chain, whose stage 1 is
    // `evaluate_numbers` + name normalization + `simplify`.
    let ca = simplify_canonical(ca);
    let cb = simplify_canonical(cb);
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

    // Stage 2: finite-field rejection. Exact evaluation in ℤ/pℤ catches
    // additive/structural differences that floating-point sampling can mask
    // (`e^(10x)` vs `e^(10x)+C`), and it is the filter that makes lenient
    // complex sampling safe. It never confirms equality — only rejects.
    if finite_field::definitely_unequal(&ca, &cb) {
        return false;
    }

    // Stage 3: numerical agreement at random complex points.
    equals_numerical(&ca, &cb, opts)
}

/// Symbolic (syntactic) equality — the port of JS `equalsViaSyntax`. This is a
/// *form* check: it applies only the four light normalization passes
/// (function-name spelling, exponents/primes outside applications, negative
/// numbers, geometry arg order) and then compares trees *order-sensitively*. It
/// does NOT flatten, reorder, fold, combine like terms, or eliminate `Div`, so
/// `ln(x)` equals `log(x)` but `(x+y)+z` does NOT equal `z+x+y` and `3+2` does
/// NOT equal `5`. This is what a teacher grading "is the answer in the requested
/// form?" needs — distinct from the aggressive [`equals`].
pub fn equals_syntactic(a: &Expr, b: &Expr, opts: &EqOptions) -> bool {
    if !opts.allow_blanks && (contains_blank(a) || contains_blank(b)) {
        return false;
    }
    let na = coerce_seqs(normalize_syntactic(a), opts);
    let nb = coerce_seqs(normalize_syntactic(b), opts);
    na == nb
}

/// Does the tree contain a `Blank` (missing operand)? A variant check, not a
/// magic-symbol scan. Public: callers (and the corpus tests) need to know
/// whether `equals`'s stage-0 blank guard will reject a tree.
pub fn contains_blank(e: &Expr) -> bool {
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

// JS numerical-equality constants (lib/expression/equality/numerical.js).
/// Clustered agreeing points needed to accept a region.
const MINIMUM_MATCHES: usize = 10;
/// Disagreeing base points tolerated before rejecting — branch-cut identities
/// disagree at many points, so this must be generous.
const NUMBER_TRIES: usize = 100;
/// Base-point sampling radii, tried in order. Large scales first so a non-identity
/// reveals its global disagreement before small scales probe near the origin;
/// neighborhoods use `scale / 100`.
const BINDING_SCALES: [f64; 6] = [10.0, 1.0, 100.0, 0.1, 1000.0, 0.01];
/// `Number.MAX_VALUE * 1e-20` — larger magnitudes are out of bounds.
const MAX_VALUE: f64 = f64::MAX * 1e-20;

/// Numerical equality by the JS `find_equality_region` strategy: prove equality
/// by finding **one** small neighborhood where both functions agree at several
/// clustered points (agreement on an open set ⟹ identical, by analyticity),
/// while *tolerating* base points that disagree — which happens for identities
/// that hold only off a branch cut, e.g. `log(a^2 b) = 2 log a + log b`. This
/// leniency is safe only because the finite-field filter (stage 2) has already
/// rejected the near-misses it would otherwise accept (`e^(10x)` vs `e^(10x)+C`).
fn equals_numerical(a: &Expr, b: &Expr, opts: &EqOptions) -> bool {
    let mut vars = std::collections::BTreeSet::new();
    free_symbols(a, &mut vars);
    free_symbols(b, &mut vars);
    let vars: Vec<String> = vars.into_iter().collect();

    // Constant expressions (no free symbols) are a single value each — compare
    // directly, including a genuine zero (`sin(pi) = 0`), which the region
    // search below deliberately excludes as underflow.
    if vars.is_empty() {
        let env = Env::new();
        return match (eval_complex(a, &env), eval_complex(b, &env)) {
            (Some(va), Some(vb))
                if va.re.is_finite()
                    && va.im.is_finite()
                    && vb.re.is_finite()
                    && vb.im.is_finite() =>
            {
                close_numeric(va, vb, opts)
            }
            _ => false,
        };
    }

    let mut rng = SmallRng::seed_from_u64(0x5EED_1234_ABCD_0001);
    let mut num_unequal = 0;
    for scale in BINDING_SCALES {
        for _ in 0..NUMBER_TRIES {
            match find_region(a, b, &vars, scale, &mut rng, opts) {
                Region::Equal => return true,
                Region::Unequal => {
                    num_unequal += 1;
                    if num_unequal > NUMBER_TRIES {
                        return false;
                    }
                }
                Region::Skip => {}
            }
        }
    }
    false
}

enum Region {
    Equal,
    Unequal,
    Skip,
}

/// Sample a base point at radius `scale`; if both sides agree there, confirm
/// across a tight neighborhood (`scale / 100`). `Equal` iff ≥ `MINIMUM_MATCHES`
/// neighborhood points are usable and agree; `Unequal` if the base or any
/// neighborhood point disagrees; `Skip` if too few points are usable.
fn find_region(
    a: &Expr,
    b: &Expr,
    vars: &[String],
    scale: f64,
    rng: &mut SmallRng,
    opts: &EqOptions,
) -> Region {
    let base = sample_point(vars, scale, None, rng);
    let (Some(va), Some(vb)) = (eval_complex(a, &base), eval_complex(b, &base)) else {
        return Region::Skip;
    };
    if !usable(va, vb) {
        return Region::Skip;
    }
    if !close_numeric(va, vb, opts) {
        return Region::Unequal;
    }

    let mut finite_tries = 0;
    for _ in 0..100 {
        let near = sample_point(vars, scale / 100.0, Some(&base), rng);
        let (Some(va2), Some(vb2)) = (eval_complex(a, &near), eval_complex(b, &near)) else {
            continue;
        };
        if !usable(va2, vb2) {
            continue;
        }
        finite_tries += 1;
        if !close_numeric(va2, vb2, opts) {
            return Region::Unequal;
        }
        if finite_tries >= MINIMUM_MATCHES {
            return Region::Equal;
        }
    }
    Region::Skip
}

/// A sample point is usable if both values are finite, in bounds, and nonzero.
/// An exact `0.0` from a *variable* expression is underflow (canonicalization
/// folds genuine zero functions before this stage), and letting it count —
/// whether as a both-zero "agreement" or a one-sided `tolerance_for_zero`
/// match — accepts distinct functions that underflow across a region
/// (`x^sin(x)` vs `x^cos(x)`, or vs a literal `0`). Note this makes an
/// unsimplified identically-zero expression (e.g. `sin²x+cos²x−1`) unprovable
/// against `0` at this stage; JS decides that pair in its *simplify* stage
/// (Pythagorean rewrite, §7e — not yet ported), not numerically.
fn usable(va: Complex64, vb: Complex64) -> bool {
    va.re.is_finite()
        && va.im.is_finite()
        && vb.re.is_finite()
        && vb.im.is_finite()
        && va.norm() < MAX_VALUE
        && vb.norm() < MAX_VALUE
        && va.norm() > 0.0
        && vb.norm() > 0.0
}

/// Sample each variable uniformly in a `scale`-radius complex box, optionally
/// centered on a prior point (for neighborhood probing). Mirrors JS
/// `randomComplexBindings`.
fn sample_point(vars: &[String], scale: f64, center: Option<&Env>, rng: &mut SmallRng) -> Env {
    vars.iter()
        .map(|v| {
            let c = center
                .and_then(|c| c.get(v).copied())
                .unwrap_or(Complex64::new(0.0, 0.0));
            let re = c.re + rng.random_range(-scale..scale);
            let im = c.im + rng.random_range(-scale..scale);
            (v.clone(), Complex64::new(re, im))
        })
        .collect()
}

/// Tolerance test matching JS `find_equality_region`: scale by the smaller
/// magnitude, and treat a genuine zero specially.
fn close_numeric(va: Complex64, vb: Complex64, opts: &EqOptions) -> bool {
    let min_mag = va.norm().min(vb.norm());
    let max_mag = va.norm().max(vb.norm());
    if max_mag == 0.0 {
        return true;
    }
    let mut tol = (min_mag * opts.relative_tolerance).min(0.1 * min_mag);
    if tol == 0.0 && (va.norm() == 0.0 || vb.norm() == 0.0) {
        tol += opts.tolerance_for_zero;
    } else {
        tol += opts.absolute_tolerance;
    }
    (va - vb).norm() < tol
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
