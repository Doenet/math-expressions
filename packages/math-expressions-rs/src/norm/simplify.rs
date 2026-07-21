//! Heuristic simplification (PORTING_PLAN.md §7e).
//!
//! **Oracle (decided 2026-07-17): own-reducedness, not tree-match to JS.** The
//! simplifier is judged on two intrinsic properties — it is *meaning-preserving*
//! (`equals(simplify(e), e)`) and *reduced* (a fixpoint: `simplify(simplify(e))
//! == simplify(e)`). JS `.simplify()` is only an advisory correctness
//! cross-check via `equals`, never a target tree shape (that would copy JS's
//! form conventions and violate the clean-slate mandate).
//!
//! **Structure.** `simplify` builds *on top of* the confluent canonical form
//! (`canonicalize`): each round rewrites bottom-up with a fixed, ordered rule
//! set, then re-canonicalizes so the smart constructors fold whatever the rules
//! produced. Rounds repeat until a pass changes nothing (the fixpoint) or fuel
//! runs out. Because every round ends in `canonicalize`, the result is always a
//! valid canonical tree, and reducedness is just "another round is a no-op".
//!
//! Rules live in three clusters (the measured gaps between `canonicalize` and
//! JS `.simplify()`): ∞/NaN folding, tuple/vector componentwise arithmetic, and
//! radical simplification. Each is assumption-free (the equality path needs only
//! that subset; full assumption-aware rewriting is deferred with §11).

use crate::assumptions::{is_nonnegative, is_real, Assumptions};
use crate::expr::{Expr, MathConst, SeqKind};
use crate::num::Number;

use super::syntactic::map_children;
use super::{add, canonicalize, mul, split_coeff};

// Max rewrite rounds: resource_limits::current().max_simplify_rounds (§7f). Every
// round strictly makes progress or we stop, so this only bounds pathological
// non-convergence on adversarial input; real inputs converge in 1–2 rounds.

/// Simplify to a meaning-preserving fixpoint (see module docs), returned in
/// display form (`norm::present`): polynomial term order, division instead of
/// negative exponents, explicit `Neg`. Internal code that needs the canonical
/// shape uses [`simplify_core`] instead.
pub fn simplify(e: &Expr) -> Expr {
    super::present(&simplify_core(e))
}

/// Simplify under variable assumptions: everything `simplify` does, plus the
/// assumption-aware rules (JS `simplify(assumptions)`), e.g.
/// `sqrt(x²) → x` under `x > 0` and `sqrt(x²) → |x|` under `x ∈ R`.
pub fn simplify_with(e: &Expr, assumptions: &Assumptions) -> Expr {
    super::present(&simplify_core_with(e, assumptions))
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
fn push_not(e: &Expr) -> Expr {
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

/// [`simplify`] without the final presentation pass: the result is canonical,
/// for internal callers that pattern-match on canonical shapes.
pub(crate) fn simplify_core(e: &Expr) -> Expr {
    simplify_core_with(e, &Assumptions::new())
}

/// [`simplify_with`] without the final presentation pass.
pub(crate) fn simplify_core_with(e: &Expr, assumptions: &Assumptions) -> Expr {
    simplify_rounds(canonicalize(e), assumptions)
}

/// Simplify a tree that is *already canonical*, skipping the initial
/// canonicalize. `equals` calls this after its canonical fast path so the
/// canonicalization it already paid for is not repeated.
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
    if let Some(r) = rule_trig_pythagorean(&e) {
        *fired = true;
        return r;
    }
    if let Some(r) = rule_seq_arith(&e) {
        *fired = true;
        return r;
    }
    if let Some(r) = rule_radical(&e) {
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
    let (Expr::Sym(f), [arg]) = (&**head, args.as_slice()) else {
        return None;
    };
    if f.name() == "abs" {
        if is_nonnegative(arg, a) == Some(true) {
            return Some(arg.clone());
        }
        if crate::assumptions::is_nonpositive(arg, a) == Some(true) {
            return Some(super::mul(vec![Expr::int(-1), arg.clone()]));
        }
        return None;
    }
    if f.name() != "sqrt" {
        return None;
    }
    // sqrt of an even power, or of a product of even powers.
    let root = even_power_root(arg)?;
    if is_nonnegative(&root, a) == Some(true) {
        Some(root)
    } else if is_real(&root, a) == Some(true) {
        Some(Expr::Apply(Box::new(Expr::sym("abs")), vec![root]))
    } else {
        None
    }
}

/// If `u` is `v^(2k)` or a product of such factors, the square root's
/// magnitude candidate `v^k · …` (before sign resolution).
fn even_power_root(u: &Expr) -> Option<Expr> {
    fn factor_root(f: &Expr) -> Option<Expr> {
        let Expr::Pow(base, exp) = f else {
            return None;
        };
        let Expr::Num(Number::Int(k)) = &**exp else {
            return None;
        };
        if *k <= 0 || k % 2 != 0 {
            return None;
        }
        Some(super::pow((**base).clone(), Expr::Num(Number::Int(k / 2))))
    }
    match u {
        Expr::Pow(..) => factor_root(u),
        Expr::Mul(fs) => {
            let roots = fs.iter().map(factor_root).collect::<Option<Vec<_>>>()?;
            Some(super::mul(roots))
        }
        _ => None,
    }
}

// ---- Cluster: ∞ / NaN folding ----
//
// Fold arithmetic that produces an infinity or NaN. Scope is deliberately the
// subset compatible with our *exact* number model, which differs from JS's
// float semantics in three principled, load-bearing ways that we do NOT emulate
// (they are documented divergences, left as known corpus gaps):
//
//   * `0 · x → 0` and `0/0 → 0`: canonicalize annihilates a zero product before
//     any infinity is seen, so `0·∞`, `0/0`, `0·(1/0)` stay `0`, not `NaN`.
//   * `0^0 → 1`: our `pow` defines this (a common CAS choice), so `(3-3)^0 → 1`,
//     not `NaN`.
//   * no signed zero: `6/-0` folds to `+∞`, not `-∞`.
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

fn rule_infnan(e: &Expr) -> Option<Expr> {
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

/// True for a `Pow(0, negative)` node — a division-by-zero pole, which we treat
/// as `+∞` (no signed zero in the exact model).
fn is_zero_pole(e: &Expr) -> bool {
    matches!(e, Expr::Pow(b, x)
        if matches!(&**b, Expr::Num(n) if n.is_zero())
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
        || matches!(e, Expr::Sym(s) if crate::sym::is_constant_symbol(&s.name()))
}

fn fold_infnan_pow(base: &Expr, exp: &Expr) -> Option<Expr> {
    // A bare pole `1/0`.
    if let (Expr::Num(b), Expr::Num(x)) = (base, exp) {
        if b.is_zero() && x.is_negative() {
            return Some(Expr::Const(MathConst::Inf));
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
/// here (it is `0`); `∞·i` folds to `∞` (matching JS), since `i` is a `Const`.
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
                    pos = true; // a pole term is +∞ in the no-signed-zero model
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
/// Only one spelling exists in the canonical layer: `canon_apply` moves a
/// function-head exponent outside the application (`sin^2(x)` → `sin(x)^2`,
/// via MOVE_EXPONENT_OUTSIDE), so `Pow(Apply(fn,[arg]), 2)` is the single
/// canonical shape.
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
fn is_vectorlike(k: SeqKind) -> bool {
    matches!(
        k,
        SeqKind::Tuple | SeqKind::Array | SeqKind::Vector | SeqKind::AltVector
    )
}

fn rule_seq_arith(e: &Expr) -> Option<Expr> {
    match e {
        Expr::Mul(factors) => distribute_mul_over_seq(factors),
        Expr::Add(terms) => combine_seqs_in_add(terms),
        _ => None,
    }
}

/// `Mul([… , Seq(k,[s1..sn]), …])` with exactly one vector-like sequence factor
/// → `Seq(k, [ (rest·s1) .. (rest·sn) ])`. More than one sequence factor is
/// left alone (the product of two vectors is not componentwise in general).
fn distribute_mul_over_seq(factors: &[Expr]) -> Option<Expr> {
    let mut seq_idx = None;
    for (i, f) in factors.iter().enumerate() {
        if matches!(f, Expr::Seq(k, _) if is_vectorlike(*k)) {
            if seq_idx.is_some() {
                return None; // two or more sequence factors: not our case
            }
            seq_idx = Some(i);
        }
    }
    let i = seq_idx?;
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
fn combine_seqs_in_add(terms: &[Expr]) -> Option<Expr> {
    // Groups keyed by (kind, len), in first-seen order; each holds term indices.
    let mut groups: Vec<((SeqKind, usize), Vec<usize>)> = Vec::new();
    for (i, t) in terms.iter().enumerate() {
        if let Expr::Seq(k, v) = t {
            if is_vectorlike(*k) {
                let key = (*k, v.len());
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
    for ((kind, len), idxs) in &groups {
        if idxs.len() < 2 {
            continue;
        }
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

// ---- Cluster: radical simplification ----
//
// Real-domain root simplification (the whole library is real-analysis
// educational): pull the sign out of an odd root of a negative
// (`cbrt(-x²) → -cbrt(x²)`), pull perfect q-th-power factors of the numeric
// coefficient out from under the radical (`cbrt(-16x⁴) → -2·cbrt(2x⁴)`), and
// fold a numeric power whose base is an exact perfect power
// (`(-8)^(1/3) → -2`). These are *false* under complex principal branches, but
// correct on the reals and what JS `.simplify()` does — the corpus's advisory
// JS cross-check confirms them.
//
// Even roots of negatives are left alone (complex). Symbolic radicands that are
// not a numeric multiple of a rest (`cbrt((-x)^3)`) need power-of-product
// expansion, which is a separate rule not yet ported.

fn rule_radical(e: &Expr) -> Option<Expr> {
    match e {
        // Numeric power with a rational exponent: fold only when it reduces to
        // an exact number (base is a perfect q-th power). Partial extraction
        // from a `Pow` form is left alone.
        Expr::Pow(base, exp) => {
            if let (Expr::Num(b), Expr::Num(Number::Rat(p, q))) = (&**base, &**exp) {
                return fold_numeric_radical(b, *p, *q);
            }
            None
        }
        // sqrt / cbrt / nthroot applications.
        Expr::Apply(head, args) => {
            let Expr::Sym(s) = &**head else { return None };
            let (degree, radicand, root) = match (s.name().as_str(), args.as_slice()) {
                ("sqrt", [r]) => (2i64, r, Root::Sqrt),
                ("cbrt", [r]) => (3, r, Root::Cbrt),
                ("nthroot", [r, Expr::Num(Number::Int(n))]) if *n >= 2 => (*n, r, Root::Nth(*n)),
                _ => return None,
            };
            simplify_root(degree, radicand, root)
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
    let base = as_small_int(b)?;
    let q = u32::try_from(q).ok()?;
    let negative = base < 0;
    if negative && q % 2 == 0 {
        return None; // even root of a negative — complex
    }
    let (m, r) = extract_qth_power(base.unsigned_abs(), q);
    if r != 1 {
        return None; // not a perfect q-th power
    }
    // value = (±m)^p, using the exact rational power.
    let root = Number::Int(if negative { -(m as i64) } else { m as i64 });
    let value = root.checked_pow_int(p)?;
    Some(Expr::Num(value))
}

/// Simplify `root_degree( radicand )`: pull the odd-root sign of a negative
/// coefficient and any perfect q-th-power factor of the (integer) coefficient
/// out front. Returns `None` when nothing can be pulled.
fn simplify_root(degree: i64, radicand: &Expr, root: Root) -> Option<Expr> {
    let q = u32::try_from(degree).ok()?;
    let (coeff, rest) = split_coeff(radicand.clone());
    let c = as_small_int(&coeff)?;
    if c == 0 {
        return None; // a zero radicand is canonicalized elsewhere
    }

    let negative = c < 0;
    if negative && q % 2 == 0 {
        return None; // even root of a negative — complex, leave alone
    }
    let sign: i64 = if negative { -1 } else { 1 };
    let (m, r) = extract_qth_power(c.unsigned_abs(), q);

    // Nothing to do: positive coefficient and no perfect-power factor.
    if sign == 1 && m == 1 {
        return None;
    }

    // Residual radicand: r · rest (r == 1 drops out; rest may be absent).
    let mut inner_factors = Vec::new();
    if r != 1 {
        inner_factors.push(Expr::Num(Number::Int(r as i64)));
    }
    if let Some(rest) = rest {
        inner_factors.push(rest);
    }
    let inner = mul(inner_factors);

    let coeff_out = Expr::Num(Number::Int(sign * m as i64));
    // If the radicand fully reduced to 1, the root vanishes; otherwise wrap the
    // residual back in the same root function.
    if matches!(&inner, Expr::Num(n) if n.is_one()) {
        Some(coeff_out)
    } else {
        Some(mul(vec![coeff_out, root.rebuild(inner)]))
    }
}

/// Largest `m` such that `m^q` divides `c`, with `r = c / m^q` the
/// q-th-power-free remainder. `c >= 1`, `q >= 2`.
///
/// Bounded on adversarial input (the "canonicalization must stay cheap on any
/// input" rule — cf. the factorial and pow caps in norm/mod.rs): the
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

/// A `Number` that is an integer fitting in `i64`, else `None` (the radical
/// rules only handle small integer coefficients — the whole simplify corpus is
/// within this range; larger/rational coefficients are left unsimplified).
fn as_small_int(n: &Number) -> Option<i64> {
    match n {
        Number::Int(i) => Some(*i),
        _ => None,
    }
}
