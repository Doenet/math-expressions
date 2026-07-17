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

use crate::expr::{Expr, MathConst, SeqKind};
use crate::num::Number;

use super::{add, canonicalize, mul};

/// Max rewrite rounds. A stand-in for the §7f `Limits`/`fuel` context (not yet
/// built): every round strictly makes progress or we stop, so this only bounds
/// pathological non-convergence on adversarial input. Generous — real inputs
/// converge in 1–2 rounds.
const MAX_ROUNDS: u32 = 32;

/// Simplify to a meaning-preserving canonical fixpoint (see module docs).
pub fn simplify(e: &Expr) -> Expr {
    let mut cur = canonicalize(e);
    for _ in 0..MAX_ROUNDS {
        let next = canonicalize(&rewrite(&cur));
        if next == cur {
            return next;
        }
        cur = next;
    }
    cur
}

/// One bottom-up rewriting pass: rewrite children, then apply node-local rules.
/// The result is not necessarily canonical; `simplify` re-canonicalizes it.
fn rewrite(e: &Expr) -> Expr {
    // Rewrite children first (post-order), so a rule sees already-simplified
    // subtrees.
    let e = map_children(e, rewrite);
    // Cluster rules, in order. Each returns `Some(replacement)` if it fired.
    if let Some(r) = rule_infnan(&e) {
        return r;
    }
    if let Some(r) = rule_trig_pythagorean(&e) {
        return r;
    }
    if let Some(r) = rule_seq_arith(&e) {
        return r;
    }
    if let Some(r) = rule_radical(&e) {
        return r;
    }
    e
}

/// Apply `f` to each immediate child, rebuilding the same variant. Mirrors the
/// traversal in `canonicalize`/`desugar_units` (a shared `fold` driver is the
/// §5b follow-up; until then each pass spells the recursion out).
fn map_children(e: &Expr, f: fn(&Expr) -> Expr) -> Expr {
    match e {
        Expr::Num(_) | Expr::Sym(_) | Expr::Const(_) | Expr::Blank | Expr::Ldots => e.clone(),

        Expr::Add(xs) => Expr::Add(xs.iter().map(f).collect()),
        Expr::Mul(xs) => Expr::Mul(xs.iter().map(f).collect()),
        Expr::Div(a, b) => Expr::Div(Box::new(f(a)), Box::new(f(b))),
        Expr::Pow(a, b) => Expr::Pow(Box::new(f(a)), Box::new(f(b))),
        Expr::Neg(x) => Expr::Neg(Box::new(f(x))),
        Expr::And(xs) => Expr::And(xs.iter().map(f).collect()),
        Expr::Or(xs) => Expr::Or(xs.iter().map(f).collect()),
        Expr::Not(x) => Expr::Not(Box::new(f(x))),
        Expr::Union(xs) => Expr::Union(xs.iter().map(f).collect()),
        Expr::Intersect(xs) => Expr::Intersect(xs.iter().map(f).collect()),
        Expr::Apply(h, xs) => Expr::Apply(Box::new(f(h)), xs.iter().map(f).collect()),
        Expr::Prime(x) => Expr::Prime(Box::new(f(x))),
        Expr::Index(a, b) => Expr::Index(Box::new(f(a)), Box::new(f(b))),
        Expr::Seq(k, xs) => Expr::Seq(*k, xs.iter().map(f).collect()),
        Expr::Interval { endpoints, closed } => Expr::Interval {
            endpoints: Box::new((f(&endpoints.0), f(&endpoints.1))),
            closed: *closed,
        },
        Expr::Relation { operands, ops } => Expr::Relation {
            operands: operands.iter().map(f).collect(),
            ops: ops.clone(),
        },
        Expr::Matrix {
            rows,
            cols,
            entries,
        } => Expr::Matrix {
            rows: *rows,
            cols: *cols,
            entries: entries.iter().map(f).collect(),
        },
        Expr::OtherOp(name, args) => Expr::OtherOp(*name, args.iter().map(f).collect()),
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
// What we DO fold: a pole `Pow(0, negative) → ∞`, infinities absorbing finite
// operands in sums/products, `x/∞ → 0`, and `∞ − ∞ → NaN`.

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
    None
}

/// A product with an infinite factor (a `±∞` constant or a zero-pole) folds to
/// `±∞`, or `NaN` if any factor is already `NaN`. Canonicalize has already
/// removed any literal zero, so `0·∞` never reaches here (it is `0`).
fn fold_infnan_mul(factors: &[Expr]) -> Option<Expr> {
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
                // Other symbolic factors (e.g. `i`) do not change the magnitude
                // being infinite — JS treats `∞·i` as `∞`.
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

/// A sum with an infinite term folds to that infinity; `+∞` together with `−∞`
/// (or any `NaN`) folds to `NaN`. Finite terms are absorbed.
fn fold_infnan_add(terms: &[Expr]) -> Option<Expr> {
    let (mut pos, mut neg, mut nan) = (false, false, false);
    for t in terms {
        match const_of(t) {
            Some(MathConst::Inf) => pos = true,
            Some(MathConst::NegInf) => neg = true,
            Some(MathConst::NaN) => nan = true,
            _ => {}
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

/// If `e` is `sin(arg)²` or `cos(arg)²`, return the function and argument. Both
/// canonical spellings are accepted: the power outside the application
/// (`sin(x)^2` → `Pow(Apply(sin,[x]), 2)`) and on the function head
/// (`sin^2(x)` → `Apply(Pow(sin, 2), [x])`) — `canonicalize` does not unify
/// these (moving the exponent out lives in the syntactic normalizer), so a
/// nested `sin^2(x)+cos^2(x)` keeps the head-power form.
fn trig_square_base(e: &Expr) -> Option<(TrigFn, Expr)> {
    match e {
        // Power outside the application: `(fn(arg))²`.
        Expr::Pow(base, exp) if is_two(exp) => {
            let Expr::Apply(head, args) = &**base else {
                return None;
            };
            let (Expr::Sym(s), [arg]) = (&**head, args.as_slice()) else {
                return None;
            };
            Some((trig_fn(&s.name())?, arg.clone()))
        }
        // Power on the function head: `fn²(arg)`.
        Expr::Apply(head, args) => {
            let Expr::Pow(f, exp) = &**head else {
                return None;
            };
            let (Expr::Sym(s), [arg]) = (&**f, args.as_slice()) else {
                return None;
            };
            if !is_two(exp) {
                return None;
            }
            Some((trig_fn(&s.name())?, arg.clone()))
        }
        _ => None,
    }
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
    let (coeff, rest) = split_numeric_coeff(radicand);
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

/// Split a radicand into (numeric coefficient, remaining non-numeric factor).
/// `None` rest means the radicand is purely numeric.
fn split_numeric_coeff(r: &Expr) -> (Number, Option<Expr>) {
    match r {
        Expr::Num(n) => (n.clone(), None),
        Expr::Mul(fs) => {
            if let Some(Expr::Num(n)) = fs.first() {
                (n.clone(), Some(mul(fs[1..].to_vec())))
            } else {
                (Number::one(), Some(r.clone()))
            }
        }
        other => (Number::one(), Some(other.clone())),
    }
}

/// Largest `m` such that `m^q` divides `c`, with `r = c / m^q` the q-th-power-free
/// remainder. `c >= 1`, `q >= 2`.
fn extract_qth_power(c: u64, q: u32) -> (u64, u64) {
    let mut m: u64 = 1;
    let mut remaining = c as u128;
    let mut d: u128 = 2;
    while let Some(dq) = d.checked_pow(q) {
        if dq > remaining {
            break;
        }
        if remaining.is_multiple_of(dq) {
            m = m.saturating_mul(d as u64);
            remaining /= dq;
        } else {
            d += 1;
        }
    }
    (m, remaining as u64)
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
