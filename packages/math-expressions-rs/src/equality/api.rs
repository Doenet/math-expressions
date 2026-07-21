//! The staged equality algorithm's public entry points and the sequence-kind
//! coercion they share.

use super::fuzzy::fuzzy_tree_eq;
use super::numeric::equals_numerical;
use super::relations::{as_comparison, relations_equal};
use super::{discrete_infinite, finite_field, plus_minus, EqOptions};
use crate::expr::{Expr, SeqKind};
use crate::norm::{canonicalize, desugar_units, normalize_syntactic, simplify_canonical};

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
    // With a number-error allowance, the structural check compares number
    // leaves within tolerance instead of exactly (port of the JS
    // `equalsViaSyntax` + `trees/basic.js equal` fuzzy path). Exponents stay
    // exact unless `include_error_in_number_exponents`.
    if opts.allowed_error_in_numbers > 0.0 && fuzzy_tree_eq(&ca, &cb, opts) {
        return true;
    }

    // When both sides fold to a bare exact number, stage 1 is *definitive*:
    // they are unequal, and the numerical stage must not override with f64
    // slop (this is the §3a exactness win — `10^20+1` ≠ `10^20+2`). Structure
    // that did not fully evaluate (roots, functions) still needs sampling.
    if matches!(ca, Expr::Num(_)) && matches!(cb, Expr::Num(_)) {
        return false;
    }

    // Plus-minus (±): a `pm` node denotes a two-element value set that the
    // finite-field and single-value sampling stages treat as an opaque atom
    // (and would reject). Dispatch to the pm-aware set comparison before those
    // stages. Port of JS `equality/numerical.js` pm branch + `pm-numerical.js`.
    if crate::pm::contains_pm(&ca) || crate::pm::contains_pm(&cb) {
        return plus_minus::pm_equals(&ca, &cb, opts);
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

    // Discrete infinite sets (periodic solution sets like `x = π/4 + nπ`)
    // are compared by residue-class covering — or against a listed sequence
    // `a, a+p, a+2p, …`. This is type-directed dispatch (like relations
    // above) and must run BEFORE the rejection stages: the field/sampling
    // stages treat the set's OtherOp tree as an opaque atom and would
    // definitively reject a pair stage 4 accepts. (JS runs its version last,
    // but its earlier stages never produce a definitive false for these.)
    if discrete_infinite::is_discrete_infinite_set(&ca)
        || discrete_infinite::is_discrete_infinite_set(&cb)
    {
        return discrete_infinite::equals_discrete_infinite(&ca, &cb, opts);
    }

    // Stage 2: finite-field rejection. Exact evaluation in ℤ/pℤ catches
    // additive/structural differences that floating-point sampling can mask
    // (`e^(10x)` vs `e^(10x)+C`), and it is the filter that makes lenient
    // complex sampling safe. It never confirms equality — only rejects.
    // (Skipped under a number-error allowance: exact field arithmetic would
    // reject the pairs the allowance is meant to accept — mirrors JS.)
    if opts.allowed_error_in_numbers == 0.0 && finite_field::definitely_unequal(&ca, &cb) {
        return false;
    }

    // Stage 3: numerical agreement at random complex points.
    equals_numerical(&ca, &cb, opts)
}

/// Numerical equality by sampling *real* points only — the port of JS
/// `equalsViaReal`. Both expressions must be analytic (no `abs`/`sign`/`arg`,
/// no logical/set operators), matching the JS gate; a non-analytic operand
/// makes this return `false`. Real-only sampling is the right tool when the
/// functions agree on the reals but differ off the real axis (branch cuts):
/// `sqrt(x²)` and `abs(x)`… — though `abs` itself is non-analytic, so callers
/// use this for real-domain agreement of analytic forms.
pub fn equals_via_real(a: &Expr, b: &Expr, opts: &EqOptions) -> bool {
    use crate::ops::{is_analytic, AnalyticOpts};
    if !opts.allow_blanks && (contains_blank(a) || contains_blank(b)) {
        return false;
    }
    let ao = AnalyticOpts::default();
    if !is_analytic(a, &ao) || !is_analytic(b, &ao) {
        return false;
    }
    let a = desugar_units(a);
    let b = desugar_units(b);
    let ca = canonicalize(&coerce_seqs(a, opts));
    let cb = canonicalize(&coerce_seqs(b, opts));
    if ca == cb {
        return true;
    }
    let mut o = opts.clone();
    o.real_only = true;
    equals_numerical(&ca, &cb, &o)
}

/// Whole-tree structural equality — the port of JS `equalsViaSyntax`, and the
/// JS-parity convenience name for the
/// [`SameStructure`](crate::StructuralComparison::SameStructure) structural comparison
/// (`equals_syntactic(a, b, o)` == `structural_equality(a, b, &SameStructure, o)`).
/// This is a *form* check: it applies only the four light normalization passes
/// (function-name spelling, exponents/primes outside applications, negative
/// numbers, geometry arg order) and then compares trees *order-sensitively*. It
/// does NOT reorder, fold, combine like terms, or eliminate `Div`, so `ln(x)`
/// equals `log(x)` but `(x+y)+z` does NOT equal `z+x+y` and `3+2` does NOT equal
/// `5`. "Is the answer in the requested form?" — distinct from the value-level
/// [`equals`]. See [`crate::equality_structural`] for the full value-vs-structural map.
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
    e.any_subexpr(&|c| matches!(c, Expr::Blank))
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
            leaf @ (Expr::Num(_)
            | Expr::Sym(_)
            | Expr::Const(_)
            | Expr::RootOf { .. }
            | Expr::Blank
            | Expr::Ldots) => leaf,
        }
    }
    recur(e, opts)
}
