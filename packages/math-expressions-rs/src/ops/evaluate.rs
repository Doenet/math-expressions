//! Numeric evaluation of an expression at variable bindings, port of
//! `me.evaluate` / `me.evaluate_to_constant`.

use crate::eval_numeric::certified_digits::tape::CompiledExpr;
use crate::eval_numeric::complex::{eval_complex, Env};
use crate::expr::Expr;
use crate::normalize::simplify_core;
use crate::ops::variables;
use num_complex::Complex64;
use std::cell::RefCell;
use std::collections::HashMap;

/// Evaluate `e` at real variable bindings in `f64`, returning its (possibly
/// complex) numeric value. `None` if a needed variable is unbound, the
/// expression is not numerically meaningful, or the result is non-finite
/// (`me.evaluate` returns `null` for e.g. `1/0`). Uses the complex principal
/// branch, matching mathjs — with one deliberate exception: an odd root of a
/// negative real is the *real* root (`x^(1/3)` at `x = -8` is `-2`, not
/// mathjs's `1 + i√3`), the branch the rest of this crate commits to (see
/// `eval_numeric::complex` and `tests/odd_root_real_branch.rs`).
///
/// **None of the returned digits are certified.** This is a plain `Complex64`
/// tree walk: every operation rounds, the errors compound, and nothing here
/// tracks how far the answer has drifted — so the value can be wrong in any
/// digit, including the first. That is not hypothetical. At `x` within `1e-8`
/// of `π/2` this returns `9.007e7` for `tan(x)`, where the true value is
/// `1.000e8`; the loss comes from `Complex64::tan`'s double-angle formula, and
/// nothing in this path can notice it happening. Cancellation, poles, and large
/// trigonometric arguments are the usual triggers.
///
/// Use it where a wrong last digit is cheaper than a slow answer — plotting,
/// bracketing, sampling — and *not* where the digits are the product. The
/// alternatives, in order of increasing trust:
///
/// - [`evaluate_many`] for sampling one variable: same precision class, ~4×
///   faster per point, and better-conditioned kernels on several functions.
/// - [`crate::evaluate_to_precision`] when the digits matter: it returns as
///   many *certified* significant digits as asked or says it cannot, at 8–75×
///   the cost, and refuses free variables.
///
/// Evaluation runs on the *canonical* tree. That is a rounding decision, not a
/// semantic one — `canonicalize` rewrites `Div`/`Neg` and flattens `Add`/`Mul`
/// without folding anything in the real domain, so the branch contract above is
/// untouched — and it exists so this and [`evaluate_many`] associate their
/// arithmetic identically. Sampling the same curve through the two entry points
/// used to give answers a few ulp apart; on everything but the handful of
/// functions noted in [`with_canonical`] it now gives the same bits.
pub fn evaluate_fast_f64(e: &Expr, bindings: &HashMap<String, f64>) -> Option<Complex64> {
    let env: Env = bindings
        .iter()
        .map(|(k, v)| (k.clone(), Complex64::new(*v, 0.0)))
        .collect();
    with_canonical(e, |canon| finite(eval_complex(canon, &env)?))
}

/// Run `f` against the canonical form of `e`, reusing a recent result.
///
/// Canonicalizing costs far more than the evaluation it precedes — measured
/// 2.7 µs against 0.4 µs on `x²−3x+1`, and 14 µs against 2 µs on a compound
/// transcendental — so paying it per call made [`evaluate_fast_f64`] ~10× slower. The
/// caller that cares is a sampler asking for the *same* expression at point
/// after point, so memoizing turns "once per point" back into "once per curve".
/// Capacity is shared with [`SAMPLER_CACHE_ENTRIES`] and justified there: the
/// working set is the expressions a caller alternates between, which is rarely
/// one and rarely more than a few.
///
/// This buys agreement on tree *shape* only. Where the two paths call different
/// kernels for the same function they still differ: the tape evaluates in `f64`
/// while [`eval_complex`] dispatches through `Complex64`, and on the real axis
/// those are not always the same algorithm. `Complex64::tan` is the double-angle
/// form `sin(2x)/(1 + cos 2x)`, whose denominator cancels, against the tape's
/// `f64::tan`; `e^x` goes through `powc`, i.e. `(ln(e)·x).exp()`, which folds to
/// `exp(x)` because `ln(e)` is exactly `1.0`, against the tape's generic
/// `powf(e, x)`.
/// Measured over a 33-expression corpus, canonicalizing here lifted
/// bit-identical agreement from 78 % to 92 %; the remainder is `tan`, `e^x`,
/// `x^x` and non-integer powers, and closing it means aligning the kernels
/// rather than the trees. `tan` is not a tie-break — see
/// `evaluate_and_batch_still_differ_on_unaligned_kernels`, where this path is
/// the inaccurate one.
fn with_canonical<R>(e: &Expr, f: impl FnOnce(&Expr) -> R) -> R {
    thread_local! {
        /// Raw tree to its canonical form, most-recently-used last.
        static CACHE: RefCell<Vec<(Expr, Expr)>> = const { RefCell::new(Vec::new()) };
    }
    CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        match cache.iter().position(|(raw, _)| raw == e) {
            // Re-seat the hit at the back, so eviction drops the coldest entry
            // rather than whichever arrived first.
            Some(i) => {
                let entry = cache.remove(i);
                cache.push(entry);
            }
            None => {
                if cache.len() >= SAMPLER_CACHE_ENTRIES {
                    cache.remove(0);
                }
                cache.push((e.clone(), crate::normalize::canonicalize(e)));
            }
        }
        f(&cache.last().expect("inserted above").1)
    })
}

/// Evaluate `e` at many values of a single variable, in one pass.
///
/// Sampling a function — plotting it, bracketing its extrema, hunting a root —
/// asks for the same expression at thousands of points, and [`evaluate_fast_f64`] is the
/// wrong shape for that: it rebuilds the environment per point, and across the
/// wasm boundary each call also marshals the variable names. Measured on
/// `x²−3x+1`, that overhead is ~1.2µs a point against ~6ns of actual
/// arithmetic. Here it is paid once.
///
/// Other variables are left unbound; substitute them first if the expression
/// has any. A point that does not evaluate to a finite real — unbound variable,
/// pole, complex value, outside a real branch — comes back as `NaN` rather than
/// being dropped, because a sampler wants one result per point it asked about
/// and `NaN` is the gap marker its consumers already handle.
pub fn evaluate_many(e: &Expr, var: &str, values: &[f64]) -> Vec<f64> {
    // Fast path: compile once to the Tier-0 tape (the same flat program the
    // quadrature and ODE paths already sample through) and run its f64 sweep
    // per point. Measured 2–6× against the `eval_complex` tree walk below.
    //
    // The tape is a *fast path*, never a replacement: it only speaks real
    // f64 and escalates on any non-positive base of a general power, so a
    // point like `x^(1/3)` at `x = -8` falls back to `eval_complex`, which
    // owns the branch decision — the real root `-2` for an odd root of a
    // negative real, the principal value (hence `NaN` here) for even roots
    // and non-rational exponents. The fallback sees the *raw* tree, which is
    // why `odd_root_exponent` in `eval_numeric::complex` matches the
    // quotient-node exponent shape as well as the folded rational.
    let mut fast: Vec<Option<f64>> = Vec::new();
    with_sampler(e, var, |t| t.eval_f64_many(values, &mut fast));

    let mut env = Env::new();
    // The binding's key never changes, so insert it once and overwrite its
    // value each point — the loop this feature exists to speed up should not
    // allocate a fresh `String` per point.
    env.insert(var.to_string(), Complex64::new(0.0, 0.0));
    values
        .iter()
        .enumerate()
        .map(|(i, &x)| {
            if let Some(Some(v)) = fast.get(i) {
                return *v;
            }
            *env.get_mut(var).expect("inserted above") = Complex64::new(x, 0.0);
            match eval_complex(e, &env).and_then(finite) {
                // The imaginary part is compared against the real one's scale,
                // the same tolerance the single-point wasm entry point applies.
                Some(v) if v.im.abs() <= 1e-10 * v.re.abs().max(1.0) => v.re,
                _ => f64::NAN,
            }
        })
        .collect()
}

/// How many compiled sampler tapes to keep.
///
/// The working set is "expressions a caller is sampling at once", which is
/// rarely one: a scan that wants `f` and `f′` at every node alternates two, and
/// a graph refining brackets across several curves alternates more. Measured
/// round-robin over K expressions, one small batch each: at K=1 capacity does
/// not matter (7.8 vs 7.2 µs a call), but at K=2 a single slot costs 27.1 µs
/// against 4.2 µs here, and at K=8, 18.8 against 2.6 — the slot misses every
/// time and pays the full ~40 µs compile.
///
/// Eight is where that curve has flattened, and eight tapes is a trivial amount
/// of retained memory — which matters because memory retained here is
/// permanent: wasm never returns linear memory to the OS.
///
/// Past capacity the guarantee is correctness, not speed. Round-robin over more
/// than eight expressions is LRU's worst case — every entry is evicted just
/// before its next use — and measured K=12 duly falls back to one-slot numbers
/// (18.5 µs). That is the old behaviour, not a new cliff, and the answers stay
/// right; a caller sampling a dozen curves in lockstep would want a larger
/// capacity, not a different algorithm.
const SAMPLER_CACHE_ENTRIES: usize = 8;

/// Run `f` against the sampler tape for `(e, var)`, if there is one.
///
/// The tape is memoized because compiling one costs ~40 µs — two orders of
/// magnitude more than the ~0.3 µs point it accelerates — and the loop this
/// feature exists for calls back with the *same* expression over and over: a
/// bisection refines 60 levels, each a fresh `evaluate_many` of a handful of
/// midpoints. Recompiling per call made a measured 5.1× per-point win land as
/// 1.3× end-to-end.
///
/// The cache is a small **LRU**, not a single slot: one slot is enough only if
/// callers sweep one expression to completion, and alternating between two is
/// enough to miss on every call. See [`SAMPLER_CACHE_ENTRIES`] for the measured
/// cost of getting that wrong. Misses are recorded too (as `None`), so an
/// expression the tape declines is not re-canonicalized and re-rejected on
/// every batch.
fn with_sampler<R>(e: &Expr, var: &str, f: impl FnOnce(&CompiledExpr) -> R) -> Option<R> {
    thread_local! {
        /// Most-recently-used last.
        static CACHE: RefCell<Vec<(Expr, String, Option<CompiledExpr>)>> =
            const { RefCell::new(Vec::new()) };
    }
    CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        match cache.iter().position(|(ce, cv, _)| cv == var && ce == e) {
            // Re-seat the hit at the back so the eviction below drops the
            // genuinely coldest entry rather than whichever arrived first.
            Some(i) => {
                let entry = cache.remove(i);
                cache.push(entry);
            }
            None => {
                if cache.len() >= SAMPLER_CACHE_ENTRIES {
                    cache.remove(0);
                }
                cache.push((e.clone(), var.to_string(), compile_sampler(e, var)));
            }
        }
        cache.last().and_then(|(_, _, tape)| tape.as_ref()).map(f)
    })
}

/// Compile `e` to a Tier-0 tape for sampling in `var`, or `None` when the tape
/// cannot stand in for [`eval_complex`] and the slow path must carry the batch.
///
/// Two gates, each one a way the tape and the complex walk would otherwise
/// disagree — and disagreement here is a behaviour change reaching JS callers,
/// so both cost the fast path rather than the contract.
///
/// (There used to be a third, for alias spellings: the tape evaluates `ln`
/// while `eval_complex` only knew the canonical `log`. That gap is now closed
/// in `eval_complex` itself, so both paths agree on `ln` and neither needs to
/// decline it.)
///
/// 1. **Canonical shape.** The tape wants no `Div`/`Neg` and flat `Add`/`Mul`
///    (see `tape::compile`), which a raw parse tree is not. `canonicalize` is
///    the pass that gets it there — deliberately *not* [`simplify_core`],
///    which additionally folds and rewrites constants, changing how the
///    arithmetic associates and so which bits come back; the parity contract
///    is with [`evaluate_fast_f64`], which walks the merely-canonical tree.
///
/// 2. **Positional bindings.** The tape's slots are positional, so the fast
///    path is only sound when its one slot *is* the variable being swept. An
///    expression in some other free variable must reach `eval_complex`, which
///    reports the unbound variable as `NaN` per point.
fn compile_sampler(e: &Expr, var: &str) -> Option<CompiledExpr> {
    let canon = crate::normalize::canonicalize(e);
    let tape = crate::eval_numeric::certified_digits::compile(&canon).ok()?;
    match tape.vars() {
        [] => Some(tape),
        [only] if only == var => Some(tape),
        _ => None,
    }
}

/// Evaluate a closed expression to its numeric constant, or `None`. Matches
/// `me.evaluate_to_constant`: `None` if the *original* expression mentions any
/// genuine free variable (the constants `pi`/`e`/`i` don't count, and it does
/// NOT cancel first — so `x − x` is `None`, not `0`); otherwise simplify
/// (real-domain reductions apply, `(-8)^(1/3)` → `-2`) and evaluate.
///
/// `±∞` is a value here, not a failure: an unbounded interval endpoint is
/// ordinary (`[-∞, ∞]` is the default domain of a function curve), and
/// reporting `None` for one reads downstream as `0`, silently turning an
/// unbounded endpoint into a bounded one (DOENET_INTEGRATION item 1). Anything
/// still undecided — a `NaN`, an unevaluable head — remains `None`.
pub fn evaluate_to_constant(e: &Expr) -> Option<Complex64> {
    if variables(e).iter().any(|v| {
        !crate::expr::sym::is_constant_symbol(v) && crate::expr::sym::mathjs_constant(v).is_none()
    }) {
        return None;
    }
    // A hole (`＿`) or a `{"$":"None"}` leaf makes the value undefined, exactly
    // like a free variable does — you cannot evaluate an expression with an
    // unfilled slot to a constant. This must be checked on the *original* tree,
    // before `simplify_core`, because simplification absorbs the hole and hides
    // it: `0·＿` folds to `0` and `＿/＿` to `1`, so an undefined line's slope
    // would come back `0`/`1` instead of undefined (DoenetML issue #83, item 3b).
    if has_undefined_leaf(e) {
        return None;
    }
    let simplified = simplify_core(e);
    if let Some(v) = signed_infinity(&simplified) {
        return Some(Complex64::new(v, 0.0));
    }
    if is_nan_constant(&simplified) {
        return Some(Complex64::new(f64::NAN, 0.0));
    }
    // Certified digits first. This is the one evaluation entry point that can
    // afford them: the expression is closed, the call is a one-shot readout
    // rather than a per-point sampler, and [`certified_constant`] escalates
    // precision only where `f64` cannot carry the answer — see its note for the
    // measured cost. Falls through on `Unknown`, so nothing this used to answer
    // becomes `None`.
    if let Some(v) = certified_constant(&simplified) {
        return finite(v);
    }
    finite(eval_complex(&simplified, &Env::new())?)
}

/// Significant digits demanded of the certified pipeline.
///
/// Twenty, and the margin over `f64`'s ~15.95 is the whole reason. Certifying
/// *d* decimal digits bounds the decimal expansion, which is not the same thing
/// as resolving which `f64` the true value is nearest: a value lying close to
/// the midpoint between two `f64`s needs the certified interval narrower than
/// half an ulp before the rounding is decided. `tan(π/2 − 10⁻⁸)` is such a
/// value — it is `9.9999999999999996…e7`, a hair under `1e8` — and it converts
/// to the wrong `f64` at 15 digits (off by ~1070 ulp) and still at 17 (off by
/// ~7). At 20 it lands on `1e8`, which is correct.
///
/// That margin is what makes this safe to do at all: measured over an
/// 18-constant corpus against a 35-digit reference, every value that certifies
/// comes back as the correctly-rounded `f64`, so this can only ever *fix* a
/// value, never move one sideways. Lower settings lack that property — fifteen
/// was tried and returned `e^2` 1 ulp *worse* than the naive walk it replaced,
/// and trading an already-correct value for a differently-wrong one is churn,
/// not improvement. In a library whose corpus compares against mathjs digit for
/// digit, churn is pure risk.
///
/// The cost is a cliff rather than a slope, because Tier 0 (`f64` carrying
/// certified error bounds) is capped at 15 digits by construction and anything
/// past it lands in `MpFix` and the Ziv loop. It is affordable *here*, and only
/// here. Measured against the uncertified walk: `2+3`, `sqrt(2)` and `pi+e`
/// unchanged (0.4/3.0/4.4 µs — [`simplify_core`] already dominates this path),
/// `sin(10^20)` 27.1 → 31.1 µs, and the worst case, a compound transcendental,
/// 9.5 → 46.6 µs. These are one-shot readouts of a closed expression, not
/// per-point sampling, which is why [`evaluate_fast_f64`] must not do this and
/// this may.
const CONSTANT_DIGITS: usize = 20;

/// The certified value of a closed, already-simplified expression, or `None`
/// when the pipeline declines and the caller must fall back.
///
/// `None` is routine, not exceptional. Certified digits are *relative*, so a
/// value that is exactly zero has none to certify and the Ziv loop exhausts
/// `max_ziv_rounds` trying — `sin(0)` reports
/// `Unknown("did not stabilize within max_ziv_rounds")`. Poles decline too.
/// Every such case falls back to the `eval_complex` walk below, which is why
/// adding this can only improve an answer, never remove one.
fn certified_constant(simplified: &Expr) -> Option<Complex64> {
    let (re, im) = crate::evaluate_to_precision(simplified, CONSTANT_DIGITS).to_complex_f64()?;
    Some(Complex64::new(re, im))
}

/// `±∞` when `e` *is* an infinite constant, after simplification has done the
/// infinity arithmetic (`∞ + 1`, `2·∞`, `1/∞`) symbolically.
///
/// Read here rather than in [`eval_complex`] on purpose. That evaluator is the
/// equality sampler's, where a sampled `∞` would poison a difference into
/// `NaN` and turn "cannot decide" into a confident wrong answer; it declines
/// the infinite constants for that reason and keeps doing so. The constant
/// itself is not ambiguous, and this function's contract is the *value*.
fn signed_infinity(e: &Expr) -> Option<f64> {
    match e {
        Expr::Const(crate::expr::MathConst::Inf) => Some(f64::INFINITY),
        Expr::Const(crate::expr::MathConst::NegInf) => Some(f64::NEG_INFINITY),
        Expr::Neg(x) => signed_infinity(x).map(|v| -v),
        _ => None,
    }
}

/// Whether simplification *proved* the value is NaN (`0/0`, `∞ − ∞`).
///
/// This is a value, not a failure to decide, and is reported as one for the
/// same reason [`signed_infinity`] is: `None` crosses to JS as `null`, and
/// `Math.abs(null)` is `0`, so an undefined intercept would read as the origin
/// — a real point — instead of as no value at all.
///
/// Read from the *simplified tree*, deliberately, and not by relaxing
/// [`finite`]. A NaN that simplification derived is a conclusion; a NaN that
/// falls out of [`eval_complex`] may only mean the sampler could not evaluate
/// there, and returning that as a value would turn "cannot decide" into a
/// confident wrong answer. The undecidable cases — free variables, and the
/// holes rejected by [`has_undefined_leaf`] — still return `None`.
fn is_nan_constant(e: &Expr) -> bool {
    match e {
        Expr::Const(crate::expr::MathConst::NaN) => true,
        Expr::Neg(x) => is_nan_constant(x),
        _ => false,
    }
}

/// Whether `e` contains a leaf that stands for "no value here" — a blank `＿` or
/// the `{"$":"None"}` special. Such a leaf poisons any constant it is part of.
fn has_undefined_leaf(e: &Expr) -> bool {
    e.any_subexpr(&|x| matches!(x, Expr::Blank | Expr::Const(crate::expr::MathConst::None)))
}

fn finite(v: Complex64) -> Option<Complex64> {
    (v.re.is_finite() && v.im.is_finite()).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TextToAst;
    fn p(s: &str) -> Expr {
        TextToAst::new(Default::default()).convert(s).unwrap()
    }

    /// An expression with an unfilled hole evaluates to undefined, not a number
    /// simplification happened to produce (item 3b: a no-argument line's slope
    /// was `0`/`1` instead of NaN).
    #[test]
    fn undefined_leaves_do_not_evaluate_to_a_constant() {
        assert_eq!(evaluate_to_constant(&p("_")), None);
        assert_eq!(evaluate_to_constant(&p("0*_")), None); // was Some(0)
        assert_eq!(evaluate_to_constant(&p("(_-_)/(_-_)")), None); // was 1
        assert_eq!(evaluate_to_constant(&p("_+3")), None);
    }

    /// The guard is specific to holes — ordinary constants still evaluate.
    #[test]
    fn ordinary_constants_still_evaluate() {
        assert_eq!(evaluate_to_constant(&p("2+3")).unwrap().re, 5.0);
        assert_eq!(evaluate_to_constant(&p("cos(0)")).unwrap().re, 1.0);
    }

    /// A closed constant comes back correctly rounded, even where evaluating it
    /// in `f64` destroys the answer — that is what [`CONSTANT_DIGITS`] buys.
    ///
    /// Both cases are catastrophic cancellation, and the uncertified walk that
    /// [`evaluate_fast_f64`] still uses gets them wrong in the *first* digit:
    /// `tan(π/2 − 10⁻⁸)` came back `9.0072e7` against a true `1.0e8`, and
    /// `log(1 + 10⁻¹⁵)` came back `1.1102e-15` against a true `1.0e-15`. The
    /// certified pipeline escalates past `f64` exactly where `f64` cannot carry
    /// the intermediate, so both are now the nearest `f64` to the true value.
    ///
    /// Asserted as exact bit equality against an independent 35-digit run,
    /// because "close" is what the old behaviour already achieved on one of
    /// these and the claim being pinned is stronger than that.
    #[test]
    fn a_constant_that_f64_cannot_carry_is_still_correctly_rounded() {
        for src in [
            "tan(pi/2 - 10^(-8))",
            "log(1+10^(-15))",
            "exp(pi*sqrt(163))",
        ] {
            let got = evaluate_to_constant(&p(src)).expect(src).re;
            let want = crate::evaluate_to_precision(&p(src), 35)
                .to_f64()
                .expect("reference certifies");
            assert_eq!(got, want, "{src}: got {got:e}, want {want:e}");
        }
    }

    /// Certified digits are *relative*, so a value that is exactly zero has none
    /// to certify: the Ziv loop escalates until it gives up and reports
    /// `Unknown`. Those cases must fall back rather than disappear — adding
    /// certification may not cost an answer that used to exist.
    ///
    /// The values here are the `eval_complex` walk's float noise (`sin(π)` is
    /// `1.2e-16`, not `0`), which is what these returned before and still do.
    /// Improving *that* would take a symbolic zero test, not more precision.
    #[test]
    fn a_value_that_cannot_be_certified_falls_back_rather_than_vanishing() {
        for src in ["sin(pi)", "sqrt(2)^2 - 2"] {
            let got = evaluate_to_constant(&p(src));
            assert!(got.is_some(), "{src} lost its answer to certification");
            assert!(got.unwrap().re.abs() < 1e-15, "{src} should be near zero");
        }
        // A pole declines certification too, and `±∞` stays a value.
        assert_eq!(evaluate_to_constant(&p("1/0")).unwrap().re, f64::INFINITY);
    }

    /// An infinite endpoint reports as `±∞`, not as "no value" — the caller
    /// cannot tell those apart, and `null` reads as `0` to `Math.max`
    /// (DOENET_INTEGRATION item 1).
    #[test]
    fn infinity_is_a_value_not_a_failure() {
        assert_eq!(
            evaluate_to_constant(&p("Infinity")).unwrap().re,
            f64::INFINITY
        );
        assert_eq!(
            evaluate_to_constant(&p("-Infinity")).unwrap().re,
            f64::NEG_INFINITY
        );
        // Reached through the arithmetic simplify does on infinities, too.
        assert_eq!(
            evaluate_to_constant(&p("Infinity+1")).unwrap().re,
            f64::INFINITY
        );
        assert_eq!(evaluate_to_constant(&p("1/0")).unwrap().re, f64::INFINITY);
        assert_eq!(evaluate_to_constant(&p("1/Infinity")).unwrap().re, 0.0);
    }

    /// An indeterminate form *evaluates* — to NaN — and is reported as that
    /// value, for the same reason `±∞` is: `None` crosses to JS as `null`, and
    /// `Math.abs(null)` is `0`, so an undefined result would read as a real
    /// point at the origin rather than as no value.
    #[test]
    fn indeterminate_forms_evaluate_to_nan() {
        // No text spelling for NaN — `"NaN"` parses as the product `N·a·N` —
        // so these are reached through the arithmetic, which is how DoenetML
        // reaches them too.
        for s in [
            "0/0",
            "Infinity-Infinity",
            "-Infinity+Infinity",
            "0*Infinity",
            "Infinity/Infinity",
            "0^0",
            "Infinity^0",
            "1^Infinity",
        ] {
            assert!(
                evaluate_to_constant(&p(s)).is_some_and(|c| c.re.is_nan()),
                "{s} should evaluate to NaN, got {:?}",
                evaluate_to_constant(&p(s)).map(|c| c.re)
            );
        }
    }

    /// What stays `None` is what is genuinely *undecided* rather than computed:
    /// a free variable, and the holes `has_undefined_leaf` rejects. This is the
    /// line [`is_nan_constant`] must not cross.
    #[test]
    fn undecidable_values_still_decline() {
        assert_eq!(evaluate_to_constant(&p("x")), None);
        assert_eq!(evaluate_to_constant(&p("x-x+1")), None);
    }
}
