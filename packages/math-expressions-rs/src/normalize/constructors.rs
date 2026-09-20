//! The `add`/`mul`/`pow` smart constructors: they assume their children are
//! already canonical and re-establish the canonical invariants (flattened,
//! sorted, exactly folded, like terms/powers combined).

use super::{cmp, identity_matrix, is_matrix_valued, is_vector_valued, matmul_literal};
use crate::expr::{Expr, Mat, MathConst};
use crate::num::Number;
use std::cell::Cell;

thread_local! {
    /// See [`without_like_term_collection`]. Ambient rather than a parameter
    /// because `add` is reached from every level of `canonicalize`'s recursion
    /// and from `mul`/`pow`; the same reasoning (and the same `Cell`/restore
    /// shape) as [`crate::resource_limits`].
    static COLLECT_LIKE_TERMS: Cell<bool> = const { Cell::new(true) };
}

/// Run `f` with like-*term* collection in [`add`] switched off, so `x + 3x`
/// stays two summands instead of becoming `4x`.
///
/// This exists for one caller: [`evaluate_numbers`], which backs DoenetML's
/// `simplify="numbers"` — specified as "fold numeric constants, leave the
/// symbolic structure alone". Collecting like terms is a correct
/// simplification but not a *numeric* one, and doing it here made
/// `simplify="numbers"` indistinguishable from `simplify="full"`.
///
/// Only sums are affected. Like *powers* in [`mul`] still combine, because the
/// legacy oracle requires it: `i·i` must fold to `−1`, which is that same
/// merge (`i^1 · i^1 → i^2 → −1`).
///
/// Terms that cancel to zero still collapse even when this is off — `3x − 3x`
/// is `0`, not two surviving summands. That is the additive-inverse identity
/// rather than a shortening rewrite, and dropping it would make
/// `evaluate_numbers` unable to see a zero it is asked about (`(3x−3x)^0`).
///
/// [`evaluate_numbers`]: crate::evaluate_numbers
pub(crate) fn without_like_term_collection<R>(f: impl FnOnce() -> R) -> R {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            COLLECT_LIKE_TERMS.with(|c| c.set(self.0));
        }
    }
    let _restore = Restore(COLLECT_LIKE_TERMS.with(|c| c.replace(false)));
    f()
}

/// Build a canonical sum from canonical terms: flatten, fold the numeric part
/// exactly, combine like terms (`3x + 2x → 5x`), drop zeros, sort. Literal
/// matrices of equal dimensions fold entrywise; mismatched dimensions (and
/// matrix + scalar) stay as separate unevaluated terms.
pub(crate) fn add(terms: Vec<Expr>) -> Expr {
    let mut flat = Vec::with_capacity(terms.len());
    for t in terms {
        match t {
            Expr::Add(xs) => flat.extend(xs),
            other => flat.push(other),
        }
    }

    // Infinity arithmetic on a sum. `∞ − ∞` is NaN, and `NaN` poisons anything
    // added to it — including a free variable — so `x + ∞ − ∞` is `NaN`, not a
    // sum that keeps `x` (the infinities do not cancel). A *single* infinity is
    // handled more conservatively: it absorbs constant terms (`∞ + 3 = ∞`) but a
    // free variable of unknown magnitude blocks the fold (`x + ∞` stays written,
    // like `x·∞`). This is purely additive (no zero factor), so the documented
    // `0/0` / `0·∞` annihilation divergences in `mul` are untouched.
    {
        let mut pos = false;
        let mut neg = false;
        let mut nan = false;
        let mut symbolic = false;
        for t in &flat {
            match classify_infinity(t) {
                Some(1) => pos = true,
                Some(-1) => neg = true,
                Some(0) => nan = true,
                _ => symbolic |= !is_finite_term(t),
            }
        }
        if nan || (pos && neg) {
            return Expr::Const(MathConst::NaN);
        }
        if (pos || neg) && !symbolic {
            return Expr::Const(if pos {
                MathConst::Inf
            } else {
                MathConst::NegInf
            });
        }
    }

    let mut constant = Number::zero();
    // (rest, summed coefficient, the coefficients as written) for each distinct
    // non-constant term. The third field only matters under
    // `without_like_term_collection`, which re-emits the summands separately
    // unless they cancel — see there for why the sum is still tracked.
    let mut parts: Vec<(Expr, Number, Vec<Number>)> = Vec::new();
    // Entrywise accumulation per matrix dimension: (rows, cols, per-entry terms).
    let mut mats: Vec<(u32, u32, Vec<Vec<Expr>>)> = Vec::new();
    for t in flat {
        if let Expr::Matrix(m) = t {
            let (rows, cols) = (m.rows(), m.cols());
            let entries = m.into_entries();
            match mats.iter_mut().find(|(r, c, _)| *r == rows && *c == cols) {
                Some((_, _, acc)) => {
                    for (slot, e) in acc.iter_mut().zip(entries) {
                        slot.push(e);
                    }
                }
                None => mats.push((rows, cols, entries.into_iter().map(|e| vec![e]).collect())),
            }
            continue;
        }
        let (coeff, rest) = split_coeff(t);
        match rest {
            None => constant = constant.add(&coeff),
            // A term carrying an independent ± must never merge with another:
            // `±x + ±x` has value set {2x, 0, −2x} whereas `2·±x` has {2x, −2x},
            // so coalescing like terms would tie the two sign choices together.
            // Keep every pm-bearing term as its own summand (JS `noPmBase`).
            Some(r) if crate::ops::pm::contains_pm(&r) => {
                parts.push((r, coeff.clone(), vec![coeff]))
            }
            Some(r) => match parts.iter_mut().find(|(k, _, _)| *k == r) {
                Some(slot) => {
                    slot.1 = slot.1.add(&coeff);
                    slot.2.push(coeff);
                }
                None => parts.push((r, coeff.clone(), vec![coeff])),
            },
        }
    }

    // `∞ − ∞` written as two like terms. Collecting `c₁·u + c₂·u` into
    // `(c₁+c₂)·u` is the additive-inverse identity, and that does not hold for
    // an infinite `u`: every one of `1/0 − 1/0`, `x/0 − x/0`, `1/0 + 2 − 1/0`
    // and `2/0 − 1/0` is `NaN`, but collection answered `0`, `0`, `2` and `∞`.
    // The pre-pass above cannot catch these because it only recognizes an
    // infinity *constant*, and a pole is still written `Pow(0, −1)` here —
    // `pow` deliberately leaves it unfolded (see [`is_pole`]) and `add` runs
    // during canonicalization, before `simplify`'s `rule_infnan` would fold it.
    //
    // Coefficients that all pull the same way are still fine, since `c·∞ = ∞`
    // for positive `c`: `1/0 + 1/0` stays `∞` and `x/0 + x/0` stays `2x/0`.
    for (rest, _, coeffs) in &parts {
        if coeffs.len() > 1
            && has_nonfinite_factor(rest)
            && coeffs.iter().any(Number::is_negative)
            && !coeffs.iter().all(Number::is_negative)
        {
            return Expr::Const(MathConst::NaN);
        }
    }

    let mut out = Vec::with_capacity(parts.len() + mats.len() + 1);
    for (rows, cols, acc) in mats {
        // `acc` carries exactly one accumulator per entry of a rows×cols
        // matrix (it was seeded from one and only ever `zip`ped), so this
        // always constructs.
        if let Some(m) = Mat::new(rows, cols, acc.into_iter().map(add).collect()) {
            out.push(Expr::Matrix(m));
        }
    }
    if !constant.is_zero() {
        out.push(Expr::Num(constant));
    }
    let collect = COLLECT_LIKE_TERMS.with(Cell::get);
    for (rest, total, coeffs) in parts {
        // A term whose coefficients cancel disappears either way.
        if total.is_zero() {
            continue;
        }
        if collect || coeffs.len() == 1 {
            out.push(mul(vec![Expr::Num(total), rest]));
            continue;
        }
        for c in coeffs {
            if c.is_zero() {
                continue; // `0·x²` was written but contributes nothing to read
            }
            out.push(mul(vec![Expr::Num(c), rest.clone()]));
        }
    }
    out.sort_by(cmp);
    match out.len() {
        0 => Expr::Num(Number::zero()),
        1 => out.pop().unwrap(),
        _ => Expr::Add(out),
    }
}

/// What a zero numeric coefficient collapses the product to. `0·x` is `0`, but
/// `0·∞` and `0/0` are *indeterminate* and must be `NaN` — DoenetML computes an
/// undefined slope as `0/0`, so annihilating it to `0` reports a degenerate line
/// as horizontal: a wrong number on a grading path rather than a visible
/// failure. Legacy made the same distinction, and in the same order — its
/// `try_evaluate_quotient_of_numbers` tests `is_nonzero(denom) === false` before
/// the `numer === 0 → 0` shortcut, and its product fold checks `!isFinite`
/// before annihilating.
fn annihilate(coeff: &Number, indeterminate: bool) -> Expr {
    if indeterminate {
        Expr::Const(crate::expr::MathConst::NaN)
    } else {
        // Preserve the coefficient's *sign of zero* (`Int(0)` or `−0`), so a
        // product like `(−1)·0` collapses to `−0` and a later `1/(−0)` can fold
        // to `−∞`. `−0` reads as plain `0` everywhere that does not ask.
        Expr::Num(coeff.clone())
    }
}

/// Whether a `base^exp` factor blocks the annihilation above — because it is
/// *provably* non-finite, or because it is already undefined and must poison
/// the product. Only a literal pole (`0^negative`, i.e. `1/0`), the non-finite
/// constants, and `None`/`NaN` qualify. A factor whose finiteness is merely
/// *unknown* — a bare symbol, `1/x`, a function application — does not:
/// legacy's `is_nonzero` returned a third `undefined` state there and fell
/// through to `0`, which is why `0·x` stays `0` and only the provable cases
/// become `NaN`.
///
/// The exponent matters for the infinities as much as it does for `0`: `∞^(-1)`
/// *is* `0`, so `0/∞` is a plain `0` and not the indeterminate `0·∞`. Ignoring
/// it here reported `0/∞` as `NaN` — the same wrong-number failure this guard
/// exists to prevent, in the opposite direction. `NaN`/`None` are undefined at
/// every exponent (`NaN^0` included: the product is still meaningless), so they
/// poison unconditionally.
fn is_infinite_factor(base: &Expr, exp: &Expr) -> bool {
    use crate::expr::MathConst;
    let negative_exponent = matches!(exp, Expr::Num(x) if x.is_negative());
    if matches!(base, Expr::Num(n) if n.is_zero()) && negative_exponent {
        return true;
    }
    match base {
        // `∞^negative` → 0, which annihilates like any other zero.
        Expr::Const(MathConst::Inf | MathConst::NegInf) => !negative_exponent,
        Expr::Const(MathConst::NaN | MathConst::None) => true,
        _ => false,
    }
}

/// Build a canonical product from canonical factors: flatten, fold the numeric
/// coefficient exactly, annihilate on zero, combine like powers
/// (`x² · x³ → x⁵`), drop ones, sort.
///
/// Matrix factors split the product into a commutative scalar segment
/// (everything below) and an **order-preserving matrix
/// segment**: adjacent dimension-compatible literal matrices fold via matrix
/// multiplication, a fully-folded product absorbs the scalar part into its
/// entries, and anything unfoldable (dimension mismatch, unevaluated matrix
/// powers) stays as `Mul([scalars…, matrices-in-order…])`.
pub(crate) fn mul(factors: Vec<Expr>) -> Expr {
    let mut flat = Vec::with_capacity(factors.len());
    for f in factors {
        match f {
            Expr::Mul(xs) => flat.extend(xs),
            other => flat.push(other),
        }
    }

    if flat.iter().any(is_matrix_valued) {
        // A coordinate vector joins the ordered segment rather than the scalar
        // one: `M·(e,f)` is a matrix *applied to* a vector, and distributing
        // the vector into the entries — which is what the scalar segment does
        // — produced a matrix of `a·(e,f)`. The contraction itself belongs to
        // `expand`; here the product is simply left in the order it was
        // written.
        let (scalars, matrices): (Vec<Expr>, Vec<Expr>) = flat
            .into_iter()
            .partition(|f| !is_matrix_valued(f) && !is_vector_valued(f));
        let scalar_part = mul(scalars); // no matrices: the commutative pipeline
                                        // Fold adjacent compatible literal matrices, left to right.
        let mut seq: Vec<Expr> = Vec::with_capacity(matrices.len());
        for m in matrices {
            match (seq.last(), &m) {
                (Some(Expr::Matrix(_)), Expr::Matrix(_)) => {
                    let prev = seq.pop().unwrap();
                    match matmul_literal(&prev, &m) {
                        Some(folded) => seq.push(folded),
                        None => {
                            seq.push(prev);
                            seq.push(m);
                        }
                    }
                }
                _ => seq.push(m),
            }
        }
        // Fully folded: the scalar part distributes into the entries.
        if seq.len() == 1 {
            if let Expr::Matrix(m) = &seq[0] {
                if !matches!(&scalar_part, Expr::Num(n) if n.is_one()) {
                    // Entrywise, so the shape carries over untouched.
                    return Expr::Matrix(m.map(|e| mul(vec![scalar_part.clone(), e.clone()])));
                }
                return seq.pop().unwrap();
            }
        }
        let mut out = match scalar_part {
            Expr::Num(n) if n.is_one() => Vec::new(),
            Expr::Mul(xs) => xs,
            other => vec![other],
        };
        out.extend(seq);
        return match out.len() {
            1 => out.pop().unwrap(),
            _ => Expr::Mul(out),
        };
    }

    // pm scaling: c · ±x → ±(c·x) when exactly one factor is a ± and every
    // other factor carries no ± of its own. A single value `c` scales across
    // the sign choice (the value set {cx, −cx} is unchanged); more than one ±
    // is left alone, since their signs are independent (JS simplify's `c · ±x`
    // rule, guarded by `c` containing no pm).
    if flat.len() > 1 {
        let pm_idx: Vec<usize> = flat
            .iter()
            .enumerate()
            .filter(|(_, f)| crate::ops::pm::is_pm(f))
            .map(|(i, _)| i)
            .collect();
        if pm_idx.len() == 1
            && flat
                .iter()
                .enumerate()
                .all(|(i, f)| i == pm_idx[0] || !crate::ops::pm::contains_pm(f))
        {
            let Expr::OtherOp(_, args) = flat.remove(pm_idx[0]) else {
                unreachable!()
            };
            let inner = args.into_iter().next().unwrap();
            flat.push(inner);
            let scaled = mul(flat);
            // If the scaled product is itself a ± (the pulled-in factor was a
            // nested ±), it already absorbs this one: ±(±y) = ±y.
            return if crate::ops::pm::is_pm(&scaled) {
                scaled
            } else {
                crate::ops::pm::make_pm(scaled)
            };
        }
    }

    let mut coeff = Number::one();
    // (base, summed exponent) for each distinct base.
    let mut parts: Vec<(Expr, Expr)> = Vec::new();
    // Coordinate vectors are kept as ordered factors, never merged into a power:
    // `(e,f)·(e,f)` is a row·column dot, not `(e,f)²` — the same reason matrices
    // are excluded from this pass (they contract via `matmul`, above). The
    // contraction itself belongs to `simplify`/`expand`, so here the product is
    // just left written.
    let mut vectors: Vec<Expr> = Vec::new();
    // Factors carrying a `±` are likewise kept as written. Each `±` is an
    // *independent* sign choice, so `(±x)·(±x)` ranges over {x², −x²} while
    // `(±x)²` ranges over {x²} only — merging equal bases into a power would
    // silently drop half the value set. A product with exactly one top-level
    // `±` never reaches this loop — the scaling rule above has already pulled
    // that one sign out front, which is sound precisely because there is no
    // second sign for it to interact with.
    let mut plus_minus: Vec<Expr> = Vec::new();
    // The summed argument of the `exp`-spelled factors, which combine exactly as
    // the `e^u` they spell: `exp(3)·exp(5) → exp(8)`. They get their own
    // accumulator instead of joining `parts` under base `e` so that the author's
    // spelling survives the round trip, and so a product mixing the two
    // spellings leaves each alone rather than rewriting one into the other.
    let mut exp_arg: Option<Expr> = None;
    for f in flat {
        if let Expr::Num(n) = &f {
            coeff = coeff.mul(n);
            continue;
        }
        if is_vector_valued(&f) {
            vectors.push(f);
            continue;
        }
        if crate::ops::pm::contains_pm(&f) {
            plus_minus.push(f);
            continue;
        }
        if let Some(u) = exp_call_arg(&f) {
            let u = u.clone();
            exp_arg = Some(match exp_arg.take() {
                Some(prev) => add(vec![prev, u]),
                None => u,
            });
            continue;
        }
        let (base, exp) = split_pow(f);
        match parts.iter_mut().find(|(b, _)| *b == base) {
            Some(slot) => slot.1 = add(vec![std::mem::replace(&mut slot.1, Expr::Blank), exp]),
            None => parts.push((base, exp)),
        }
    }

    if coeff.is_zero() {
        return annihilate(&coeff, parts.iter().any(|(b, x)| is_infinite_factor(b, x)));
    }

    let mut out = Vec::with_capacity(parts.len() + vectors.len() + plus_minus.len() + 1);
    let mut refold = false;
    for (base, exp) in parts {
        match pow(base, exp) {
            // A folded power may collapse to a number (e.g. exponent 0 → 1).
            Expr::Num(n) => coeff = coeff.mul(&n),
            // A combined power may come back as a *product* (the integer
            // power-of-product rule: `(x·y)^(1/2)·(x·y)^(3/2)` combines to
            // `(x·y)^2`, which distributes to `x²·y²`). Its factors must merge
            // with the others — pushing it whole would nest a Mul inside a Mul
            // and break the flat canonical invariant.
            Expr::Mul(xs) => {
                refold = true;
                out.extend(xs);
            }
            other => out.push(other),
        }
    }
    if let Some(u) = exp_arg {
        match exp_call(u) {
            // `exp(0)` is the 1 that `pow` would have produced for `e^0`.
            Expr::Num(n) => coeff = coeff.mul(&n),
            other => out.push(other),
        }
    }
    // Vectors rejoin the factor list as-is; the product stays written until a
    // `simplify`/`expand` rule contracts it. So do the `±` factors, which
    // nothing later contracts.
    out.extend(vectors);
    out.extend(plus_minus);
    if coeff.is_zero() {
        return annihilate(
            &coeff,
            out.iter().cloned().any(|f| {
                let (b, x) = split_pow(f);
                is_infinite_factor(&b, &x)
            }),
        );
    }
    // Re-run the combining pass so distributed factors pair up with the rest
    // (e.g. an existing `x⁻²` cancels the distributed `x²`). Terminates: the
    // distribution only fires for Mul bases, and its output powers have
    // non-Mul bases, so nesting strictly decreases each round.
    if refold {
        if !coeff.is_one() {
            out.push(Expr::Num(coeff));
        }
        return mul(out);
    }
    out.sort_by(cmp);
    if out.is_empty() {
        return Expr::Num(coeff);
    }
    // The numeric coefficient sorts first (Num has the lowest rank).
    if !coeff.is_one() {
        out.insert(0, Expr::Num(coeff));
    }
    if out.len() == 1 {
        out.pop().unwrap()
    } else {
        Expr::Mul(out)
    }
}

/// Build a canonical power, applying the identities and constant folding that
/// hold without assumptions. `0` to a negative power is left unfolded (an
/// exact division by zero).
/// See through the wrappers that multiply by a nonzero constant: negation and
/// a scaling unit. A value behind them is zero, infinite or `NaN` exactly when
/// what they wrap is.
///
/// `equals` runs `desugar_units` before it evaluates, so it reads `(0 deg)^0`
/// as `0^0`; a predicate here that matched only a bare `Num(0)` let `simplify`
/// fold that to `1` while `equals` called the same tree `NaN`, making
/// `equals(full_simplify(e), e)` false — the invariant `normalize/full.rs`
/// documents as holding by construction. Negation is peeled for the same
/// reason one step further out: `-1/0` is `Neg(Pow(0, -1))`, so `(-1/0)^0`
/// would otherwise fold to `1` beside `(-Infinity)^0 → NaN`.
///
/// Only the units `desugar_units` actually rewrites are peeled, so `circ` (a
/// unit symbol with no scaling rule) is left alone here for the same reason it
/// is left alone there.
fn peel_nonzero_scaling(e: &Expr) -> &Expr {
    let mut cur = e;
    loop {
        match cur {
            Expr::Neg(inner) => cur = inner,
            Expr::OtherOp(name, args) if name.name() == "unit" => {
                match crate::normalize::units::desugarable_unit_body(args) {
                    Some(inner) => cur = inner,
                    None => return cur,
                }
            }
            _ => return cur,
        }
    }
}

/// A term of known-finite magnitude, which an `∞` in the same sum absorbs.
/// Numbers, and the named constants in either spelling *while declared* — an
/// undeclared `pi` is a free variable and blocks the fold like any other.
///
/// This must stay the same notion `simplify`'s `rule_infnan` uses
/// (`is_infnan_constant`), or `canonicalize` and `simplify` disagree about
/// `π + ∞`: the one that never folded would leave a sum the other reduced,
/// and the two results would not compare equal.
fn is_finite_term(e: &Expr) -> bool {
    match e {
        Expr::Num(_) => true,
        Expr::Neg(b) => is_finite_term(b),
        _ => {
            crate::constant_policy::is_pi(e)
                || crate::constant_policy::is_e(e)
                || crate::constant_policy::is_i(e)
        }
    }
}

/// Classify a term as `+∞` (`Some(1)`), `−∞` (`Some(-1)`), `NaN` (`Some(0)`), or
/// finite/unknown (`None`), seeing through a leading negation so both `−∞` and
/// `Neg(∞)` are recognized. Used by `add`'s infinity fold.
fn classify_infinity(e: &Expr) -> Option<i8> {
    match e {
        Expr::Const(MathConst::Inf) => Some(1),
        Expr::Const(MathConst::NegInf) => Some(-1),
        Expr::Const(MathConst::NaN) => Some(0),
        Expr::Neg(b) => classify_infinity(b).map(|s| if s == 0 { 0 } else { -s }),
        _ => None,
    }
}

/// Is `e` one of the non-finite constants (`±∞`, `NaN`)?
fn is_nonfinite_const(e: &Expr) -> bool {
    matches!(
        peel_nonzero_scaling(e),
        Expr::Const(MathConst::Inf) | Expr::Const(MathConst::NegInf) | Expr::Const(MathConst::NaN)
    )
}

/// An unfolded *literal pole*: `1/0` is `Pow(0, -1)` and `1/∞` is
/// `Pow(∞, -1)`, which are `∞` and `0` respectively.
///
/// `pow`'s fast paths run before those fold, so matching only the folded
/// spellings made the rules non-confluent: `Infinity^0` gave `NaN` while
/// `(1/0)^0` — the same value, written the way a student writes it — gave `1`.
fn is_pole(e: &Expr) -> bool {
    let Expr::Pow(base, exp) = peel_nonzero_scaling(e) else {
        return false;
    };
    if !matches!(exp.as_ref(), Expr::Num(n) if n.is_negative()) {
        return false;
    }
    let base = peel_nonzero_scaling(base);
    is_nonfinite_const(base) || matches!(base, Expr::Num(n) if n.is_zero())
}

/// Does a term with this non-constant part have an infinite *factor* — an
/// unfolded pole or a non-finite constant?
///
/// Only the top-level factors are asked, because a non-finite subexpression
/// does not make the product non-finite: `1/(1 + 1/0)` is an exact `0`, and a
/// recursive scan would refuse to cancel two of them.
fn has_nonfinite_factor(rest: &Expr) -> bool {
    let factors: &[Expr] = match rest {
        Expr::Mul(xs) => xs,
        other => std::slice::from_ref(other),
    };
    factors.iter().any(|f| is_pole(f) || is_nonfinite_const(f))
}

/// Is `e` provably infinite or `NaN`?
///
/// The exponent counterpart of [`is_indeterminate_power_base`], and
/// deliberately *narrower*: it excludes zero, because `1^0` is `1` while
/// `1^∞` is not. Using the base predicate here would have turned `1^(0 deg)`
/// into `NaN`.
fn is_nonfinite_value(e: &Expr) -> bool {
    let e = peel_nonzero_scaling(e);
    if is_nonfinite_const(e) {
        return true;
    }
    // `0^negative` is ±∞. `∞^negative` is `0`, so unlike [`is_pole`] only the
    // zero base counts here.
    if let Expr::Pow(base, exp) = e {
        if matches!(exp.as_ref(), Expr::Num(n) if n.is_negative())
            && matches!(peel_nonzero_scaling(base), Expr::Num(n) if n.is_zero())
        {
            return true;
        }
    }
    // One infinite factor makes the product infinite — or `NaN`, if another is
    // zero. Either way `1^e` must not collapse, so both answers are handled by
    // the same test.
    match e {
        Expr::Mul(factors) => factors.iter().any(is_nonfinite_value),
        Expr::Div(num, den) => {
            is_nonfinite_value(num)
                || matches!(peel_nonzero_scaling(den), Expr::Num(n) if n.is_zero())
        }
        _ => false,
    }
}

/// Bases for which `base^0` is an indeterminate form rather than 1: zero and
/// the non-finite constants, under any spelling that reaches `pow` unfolded.
///
/// `0^0` is the debatable one — combinatorics and power series take it as 1,
/// and IEEE `pow(0,0)` is 1. As a *limit* form it is indeterminate (`x^0 → 1`
/// but `0^x → 0`), which is the reading a mathematics course teaches and the
/// one this engine reports, alongside `∞ − ∞` and `0 · ∞`.
fn is_indeterminate_power_base(e: &Expr) -> bool {
    let peeled = peel_nonzero_scaling(e);
    if is_nonfinite_const(peeled)
        || matches!(peeled, Expr::Num(n) if n.is_zero())
        || is_pole(peeled)
    {
        return true;
    }
    // Products and quotients inherit it. A factor that is zero, infinite or
    // `NaN` leaves the whole expression zero, infinite or `NaN`; no other
    // operand can return it to the finite nonzero value that `base^0 → 1`
    // needs. This is the path `-1/0` actually arrives on — canonicalization
    // rewrites the negation as a `-1` *factor*, so the base reaching `pow` is
    // `Mul[-1, Pow(0, -1)]` rather than anything `peel_nonzero_scaling` can
    // unwrap, which is what kept `(-1/0)^0 → 1` beside `(-Infinity)^0 → NaN`.
    match peeled {
        Expr::Mul(factors) => factors.iter().any(is_indeterminate_power_base),
        Expr::Div(num, den) => is_indeterminate_power_base(num) || is_indeterminate_power_base(den),
        _ => false,
    }
}

pub(crate) fn pow(base: Expr, exp: Expr) -> Expr {
    // Matrix base (MATRIX_PLAN §1a): integer k ≥ 2 on a square matrix folds by
    // binary powering, k = 0 gives the identity, k = 1 the base. Everything
    // else (negative — inverse is Layer 2 —, symbolic, non-square) stays an
    // unevaluated Pow. Ordered before the scalar fast paths: `A^0` must be I,
    // not the scalar 1.
    if let Expr::Matrix(m) = &base {
        let (rows, cols) = (m.rows(), m.cols());
        match as_int(&exp) {
            Some(1) => return base,
            Some(0) if rows == cols => return identity_matrix(rows),
            Some(k)
                if rows == cols
                    && k >= 2
                    && k <= crate::resource_limits::current().max_expand_power =>
            {
                let mut acc = identity_matrix(rows);
                let mut sq = base.clone();
                let mut k = k as u64;
                loop {
                    if k & 1 == 1 {
                        match matmul_literal(&acc, &sq) {
                            Some(m) => acc = m,
                            None => return Expr::Pow(Box::new(base), Box::new(exp)),
                        }
                    }
                    k >>= 1;
                    if k == 0 {
                        return acc;
                    }
                    match matmul_literal(&sq, &sq) {
                        Some(m) => sq = m,
                        None => return Expr::Pow(Box::new(base), Box::new(exp)),
                    }
                }
            }
            // Negative integer power of an *invertible rational* matrix folds
            // through the exact inverse (MATRIX_PLAN §1b); symbolic or
            // singular matrices keep the unevaluated Pow (the assumption-
            // gated inverse is `matrix::matrix_inverse`).
            Some(k) if rows == cols && k < 0 && k > i64::MIN => {
                if let Some(inv) = crate::matrix::invert_rational_literal(&base) {
                    return pow(inv, Expr::int(-k));
                }
                return Expr::Pow(Box::new(base), Box::new(exp));
            }
            _ => return Expr::Pow(Box::new(base), Box::new(exp)),
        }
    }
    if let Expr::Num(e) = &exp {
        if e.is_zero() {
            // `x^0 = 1` requires `x` finite and nonzero. `0^0`, `(±∞)^0` and
            // `NaN^0` are indeterminate forms — folding them to 1 asserts a
            // limit that does not exist. This is also the path
            // `∞/∞` arrives on: it collects to `∞^(1−1)` = `∞^0`.
            //
            // Under non-strict `pow` (`pow_strict` off, legacy's `me.math`
            // knob) that judgement is waived and *any* `base^0` folds to `1`.
            if crate::constant_policy::current().pow_strict && is_indeterminate_power_base(&base) {
                return Expr::Const(MathConst::NaN);
            }
            return Expr::Num(Number::one());
        }
        if e.is_one() {
            return base;
        }
    }
    // `i^n` for an integer `n` walks the four-cycle `1, i, −1, −i`. Without it
    // the imaginary unit is the one number in the engine that does not
    // arithmetic: `i²` stayed `i²`, so `(a+bi)(c+di)` expanded to a form still
    // carrying `i²` instead of `ac − bd + (ad + bc)i`, and a product of three
    // `i`s collected to `i³` and stopped. Negative exponents come out of
    // `rem_euclid`, which is why `1/i` is `−i` rather than an unevaluated
    // reciprocal.
    if matches!(base, Expr::Const(MathConst::I)) {
        if let Some(k) = as_int(&exp) {
            return match k.rem_euclid(4) {
                0 => Expr::Num(Number::one()),
                1 => base,
                2 => Expr::int(-1),
                _ => mul(vec![Expr::int(-1), Expr::Const(MathConst::I)]),
            };
        }
    }
    // RootOf power reduction (MATRIX_PLAN §2d): an integer exponent ≥ deg p
    // (or negative) rewrites through t^n mod p, so polynomials in an abstract
    // root always stay below deg p — `p(RootOf(p,k)) = 0` falls out of this
    // plus like-term folding.
    if matches!(base, Expr::RootOf { .. }) {
        if let Some(k) = as_int(&exp) {
            if let Some(reduced) = crate::polynomials::rootof::power_reduced(&base, k) {
                return reduced;
            }
        }
    }
    // `exp(u)^k = exp(u·k)` for integer `k` — the `exp`-spelled twin of the
    // nested-power flatten just below, carrying the same integer restriction for
    // the same reason. This is what lets `exp(3)/exp(5)` collect: its second
    // factor canonicalizes to `exp(5)^(−1)`, which has to become `exp(−5)`
    // before `mul`'s accumulator can add the two arguments.
    if let Some(u) = exp_call_arg(&base) {
        if as_int(&exp).is_some() {
            return exp_call(mul(vec![u.clone(), exp]));
        }
    }
    // Flatten a nested power when the OUTER exponent is an integer:
    // `(b^a)^k = b^(a·k)` (repeated multiplication/division), valid for any base
    // and integer `k`. Restricting to integer `k` avoids the `(x^2)^(1/2) = |x|`
    // trap. This lets e.g. `x·(x^2)^(-1)` collapse to `x^(-1)` and removable
    // singularities like `d/dx((y/x)·x)` reduce to 0. (§7d nested-Pow flatten.)
    if let Expr::Pow(inner_base, inner_exp) = &base {
        if as_int(&exp).is_some() {
            let combined = mul(vec![(**inner_exp).clone(), exp]);
            return pow((**inner_base).clone(), combined);
        }
    }
    // Distribute an integer power over a product: `(a·b)^k = a^k·b^k` (valid for
    // any factors and integer `k`). Extracts numeric coefficients (`(2x)^(-1) =
    // x^(-1)/2`) and enables cancellations like `x·(2x)^(-1) → 1/2`.
    if let Expr::Mul(factors) = &base {
        // Not valid over a non-commutative (matrix) product: (A·B)² ≠ A²·B².
        if as_int(&exp).is_some() && !factors.iter().any(is_matrix_valued) {
            return mul(factors
                .iter()
                .map(|f| pow(f.clone(), exp.clone()))
                .collect());
        }
    }
    if let Expr::Num(b) = &base {
        if b.is_one() {
            // `1^x = 1` for every finite `x`, including a free variable — but
            // `1^∞` is the classic indeterminate form (it is the shape behind
            // `(1 + 1/n)^n → e`), so an infinite or NaN exponent must not fold.
            // `is_nonfinite_value` covers the unfolded spellings of the same
            // thing — `1^(1/0)` and `1^(-1/0)`, where canonicalization has left
            // a pole, or a `-1` factor over one, in place of the constant.
            if is_nonfinite_value(&exp) {
                return Expr::Const(MathConst::NaN);
            }
            return Expr::Num(Number::one()); // 1^x = 1
        }
        // `as_int` matches only an integer exponent, the case we fold.
        if let Some(k) = as_int(&exp) {
            if let Some(v) = b.checked_pow_int(k) {
                return Expr::Num(v);
            }
            // 0^negative: fall through, stays a Pow node.
        }
    }
    Expr::Pow(Box::new(base), Box::new(exp))
}

/// A term split into (coefficient, remaining factor). `None` remainder means
/// the term is a pure number.
pub(crate) fn split_coeff(t: Expr) -> (Number, Option<Expr>) {
    match t {
        Expr::Num(n) => (n, None),
        Expr::Mul(xs) => {
            if let Some(Expr::Num(n)) = xs.first() {
                let n = n.clone();
                let rest = mul(xs[1..].to_vec());
                (n, Some(rest))
            } else {
                (Number::one(), Some(Expr::Mul(xs)))
            }
        }
        other => (Number::one(), Some(other)),
    }
}

/// The argument `u` when `f` is the `exp`-spelled power of e, `exp(u)`.
///
/// `exp(u)` and `e^u` are one value written two ways, and both layers keep
/// whichever the author wrote — `evaluate_numbers` hands `exp(8)` back as
/// `exp(8)`, not `e^8`, and the JS library does the same. So the exponent rules
/// that already apply to `e^u` are restated for this spelling (here and in
/// [`pow`]) rather than normalizing one spelling into the other, which would
/// rewrite input nobody asked to have rewritten.
pub(crate) fn exp_call_arg(f: &Expr) -> Option<&Expr> {
    if let Expr::Apply(head, args) = f {
        if let (Expr::Sym(s), [u]) = (&**head, args.as_slice()) {
            if s.name() == "exp" {
                return Some(u);
            }
        }
    }
    None
}

/// `exp(u)`, folding `exp(0)` to 1 as [`pow`] folds `b^0` — an accumulated
/// argument really can cancel to zero (`exp(t)/exp(t)`).
pub(crate) fn exp_call(u: Expr) -> Expr {
    if matches!(&u, Expr::Num(n) if n.is_zero()) {
        return Expr::Num(Number::one());
    }
    Expr::Apply(Box::new(Expr::sym("exp")), vec![u])
}

/// A factor split into (base, exponent).
fn split_pow(f: Expr) -> (Expr, Expr) {
    match f {
        Expr::Pow(b, e) => (*b, *e),
        other => (other, Expr::Num(Number::one())),
    }
}

fn as_int(n: &Expr) -> Option<i64> {
    match n {
        Expr::Num(Number::Int(i)) => Some(*i),
        _ => None,
    }
}
