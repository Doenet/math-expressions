//! Arbitrary-precision evaluation.
//!
//! Pipeline: canonical tree → flat tape (`tape.rs`, iterative) → Tier R
//! (exact rational — the canonicalizer already folded it) → Tier 0 (f64 with
//! certified error bounds, `tier0.rs`) → Tier 2 (`MpFix` fixed point at a
//! working precision chosen by a magnitude-informed backward planning pass,
//! escalated by a Ziv loop). Failures are values (`Precise::Unknown`), never
//! hangs or panics; every loop is operation-counted under the configured
//! resource limits.

pub mod complex;
pub mod diverge;
pub mod fix;
pub mod kernels;
pub mod quad;
pub mod tape;
pub mod tier0;

use crate::expr::Expr;
use crate::num::Number;
use fix::MpFix;
use kernels::{registry, Budget, FixId};
use num_bigint::BigInt;
use num_traits::{Signed, Zero};
use tape::{arity, CompiledExpr, Op};

pub use diverge::{integrate_analyzed, IntegralVerdict, SingularPoint};
pub use quad::integrate_to_precision;
pub use tape::{compile, CompileError};

/// How a precision readout renders its significant digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DecimalFormat {
    /// Plain decimal expansion (`"1.4142…"`; whole numbers get no point,
    /// sub-1 values a leading `"0."`). No scientific notation at any magnitude.
    #[default]
    Plain,
    /// Normalized scientific form (`"1.4142…e0"`).
    Scientific,
}

/// The tri-state result of a precision request.
#[derive(Clone, Debug)]
pub enum Precise {
    /// The value is exactly this rational number.
    Exact(Number),
    /// `value ± 1` ulp at the stored scale, carrying at least the requested
    /// digits.
    Bounded(MpFix),
    /// A complex value (principal branches), each component ±1 ulp.
    Complex { re: MpFix, im: MpFix },
    /// Not decidable within budget (with the reason).
    Unknown(&'static str),
}

impl Precise {
    pub fn to_f64(&self) -> Option<f64> {
        match self {
            Precise::Exact(n) => Some(n.to_f64()),
            Precise::Bounded(m) => kernels::to_f64_checked(m),
            Precise::Complex { .. } | Precise::Unknown(_) => None,
        }
    }

    /// (re, im) as f64s (real results have im = 0).
    pub fn to_complex_f64(&self) -> Option<(f64, f64)> {
        match self {
            Precise::Exact(n) => Some((n.to_f64(), 0.0)),
            Precise::Bounded(m) => kernels::to_f64_checked(m).map(|v| (v, 0.0)),
            Precise::Complex { re, im } => Some((
                kernels::to_f64_checked(re)?,
                kernels::to_f64_checked(im)?,
            )),
            Precise::Unknown(_) => None,
        }
    }

    /// First `digits` significant decimal digits (`"1.4142…e0"` form).
    ///
    /// **Display-only string** — NOT re-parseable by this crate's parsers
    /// (lowercase `e` in the exponent reads as Euler's number; the complex
    /// form is hand-assembled) and NOT notation-aware (always `.`-decimal).
    /// It is a diagnostic/precision readout, deliberately outside the
    /// printers' round-trip contract; anything user-round-trippable must go
    /// through `output::to_text`/`to_latex` on an `Expr` instead.
    pub fn to_decimal_string(&self, digits: usize) -> Option<String> {
        self.to_decimal_string_fmt(digits, DecimalFormat::Scientific)
    }

    /// Like [`Self::to_decimal_string`], but `format` selects plain decimal vs
    /// normalized scientific form. Still display-only (see the note above).
    pub fn to_decimal_string_fmt(
        &self,
        digits: usize,
        format: DecimalFormat,
    ) -> Option<String> {
        match self {
            Precise::Exact(n) => {
                // Route through MpFix so formatting is uniform.
                let bits = needed_bits(digits) + 16;
                let msb = n.magnitude_log10().unwrap_or(0) as f64 * std::f64::consts::LOG2_10;
                let scale =
                    i32::try_from((msb as i64 - i64::from(bits) - 8).min(0)).unwrap_or(i32::MIN / 2);
                Some(MpFix::from_number(n, scale)?.to_decimal_string_fmt(digits, format))
            }
            Precise::Bounded(m) => Some(m.to_decimal_string_fmt(digits, format)),
            Precise::Complex { re, im } => Some(format!(
                "{} + {} i",
                re.to_decimal_string_fmt(digits, format),
                im.to_decimal_string_fmt(digits, format)
            )),
            Precise::Unknown(_) => None,
        }
    }
}

fn needed_bits(digits: usize) -> u32 {
    (digits as f64 * std::f64::consts::LOG2_10).ceil() as u32 + 4
}

/// Evaluate a constant expression to `digits` significant decimal digits.
/// Mirrors `evaluate_to_constant`'s contract (free variables → `Unknown`;
/// simplifies first so real-domain reductions apply), but exact and
/// arbitrary-precision instead of f64.
pub fn evaluate_to_precision(e: &Expr, digits: usize) -> Precise {
    let digits = digits.max(1);
    if needed_bits(digits) > crate::resource_limits::current().max_eval_precision_bits {
        return Precise::Unknown("digits beyond max_eval_precision_bits");
    }
    // Simplify FIRST, then check for free variables: bound notation like
    // `rootof(t^3 - t - 1, 0)` only becomes a closed leaf in canonical form.
    let c = crate::norm::simplify_core(e);
    if crate::ops::variables(&c)
        .iter()
        .any(|v| !crate::sym::is_constant_symbol(v))
    {
        return Precise::Unknown("free variables");
    }
    if let Expr::Num(n) = &c {
        return Precise::Exact(n.clone());
    }
    let tape = match compile(&c) {
        Ok(t) => t,
        Err(CompileError::NotFinite) => return Precise::Unknown("non-finite constant"),
        Err(CompileError::TooLarge) => return Precise::Unknown("expression too large"),
        Err(CompileError::NotNumeric(why)) => return Precise::Unknown(why),
    };
    eval_tape(&tape, &[], digits)
}

/// Evaluate a compiled tape at bindings to `digits` significant digits.
/// Real tiers first; expressions with complex intermediates (√ of a
/// negative, `i`, principal powers) fall through to the complex tiers.
pub fn eval_tape(tape: &CompiledExpr, bindings: &[f64], digits: usize) -> Precise {
    // ---- Tier 0 (real) ----
    let mut record: Vec<f64> = Vec::new();
    let t0 = tier0::run(tape, bindings, &mut record);
    if let tier0::Tier0Outcome::Ok(a) = &t0 {
        // Success criterion: err small enough for the requested digits.
        if a.val != 0.0 && digits <= 15 {
            let tol = 0.5 * a.val.abs() * 10f64.powi(1 - digits as i32);
            if a.err <= tol {
                let scale = a.err.max(a.val.abs() * f64::EPSILON).log2().floor() as i32;
                if let Some(m) = MpFix::from_f64(a.val, scale) {
                    return Precise::Bounded(m);
                }
            }
        }
    }
    let est_msb = match &t0 {
        tier0::Tier0Outcome::Ok(a) if a.val != 0.0 => Some(a.val.abs().log2().ceil() as i64),
        _ => None,
    };
    let real = real_ziv(tape, bindings, &record, digits, est_msb);
    if !matches!(real, Precise::Unknown(_)) {
        return real;
    }
    // ---- complex fallback ----
    match complex_path(tape, bindings, digits) {
        Precise::Unknown(_) => real, // keep the (more specific) real reason
        complex => complex,
    }
}

fn real_ziv(
    tape: &CompiledExpr,
    bindings: &[f64],
    record: &[f64],
    digits: usize,
    est_msb: Option<i64>,
) -> Precise {
    let lim = crate::resource_limits::current();
    let need = needed_bits(digits) as i64;
    let mut target_scale = est_msb.unwrap_or(0) - need - 16;
    for _round in 0..lim.max_ziv_rounds {
        if -target_scale > i64::from(lim.max_eval_precision_bits) {
            return Precise::Unknown("precision budget exhausted");
        }
        let Ok(target) = i32::try_from(target_scale) else {
            return Precise::Unknown("precision budget exhausted");
        };
        match tier2_run(tape, bindings, record, target) {
            Tier2Outcome::Value(m) => {
                let bits = m.mant.bits() as i64;
                if m.mant.is_zero() || bits < need + 8 {
                    // Cancellation: actual magnitude below the estimate —
                    // refine and retry (the Ziv role, plan §5).
                    let actual_msb = m.msb().unwrap_or(target_scale);
                    target_scale = actual_msb - need - 16 - 8;
                    continue;
                }
                return Precise::Bounded(m);
            }
            Tier2Outcome::Unknown(why) => return Precise::Unknown(why),
        }
    }
    Precise::Unknown("did not stabilize within max_ziv_rounds")
}

fn complex_path(tape: &CompiledExpr, bindings: &[f64], digits: usize) -> Precise {
    use num_complex::Complex64;
    let mut crecord: Vec<Complex64> = Vec::new();
    let ct0 = tier0::run_complex(tape, bindings, &mut crecord);
    if let tier0::CTier0Outcome::Ok(a) = &ct0 {
        let mag = a.val.norm();
        if mag != 0.0 && digits <= 15 {
            let tol = 0.5 * mag * 10f64.powi(1 - digits as i32);
            if a.err <= tol {
                let scale = a.err.max(mag * f64::EPSILON).log2().floor() as i32;
                let re = MpFix::from_f64(a.val.re, scale);
                let im = MpFix::from_f64(a.val.im, scale);
                if let (Some(re), Some(im)) = (re, im) {
                    return if a.val.im.abs() <= a.err {
                        Precise::Bounded(re)
                    } else {
                        Precise::Complex { re, im }
                    };
                }
            }
        }
    }
    let est_msb = match &ct0 {
        tier0::CTier0Outcome::Ok(a) if a.val.norm() != 0.0 => {
            Some(a.val.norm().log2().ceil() as i64)
        }
        _ => None,
    };
    let lim = crate::resource_limits::current();
    let need = needed_bits(digits) as i64;
    let mut target_scale = est_msb.unwrap_or(0) - need - 16;
    for _round in 0..lim.max_ziv_rounds {
        if -target_scale > i64::from(lim.max_eval_precision_bits) {
            return Precise::Unknown("precision budget exhausted");
        }
        let Ok(target) = i32::try_from(target_scale) else {
            return Precise::Unknown("precision budget exhausted");
        };
        match tier2_run_complex(tape, bindings, &crecord, target) {
            Tier2Outcome2::Value(z) => {
                let bits = z.re.mant.bits().max(z.im.mant.bits()) as i64;
                if bits < need + 8 {
                    if z.re.mant.is_zero() && z.im.mant.is_zero() {
                        target_scale -= need + 32;
                    } else {
                        let msb = i64::from(z.re.scale)
                            + z.re.mant.bits().max(z.im.mant.bits()) as i64;
                        target_scale = msb - need - 24;
                    }
                    continue;
                }
                return if z.is_real_within_ulp() {
                    Precise::Bounded(z.re)
                } else {
                    Precise::Complex { re: z.re, im: z.im }
                };
            }
            Tier2Outcome2::Unknown(why) => return Precise::Unknown(why),
        }
    }
    Precise::Unknown("did not stabilize within max_ziv_rounds (complex)")
}

enum Tier2Outcome {
    Value(MpFix),
    Unknown(&'static str),
}

/// One Tier-2 pass: backward precision planning (magnitude-informed via the
/// Tier-0 record), then a single forward sweep over `MpFix`.
fn tier2_run(
    tape: &CompiledExpr,
    bindings: &[f64],
    record: &[f64],
    target_scale: i32,
) -> Tier2Outcome {
    let lim = crate::resource_limits::current();
    let n = tape.ops.len();

    // Magnitude (log2 |value|) per op, with kernel fallbacks where Tier 0
    // overflowed (plan §5: magnitude-blind planning is a proven dead-end).
    let mag = |i: usize| -> f64 {
        let v = record.get(i).copied().unwrap_or(f64::NAN);
        if v.is_finite() && v != 0.0 {
            v.abs().log2()
        } else {
            estimate_mag(tape, record, i)
        }
    };

    // ---- backward planning pass ----
    // Walk ops in reverse with a requirement stack (the reverse image of the
    // forward value stack), assigning each op its output scale.
    let mut plan = vec![0i32; n];
    let mut req_stack: Vec<i64> = vec![i64::from(target_scale)];
    // To pop children in the right order we need, for each op, the op index at
    // which its left-most descendant subtree begins; recover it with a forward
    // arity scan. (Despite the name, each slot holds that deepest-child *root*
    // index, not a half-open range start — `child_indices` walks from it.)
    let mut child_starts: Vec<usize> = Vec::with_capacity(n);
    {
        let mut pos_stack: Vec<usize> = Vec::new();
        for (i, op) in tape.ops.iter().enumerate() {
            let a = arity(op);
            let start = if a == 0 {
                i
            } else {
                pos_stack[pos_stack.len() - a]
            };
            child_starts.push(start);
            pos_stack.truncate(pos_stack.len() - a);
            pos_stack.push(i);
        }
    }
    let max_bits = i64::from(lim.max_eval_precision_bits);
    for i in (0..n).rev() {
        let Some(req) = req_stack.pop() else {
            return Tier2Outcome::Unknown("planner stack imbalance");
        };
        if -req + mag(i).abs().ceil() as i64 > 4 * max_bits {
            return Tier2Outcome::Unknown("planned precision exceeds budget");
        }
        plan[i] = req.clamp(i64::from(i32::MIN / 2), i64::from(i32::MAX / 2)) as i32;
        // Push child requirements (reverse order: last child popped first in
        // this reverse walk is the *rightmost*, matching the stack layout).
        match &tape.ops[i] {
            Op::Const(_) | Op::Var(_) | Op::Pi | Op::E | Op::I | Op::Root(_) => {}
            Op::Add(k) => {
                let g = i64::from(64 - (u64::from(*k) + 1).leading_zeros()) + 1;
                for _ in 0..*k {
                    req_stack.push(req - g);
                }
            }
            Op::Mul(k) => {
                let total: f64 = mag(i);
                let kids = child_indices(&child_starts, tape, i);
                let g = i64::from(64 - (u64::from(*k) + 1).leading_zeros()) + 2;
                for &c in kids.iter() {
                    let child_req = req - (total - mag(c)).ceil() as i64 - g;
                    req_stack.push(child_req);
                }
            }
            Op::PowInt(k) => {
                let c = child_starts[i];
                let child_req =
                    req - (mag(i) - mag(c)).ceil() as i64 - i64::from(bits_of(*k)) - 2;
                req_stack.push(child_req);
            }
            Op::Pow => {
                let kids = child_indices(&child_starts, tape, i);
                let (a, b) = (kids[0], kids[1]);
                // ∂/∂a = b·out/a, ∂/∂b = out·ln a.
                let da = mag(b).max(0.0) + mag(i) - mag(a);
                let db = mag(i)
                    + record
                        .get(a)
                        .map(|v| v.abs().ln().abs().max(1.0).log2())
                        .unwrap_or(1.0);
                // LIFO: base pushed first so the exponent (rightmost child,
                // reached first in this reverse walk) pops first.
                req_stack.push(req - da.ceil() as i64 - 3);
                req_stack.push(req - db.ceil() as i64 - 3);
            }
            Op::Call(id) => {
                let c = child_starts[i];
                let d = record
                    .get(c)
                    .copied()
                    .filter(|v| v.is_finite())
                    .map(|v| kernels::dlog2(*id, v))
                    .unwrap_or(0.0);
                req_stack.push(req - d.ceil() as i64 - 2);
            }
        }
    }

    // ---- forward Tier-2 sweep ----
    let mut budget = Budget {
        remaining: i64::from(lim.max_series_terms) * 8,
    };
    let mut stack: Vec<MpFix> = Vec::with_capacity(tape.max_stack);
    for (i, op) in tape.ops.iter().enumerate() {
        let s = plan[i];
        let v = match op {
            Op::Const(ci) => match MpFix::from_number(&tape.consts[*ci as usize], s) {
                Some(m) => m,
                None => return Tier2Outcome::Unknown("non-finite constant"),
            },
            Op::Var(vi) => match bindings
                .get(*vi as usize)
                .and_then(|v| MpFix::from_f64(*v, s))
            {
                Some(m) => m,
                None => return Tier2Outcome::Unknown("unbound variable"),
            },
            Op::Pi => kernels::const_pi(s),
            Op::E => kernels::const_e(s),
            Op::I => return Tier2Outcome::Unknown("imaginary unit in the real tier"),
            Op::Root(ri) => {
                let (poly, idx) = &tape.roots[*ri as usize];
                match crate::rootof::refine_real(poly, *idx, s) {
                    Some(m) => m,
                    None => {
                        return Tier2Outcome::Unknown("root not refinable in the real tier")
                    }
                }
            }
            Op::Add(k) => {
                let k = *k as usize;
                let start = stack.len() - k;
                let child_scale = stack[start..].iter().map(|m| m.scale).min().unwrap();
                let mut sum = BigInt::zero();
                for m in &stack[start..] {
                    sum += &m.at_scale(child_scale).mant;
                }
                stack.truncate(start);
                MpFix {
                    mant: sum,
                    scale: child_scale,
                }
                .rescale(s.max(child_scale))
            }
            Op::Mul(k) => {
                let k = *k as usize;
                let start = stack.len() - k;
                let mut mant = BigInt::from(1);
                let mut scale: i64 = 0;
                for m in &stack[start..] {
                    mant *= &m.mant;
                    scale += i64::from(m.scale);
                }
                stack.truncate(start);
                let Ok(scale) = i32::try_from(scale) else {
                    return Tier2Outcome::Unknown("scale overflow");
                };
                let m = MpFix { mant, scale };
                if m.scale >= s {
                    m
                } else {
                    m.rescale(s)
                }
            }
            Op::PowInt(k) => {
                let x = stack.pop().unwrap();
                match powint_fix(&x, *k, s, &mut budget) {
                    Some(m) => m,
                    None => return Tier2Outcome::Unknown("integer power failed"),
                }
            }
            Op::Pow => {
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                // a^b = exp(b·ln a), a > 0.
                let guard = s - 16;
                let Some(ln_a) = kernels::ln_fix(&a, guard, &mut budget) else {
                    return Tier2Outcome::Unknown("power base not positive");
                };
                let prod = MpFix {
                    mant: &b.mant * &ln_a.mant,
                    scale: match b.scale.checked_add(ln_a.scale) {
                        Some(v) => v,
                        None => return Tier2Outcome::Unknown("scale overflow"),
                    },
                };
                match kernels::exp_fix(&prod, s, &mut budget) {
                    Some(m) => m,
                    None => return Tier2Outcome::Unknown("exp overflow in power"),
                }
            }
            Op::Call(id) => {
                let x = stack.pop().unwrap();
                let out = match registry()[*id as usize].fix {
                    Some(FixId::Sqrt) => kernels::sqrt_fix(&x, s, &mut budget),
                    Some(FixId::Exp) => kernels::exp_fix(&x, s, &mut budget),
                    Some(FixId::Ln) => kernels::ln_fix(&x, s, &mut budget),
                    Some(FixId::Abs) => Some(MpFix {
                        mant: x.mant.abs(),
                        scale: x.scale,
                    }),
                    Some(FixId::Sin) => kernels::sin_fix(&x, s, &mut budget),
                    Some(FixId::Cos) => kernels::cos_fix(&x, s, &mut budget),
                    Some(FixId::Tan) => kernels::tan_fix(&x, s, &mut budget),
                    Some(FixId::Asin) => kernels::asin_fix(&x, s, &mut budget),
                    Some(FixId::Acos) => kernels::acos_fix(&x, s, &mut budget),
                    Some(FixId::Atan) => kernels::atan_fix(&x, s, &mut budget),
                    Some(FixId::Sinh) => kernels::sinh_fix(&x, s, &mut budget),
                    Some(FixId::Cosh) => kernels::cosh_fix(&x, s, &mut budget),
                    Some(FixId::Tanh) => kernels::tanh_fix(&x, s, &mut budget),
                    Some(FixId::Log10) => kernels::log10_fix(&x, s, &mut budget),
                    None => return Tier2Outcome::Unknown("function kernel not yet ported"),
                };
                match out {
                    Some(m) => m,
                    None => return Tier2Outcome::Unknown("kernel domain/budget failure"),
                }
            }
        };
        if budget.remaining < 0 {
            return Tier2Outcome::Unknown("series budget exhausted");
        }
        stack.push(v);
    }
    let result = stack.pop().expect("tape leaves one value");
    Tier2Outcome::Value(result.at_scale(target_scale.max(result.scale)))
}

/// Integer power on MpFix: exact `mant^k` when small enough, otherwise
/// square-and-multiply with intermediate rounding; negative k inverts first.
fn powint_fix(x: &MpFix, k: i64, s: i32, budget: &mut Budget) -> Option<MpFix> {
    if k == 0 {
        return MpFix::from_number(&Number::Int(1), s.min(0));
    }
    if k < 0 {
        if k == i64::MIN || x.mant.is_zero() {
            return None;
        }
        let pos = powint_fix(x, -k, s - 8 - 2 * bits_of(k) as i32, budget)?;
        return inv_fix(&pos, s);
    }
    // Guard the *result magnitude* before the loop. |x^k| ≈ 2^(k·msb(x)) bits,
    // so a huge positive power of a |base| > 1 would materialize a mantissa of
    // ~k·msb bits at the fixed working scale — gigabytes for inputs like
    // `2^100000000000`, i.e. an allocation failure that is a hard abort under
    // `panic = "abort"`. Refuse with `None` (→ `Unknown`), mirroring
    // `exp_fix`'s cap; the value is legitimately astronomical, not a hang.
    if let Some(m) = x.msb() {
        let cap = 8 * i64::from(crate::resource_limits::current().max_eval_precision_bits);
        if (i128::from(k) * i128::from(m)).abs() > i128::from(cap) {
            return None;
        }
    }
    let steps = bits_of(k) as i32;
    let work = s - 2 * steps - 4;
    let mut acc: Option<MpFix> = None;
    let mut sq = x.at_scale(x.scale.max(work));
    let mut kk = k as u64;
    loop {
        if !budget.tick() {
            return None;
        }
        if kk & 1 == 1 {
            acc = Some(match acc {
                None => sq.clone(),
                Some(a) => mul_round(&a, &sq, work)?,
            });
        }
        kk >>= 1;
        if kk == 0 {
            break;
        }
        sq = mul_round(&sq, &sq, work)?;
    }
    // Deliver at the requested scale when we have it; if the accumulator is
    // coarser (k = 1 pass-through of a coarse input), keep it honestly —
    // the Ziv loop inspects actual mantissa bits.
    let acc = acc.unwrap();
    Some(if acc.scale >= s { acc } else { acc.rescale(s) })
}

fn mul_round(a: &MpFix, b: &MpFix, target: i32) -> Option<MpFix> {
    let scale = i64::from(a.scale) + i64::from(b.scale);
    let m = MpFix {
        mant: &a.mant * &b.mant,
        scale: i32::try_from(scale).ok()?,
    };
    Some(if m.scale >= target { m } else { m.rescale(target) })
}

/// 1/v at scale `s` (one rounded division).
fn inv_fix(x: &MpFix, s: i32) -> Option<MpFix> {
    if x.mant.is_zero() {
        return None;
    }
    let sh = i64::from(-s) - i64::from(x.scale);
    if sh < 0 {
        return Some(MpFix::zero(s)); // |1/v| < 1 ulp at this scale
    }
    if sh > 4 * i64::from(crate::resource_limits::current().max_eval_precision_bits) {
        return None;
    }
    let num = BigInt::from(1) << u32::try_from(sh).ok()?;
    Some(MpFix {
        mant: fix::div_round(&num, &x.mant),
        scale: s,
    })
}

fn bits_of(k: i64) -> u32 {
    64 - k.unsigned_abs().leading_zeros()
}

/// The op indices of `i`'s children, left to right (from the arity scan).
fn child_indices(child_starts: &[usize], tape: &CompiledExpr, i: usize) -> Vec<usize> {
    let a = arity(&tape.ops[i]);
    let mut kids = Vec::with_capacity(a);
    // Children are the maximal subtapes ending just before i; walk backward.
    let mut end = i; // exclusive
    for _ in 0..a {
        let start = child_starts[end - 1];
        kids.push(end - 1);
        end = start;
    }
    kids.reverse();
    kids
}

/// Fallback log2-magnitude estimate when Tier 0 recorded NaN/∞ at op `i`.
fn estimate_mag(tape: &CompiledExpr, record: &[f64], i: usize) -> f64 {
    match &tape.ops[i] {
        Op::Call(id) => {
            // The argument's Tier-0 value is often still finite (e.g.
            // exp(1000)); kernel-specific estimators use it.
            let arg = (0..i)
                .rev()
                .find_map(|j| record.get(j).copied().filter(|v| v.is_finite()))
                .unwrap_or(0.0);
            kernels::result_msb_estimate(*id, arg)
        }
        _ => 0.0,
    }
}

// ---- P4: complex Tier 2 ----

enum Tier2Outcome2 {
    Value(complex::CFix),
    Unknown(&'static str),
}

/// Complex mirror of [`tier2_run`]: the same magnitude-informed backward
/// planning (|z| magnitudes, complex derivative moduli), then one forward
/// sweep over `CFix` with the composed principal-branch kernels.
fn tier2_run_complex(
    tape: &CompiledExpr,
    bindings: &[f64],
    crecord: &[num_complex::Complex64],
    target_scale: i32,
) -> Tier2Outcome2 {
    use complex::CFix;
    let lim = crate::resource_limits::current();
    let n = tape.ops.len();
    let mag = |i: usize| -> f64 {
        let v = crecord.get(i).map(|z| z.norm()).unwrap_or(f64::NAN);
        if v.is_finite() && v != 0.0 {
            v.log2()
        } else {
            0.0
        }
    };

    let mut plan = vec![0i32; n];
    let mut req_stack: Vec<i64> = vec![i64::from(target_scale)];
    let mut child_starts: Vec<usize> = Vec::with_capacity(n);
    {
        let mut pos_stack: Vec<usize> = Vec::new();
        for (i, op) in tape.ops.iter().enumerate() {
            let a = arity(op);
            let start = if a == 0 {
                i
            } else {
                pos_stack[pos_stack.len() - a]
            };
            child_starts.push(start);
            pos_stack.truncate(pos_stack.len() - a);
            pos_stack.push(i);
        }
    }
    let max_bits = i64::from(lim.max_eval_precision_bits);
    for i in (0..n).rev() {
        let Some(req) = req_stack.pop() else {
            return Tier2Outcome2::Unknown("planner stack imbalance");
        };
        if -req + mag(i).abs().ceil() as i64 > 4 * max_bits {
            return Tier2Outcome2::Unknown("planned precision exceeds budget");
        }
        plan[i] = req.clamp(i64::from(i32::MIN / 2), i64::from(i32::MAX / 2)) as i32;
        match &tape.ops[i] {
            Op::Const(_) | Op::Var(_) | Op::Pi | Op::E | Op::I | Op::Root(_) => {}
            Op::Add(k) => {
                let g = i64::from(64 - (u64::from(*k) + 1).leading_zeros()) + 1;
                for _ in 0..*k {
                    req_stack.push(req - g);
                }
            }
            Op::Mul(k) => {
                let total: f64 = mag(i);
                let kids = child_indices(&child_starts, tape, i);
                let g = i64::from(64 - (u64::from(*k) + 1).leading_zeros()) + 2;
                for &c in kids.iter() {
                    req_stack.push(req - (total - mag(c)).ceil() as i64 - g);
                }
            }
            Op::PowInt(k) => {
                let c = child_starts[i];
                req_stack
                    .push(req - (mag(i) - mag(c)).ceil() as i64 - i64::from(bits_of(*k)) - 2);
            }
            Op::Pow => {
                let kids = child_indices(&child_starts, tape, i);
                let (a, b) = (kids[0], kids[1]);
                let da = mag(b).max(0.0) + mag(i) - mag(a);
                let db = mag(i)
                    + crecord
                        .get(a)
                        .map(|z| z.norm().ln().abs().max(1.0).log2())
                        .unwrap_or(1.0);
                req_stack.push(req - da.ceil() as i64 - 3);
                req_stack.push(req - db.ceil() as i64 - 3);
            }
            Op::Call(id) => {
                let c = child_starts[i];
                let d = crecord
                    .get(c)
                    .copied()
                    .filter(|z| z.re.is_finite() && z.im.is_finite())
                    .map(|z| {
                        let d = (registry()[*id as usize].cdfm)(z);
                        if d > 0.0 && d.is_finite() {
                            d.log2()
                        } else {
                            0.0
                        }
                    })
                    .unwrap_or(0.0);
                req_stack.push(req - d.ceil() as i64 - 2);
            }
        }
    }

    let mut budget = Budget {
        remaining: i64::from(lim.max_series_terms) * 8,
    };
    let mut stack: Vec<CFix> = Vec::with_capacity(tape.max_stack);
    for (i, op) in tape.ops.iter().enumerate() {
        let s = plan[i];
        let v: Option<CFix> = match op {
            Op::Const(ci) => {
                MpFix::from_number(&tape.consts[*ci as usize], s).map(CFix::real)
            }
            Op::Var(vi) => bindings
                .get(*vi as usize)
                .and_then(|v| MpFix::from_f64(*v, s))
                .map(CFix::real),
            Op::Pi => Some(CFix::real(kernels::const_pi(s))),
            Op::E => Some(CFix::real(kernels::const_e(s))),
            Op::I => Some(CFix::i(s)),
            Op::Root(ri) => {
                let (poly, idx) = &tape.roots[*ri as usize];
                crate::rootof::refine_complex(poly, *idx, s, &mut budget)
            }
            Op::Add(k) => {
                let k = *k as usize;
                let start = stack.len() - k;
                let items: Vec<&CFix> = stack[start..].iter().collect();
                let out = complex::cadd(&items, s);
                drop(items);
                stack.truncate(start);
                Some(out)
            }
            Op::Mul(k) => {
                let k = *k as usize;
                let start = stack.len() - k;
                let work = s - 2 * (k as i32) - 2;
                let mut acc = stack[start].clone();
                let mut ok = true;
                for x in &stack[start + 1..] {
                    match complex::cmul(&acc, x, work) {
                        Some(m) => acc = m,
                        None => {
                            ok = false;
                            break;
                        }
                    }
                }
                stack.truncate(start);
                ok.then(|| acc.rescale(s))
            }
            Op::PowInt(k) => {
                let x = stack.pop().unwrap();
                complex::cpowint(&x, *k, s, &mut budget)
            }
            Op::Pow => {
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                complex::cpow(&a, &b, s, &mut budget)
            }
            Op::Call(id) => {
                let x = stack.pop().unwrap();
                match registry()[*id as usize].fix {
                    Some(FixId::Sqrt) => complex::csqrt(&x, s, &mut budget),
                    Some(FixId::Exp) => complex::cexp(&x, s, &mut budget),
                    Some(FixId::Ln) => complex::cln(&x, s, &mut budget),
                    Some(FixId::Abs) => complex::cabs(&x, s, &mut budget),
                    Some(FixId::Sin) => complex::csin(&x, s, &mut budget),
                    Some(FixId::Cos) => complex::ccos(&x, s, &mut budget),
                    Some(FixId::Tan) => complex::ctan(&x, s, &mut budget),
                    Some(FixId::Asin) => complex::casin(&x, s, &mut budget),
                    Some(FixId::Acos) => complex::cacos(&x, s, &mut budget),
                    Some(FixId::Atan) => complex::catan(&x, s, &mut budget),
                    Some(FixId::Sinh) => complex::csinh(&x, s, &mut budget),
                    Some(FixId::Cosh) => complex::ccosh(&x, s, &mut budget),
                    Some(FixId::Tanh) => complex::ctanh(&x, s, &mut budget),
                    Some(FixId::Log10) => complex::clog10(&x, s, &mut budget),
                    None => None,
                }
            }
        };
        let Some(v) = v else {
            return Tier2Outcome2::Unknown("complex kernel failure");
        };
        if budget.remaining < 0 {
            return Tier2Outcome2::Unknown("series budget exhausted");
        }
        stack.push(v);
    }
    let result = stack.pop().expect("tape leaves one value");
    Tier2Outcome2::Value(result.rescale(target_scale))
}

// ---- P5: quadrature hooks ----

impl CompiledExpr {
    /// The Tier-0 fast path alone: value and certified absolute error bound
    /// at `bindings` (slot order = [`CompiledExpr::vars`]). `None` when f64
    /// evaluation escalates (domain edge / overflow). This is the per-
    /// abscissa workhorse for quadrature: no bignum, one stack sweep.
    pub fn eval_f64(&self, bindings: &[f64]) -> Option<(f64, f64)> {
        let mut record = Vec::new();
        match tier0::run(self, bindings, &mut record) {
            tier0::Tier0Outcome::Ok(a) => Some((a.val, a.err)),
            tier0::Tier0Outcome::Escalate(_) => None,
        }
    }

    /// Evaluate a single-variable tape at many points to `digits` significant
    /// digits — the adaptive-quadrature entry point: each point pays the f64
    /// tier only, escalating to the bignum tiers per point as needed.
    pub fn eval_batch(&self, points: &[f64], digits: usize) -> Vec<Precise> {
        points
            .iter()
            .map(|&x| eval_tape(self, &[x], digits))
            .collect()
    }
}
