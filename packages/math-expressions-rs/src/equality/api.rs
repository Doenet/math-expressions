//! The staged equality algorithm's public entry points and the sequence-kind
//! coercion they share.

use super::fuzzy::fuzzy_tree_eq;
use super::numeric::{close_numeric_fuzzy, equals_numerical};
use super::relations::{as_comparison, relations_equal};
use super::{discrete_infinite, finite_field, plus_minus, EqOptions};
use crate::expr::{Expr, SeqKind};
use crate::normalize::{canonicalize, desugar_units, normalize_syntactic, simplify_canonical};
use num_complex::Complex64;

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

    // Union equality is a set match on the *raw* members: each candidate pair is
    // coerced in isolation by the recursive `equals`, which is what reproduces
    // the legacy coercion graph's non-transitivity. A single per-side interval
    // rewrite would force a tuple to an interval whenever *any* member on the
    // other side is one — wrongly, when that tuple should have paired with a
    // vector. Accept-only: a missing matching falls through to the canonical
    // path below (which already handles the deduped/sorted cases).
    if let (Expr::Union(xa), Expr::Union(xb)) = (&a, &b) {
        if xa.len() == xb.len() && union_set_equal(xa, xb, opts) {
            return true;
        }
    }

    let (a, b) = coerce_intervals(a, b, opts);

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
    if opts.allowed_error_in_numbers > 0.0 {
        if fuzzy_tree_eq(&ca, &cb, opts) {
            return true;
        }
        // Retry once on the *syntactically* normalized forms. A tolerance can
        // only forgive a difference in a number leaf; it cannot forgive the two
        // sides having chosen different spellings for the same operation. That
        // matters here because the spelling is chosen BY the numbers the
        // tolerance is meant to blur: `sqrt(q)` and `q^(1/2)` are distinct
        // canonical trees (deliberately — `ops::transforms`), and they meet at
        // stage 3 only because sampling agrees. Perturb the exponent and it is
        // no longer exactly ½, so the response is pinned to `Pow` while the
        // key stays `Apply` and the walk dies on a variant tag with every
        // number inside tolerance.
        //
        // `normalize_syntactic`'s first pass rewrites roots to explicit powers,
        // which is exactly the reconciliation needed, and is what the JS
        // `equals` chain does before *its* `equalsViaSyntax` stage. Re-
        // canonicalized because that pass emits `Div(1, n)` exponents for
        // `cbrt`/`nthroot` that must fold to a `Num` before a number-leaf
        // comparison can see them.
        //
        // Gated on a tolerance being set: without one, stage 3 already decides
        // these pairs correctly, and this would be pure cost.
        let na = canonicalize(&normalize_syntactic(&ca));
        let nb = canonicalize(&normalize_syntactic(&cb));
        if (na != ca || nb != cb) && fuzzy_tree_eq(&na, &nb, opts) {
            return true;
        }
    }

    // When both sides fold to a bare number, stage 1 is *definitive*. Structure
    // that did not fully evaluate (roots, functions) still needs sampling.
    //
    // Exact against exact is decided exactly — the §3a exactness win, and the
    // reason `10^20+1` ≠ `10^20+2` and `0.3` ≠ `0.30000000000000004` when both
    // were *written* that way (decimal literals parse to rationals).
    //
    // A `Float` operand is different in kind: it is the mark of an inexact
    // evaluation, and its low digits are an artifact of the route taken, not a
    // claim about the value. `0.1 + 2·0.1` is `0.30000000000000004` in f64 and
    // `3/10` exactly, and the JS library — which had no exact numbers at all —
    // called both equal, comparing every numeric pair against a relative
    // epsilon (`equality/numerical.js`, `1e-12`). Callers depend on that: a
    // Doenet `<sequence type="math" from=".1" step=".1">` excludes `.3` by
    // comparing generated terms to it. So when either side carries a float,
    // compare within `relative_tolerance` rather than bit-for-bit.
    if let (Expr::Num(na), Expr::Num(nb)) = (&ca, &cb) {
        if !na.is_inexact() && !nb.is_inexact() {
            return false;
        }
        return close_numeric_fuzzy(
            Complex64::new(na.to_f64(), 0.0),
            Complex64::new(nb.to_f64(), 0.0),
            opts,
            0.0,
        );
    }

    // Plus-minus (±): a `pm` node denotes a two-element value set that the
    // finite-field and single-value sampling stages treat as an opaque atom
    // (and would reject). Dispatch to the pm-aware set comparison before those
    // stages. Port of JS `equality/numerical.js` pm branch + `pm-numerical.js`.
    if crate::ops::pm::contains_pm(&ca) || crate::ops::pm::contains_pm(&cb) {
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
        // No assumptions: `equals` is assumption-free by construction. A caller
        // holding an assumption store (the JS `Context`) reaches the same stage
        // through `equals_discrete_infinite_sets`, which is the only way a
        // symbolic period can be known nonzero.
        return discrete_infinite::equals_discrete_infinite(
            &ca,
            &cb,
            opts,
            &crate::Assumptions::new(),
        );
    }

    // Stage 1c: certified exact equality (accept-only, sound). When the
    // difference is *provably* zero — surd/π/rational identities the structural
    // stages miss, e.g. `cos(π/3) − 1/2` or `√8 − 2√2` — confirm it here. This
    // must run BEFORE the rejection stages below, both of which false-reject
    // these constants: finite-field evaluates `cos(π/3)` to a meaningless
    // ℤ/pℤ value, and the numeric sampler is ill-conditioned on transcendental
    // constants (`cos(π/3) → 0.5000…1 − 0i`). Accept-only ⇒ it can only turn a
    // false negative into the correct `true`, never a false positive.
    if certified_equal(&ca, &cb) {
        return true;
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

/// Accept-only exact-equality certificate (stage 1c): is `ca − cb` *provably*
/// zero? Sampling-free — it evaluates the difference in the certified exact
/// tower ([`crate::eval_exact::exact_eval`], FULL_SIMPLIFY S1) — so a `true` is
/// a proof of equality and a `false` is merely "not certified" (fall through to
/// the rejection stages and sampling). Gated to variable-free operands: the
/// exact tower decides constants (`cos(π/3)`, surds) cheaply and definitively,
/// whereas expressions with free variables are the sampler's job and would only
/// pay `expand`/`ratform` cost here for little gain.
fn certified_equal(ca: &Expr, cb: &Expr) -> bool {
    let var_free = |e: &Expr| {
        crate::ops::variables(e)
            .iter()
            .all(|v| crate::expr::sym::is_constant_symbol(v))
    };
    if !var_free(ca) || !var_free(cb) {
        return false;
    }
    // Direct exact evaluation of the difference — NOT the full
    // `eval_exact::certified_zero`, whose `expand`/`ratform` stages target
    // *variable* rational identities and are wasted on constants (they roughly
    // doubled the corpus cost). `exact_eval` on the canonical difference decides the
    // constant tower (ℚ, surds, π, e, trig/exp/log special values) directly;
    // a value it can't evaluate returns `None` and falls through to sampling.
    let diff = crate::normalize::canonicalize(&Expr::Add(vec![
        ca.clone(),
        Expr::Neg(Box::new(cb.clone())),
    ]));
    crate::eval_exact::exact_eval(&diff).is_some_and(|v| v.is_zero())
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
    // Same interval reading as `equals`. The form check already coerces tuple,
    // array and vector spellings of a pair, so leaving the interval out made
    // it the one notational difference a *form* check refused — and it is not
    // even a difference the author wrote, but which parse built the tree.
    let (a, b) = coerce_intervals(normalize_syntactic(a), normalize_syntactic(b), opts);
    let na = coerce_seqs(a, opts);
    let nb = coerce_seqs(b, opts);
    // `allowed_error_in_numbers` compares number *leaves* within the allowed
    // error while the structure still has to match exactly — the same
    // primitive [`equals`] uses for it, so a tolerance means the same thing on
    // both paths.
    //
    // The comparison stays fuzzy even with *no* tolerance set, because "no
    // allowance" was never bit-identical in JS: `trees/basic.js equal` floors
    // its tolerance at a 1e-14 *relative* difference before `allowed_error_in_
    // numbers` is consulted at all, so two floats a few ULPs apart have always
    // compared equal here. That floor is load-bearing rather than incidental:
    // an author's `x²−x²/3` and `2x²/3` are the same number, but the grading
    // path floats each *before* like terms are collected, so one arrives as
    // `1−0.3333333333333333 = 0.6666666666666667` and the other as
    // `2/3 = 0.6666666666666666`. Exact tree equality reads the last ULP as a
    // wrong answer.
    super::fuzzy::fuzzy_tree_eq(&na, &nb, opts)
}

/// Does the tree contain a `Blank` (missing operand)? A variant check, not a
/// magic-symbol scan. Public: callers (and the corpus tests) need to know
/// whether `equals`'s stage-0 blank guard will reject a tree.
pub fn contains_blank(e: &Expr) -> bool {
    e.any_subexpr(&|c| matches!(c, Expr::Blank))
}

/// Does the tree contain an interval anywhere? The trigger for reading
/// 2-element tuples and arrays on the *other* side as intervals too.
fn has_interval(e: &Expr) -> bool {
    e.any_subexpr(&|c| matches!(c, Expr::Interval { .. }))
}

/// Two unions are equal iff their members admit a perfect pairing under
/// `equals` — a set match, since a union denotes a set (canonicalization already
/// sorts and dedups them). Matching, not greedy: pairwise equality here is not
/// transitive (a tuple equals a vector and a closed-interval-spelled array in
/// *different* pairs), so a first-fit could miss a pairing that exists. Reuses
/// the augmenting-path matcher from `fuzzy`.
fn union_set_equal(xa: &[Expr], xb: &[Expr], opts: &EqOptions) -> bool {
    const MAX_MEMBERS: usize = 32;
    let n = xa.len();
    if n == 0 || n > MAX_MEMBERS {
        return false;
    }
    let edges: Vec<Vec<usize>> = xa
        .iter()
        .map(|x| (0..n).filter(|&j| equals(x, &xb[j], opts)).collect())
        .collect();
    let mut paired: Vec<Option<usize>> = vec![None; n];
    (0..n).all(|i| super::fuzzy::augment(i, &edges, &mut vec![false; n], &mut paired))
}

/// Read a 2-element tuple or array as an interval — `(1,2)` open, `[3,4]`
/// closed — on both sides, when either side has an interval in it. It is the
/// same notation: `(1,2) union (3,4)` and the same text parsed with intervals
/// built *are* the same set, each written the only way its parse can write it.
///
/// Nothing happens when neither side mentions an interval, so an ordinary
/// point or pair is never silently reinterpreted — it takes an interval across
/// from it to make interval the reading in play.
///
/// Rides on `coerce_tuples_arrays` because it is the same notational coercion,
/// and because that is the flag the JS spec pins it to
/// (`slow_math-expressions.spec.ts`, "tuples, vectors, intervals, altvectors"
/// and "arrays, intervals"). Runs before [`coerce_seqs`], which would
/// otherwise unify Tuple and Array first and lose the open/closed distinction;
/// and it reads Seq kinds directly, so a *vector* never becomes an interval,
/// however `coerce_vectors` is set.
fn coerce_intervals(a: Expr, b: Expr, opts: &EqOptions) -> (Expr, Expr) {
    if opts.coerce_tuples_arrays && (has_interval(&a) || has_interval(&b)) {
        (crate::ops::to_intervals(&a), crate::ops::to_intervals(&b))
    } else {
        (a, b)
    }
}

/// Map coerced sequence kinds to a common kind so `(1,2)`, `[1,2]`, and vector
/// forms compare equal when the corresponding flag is set. Recurses through
/// every variant — a tuple nested inside a relation, interval, or matrix must
/// coerce too.
fn coerce_seqs(e: Expr, opts: &EqOptions) -> Expr {
    fn recur(e: &Expr, opts: &EqOptions) -> Expr {
        // One variant-specific rewrite (the Seq kind); child recursion is the
        // blessed traversal, so new `Expr` variants need no edit here.
        if let Expr::Seq(k, xs) = e {
            // The legacy coercion graph is non-transitive: `coerce_vectors`
            // governs *only* vector↔altvector, while `coerce_tuples_arrays`
            // governs tuple↔array and tuple↔vector. Applied as two gated steps so
            // that turning one flag off does not drag the other's edges with it —
            // `{coerce_tuples_arrays:false}` must keep a vector distinct from a
            // tuple even while `coerce_vectors` still unifies vector and
            // altvector.
            let k1 = match k {
                SeqKind::AltVector if opts.coerce_vectors => SeqKind::Vector,
                other => *other,
            };
            let mapped = match k1 {
                SeqKind::Array | SeqKind::Vector if opts.coerce_tuples_arrays => SeqKind::Tuple,
                other => other,
            };
            return Expr::Seq(mapped, xs.iter().map(|x| recur(x, opts)).collect());
        }
        crate::expr::map_children(e, |c| recur(c, opts))
    }
    recur(&e, opts)
}
