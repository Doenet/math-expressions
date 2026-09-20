//! Heuristic simplification.
//!
//! The simplifier is judged on two intrinsic properties — it is
//! *meaning-preserving* (`equals(simplify(e), e)`) and *reduced* (a fixpoint:
//! `simplify(simplify(e)) == simplify(e)`). It does not aim to reproduce the
//! output tree shape of JS `.simplify()`, which serves only as an advisory
//! correctness cross-check via `equals`.
//!
//! **Structure.** `simplify` builds *on top of* the confluent canonical form
//! (`canonicalize`): each round rewrites bottom-up with a fixed, ordered rule
//! set, then re-canonicalizes so the smart constructors fold whatever the rules
//! produced. Rounds repeat until a pass changes nothing (the fixpoint) or fuel
//! runs out. Because every round ends in `canonicalize`, the result is always a
//! valid canonical tree, and reducedness is just "another round is a no-op".
//!
//! **Rule clusters**, applied in this order at every node:
//!
//! 1. `rule_assumptions` — the only assumption-aware cluster, and skipped
//!    outright when no facts are in scope (`sqrt(x²) → |x|` under `x ∈ R`,
//!    `|u| → u` under `u ≥ 0`).
//! 2. `rule_infnan` — ∞/NaN folding.
//! 3. `rule_gaussian` — exact arithmetic in Q(i) (`(2+i)(2-i) → 5`).
//! 4. `rule_trig_pythagorean` — `sin²+cos² → 1` and its relatives.
//! 5. `rule_seq_arith` — componentwise arithmetic on tuples/vectors.
//! 6. `rule_radical` — numeric root extraction (`sqrt(8) → 2√2`, `cbrt(-8) → -2`).
//! 7. `rule_flatten_negated_sum_term` — splicing a negated sum into its parent
//!    sum (`a - (b + c) → a - b - c`), run before the two sign clusters so they
//!    see settled term counts.
//! 8. `rule_factor_sign_out_of_sum` — pulling a common minus sign out of a sum
//!    (`-a - b → -(a + b)`), when that shortens it.
//! 9. `rule_distribute_sign` — moving a product's sign into one factor
//!    (`-2(1-x) → 2(x-1)`), when doing so does not add minus signs.
//!
//! Only cluster 1 *needs* facts to do anything. Cluster 6 reads them to decline
//! a rewrite it cannot justify (an odd root's sign stays put over a radicand
//! known to be non-real), so an empty context makes it more eager, never wrong;
//! the rest ignore them outright. That is what lets the equality path reuse the
//! whole set with no assumptions in scope.

use crate::assumptions::{is_nonnegative, is_real, Assumptions};
use crate::expr::{Expr, MathConst, SeqKind};
use crate::num::{Number, Spelling};
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};

use super::{add, canonicalize, mul, split_coeff};
use crate::expr::map_children;

// Max rewrite rounds: resource_limits::current().max_simplify_rounds (§7f). Every
// round strictly makes progress or we stop, so this only bounds pathological
// non-convergence on adversarial input; real inputs converge in 1–2 rounds.

/// Simplify to a meaning-preserving fixpoint, returned in display form
/// (`normalize::present`): polynomial term order, division instead of negative
/// exponents, explicit `Neg`.
///
/// This is the **aggressive** simplifier — it is exactly
/// `full_simplify` with no assumptions: the base
/// canonical simplify plus the sound special-value (`exp(ln x) → x`, trig at
/// the π/12 lattice, `ln 1`, …) and rational-cancellation passes, iterated to a
/// fixpoint. (Previously `simplify` was the base only, kept byte-compatible
/// with the JS `.simplify()` corpus; the aggressive form was the opt-in
/// `full_simplify`.) Internal code that specifically needs the base behavior
/// uses [`simplify_base_with`]; code that needs the canonical (non-display)
/// shape uses [`simplify_core`].
pub fn simplify(e: &Expr) -> Expr {
    simplify_with(e, &Assumptions::new())
}

/// The base canonical simplify in display form — `simplify`'s
/// pre-`full_simplify` behavior (JS-corpus compatible: no `exp(ln x) → x` etc.)
/// under the given assumptions. Used by
/// `full_simplify` as its per-round base, so the
/// fixpoint driver does not recurse into the now-aggressive public
/// [`simplify`] / [`simplify_with`].
pub(crate) fn simplify_base_with(e: &Expr, assumptions: &Assumptions) -> Expr {
    super::present(&simplify_core_with(e, assumptions))
}

/// Simplify under variable assumptions: everything [`simplify`] does, plus the
/// assumption-aware rules (JS `simplify(assumptions)`), e.g.
/// `sqrt(x²) → x` under `x > 0` and `sqrt(x²) → |x|` under `x ∈ R`.
///
/// Like [`simplify`] this runs the full aggressive pipeline, so an empty
/// `assumptions` set makes it *identical* to [`simplify`] — adding a fact can
/// only ever make the simplifier stronger, never weaker.
pub fn simplify_with(e: &Expr, assumptions: &Assumptions) -> Expr {
    // A blank is a hole, not an unknown, and two holes are not the same hole:
    // folding across them turns "not filled in" into a definite answer. `＿/＿`
    // is not `1`, `＿−＿` is not `0`. So an incomplete expression is returned as
    // written — the JS library guards `simplify` the same way, and for the same
    // reason. `<line>` reports a degenerate line's coefficients as blanks and
    // computes its slope as `−a/b`; with the blanks cancelling, a line with no
    // slope claimed a slope of `−1`.
    //
    // The guard belongs here rather than inside the rewrite rules because it is
    // about the whole expression: whatever else the tree contains, once one
    // operand is missing there is no simplification to claim.
    if crate::equality::contains_blank(e) {
        return e.clone();
    }
    crate::normalize::full_simplify(e, assumptions)
}

/// Port of `me.simplify_logical`: numeric folding under assumptions, then push
/// every `not` inward — double-negation collapse, De Morgan over `and`/`or`,
/// and negation of relations (`not(a < b)` → `a ≥ b`). Blanks pass through
/// untouched, matching JS. (Deviation: we also negate `le`/`ge` relations,
/// which the JS left as a no-op.)
pub fn simplify_logical(e: &Expr, assumptions: &Assumptions) -> Expr {
    if crate::equality::contains_blank(e) {
        return e.clone();
    }
    super::present(&push_not(&simplify_core_with(e, assumptions)))
}

/// Rewrite `not(...)` toward the leaves. Recurses into the negated operand
/// first so nested negations collapse bottom-up.
///
/// Public because [`simplify_logical`] is *not* a substitute for it: that one
/// simplifies and canonicalizes first, and canonicalization turns `x > a` into
/// `a < x`. A caller filing the relation it was handed — the assumptions store
/// — needs the not-pushdown without the reorientation, and this is it. Note
/// that the relation branch below rebuilds the node from the same `operands`
/// vector, so operand order survives.
pub fn push_not(e: &Expr) -> Expr {
    if let Expr::Not(inner) = e {
        let inner = push_not(inner);
        let neg = |x: &Expr| push_not(&Expr::Not(Box::new(x.clone())));
        return match inner {
            // Double negation.
            Expr::Not(a) => *a,
            // De Morgan.
            Expr::And(xs) => Expr::Or(xs.iter().map(neg).collect()),
            Expr::Or(xs) => Expr::And(xs.iter().map(neg).collect()),
            // Negate a simple (unchained) relation.
            Expr::Relation { operands, ops } if ops.len() == 1 => Expr::Relation {
                operands,
                ops: vec![ops[0].negate()],
            },
            // Nothing to push through: keep the `not`.
            other => Expr::Not(Box::new(other)),
        };
    }
    map_children(e, push_not)
}

/// Merge nested `and`/`or` into their parent and drop a connective left with a
/// single operand — the JS `flatten_logical`.
///
/// [`super::default_order::flatten`] does the merging (for every associative
/// operator, which is a superset and harmless here). The one-operand collapse
/// is the part that matters to a caller comparing facts for equality: `and(x)`
/// and `x` are the same assumption, and a store that filed both would hand back
/// duplicates.
pub fn flatten_logical(e: &Expr) -> Expr {
    fn collapse(e: &Expr) -> Expr {
        let e = map_children(e, collapse);
        match &e {
            Expr::And(xs) | Expr::Or(xs) if xs.len() == 1 => xs[0].clone(),
            _ => e,
        }
    }
    collapse(&super::default_order::flatten(e))
}

/// The **base** rewrite rounds without the final presentation pass: the result
/// is canonical, for internal callers that pattern-match on canonical shapes.
/// This is [`simplify_base_with`] minus `present`, *not* the public (now
/// aggressive) [`simplify`] minus `present` — the special-value and
/// rational-cancellation passes are not run. Callers that want those want
/// `full_simplify`.
pub(crate) fn simplify_core(e: &Expr) -> Expr {
    simplify_core_with(e, &Assumptions::new())
}

/// [`simplify_base_with`] without the final presentation pass.
pub(crate) fn simplify_core_with(e: &Expr, assumptions: &Assumptions) -> Expr {
    simplify_rounds(canonicalize(e), assumptions)
}

/// The base rewrite rounds on a tree that is *already canonical*, skipping the
/// initial canonicalize. `equals` calls this after its canonical fast path so
/// the canonicalization it already paid for is not repeated.
pub(crate) fn simplify_canonical(cur: Expr) -> Expr {
    simplify_rounds(cur, &Assumptions::new())
}

fn simplify_rounds(mut cur: Expr, assumptions: &Assumptions) -> Expr {
    for _ in 0..crate::resource_limits::current().max_simplify_rounds {
        let mut fired = false;
        let rewritten = rewrite(&cur, &mut fired, assumptions);
        // No rule applied anywhere: `cur` came out of canonicalize, so it is
        // already the fixpoint — skip the re-canonicalize and tree compare.
        if !fired {
            return cur;
        }
        let next = canonicalize(&rewritten);
        // Rules fired but the canonical result is unchanged (ping-pong guard).
        if next == cur {
            return next;
        }
        cur = next;
    }
    cur
}

/// One bottom-up rewriting pass: rewrite children, then apply node-local rules.
/// Sets `fired` when any rule applied; the result is not necessarily canonical
/// (`simplify_canonical` re-canonicalizes after a fired pass).
fn rewrite(e: &Expr, fired: &mut bool, assumptions: &Assumptions) -> Expr {
    // Rewrite children first (post-order), so a rule sees already-simplified
    // subtrees.
    let e = map_children(e, |c| rewrite(c, fired, assumptions));
    if !assumptions.is_empty() {
        if let Some(r) = rule_assumptions(&e, assumptions) {
            *fired = true;
            return r;
        }
    }
    // Cluster rules, in order. Each returns `Some(replacement)` if it fired.
    if let Some(r) = rule_infnan(&e) {
        *fired = true;
        return r;
    }
    if let Some(r) = rule_gaussian(&e) {
        *fired = true;
        return r;
    }
    if let Some(r) = rule_trig_pythagorean(&e) {
        *fired = true;
        return r;
    }
    if let Some(r) = rule_seq_arith(&e) {
        *fired = true;
        return r;
    }
    if let Some(r) = rule_radical(&e, assumptions) {
        *fired = true;
        return r;
    }
    // Flatten before factoring: a negated sum spliced into its parent changes
    // the parent's term counts, so the threshold must see the settled sum.
    if let Some(r) = rule_flatten_negated_sum_term(&e) {
        *fired = true;
        return r;
    }
    if let Some(r) = rule_factor_sign_out_of_sum(&e) {
        *fired = true;
        return r;
    }
    if let Some(r) = rule_distribute_sign(&e) {
        *fired = true;
        return r;
    }
    e
}

// ---- Cluster: assumption-aware rules ----
//
// Active only under a non-empty [`Assumptions`] context (JS
// `simplify(assumptions)`). `sqrt` of even powers resolves by the base's
// known sign (`sqrt(x²) → x` when `x ≥ 0`, `→ |x|` when merely real), and —
// a deliberate divergence from the JS, which never rewrites `abs` — `|u|`
// itself simplifies away when the sign is known: `|u| → u` under `u ≥ 0`,
// `|u| → −u` under `u ≤ 0`. Composed, `sqrt(x²) | x<0` → `−x` (JS: `|x|`).

fn rule_assumptions(e: &Expr, a: &Assumptions) -> Option<Expr> {
    let Expr::Apply(head, args) = e else {
        return None;
    };
    let Expr::Sym(f) = &**head else {
        return None;
    };
    if f.name() == "abs" {
        let [arg] = args.as_slice() else {
            return None;
        };
        if is_nonnegative(arg, a) == Some(true) {
            return Some(arg.clone());
        }
        if crate::assumptions::is_nonpositive(arg, a) == Some(true) {
            return Some(super::mul(vec![Expr::int(-1), arg.clone()]));
        }
        // Distribute over real operands: `|uⁿ| = |u|ⁿ`, `|∏ uᵢ| = ∏ |uᵢ|`. This
        // is what turns the `sqrt(16x⁶) → 4·|x³|` an even-root extraction yields
        // into the conventional `4·|x|³`.
        //
        // The *exponent* has to be real too, not just the base: `|b^w| = |b|^w`
        // holds because `|b^w| = |b|^w · e^(−arg(b)·Im w)`, and that correction
        // vanishes only for real `w`. With `w = i` and `b` real positive the
        // left side is 1 while `|b|^i` is not even real — an `abs` that returned
        // a complex value.
        let abs = |x: &Expr| Expr::Apply(Box::new(Expr::sym("abs")), vec![x.clone()]);
        match arg {
            Expr::Pow(b, x) if is_real(b, a) == Some(true) && is_real(x, a) == Some(true) => {
                return Some(super::pow(abs(b), (**x).clone()));
            }
            Expr::Mul(fs) if fs.iter().all(|f| is_real(f, a) == Some(true)) => {
                return Some(super::mul(fs.iter().map(abs).collect()));
            }
            _ => {}
        }
        return None;
    }
    let (degree, radicand, root) = as_root_call(&f.name(), args)?;
    extract_powers_from_root(degree, radicand, root, a)
}

/// `sqrt(r)`, `cbrt(r)` and `nthroot(r, n)` as the one `(degree, radicand,
/// Root)` triple the two radical passes both work in. `None` for any other
/// head, and for an `nthroot` whose degree is not an integer `≥ 2`.
fn as_root_call<'a>(name: &str, args: &'a [Expr]) -> Option<(i64, &'a Expr, Root)> {
    Some(match (name, args) {
        ("sqrt", [r]) => (2i64, r, Root::Sqrt),
        ("cbrt", [r]) => (3, r, Root::Cbrt),
        ("nthroot", [r, Expr::Num(Number::Int(n))]) if *n >= 2 => (*n, r, Root::Nth(*n)),
        _ => return None,
    })
}

/// Pull whole `q`-th powers of a *variable* factor out from under a root:
///
/// ```text
/// nthroot(∏ vᵢ^kᵢ · C, q)  →  ∏ vᵢ^⌊kᵢ/q⌋ · nthroot(∏ vᵢ^(kᵢ mod q) · C, q)
/// ```
///
/// This is the assumption-gated companion to [`simplify_root`], which extracts
/// only the perfect `q`-th-power part of the *numeric* coefficient. A variable
/// factor needs a known sign, so it can only move under an [`Assumptions`]
/// context — with none, `sqrt(x²)` and `cbrt(x³)` stay as written, per the
/// settled root spec.
///
/// Soundness of the split, in the two cases the root degree forces apart:
///
/// * **Even `q`.** The extracted part `(∏ vᵢ^⌊kᵢ/q⌋)^q` is an even power of a
///   real, hence `≥ 0`, and pulling a *nonnegative* factor out of a principal
///   root is always valid — so the residual's sign does not matter and even a
///   non-real residual is fine (`sqrt(x²·i) = |x|·sqrt(i)`). What comes out is
///   the magnitude: `|∏ vᵢ^⌊kᵢ/q⌋|`, which drops its `abs` when the assumptions
///   already put it at `≥ 0`. This subsumes the former `sqrt`-only rule, which
///   handled just the case where every exponent was a multiple of 2.
/// * **Odd `q`.** The extracted part keeps its sign, so the split needs the
///   residual to stay on the real line — `cbrt(x³·i) ≠ x·cbrt(i)` for `x < 0`,
///   the two principal branches differing by a third of a turn. Either a
///   nonnegative extracted part or a real residual rules that out.
///
/// Returns `None` when no factor has a `q`-th power to give, so a residual that
/// cannot reduce further does not re-fire the rule.
fn extract_powers_from_root(q: i64, radicand: &Expr, root: Root, a: &Assumptions) -> Option<Expr> {
    let q_even = u32::try_from(q).ok()? % 2 == 0;
    let factors: Vec<&Expr> = match radicand {
        Expr::Mul(fs) => fs.iter().collect(),
        other => vec![other],
    };

    let mut outside = Vec::new();
    let mut inside = Vec::new();
    for f in factors {
        // A numeric coefficient is `simplify_root`'s business; leave it under
        // the radical and let that rule take its perfect-power part.
        let (base, k) = match f {
            Expr::Num(_) => (None, 0),
            Expr::Pow(b, x) => match &**x {
                Expr::Num(Number::Int(k)) if *k > 0 => (Some((**b).clone()), *k),
                _ => (None, 0),
            },
            other => (Some(other.clone()), 1),
        };
        let Some(base) = base.filter(|_| k / q > 0) else {
            inside.push(f.clone());
            continue;
        };
        // Splitting the root across factors needs each moved base to be real.
        if is_real(&base, a) != Some(true) {
            return None;
        }
        outside.push(super::pow(base.clone(), Expr::Num(Number::Int(k / q))));
        if k % q > 0 {
            inside.push(super::pow(base, Expr::Num(Number::Int(k % q))));
        }
    }
    if outside.is_empty() {
        return None;
    }

    let outside = super::mul(outside);
    let nonneg = is_nonnegative(&outside, a) == Some(true);
    // An empty residual is the empty product, not `Mul([])`.
    let inner = if inside.is_empty() {
        Expr::int(1)
    } else {
        super::mul(inside)
    };
    if !q_even && !nonneg && is_real(&inner, a) != Some(true) {
        return None;
    }
    let outside = if q_even && !nonneg {
        Expr::Apply(Box::new(Expr::sym("abs")), vec![outside])
    } else {
        outside
    };

    if matches!(&inner, Expr::Num(n) if n.is_one()) {
        Some(outside)
    } else {
        Some(super::mul(vec![outside, root.rebuild(inner)]))
    }
}

// ---- Cluster: ∞ / NaN folding ----
//
// Fold arithmetic that produces an infinity or NaN. Scope is deliberately the
// subset compatible with our *exact* number model.
//
// The indeterminate forms fold to `NaN` rather than annihilating: `0·∞`, `0/0`
// and `0·(1/0)` unconditionally (see `normalize::constructors::annihilate`),
// and `0^0` under the default `pow_strict` policy. An earlier revision of this
// crate folded them to `0`/`1` and this comment described that; the change is
// deliberate — DoenetML computes an undefined slope as `0/0`, so annihilating
// it to `0` reports a degenerate line as horizontal, a wrong number on a
// grading path rather than a visible failure. Pinned by
// `tests/signed_zero.rs::indeterminate_forms_do_not_annihilate`.
//
// Two of those *are* divergences from JS, deliberately: JS evaluates `0 ** 0`
// as `1`, and `0 * Infinity` only reaches `NaN` because JS has no exact zero to
// annihilate with. `0^0` is therefore policy-dependent — clearing
// `constant_policy::pow_strict` (legacy's `me.math.pow_strict`) restores the JS
// answer. `0·∞` is not: the `skip_ordering` path folds it in
// `ops::preserve_order`, which does not read that policy.
//
// We DO track a signed zero (`Number::NegZero`), narrowly: it is value-equal to
// `0` everywhere except the pole fold, so `6/-0 → −∞` and `1/((−1)·0) → −∞`
// while `−0` on its own still prints as `0`. See `Number::NegZero`.
//
// What we DO fold: a pole `Pow(0, negative) → ∞`, infinities absorbing
// *constant* co-operands in sums/products, `x/∞ → 0`, and `∞ − ∞ → NaN`.
//
// The sum/product folds fire ONLY when every operand is a constant (a `Num`, a
// `Const`, or a zero-pole). A symbolic factor blocks the fold: `x·∞` is +∞,
// −∞, or NaN depending on x's sign (folding it to +∞ made `x·∞ == ∞` and
// `x/0 == 1/0` wrongly true), a `Seq` factor is not even a scalar
// (`∞·(a,b)` must not collapse to ∞), and `x + ∞ − ∞` must not drop `x`.
// Two *pure-constant* indeterminate forms both folding to `NaN` (and thus
// comparing equal) is accepted: that matches JS `.simplify()`, which returns
// the NaN literal for them.

pub(crate) fn rule_infnan(e: &Expr) -> Option<Expr> {
    match e {
        Expr::Pow(base, exp) => fold_infnan_pow(base, exp),
        Expr::Mul(factors) => fold_infnan_mul(factors),
        Expr::Add(terms) => fold_infnan_add(terms),
        _ => None,
    }
}

fn const_of(e: &Expr) -> Option<MathConst> {
    match e {
        Expr::Const(c) => Some(*c),
        _ => None,
    }
}

/// True for a `Pow(0, negative)` node — a division-by-zero pole. `+0` gives a
/// `+∞` pole, `−0` a `−∞` pole (see [`neg_zero_pole`]).
fn is_zero_pole(e: &Expr) -> bool {
    matches!(e, Expr::Pow(b, x)
        if matches!(&**b, Expr::Num(n) if n.is_zero())
        && matches!(&**x, Expr::Num(n) if n.is_negative()))
}

/// True for a `Pow(−0, negative)` node — a pole whose base is exact negative
/// zero, so it folds to `−∞` rather than `+∞`. Implies [`is_zero_pole`].
fn neg_zero_pole(e: &Expr) -> bool {
    matches!(e, Expr::Pow(b, x)
        if matches!(&**b, Expr::Num(n) if n.is_neg_zero())
        && matches!(&**x, Expr::Num(n) if n.is_negative()))
}

/// An operand whose value is a definite constant for ∞/NaN folding purposes: a
/// number, a math constant, a zero-pole, or one of the constant *symbols*
/// `pi`/`e`/`i` (the parsers emit these as `Sym`; the same name set the
/// evaluator treats as bound constants — see `free_symbols`). All three are
/// finite, nonzero, and not negative reals, so they never flip the fold's
/// sign. Anything else (variables, function applications, sequences, …) has
/// unknown sign / finiteness / shape and must block the fold.
fn is_infnan_constant(e: &Expr) -> bool {
    matches!(e, Expr::Num(_) | Expr::Const(_))
        || is_zero_pole(e)
        || matches!(e, Expr::Sym(s) if crate::expr::sym::is_constant_symbol(&s.name()))
}

fn fold_infnan_pow(base: &Expr, exp: &Expr) -> Option<Expr> {
    // A bare pole `1/0` → `+∞`, or `1/(−0)` → `−∞`.
    if let (Expr::Num(b), Expr::Num(x)) = (base, exp) {
        if b.is_zero() && x.is_negative() {
            return Some(Expr::Const(if b.is_neg_zero() {
                MathConst::NegInf
            } else {
                MathConst::Inf
            }));
        }
    }
    // `∞^n`: → 0 for n < 0, → ∞ for n > 0 (n == 0 is handled by `pow`).
    if let (Some(MathConst::Inf), Expr::Num(x)) = (const_of(base), exp) {
        if x.is_negative() {
            return Some(Expr::Num(Number::zero()));
        }
        if x.is_positive() {
            return Some(Expr::Const(MathConst::Inf));
        }
    }
    // `(−∞)^n`: → 0 for n < 0; for a positive *integer* n the sign follows
    // parity. A non-integer exponent of −∞ is complex/undefined — left alone.
    if let (Some(MathConst::NegInf), Expr::Num(x)) = (const_of(base), exp) {
        if x.is_negative() {
            return Some(Expr::Num(Number::zero()));
        }
        if let Number::Int(k) = x {
            if *k > 0 {
                return Some(Expr::Const(if k % 2 == 0 {
                    MathConst::Inf
                } else {
                    MathConst::NegInf
                }));
            }
        }
    }
    None
}

/// An all-constant product with an infinite factor (a `±∞` constant or a
/// zero-pole) folds to `±∞`, or `NaN` if any factor is already `NaN`.
/// Canonicalize has already removed any literal zero, so `0·∞` never reaches
/// here (it is `0`).
///
/// `∞·i` folds to `∞`, matching the JS library's `simplify` — see
/// `equality.rs`, `infnan_folds_are_conservative`. This engine has one
/// infinity and it lies on the real axis; an infinity in the imaginary
/// direction has nowhere else to go. (JS's *evaluator* disagreed with its own
/// simplifier here and answered complex.js's `Complex.INFINITY`, which is why
/// DoenetML's `<number>Infinity i</number>` used to render `NaN + NaN i`.)
fn fold_infnan_mul(factors: &[Expr]) -> Option<Expr> {
    if !factors.iter().all(is_infnan_constant) {
        return None; // a symbolic factor: sign/shape unknown, do not fold
    }
    let mut saw_infinite = false;
    let mut sign: i64 = 1;
    for f in factors {
        match const_of(f) {
            Some(MathConst::NaN) => return Some(Expr::Const(MathConst::NaN)),
            Some(MathConst::Inf) => saw_infinite = true,
            Some(MathConst::NegInf) => {
                saw_infinite = true;
                sign = -sign;
            }
            _ => {
                if is_zero_pole(f) {
                    saw_infinite = true;
                    if neg_zero_pole(f) {
                        sign = -sign; // a `1/(−0)` factor is `−∞`
                    }
                } else if let Expr::Num(n) = f {
                    if n.is_negative() {
                        sign = -sign;
                    }
                }
            }
        }
    }
    if !saw_infinite {
        return None;
    }
    Some(Expr::Const(if sign < 0 {
        MathConst::NegInf
    } else {
        MathConst::Inf
    }))
}

/// An all-constant sum with an infinite term folds to that infinity; `+∞`
/// together with `−∞` (or any `NaN`) folds to `NaN`. Finite constant terms are
/// absorbed. A symbolic term blocks the fold (`x + ∞ − ∞` must not drop `x`).
fn fold_infnan_add(terms: &[Expr]) -> Option<Expr> {
    if !terms.iter().all(is_infnan_constant) {
        return None;
    }
    let (mut pos, mut neg, mut nan) = (false, false, false);
    for t in terms {
        match const_of(t) {
            Some(MathConst::Inf) => pos = true,
            Some(MathConst::NegInf) => neg = true,
            Some(MathConst::NaN) => nan = true,
            _ => {
                if is_zero_pole(t) {
                    // `1/0` is a `+∞` term, `1/(−0)` a `−∞` term.
                    if neg_zero_pole(t) {
                        neg = true;
                    } else {
                        pos = true;
                    }
                }
            }
        }
    }
    if !(pos || neg || nan) {
        return None;
    }
    Some(Expr::Const(if nan || (pos && neg) {
        MathConst::NaN
    } else if pos {
        MathConst::Inf
    } else {
        MathConst::NegInf
    }))
}

// ---- Cluster: exact arithmetic in ℚ(i) ----

/// Evaluate a variable-free subtree that mentions `i` exactly, in ℚ(i).
///
/// `i` is a symbol here, not a numeric type, so nothing in the numeric fold
/// multiplies two complex numbers: `(1+i)(1-i)` is a product of two sums and
/// stays one, where a student writing `<math simplify>` expects `2`. The
/// smart-constructor fold for `i^n` gets `i·i·i` and `(a+bi)(c+di)` *after
/// expansion*, but a product of sums is never expanded by `simplify`, so this
/// rule closes the case the constructor cannot see.
///
/// Exact, and it stays in ℚ(i): a `√2` or a `π` anywhere makes the evaluation
/// decline rather than approximate. The rule also declines when the value it
/// computes is the expression it was handed — otherwise `2i` would "fire"
/// forever against its own output and the fixpoint would never settle.
fn rule_gaussian(e: &Expr) -> Option<Expr> {
    // Only worth attempting where an `i` is involved: a real subtree is the
    // canonical fold's business and rebuilding it here would churn spellings
    // (an exact `1/3` would come back as `1/3` through a different path).
    if !mentions_i(e) {
        return None;
    }
    let (re, im) = gaussian_eval(e)?;
    let rebuilt = canonicalize(&gaussian_expr(&re, &im)?);
    (rebuilt != canonicalize(e)).then_some(rebuilt)
}

fn mentions_i(e: &Expr) -> bool {
    is_imaginary_unit(e) || e.children().into_iter().any(mentions_i)
}

/// `i` in either spelling. The parser produces the symbol; the constructors
/// produce the constant, and both reach the rules. The symbol counts only while
/// `define_i` holds — a document whose variables run `g, h, i` gets no complex
/// arithmetic out of its third coordinate.
use crate::constant_policy::is_i as is_imaginary_unit;

/// `(re, im)` of `e` as exact rationals, or `None` if `e` leaves ℚ(i).
fn gaussian_eval(e: &Expr) -> Option<(BigRational, BigRational)> {
    let zero = BigRational::zero();
    match e {
        Expr::Num(n) => Some((n.to_bigrational()?, zero)),
        _ if is_imaginary_unit(e) => Some((zero, BigRational::one())),
        Expr::Neg(a) => {
            let (r, i) = gaussian_eval(a)?;
            Some((-r, -i))
        }
        Expr::Add(ts) => ts.iter().try_fold((zero.clone(), zero), |(ar, ai), t| {
            let (br, bi) = gaussian_eval(t)?;
            Some((ar + br, ai + bi))
        }),
        Expr::Mul(fs) => fs
            .iter()
            .try_fold((BigRational::one(), zero), |(ar, ai), f| {
                let (br, bi) = gaussian_eval(f)?;
                Some((&ar * &br - &ai * &bi, ar * bi + ai * br))
            }),
        Expr::Div(a, b) => {
            let (ar, ai) = gaussian_eval(a)?;
            let (br, bi) = gaussian_eval(b)?;
            gaussian_div(ar, ai, br, bi)
        }
        Expr::Pow(b, k) => {
            // Integer exponents only: `i^(1/2)` is a branch cut, not a walk of
            // the four-cycle, and `to_integer` would truncate it to `i^0 = 1`.
            let k = match k.as_ref() {
                Expr::Num(n) => {
                    let q = n.to_bigrational()?;
                    if !q.is_integer() {
                        return None;
                    }
                    q.to_integer().to_i64()?
                }
                _ => return None,
            };
            // Bounded so a huge exponent cannot spend the budget here; past the
            // bound the value is left to whoever can afford it. Checked *before*
            // the loop so the refusal is free: clamping the iteration count
            // instead meant `(2+i)^1000000` paid for 64 bigint multiplications,
            // on operands growing to hundreds of digits, and then threw them
            // away — at every node of every fixpoint pass.
            if k.unsigned_abs() > 64 {
                return None;
            }
            let (br, bi) = gaussian_eval(b)?;
            // The exponent bound alone is *per node*, and `rewrite` is
            // bottom-up, so nesting multiplies it: `(((2+i)^64+1)^64+1)^64` is
            // about thirty typeable characters and did not finish in twenty
            // seconds, with one more level far worse again. The base arrives
            // already folded, so bound the size of the *result* as well —
            // untrusted student input is what this pass runs on.
            let base_bits = [&br, &bi]
                .into_iter()
                .map(|q| q.numer().bits().max(q.denom().bits()))
                .max()
                .unwrap_or(0);
            if base_bits > 1
                && k.unsigned_abs().saturating_mul(base_bits)
                    > crate::resource_limits::current().max_gaussian_pow_bits
            {
                return None;
            }
            let (mut ar, mut ai) = (BigRational::one(), BigRational::zero());
            for _ in 0..k.unsigned_abs() {
                let (nr, ni) = (&ar * &br - &ai * &bi, &ar * &bi + &ai * &br);
                ar = nr;
                ai = ni;
            }
            if k < 0 {
                return gaussian_div(BigRational::one(), BigRational::zero(), ar, ai);
            }
            Some((ar, ai))
        }
        _ => None,
    }
}

/// `(ar + ai·i) / (br + bi·i)`, by the conjugate. `None` on division by zero,
/// which is a pole rather than a value and belongs to the ∞ rules.
fn gaussian_div(
    ar: BigRational,
    ai: BigRational,
    br: BigRational,
    bi: BigRational,
) -> Option<(BigRational, BigRational)> {
    let d = &br * &br + &bi * &bi;
    if d.is_zero() {
        return None;
    }
    Some(((&ar * &br + &ai * &bi) / &d, (&ai * &br - &ar * &bi) / d))
}

/// `re + im·i` as an expression, exactly. `None` if either part leaves the
/// range the engine's rationals hold.
fn gaussian_expr(re: &BigRational, im: &BigRational) -> Option<Expr> {
    let part = |q: &BigRational| -> Option<Expr> {
        Some(Expr::Num(Number::from_bigrational_spelled(
            q.clone(),
            Spelling::Fraction,
        )))
    };
    let mut terms = Vec::new();
    if !re.is_zero() {
        terms.push(part(re)?);
    }
    if !im.is_zero() {
        terms.push(if im.is_one() {
            Expr::Const(MathConst::I)
        } else {
            mul(vec![part(im)?, Expr::Const(MathConst::I)])
        });
    }
    Some(match terms.len() {
        0 => Expr::Num(Number::zero()),
        1 => terms.pop()?,
        _ => add(terms),
    })
}

// ---- Cluster: trigonometric Pythagorean identity ----
//
// `C·sin(θ)² + C·cos(θ)² → C` for a shared coefficient `C` and argument `θ`.
// Runs on a canonical `Add`, pairing each `sin` square with a matching `cos`
// square; unmatched terms pass through. This is the one trig identity the
// equality path needs (the `sin²+cos²` corpus cases, including one nested inside
// a set membership). Broader trig normalization is a later addition.

fn rule_trig_pythagorean(e: &Expr) -> Option<Expr> {
    let Expr::Add(terms) = e else { return None };

    // Classify each term as `coeff · fn(arg)²` with fn ∈ {sin, cos}.
    let classified: Vec<Option<TrigSquare>> = terms.iter().map(as_trig_square).collect();

    let mut used = vec![false; terms.len()];
    let mut folded_coeffs: Vec<Expr> = Vec::new();
    let mut any = false;

    for i in 0..terms.len() {
        let Some(si) = &classified[i] else { continue };
        if used[i] || si.func != TrigFn::Sin {
            continue;
        }
        // Find an unused `cos` square with the same coefficient and argument.
        for j in 0..terms.len() {
            if used[j] || j == i {
                continue;
            }
            let Some(cj) = &classified[j] else { continue };
            if cj.func == TrigFn::Cos && cj.coeff == si.coeff && cj.arg == si.arg {
                used[i] = true;
                used[j] = true;
                folded_coeffs.push(si.coeff.clone());
                any = true;
                break;
            }
        }
    }

    if !any {
        return None;
    }
    let mut out: Vec<Expr> = terms
        .iter()
        .enumerate()
        .filter(|(k, _)| !used[*k])
        .map(|(_, t)| t.clone())
        .collect();
    out.append(&mut folded_coeffs);
    Some(add(out))
}

#[derive(PartialEq)]
enum TrigFn {
    Sin,
    Cos,
}

struct TrigSquare {
    coeff: Expr,
    func: TrigFn,
    arg: Expr,
}

/// Recognize a term of the form `coeff · fn(arg)²` (fn ∈ {sin, cos}). The
/// coefficient is whatever multiplies the square (`Num(1)` when there is none);
/// a term with more than one trig-square factor is rejected (ambiguous).
fn as_trig_square(term: &Expr) -> Option<TrigSquare> {
    // `… · fn(arg)² · …`: exactly one factor is a trig square, the rest form the
    // coefficient.
    if let Expr::Mul(factors) = term {
        let mut hit = None;
        for (i, f) in factors.iter().enumerate() {
            if let Some((func, arg)) = trig_square_base(f) {
                if hit.is_some() {
                    return None; // two trig squares — not our shape
                }
                hit = Some((i, func, arg));
            }
        }
        let (i, func, arg) = hit?;
        let coeff = mul(factors
            .iter()
            .enumerate()
            .filter(|(k, _)| *k != i)
            .map(|(_, f)| f.clone())
            .collect());
        return Some(TrigSquare { coeff, func, arg });
    }
    // Bare `fn(arg)²` (either canonical spelling), coefficient 1.
    let (func, arg) = trig_square_base(term)?;
    Some(TrigSquare {
        coeff: Expr::Num(Number::one()),
        func,
        arg,
    })
}

/// If `e` is `sin(arg)²` or `cos(arg)²`, return the function and argument.
/// Only one spelling exists in the canonical layer: canonicalization moves a
/// function-head exponent outside the application (`sin^2(x)` → `sin(x)^2`), so
/// `Pow(Apply(fn,[arg]), 2)` is the single canonical shape.
fn trig_square_base(e: &Expr) -> Option<(TrigFn, Expr)> {
    let Expr::Pow(base, exp) = e else { return None };
    if !is_two(exp) {
        return None;
    }
    let Expr::Apply(head, args) = &**base else {
        return None;
    };
    let (Expr::Sym(s), [arg]) = (&**head, args.as_slice()) else {
        return None;
    };
    Some((trig_fn(&s.name())?, arg.clone()))
}

fn is_two(e: &Expr) -> bool {
    matches!(e, Expr::Num(n) if *n == Number::Int(2))
}

fn trig_fn(name: &str) -> Option<TrigFn> {
    match name {
        "sin" => Some(TrigFn::Sin),
        "cos" => Some(TrigFn::Cos),
        _ => None,
    }
}

// ---- Cluster: tuple / vector componentwise arithmetic ----
//
// A scalar multiple of a vector-like sequence distributes over its components
// (`c·(a,b) → (ca, cb)`), and same-shape sequences in a sum add componentwise
// (`(a,b)+(c,d) → (a+c, b+d)`). Together these two rules, run to a fixpoint,
// also cover subtraction (`(a,b)-(c,d)` canonicalizes to a sum with a `-1·(c,d)`
// term, which the first rule turns into a sequence before the second combines
// it) and mixed shapes (only equal-length, equal-kind sequences merge).

/// Sequence kinds that behave like coordinate vectors, so arithmetic acts
/// componentwise. Sets and plain lists are excluded — componentwise arithmetic
/// over an unordered/heterogeneous collection is not meaningful.
///
/// `Array` is included even though [`vector_class`] below notes that `[a, b]`
/// is the container `createIntervals` reads as an interval. The two are not in
/// conflict: an interval in *this* crate is [`Expr::Interval`], which is not a
/// `Seq` at all and is left alone here (negating one would have to swap its
/// endpoints, and `-[1, 2]` as an interval is `[-2, -1]`). A `Seq(Array)` is
/// the bracket *spelling* of a coordinate list, so `-[1, 2]` is `[-1, -2]`;
/// what `vector_class` withholds from it is merging with a tuple, not
/// componentwise arithmetic.
fn is_vectorlike(k: SeqKind) -> bool {
    matches!(
        k,
        SeqKind::Tuple | SeqKind::Array | SeqKind::Vector | SeqKind::AltVector
    )
}

/// Which vector-like kinds add together. Kinds in the same class denote the
/// same object in different notation, so a sum of them folds.
///
/// `(a,b)`, `⟨a,b⟩` and the vector spelling are one class: DoenetML authors
/// write a vector all three ways, and `⟨a,b⟩ + (c,d)` is reachable from
/// ordinary markup. `Array` is its own class — `[a,b]` is a different
/// container (`createIntervals` reads it as an interval), and `equals` keeps
/// tuple↔array coercion a separate opt-in from tuple↔vector for the same
/// reason. Length is still part of the key, so different arities never merge.
pub(crate) fn vector_class(k: SeqKind) -> Option<u8> {
    match k {
        SeqKind::Tuple | SeqKind::Vector | SeqKind::AltVector => Some(0),
        SeqKind::Array => Some(1),
        _ => None,
    }
}

/// The container a folded group carries: the members' own kind when they all
/// agree, else the class's canonical one.
///
/// Deliberately *not* "the left operand's". `Add` is commutative and its
/// operands are canonically sorted, so by the time this runs there is no left
/// operand to read — keying off position would make `u + v` and `v + u`
/// canonicalize to different trees, which is exactly what a canonical form
/// must not do.
fn folded_seq_kind(kinds: impl Iterator<Item = SeqKind>) -> SeqKind {
    let mut kinds = kinds.peekable();
    let first = *kinds.peek().expect("group is non-empty");
    let mut all_same = true;
    let mut any_vector = false;
    for k in kinds {
        if k != first {
            all_same = false;
        }
        if matches!(k, SeqKind::Vector | SeqKind::AltVector) {
            any_vector = true;
        }
    }
    if all_same {
        return first;
    }
    // A `vector`/`altvector` anywhere in the group is the stronger reading and
    // wins: `⟨a,b⟩ + (c,d)` and `(a,b) + <vector>(c,d)` are a vector, not a
    // point — collapsing to `tuple` handed DoenetML's `<vector>` a point. Mixed
    // `vector`/`altvector` land on the plain `vector` spelling. Only when no
    // member carries vector-ness does the weaker `tuple` reading remain.
    //
    // There is no `array` case to write: `array` is the only member of its
    // class, so an all-`array` group is `all_same` and a mixed group can never
    // contain one.
    if any_vector {
        SeqKind::Vector
    } else {
        SeqKind::Tuple
    }
}

fn rule_seq_arith(e: &Expr) -> Option<Expr> {
    match e {
        Expr::Mul(factors) => distribute_mul_over_seq(factors),
        Expr::Add(terms) => combine_seqs_in_add(terms),
        _ => None,
    }
}

/// Contract a product involving vector-like sequences, matching `expand`:
/// - **one** vector factor → distribute the scalar rest into its components
///   (`c·(a,b) → (ca, cb)`);
/// - **two** vector factors → the row·column dot product (`(a,b)·(c,d) → ac+bd`),
///   through the shared [`contract_pair`](crate::normalize) core, times the rest.
///
/// A matrix among the factors is not a scalar: `M·(e,f)` is a contraction that
/// belongs to `expand` (canonicalization renders what the author wrote), so we
/// decline and leave it written.
pub(crate) fn distribute_mul_over_seq(factors: &[Expr]) -> Option<Expr> {
    if factors.iter().any(crate::normalize::is_matrix_valued) {
        return None;
    }
    let seqs: Vec<usize> = factors
        .iter()
        .enumerate()
        .filter(|(_, f)| matches!(f, Expr::Seq(k, _) if is_vectorlike(*k)))
        .map(|(i, _)| i)
        .collect();
    // Two vectors contract to their dot; the two factors are replaced by that
    // scalar and the product re-formed (a third vector, if any, is then the
    // one-vector scalar-distribution case on the next fixpoint pass).
    if let [i, j, ..] = seqs[..] {
        let dotted = crate::normalize::contract_pair(&factors[i], &factors[j])?;
        let rest: Vec<Expr> = factors
            .iter()
            .enumerate()
            .filter(|(k, _)| *k != j)
            .map(|(k, f)| if k == i { dotted.clone() } else { f.clone() })
            .collect();
        return Some(mul(rest));
    }
    let i = *seqs.first()?;
    let Expr::Seq(kind, comps) = &factors[i] else {
        return None;
    };
    let others: Vec<&Expr> = factors
        .iter()
        .enumerate()
        .filter(|(j, _)| *j != i)
        .map(|(_, f)| f)
        .collect();
    let new_comps = comps
        .iter()
        .map(|c| {
            let mut fs: Vec<Expr> = others.iter().map(|f| (*f).clone()).collect();
            fs.push(c.clone());
            mul(fs)
        })
        .collect();
    Some(Expr::Seq(*kind, new_comps))
}

/// Within a sum, group vector-like sequence terms by (kind, length) and replace
/// each group of ≥2 with a single componentwise sum. Non-sequence terms and
/// lone sequences pass through untouched. Returns `None` when no group has ≥2
/// members (nothing to combine — keeps the pass a strict fixpoint).
pub(crate) fn combine_seqs_in_add(terms: &[Expr]) -> Option<Expr> {
    // Groups keyed by (vector class, len), in first-seen order.
    let mut groups: Vec<((u8, usize), Vec<usize>)> = Vec::new();
    for (i, t) in terms.iter().enumerate() {
        if let Expr::Seq(k, v) = t {
            if let Some(class) = vector_class(*k) {
                let key = (class, v.len());
                match groups.iter_mut().find(|(gk, _)| *gk == key) {
                    Some((_, idxs)) => idxs.push(i),
                    None => groups.push((key, vec![i])),
                }
            }
        }
    }
    if !groups.iter().any(|(_, idxs)| idxs.len() >= 2) {
        return None;
    }

    // Index of the first member of each ≥2 group → its componentwise sum.
    // Other members of such groups are dropped from the output.
    let mut skip = vec![false; terms.len()];
    let mut combined: Vec<(usize, Expr)> = Vec::new();
    for ((_, len), idxs) in &groups {
        if idxs.len() < 2 {
            continue;
        }
        let kind = &folded_seq_kind(idxs.iter().map(|&i| match &terms[i] {
            Expr::Seq(k, _) => *k,
            _ => unreachable!("group members are sequences"),
        }));
        let sum = (0..*len)
            .map(|p| {
                add(idxs
                    .iter()
                    .map(|&i| match &terms[i] {
                        Expr::Seq(_, v) => v[p].clone(),
                        _ => unreachable!("group members are sequences"),
                    })
                    .collect())
            })
            .collect();
        combined.push((idxs[0], Expr::Seq(*kind, sum)));
        for &i in idxs {
            skip[i] = true;
        }
    }

    let mut out = Vec::with_capacity(terms.len());
    for (i, t) in terms.iter().enumerate() {
        if let Some((_, seq)) = combined.iter().find(|(first, _)| *first == i) {
            out.push(seq.clone());
        } else if !skip[i] {
            out.push(t.clone());
        }
    }
    Some(add(out))
}

// ---- Cluster: sign normalization ----
//
// `−(1 − x)` is `x − 1`, and a reader expects to see it written that way. The
// canonical form spells a leading minus as a negative numeric coefficient, so
// what arrives here is `Mul(−1, Add(…))`; pushing that sign into the sum is the
// only way the outer minus can disappear.
//
// Two things are easy to conflate here and are *not* the same operation:
//
//   * moving the **sign** of a product into one of its factors — `−2·(1 − x)`
//     is `2·(x − 1)`, and the `2` never moves;
//   * distributing the **coefficient** — `2·(1 − x)` to `2 − 2x`, which is
//     `expand`'s job and not done here.
//
// So the rule is about the sign alone, and it applies to any product whose
// coefficient is negative, not only to a bare `−1` with a sum beside it. The
// sign goes into exactly *one* factor (moving it into two would cancel it), so
// where several factors could take it we pick the one that sheds the most
// signs.
//
// **Which factors can take a sign.** A sum can: negate every term. A power can
// when its exponent is an odd integer, since `(−b)^m = −(b^m)` there — and that
// includes `m = −1`, so `−1/(1 − x)` reaches `1/(x − 1)`. An even exponent
// cannot: `−(1 − x)²` is not `(x − 1)²`. A non-integer exponent cannot either,
// which keeps the rule away from `−√(1 − x)`.
//
// **When it is worth doing.** Count minus signs. For a candidate sum of `k`
// terms of which `n` are negated, the product costs `1 + n` signs as written
// and `k − n` with the sign pushed in, so it is worth doing iff
// `1 + n ≥ k − n`, i.e. `2n ≥ k − 1`. Writing `saving = 2n − k`, the rule fires
// iff the best candidate has `saving ≥ −1` — the `−1` being the exact tie,
// which goes to the pushed-in form so that a tied sum has one fixpoint and not
// two (see [`rule_factor_sign_out_of_sum`]):
//
// | input | n / k | result |
// | --- | --- | --- |
// | `−(1 − x)` | 1 / 2 | `x − 1` — the tie, and still one sign fewer |
// | `−(−x − 1)` | 2 / 2 | `x + 1` |
// | `−(x − y − z)` | 2 / 3 | `−x + y + z` |
// | `−2(1 − x)` | 1 / 2 | `2(x − 1)` — the sign moves, the `2` does not |
// | `−y(1 − x)` | 1 / 2 | `y(x − 1)` |
// | `−(1 − x)(1 − y)` | 1 / 2 each | `(x − 1)(1 − y)` — one factor takes it |
// | `−(1 − x)³` | 1 / 2 | `(x − 1)³` — odd exponent |
// | `−(1 − x)²` | — | unchanged — even exponent cannot take a sign |
// | `−(x + y)` | 0 / 2 | unchanged — distributing would *add* a sign |
// | `−(x + y − z)` | 1 / 3 | `−x − y + z` — a tie, and ties push in |
//
// Counting signs rather than reading the leading term keeps the rule
// independent of how the sum happens to be ordered, so it fires the same way on
// `−(1 − x)` and `−(−x + 1)`. The rewrite always leaves a positive coefficient
// behind, so it cannot fire on its own output.

/// Whether a term of a sum carries a minus sign — a negative number, or a
/// product whose numeric coefficient is negative (the canonical spelling of a
/// negated term; `split_coeff` reads the coefficient from the same position).
fn is_negated_term(t: &Expr) -> bool {
    match t {
        Expr::Num(n) => n.is_negative(),
        Expr::Mul(fs) => matches!(fs.first(), Some(Expr::Num(n)) if n.is_negative()),
        _ => false,
    }
}

/// Net minus signs removed by pushing a sign into `f`, or `None` if `f` cannot
/// take one at all. Negative means pushing the sign in would *add* signs.
fn sign_absorption(f: &Expr) -> Option<i64> {
    match f {
        Expr::Add(terms) => {
            let negated = terms.iter().filter(|t| is_negated_term(t)).count();
            Some(2 * negated as i64 - terms.len() as i64)
        }
        // `(−b)^m = −(b^m)` for odd integer `m`, so the sign passes straight
        // through to the base. Even and non-integer exponents cannot.
        Expr::Pow(b, x) => match &**x {
            Expr::Num(Number::Int(m)) if m % 2 != 0 => sign_absorption(b),
            _ => None,
        },
        _ => None,
    }
}

/// Push a sign into `f`. Mirrors [`sign_absorption`] case for case; call only
/// where that returned `Some`.
fn absorb_sign(f: &Expr) -> Expr {
    match f {
        Expr::Add(terms) => add(terms
            .iter()
            .map(|t| mul(vec![Expr::int(-1), t.clone()]))
            .collect()),
        Expr::Pow(b, x) => super::pow(absorb_sign(b), (**x).clone()),
        // `sign_absorption` returned `None` for everything else.
        other => other.clone(),
    }
}

/// `−a − b → −(a + b)`: factor a sign out of a sum that is mostly negated.
///
/// The converse of [`rule_distribute_sign`], and the reason `simplify` has a
/// *single* fixpoint per value rather than one per spelling. That rule alone
/// only ever pushes a sign **into** a sum, so a sum that arrived already
/// distributed was a fixpoint by default and `−(a+b)/3` and `(−a−b)/3` — the
/// same value — both stood still. A preference expressed by only one of the two
/// rewrites is not a normal form.
///
/// **The threshold.** For a sum of `k` terms of which `n` are negated, the
/// spelling costs `n` signs as written and `1 + (k − n)` factored, so factoring
/// is worth it iff `1 + (k − n) < n`, i.e. `2n ≥ k + 2`. Strict: a tie stays
/// distributed, which is what keeps `−(1 − x) → x − 1` and `a − b − c` as
/// written.
///
/// | sum | n / k | result |
/// | --- | --- | --- |
/// | `−a − b` | 2 / 2 | `−(a + b)` — strictly fewer signs |
/// | `−a − b − c` | 3 / 3 | `−(a + b + c)` |
/// | `a − b − c` | 2 / 3 | unchanged — a tie, and ties stay distributed |
/// | `−a − b + c` | 2 / 3 | unchanged |
/// | `x − 1` | 1 / 2 | unchanged |
///
/// **Exactly one of the two spellings is stable, always.** This is the property
/// that makes `simplify` single-fixpoint on sign placement, and it needs both
/// thresholds to line up. Take a value whose distributed spelling has `k` terms
/// with `n` negated; the factored spelling `−(S)` then has `m = k − n` negated.
///
/// - Distributed is stable iff this rule declines: `2n ≤ k + 1`.
/// - Factored is stable iff [`rule_distribute_sign`] declines: `2m ≤ k − 2`,
///   i.e. `2n ≥ k + 2`.
///
/// The two conditions are exact complements, so precisely one holds: never
/// both (which would be two fixpoints for one value — the bug this rule was
/// added to fix) and never neither (which would ping-pong). That is also why
/// the tie in `rule_distribute_sign` must go *in* while the tie here stays out;
/// moving either threshold by one re-opens one of the two failure modes.
fn rule_factor_sign_out_of_sum(e: &Expr) -> Option<Expr> {
    let Expr::Add(terms) = e else { return None };
    let negated = terms.iter().filter(|t| is_negated_term(t)).count();
    if 2 * negated < terms.len() + 2 {
        return None;
    }
    Some(mul(vec![Expr::int(-1), absorb_sign(e)]))
}

/// `q + 12 − (q + 2) → q + 12 − q − 2`: flatten a negated sum that is a *term*
/// of a larger sum.
///
/// This is what is left of the old unconditional `−1 · (sum) → −a − b − c`, and
/// the narrowing is the point. Distributing a lone `−(x + y)` only spends a
/// sign; the reason to distribute at all is to let terms meet and cancel, and
/// that can only happen inside a parent sum. So the rewrite is restricted to
/// exactly the position where it can pay.
///
/// Without it a difference of two sums never cancels: `(q + 12 - (q+2))/2`
/// stayed unreduced where the JS library gives `5` — the shape `<lineSegment>`
/// produces for a symbolic midpoint, so a user-visible coordinate was showing
/// its own derivation instead of its value.
///
/// Only a coefficient of exactly `−1` qualifies. `x − 2(a + b)` is left alone
/// because distributing a *magnitude* turns one term into two, which is
/// `expand`'s job — the same line [`rule_distribute_sign`] draws.
fn rule_flatten_negated_sum_term(e: &Expr) -> Option<Expr> {
    let Expr::Add(terms) = e else { return None };
    if !terms.iter().any(|t| negated_sum_terms(t).is_some()) {
        return None;
    }
    let mut out = Vec::new();
    for t in terms {
        match negated_sum_terms(t) {
            Some(inner) => out.extend(inner),
            None => out.push(t.clone()),
        }
    }
    Some(add(out))
}

/// The terms of `t` with their signs flipped, when `t` is exactly `−1 · (sum)`.
/// Canonical form keeps the coefficient first, so a second factor or any other
/// coefficient means this is not a bare negated sum.
fn negated_sum_terms(t: &Expr) -> Option<Vec<Expr>> {
    let Expr::Mul(factors) = t else { return None };
    let [Expr::Num(coeff), Expr::Add(inner)] = factors.as_slice() else {
        return None;
    };
    if coeff.to_f64() != -1.0 {
        return None;
    }
    Some(
        inner
            .iter()
            .map(|x| mul(vec![Expr::int(-1), x.clone()]))
            .collect(),
    )
}

fn rule_distribute_sign(e: &Expr) -> Option<Expr> {
    let Expr::Mul(factors) = e else { return None };
    // The sign lives on the numeric coefficient, which canonical form keeps
    // first (see `split_coeff`). A zero coefficient is not a sign to move —
    // the product is zero and canonicalize collapses it.
    let Some(Expr::Num(coeff)) = factors.first() else {
        return None;
    };
    if !coeff.is_negative() || coeff.is_zero() {
        return None;
    }
    // The sign can go into exactly one factor — into two it would cancel — so
    // take the one that sheds the most signs. Ties go to the earliest factor,
    // which canonical ordering makes deterministic.
    let (idx, saving) = factors
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(i, f)| sign_absorption(f).map(|s| (i, s)))
        .max_by_key(|&(i, s)| (s, std::cmp::Reverse(i)))?;
    // Dropping the outer sign saves one; taking it costs `-saving` inside, so
    // `saving == -1` is an exact tie in sign count. Ties push **in**, and that
    // is load-bearing rather than cosmetic: [`rule_factor_sign_out_of_sum`]
    // pulls out only on a strict win, so declining ties here would leave both
    // spellings of a tied sum stable and `simplify` would have two fixpoints
    // for one value again. See that rule's docs for the disjointness argument.
    if saving < -1 {
        return None;
    }
    let mut out = factors.clone();
    out[0] = Expr::Num(coeff.neg());
    out[idx] = absorb_sign(&factors[idx]);
    Some(mul(out))
}

// ---- Cluster: radical simplification ----
//
// Numeric root simplification. The confirmed DoenetML rule: a *number* under a
// root folds — preferring a real root when one exists, else the correct
// principal complex root — while a *variable* radicand never folds.
//
// - Odd root of a negative: the real root wins, so pull the sign out
//   (`cbrt(-16x⁴) → -2·cbrt(2x⁴)`, `(-8)^(1/3) → -2`, and for a base that is
//   not a perfect power, `(-2)^(1/3) → -2^(1/3)`). This is the *real* branch,
//   and it only applies while the rest of the radicand could be real — see
//   `simplify_root`.
// - Perfect q-th-power factors of the numeric coefficient come out front
//   (`sqrt(8) → 2·sqrt(2)`). The coefficient may be a fraction, which extracts
//   independently in the numerator and the denominator (`sqrt(2/9) → sqrt(2)/3`).
// - Even root of a *negative number*: the positive perfect-power factor comes
//   out and the sign stays under the radical (`sqrt(-18) → 3·sqrt(-2)`), which
//   is valid on either branch. `i` surfaces only when the radicand is exactly
//   minus a perfect square at q = 2 (`sqrt(-4) → 2i`, `sqrt(-1) → i`); a
//   non-square residual (`sqrt(-2)`) is left alone, and higher even roots need
//   the surd `cos(π/q)+i·sin(π/q)` form we don't build yet. See
//   `negative_even_root`.
//
// A variable radicand of an even root (`sqrt(-4x)`, `sqrt(x²)`) has unknown
// sign, so it never folds. Symbolic radicands that are not a numeric multiple
// of a rest (`cbrt((-x)^3)`) need power-of-product expansion, a separate rule
// not yet ported.

fn rule_radical(e: &Expr, assumptions: &Assumptions) -> Option<Expr> {
    match e {
        // Numeric power with a rational exponent: fold when it reduces to an
        // exact number (base is a perfect q-th power); otherwise, for a
        // negative base under an odd root, pull the sign out on the real
        // branch — `(-2)^(1/3) → -(2^(1/3))`, `(-2)^(2/3) → 2^(2/3)` — the
        // same branch `fold_numeric_radical` takes for perfect powers and
        // `simplify_root` for the `cbrt`/`nthroot` spellings. It has to happen
        // *here*, not in the evaluator: `evaluate_to_constant` runs this pass
        // and then the certified-digits tape before any evaluator is
        // consulted, so a sign left on the base keeps the tape on the
        // principal branch no matter what the evaluators say. With the base
        // positive, no numeric path has a branch to choose. Partial extraction
        // of perfect-power *factors* from a `Pow` form is still left alone
        // (`8^(1/2)` stays symbolic while `sqrt(8)` reduces), and an even root
        // of a negative keeps its sign (no real branch to prefer).
        Expr::Pow(base, exp) => {
            if let (Expr::Num(b), Expr::Num(Number::Rat(p, q, _))) = (&**base, &**exp) {
                if let Some(folded) = fold_numeric_radical(b, *p, *q) {
                    return Some(folded);
                }
                if b.is_negative() && q % 2 != 0 {
                    let pos = Expr::Pow(Box::new(Expr::Num(b.neg())), exp.clone());
                    return Some(if p % 2 == 0 {
                        pos
                    } else {
                        mul(vec![Expr::int(-1), pos])
                    });
                }
            }
            None
        }
        // sqrt / cbrt / nthroot applications.
        Expr::Apply(head, args) => {
            let Expr::Sym(s) = &**head else { return None };
            let (degree, radicand, root) = as_root_call(&s.name(), args)?;
            simplify_root(degree, radicand, root, assumptions)
        }
        _ => None,
    }
}

/// How to rebuild a residual radical after extraction.
enum Root {
    Sqrt,
    Cbrt,
    Nth(i64),
}

impl Root {
    fn rebuild(&self, radicand: Expr) -> Expr {
        match self {
            Root::Sqrt => Expr::Apply(Box::new(Expr::sym("sqrt")), vec![radicand]),
            Root::Cbrt => Expr::Apply(Box::new(Expr::sym("cbrt")), vec![radicand]),
            Root::Nth(n) => Expr::Apply(
                Box::new(Expr::sym("nthroot")),
                vec![radicand, Expr::Num(Number::Int(*n))],
            ),
        }
    }
}

/// `b^(p/q)` on the reals, folded only when it is an exact number: `b` must be a
/// perfect q-th power (root `m`), giving `sign · m^p` with the odd-root sign
/// rule. Non-perfect bases and even roots of negatives return `None`.
fn fold_numeric_radical(b: &Number, p: i64, q: i64) -> Option<Expr> {
    let (bn, bd) = as_small_rational(b)?;
    let q = u32::try_from(q).ok()?;
    let negative = bn < 0;
    let spelling = b.spelling();
    if negative && q % 2 == 0 {
        // Even root of a negative — no real value. At q = 2 the principal value
        // is `m^p · i^p` when `|base| = m²` is a perfect square (this form does
        // not partial-extract, matching the positive `b^(p/q)` path — `8^(1/2)`
        // stays symbolic while `sqrt(8)` reduces). Higher even roots need a surd
        // form we don't build, so they stay symbolic.
        if q == 2 {
            let (m, r) = extract_qth_power_rational(bn.unsigned_abs(), bd, 2, spelling)?;
            if !r.is_one() {
                return None;
            }
            let mag = m.checked_pow_int(p)?;
            // p is odd (p/q is reduced, q = 2), so `i^p` is `±i`; fold the sign
            // into the magnitude.
            let mag = if p.rem_euclid(4) == 3 { mag.neg() } else { mag };
            return Some(mul(vec![Expr::Num(mag), Expr::sym("i")]));
        }
        return None;
    }
    let (m, r) = extract_qth_power_rational(bn.unsigned_abs(), bd, q, spelling)?;
    if !r.is_one() {
        return None; // not a perfect q-th power
    }
    // value = (±m)^p, using the exact rational power.
    let root = if negative { m.neg() } else { m };
    let value = root.checked_pow_int(p)?;
    Some(Expr::Num(value))
}

/// Simplify `root_degree( radicand )`: pull the odd-root sign of a negative
/// coefficient and any perfect q-th-power factor of the (rational) coefficient
/// out front. Returns `None` when nothing can be pulled.
fn simplify_root(
    degree: i64,
    radicand: &Expr,
    root: Root,
    assumptions: &Assumptions,
) -> Option<Expr> {
    let q = u32::try_from(degree).ok()?;
    let (coeff, rest) = split_coeff(radicand.clone());
    let (cn, cd) = as_small_rational(&coeff)?;
    if cn == 0 {
        return None; // a zero radicand is canonicalized elsewhere
    }
    let spelling = coeff.spelling();

    let negative = cn < 0;
    if negative && q % 2 == 0 {
        // No real even root of a negative, but the *positive* perfect-power
        // factor still comes out — valid on either branch — with the sign (and
        // any variable part) kept under the root: `sqrt(-16x) → 4·sqrt(-x)`,
        // `nthroot(-16x³,4) → 2·nthroot(-x³,4)`, `sqrt(-4) → 2i`. A *purely
        // numeric* higher even root (`nthroot(-16,4)`) has no simpler form we
        // build — its residual `⁴√(-1)` needs a surd lattice — so it stays
        // symbolic.
        return negative_even_root(cn.unsigned_abs(), cd, q, rest.as_ref(), &root, spelling);
    }

    // Pulling the sign out of an odd root picks the *real* branch:
    // `cbrt(-u) = -cbrt(u)` holds for real `u`, but not on the principal
    // complex branch — `cbrt(-i)` is `e^(-iπ/6)` while `-cbrt(i)` is
    // `e^(-i5π/6)`, a different number. A residual whose realness is merely
    // *unproven* still counts as real, since a bare symbol is exactly that
    // (`is_real("x")` is `None`) and `cbrt(-x) → -cbrt(x)` is the convention
    // this crate and the JS library share. What must not count as real is a
    // residual that visibly mentions `i`: `is_real` answers `None` for `i·x`
    // and `x + i` too, because it cannot rule out an imaginary `x`, and taking
    // that for real gave `cbrt(-i·x) → -cbrt(i·x)` — a different number, as
    // this crate's own `equals` reports for the constant case. The check is
    // therefore *per subexpression* rather than on the residual as a whole:
    // realness is not a fact this engine propagates through `+`, `*` or `^`
    // (`is_real(sqrt(-2))` is `Some(false)` but `is_real(x·sqrt(-2))` is
    // `None`), so asking only the root would take every one of
    // `x·sqrt(-2)`, `x·ln(-1)`, `x·arcsin(2)`, `x + sqrt(-2)` — and, under an
    // `x ∉ R` assumption, `x + 1` and `x·y` — for real. A residual declines
    // when any part of it is *provably* non-real, or is spelled `i`
    // ([`constant_policy::is_i`], asked separately so that a document
    // declaring `i` an ordinary variable keeps the real-branch rewrite while
    // one that does not still declines on `i·x`, whose `is_real` is `None`).
    // A part being non-real does not make the whole non-real, so this
    // over-declines — which only ever leaves an expression as written.
    //
    // When the residual declines, the sign stays under the radical and just
    // the perfect power comes out (`cbrt(-8i) → 2·cbrt(-i)`), which holds on
    // either branch because a positive real factor does not move the argument.
    let sign_is_real = rest.as_ref().is_none_or(|r| {
        !r.any_subexpr(&|e| {
            crate::constant_policy::is_i(e) || is_real(e, assumptions) == Some(false)
        })
    });
    let sign: i64 = if negative && sign_is_real { -1 } else { 1 };
    let inner_negated = negative && !sign_is_real;
    let (m, r) = extract_qth_power_rational(cn.unsigned_abs(), cd, q, spelling)?;

    // Nothing to do: the sign is staying where it is and there is no
    // perfect-power factor to pull out.
    if sign == 1 && m.is_one() {
        return None;
    }

    // Residual radicand: r · rest (r == 1 drops out; rest may be absent).
    let r = if inner_negated { r.neg() } else { r };
    let mut inner_factors = Vec::new();
    if !r.is_one() {
        inner_factors.push(Expr::Num(r));
    }
    if let Some(rest) = rest {
        inner_factors.push(rest);
    }
    let inner = mul(inner_factors);

    let coeff_out = Expr::Num(if sign < 0 { m.neg() } else { m });
    // If the radicand fully reduced to 1, the root vanishes; otherwise wrap the
    // residual back in the same root function.
    if matches!(&inner, Expr::Num(n) if n.is_one()) {
        Some(coeff_out)
    } else {
        Some(mul(vec![coeff_out, root.rebuild(inner)]))
    }
}

/// Even root of a negative `−(num/den)·rest` (`num/den > 0`): pull the positive
/// perfect-`q`-th-power factor `m` out (`sqrt(−16x) → 4·sqrt(−x)`,
/// `nthroot(−16x³,4) → 2·nthroot(−x³,4)`), keeping the sign and any variable
/// `rest` under the root. `i` surfaces only for `q = 2` when the whole radicand
/// is `−(perfect square)` (`sqrt(−4) → 2i`, `sqrt(−1) → i`). Declines when
/// nothing extracts (so the fixpoint does not spin on `sqrt(−10)`), and for a
/// purely numeric `q ≥ 4` root, whose residual (`⁴√(−1)`) has no simpler form.
fn negative_even_root(
    num: u64,
    den: i64,
    q: u32,
    rest: Option<&Expr>,
    root: &Root,
    spelling: Spelling,
) -> Option<Expr> {
    if q > 2 && rest.is_none() {
        return None;
    }
    let (m, r) = extract_qth_power_rational(num, den, q, spelling)?;
    let mut factors = Vec::new();
    if !m.is_one() {
        factors.push(Expr::Num(m.clone()));
    }
    if q == 2 && r.is_one() && rest.is_none() {
        // `|radicand|` is a perfect square: `sqrt(−m²) = m·i`.
        factors.push(Expr::sym("i"));
    } else {
        if m.is_one() {
            return None; // nothing to pull out; the root stays as written
        }
        // Residual under the root, negated: `−(r · rest)`.
        let mut inner = vec![Expr::Num(r.neg())];
        if let Some(rest) = rest {
            inner.push(rest.clone());
        }
        factors.push(root.rebuild(mul(inner)));
    }
    Some(mul(factors))
}

/// Split a positive rational `num/den` into `(m, r)` with `num/den = m^q · r`:
/// `m` is the largest rational whose q-th power divides out and `r` the
/// q-th-power-free remainder. `num` and `den` are coprime, so the two sides
/// extract independently. `None` if either piece leaves `i64`.
fn extract_qth_power_rational(
    num: u64,
    den: i64,
    q: u32,
    spelling: Spelling,
) -> Option<(Number, Number)> {
    let (m_num, r_num) = extract_qth_power(num, q);
    let (m_den, r_den) = extract_qth_power(den.unsigned_abs(), q);
    let to_rat = |n: u64, d: u64| {
        Some(Number::rat_spelled(
            i64::try_from(n).ok()?,
            i64::try_from(d).ok()?,
            spelling,
        ))
    };
    Some((to_rat(m_num, m_den)?, to_rat(r_num, r_den)?))
}

/// Largest `m` such that `m^q` divides `c`, with `r = c / m^q` the
/// q-th-power-free remainder. `c >= 1`, `q >= 2`.
///
/// Bounded on adversarial input (the "canonicalization must stay cheap on any
/// input" rule — cf. the factorial and pow caps in normalize/constructors.rs): the
/// perfect-power case is decided in O(log c) by an integer nth-root, and
/// partial extraction trial-divides only up to a small cap, so
/// `sqrt(<19-digit prime>)` cannot stall `equals()`. Beyond the cap a large
/// prime-power factor stays under the radical — still correct, just less
/// simplified (classroom coefficients are far below the cap).
fn extract_qth_power(c: u64, q: u32) -> (u64, u64) {
    // Fast path: c is a perfect q-th power.
    let root = integer_nth_root(c, q);
    if pow_u128(root, q) == Some(c as u128) {
        return (root, 1);
    }
    let mut m: u64 = 1;
    let mut remaining = c;
    let mut d: u64 = 2;
    while d <= crate::resource_limits::current().max_trial_divisor {
        let Some(dq) = pow_u128(d, q) else { break };
        if dq > remaining as u128 {
            break;
        }
        let dq = dq as u64;
        if remaining.is_multiple_of(dq) {
            m *= d; // m^q divides c <= u64::MAX, so m cannot overflow
            remaining /= dq;
        } else {
            d += 1;
        }
    }
    (m, remaining)
}

fn pow_u128(base: u64, exp: u32) -> Option<u128> {
    (base as u128).checked_pow(exp)
}

/// ⌊c^(1/q)⌋ via a float seed corrected exactly with integer arithmetic.
fn integer_nth_root(c: u64, q: u32) -> u64 {
    if c <= 1 {
        return c;
    }
    let mut r = (c as f64).powf(1.0 / f64::from(q)).round() as u64;
    while r > 0 && pow_u128(r, q).is_none_or(|v| v > c as u128) {
        r -= 1;
    }
    while pow_u128(r + 1, q).is_some_and(|v| v <= c as u128) {
        r += 1;
    }
    r
}

/// A `Number` as an exact rational `(num, den)` in lowest terms with `den > 0`,
/// both fitting in `i64`; else `None`. The radical rules stay on small exact
/// rationals — the whole simplify corpus is within this range, and bignum or
/// float coefficients are left unsimplified. A decimal literal parses to `Rat`,
/// so `sqrt(0.25)` reduces here too and keeps its decimal spelling.
fn as_small_rational(n: &Number) -> Option<(i64, i64)> {
    match n {
        Number::Int(i) => Some((*i, 1)),
        Number::Rat(num, den, _) => Some((*num, *den)),
        _ => None,
    }
}
