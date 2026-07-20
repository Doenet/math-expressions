//! Certified numeric integration (the quadrature consumer of
//! ARBITRARY_PERCISION_PLAN P5, §7f discipline throughout).
//!
//! `integrate_to_precision(f, x, a, b, digits)` returns the definite
//! integral with a **rigorous** error bound at or below the requested
//! accuracy, or an honest `Unknown` — never a heuristic estimate:
//!
//! - node values come from the Tier-0 certified evaluator (`eval_f64`:
//!   value + proven absolute error bound per abscissa);
//! - the quadrature remainder is bounded rigorously: composite Simpson with
//!   per-segment remainder `(w⁵/2880)·sup|f⁗|`, where `sup|f⁗|` comes from a
//!   conservative **interval extension** of the symbolically-differentiated
//!   integrand evaluated over the segment (outward-widened f64 interval
//!   arithmetic — every op pads beyond its worst-case rounding);
//! - adaptive bisection splits the worst segment until the total certified
//!   error (remainder + node bounds + summation rounding) meets the target,
//!   under `max_quadrature_segments`.
//!
//! Node arithmetic is f64, which caps honest output at ~13 significant
//! digits; larger requests refuse rather than pretend.

use super::tape::{compile, CompiledExpr, Op};
use super::{kernels::FixId, kernels::REGISTRY, needed_bits, Precise};
use crate::expr::Expr;
use crate::num::Number;
use crate::precise::fix::MpFix;
use std::collections::BinaryHeap;

const EPS: f64 = f64::EPSILON;

/// A closed interval with outward-safe bounds.
#[derive(Clone, Copy, Debug)]
struct Iv {
    lo: f64,
    hi: f64,
}

impl Iv {
    fn point(v: f64) -> Iv {
        Iv { lo: v, hi: v }.widen()
    }
    fn mag(&self) -> f64 {
        self.lo.abs().max(self.hi.abs())
    }
    /// Outward padding beyond any single f64 op's rounding (≤ 2 ulp for
    /// libm, ½ ulp for arithmetic): 16 ε relative plus a subnormal floor.
    fn widen(self) -> Iv {
        let pad = 16.0 * EPS * self.mag() + f64::MIN_POSITIVE;
        Iv {
            lo: self.lo - pad,
            hi: self.hi + pad,
        }
    }
    fn ok(&self) -> bool {
        self.lo.is_finite() && self.hi.is_finite() && self.lo <= self.hi
    }
    fn contains_zero(&self) -> bool {
        self.lo <= 0.0 && self.hi >= 0.0
    }
}

fn iv_from_pair(a: f64, b: f64) -> Iv {
    Iv {
        lo: a.min(b),
        hi: a.max(b),
    }
    .widen()
}

fn monotone(x: Iv, f: impl Fn(f64) -> f64) -> Iv {
    iv_from_pair(f(x.lo), f(x.hi))
}

/// Conservative interval sweep of a compiled tape over `var ∈ x`.
/// `None` = domain edge, pole, or unbounded — the caller subdivides.
fn interval_eval(tape: &CompiledExpr, x: Iv) -> Option<Iv> {
    let mut stack: Vec<Iv> = Vec::with_capacity(tape.max_stack);
    for op in &tape.ops {
        let v: Iv = match op {
            Op::Const(i) => Iv::point(tape.consts[*i as usize].to_f64()),
            Op::Var(_) => x,
            Op::Pi => Iv::point(std::f64::consts::PI),
            Op::E => Iv::point(std::f64::consts::E),
            Op::I => return None,
            Op::Root(i) => {
                let (poly, idx) = &tape.roots[*i as usize];
                let z = crate::rootof::numeric_root(poly, *idx)?;
                if z.im != 0.0 {
                    return None;
                }
                Iv::point(z.re)
            }
            Op::Add(n) => {
                let start = stack.len() - *n as usize;
                let (mut lo, mut hi) = (0.0f64, 0.0f64);
                for t in &stack[start..] {
                    lo += t.lo;
                    hi += t.hi;
                }
                stack.truncate(start);
                Iv { lo, hi }.widen()
            }
            Op::Mul(n) => {
                let start = stack.len() - *n as usize;
                let (head, rest) = stack.split_at(start + 1);
                let mut acc = head[start];
                for &t in rest {
                    let cs = [
                        acc.lo * t.lo,
                        acc.lo * t.hi,
                        acc.hi * t.lo,
                        acc.hi * t.hi,
                    ];
                    acc = Iv {
                        lo: cs.iter().cloned().fold(f64::INFINITY, f64::min),
                        hi: cs.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                    }
                    .widen();
                }
                stack.truncate(start);
                acc
            }
            Op::PowInt(k) => {
                let b = stack.pop().unwrap();
                let k = *k;
                if k < 0 && b.contains_zero() {
                    return None;
                }
                let f = |v: f64| v.powi(k.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32);
                if k >= 0 && k % 2 == 0 {
                    let hi = f(b.lo.abs().max(b.hi.abs()));
                    let lo = if b.contains_zero() { 0.0 } else { f(b.lo.abs().min(b.hi.abs())) };
                    Iv { lo, hi }.widen()
                } else {
                    // Odd power, or negative power on a sign-definite
                    // interval: monotone on the interval.
                    monotone(b, f)
                }
            }
            Op::Pow => {
                let e = stack.pop().unwrap();
                let b = stack.pop().unwrap();
                if b.lo <= 0.0 {
                    return None;
                }
                // a^b is monotone in each argument over a positive-base box:
                // extrema at corners.
                let cs = [
                    b.lo.powf(e.lo),
                    b.lo.powf(e.hi),
                    b.hi.powf(e.lo),
                    b.hi.powf(e.hi),
                ];
                Iv {
                    lo: cs.iter().cloned().fold(f64::INFINITY, f64::min),
                    hi: cs.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                }
                .widen()
            }
            Op::Call(id) => {
                let a = stack.pop().unwrap();
                interval_call(REGISTRY[*id as usize].fix?, a)?
            }
        };
        if !v.ok() {
            return None;
        }
        stack.push(v);
    }
    stack.pop()
}

fn interval_call(fix: FixId, a: Iv) -> Option<Iv> {
    use std::f64::consts::{FRAC_PI_2, PI};
    Some(match fix {
        FixId::Sqrt => {
            if a.lo < 0.0 {
                return None;
            }
            monotone(a, f64::sqrt)
        }
        FixId::Exp => {
            if a.hi > 700.0 {
                return None;
            }
            monotone(a, f64::exp)
        }
        FixId::Ln => {
            if a.lo <= 0.0 {
                return None;
            }
            monotone(a, f64::ln)
        }
        FixId::Log10 => {
            if a.lo <= 0.0 {
                return None;
            }
            monotone(a, f64::log10)
        }
        FixId::Abs => {
            let hi = a.mag();
            let lo = if a.contains_zero() {
                0.0
            } else {
                a.lo.abs().min(a.hi.abs())
            };
            Iv { lo, hi }.widen()
        }
        FixId::Sin | FixId::Cos => {
            if a.hi - a.lo >= 2.0 * PI {
                Iv { lo: -1.0, hi: 1.0 }
            } else {
                let f: fn(f64) -> f64 = if fix == FixId::Sin { f64::sin } else { f64::cos };
                let mut lo = f(a.lo).min(f(a.hi));
                let mut hi = f(a.lo).max(f(a.hi));
                // Critical points: sin at π/2 + kπ, cos at kπ (padded k-range
                // so f64 rounding of the division can't drop one).
                let offset = if fix == FixId::Sin { FRAC_PI_2 } else { 0.0 };
                let k0 = ((a.lo - offset) / PI).floor() as i64 - 1;
                let k1 = ((a.hi - offset) / PI).ceil() as i64 + 1;
                for k in k0..=k1 {
                    let c = offset + k as f64 * PI;
                    if c >= a.lo && c <= a.hi {
                        if k.rem_euclid(2) == 0 {
                            hi = 1.0;
                        } else {
                            lo = -1.0;
                        }
                    }
                }
                Iv { lo, hi }.widen()
            }
        }
        FixId::Tan => {
            // Any pole π/2 + kπ inside the (padded) interval is a refusal.
            let pad = 4.0 * EPS * a.mag() + f64::MIN_POSITIVE;
            let k0 = ((a.lo - pad - FRAC_PI_2) / PI).floor() as i64;
            let k1 = ((a.hi + pad - FRAC_PI_2) / PI).floor() as i64;
            if k0 != k1 {
                return None;
            }
            monotone(a, f64::tan)
        }
        FixId::Asin | FixId::Acos => {
            if a.lo < -1.0 || a.hi > 1.0 {
                return None;
            }
            let f: fn(f64) -> f64 = if fix == FixId::Asin {
                f64::asin
            } else {
                f64::acos
            };
            monotone(a, f)
        }
        FixId::Atan => monotone(a, f64::atan),
        FixId::Sinh => {
            if a.mag() > 700.0 {
                return None;
            }
            monotone(a, f64::sinh)
        }
        FixId::Cosh => {
            if a.mag() > 700.0 {
                return None;
            }
            let hi = a.lo.cosh().max(a.hi.cosh());
            let lo = if a.contains_zero() {
                1.0
            } else {
                a.lo.cosh().min(a.hi.cosh())
            };
            Iv { lo, hi }.widen()
        }
        FixId::Tanh => monotone(a, f64::tanh),
    })
}

/// One adaptive segment: certified Simpson value + the two error components.
struct Seg {
    lo: f64,
    hi: f64,
    val: f64,
    /// Node evaluation + summation rounding bound (certified).
    node_err: f64,
    /// Simpson remainder bound (w⁵/2880)·sup|f⁗| (certified via intervals).
    rem_err: f64,
    /// Splits spent producing this segment (depth cap for failing regions).
    depth: u32,
}

impl Seg {
    fn err(&self) -> f64 {
        self.node_err + self.rem_err
    }
}

impl PartialEq for Seg {
    fn eq(&self, other: &Self) -> bool {
        self.err() == other.err()
    }
}
impl Eq for Seg {}
impl PartialOrd for Seg {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Seg {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.err().total_cmp(&other.err())
    }
}

fn eval_segment(f: &CompiledExpr, d4: &CompiledExpr, lo: f64, hi: f64, depth: u32) -> Option<Seg> {
    let w = hi - lo;
    let m = lo + w / 2.0;
    let (fl, el) = f.eval_f64(&[lo])?;
    let (fm, em) = f.eval_f64(&[m])?;
    let (fr, er) = f.eval_f64(&[hi])?;
    let val = w / 6.0 * (fl + 4.0 * fm + fr);
    let node_err = w / 6.0 * (el + 4.0 * em + er)
        + 8.0 * EPS * w * (fl.abs() + 4.0 * fm.abs() + fr.abs());
    let d4iv = interval_eval(d4, Iv { lo, hi })?;
    let rem_err = w.powi(5) / 2880.0 * d4iv.mag() * (1.0 + 32.0 * EPS);
    if !val.is_finite() || !node_err.is_finite() || !rem_err.is_finite() {
        return None;
    }
    Some(Seg {
        lo,
        hi,
        val,
        node_err,
        rem_err,
        depth,
    })
}

/// Definite integral of `f` in `var` over `[a, b]` to `digits` significant
/// digits, certified ("or better": the returned `Bounded` value is within
/// ±1 ulp at its scale, and that ulp is at or below the digit target).
pub fn integrate_to_precision(
    f: &Expr,
    var: &str,
    a: &Expr,
    b: &Expr,
    digits: usize,
) -> Precise {
    let digits = digits.max(1);
    if digits > 13 {
        return Precise::Unknown("quadrature is certified through f64 nodes (≤ 13 digits)");
    }
    let (Some(lo), Some(hi)) = (endpoint(a), endpoint(b)) else {
        return Precise::Unknown("endpoint not a finite constant");
    };
    if lo == hi {
        return Precise::Exact(Number::Int(0));
    }
    let (lo, hi, negate) = if lo < hi { (lo, hi, false) } else { (hi, lo, true) };

    let fc = crate::norm::simplify_core(f);
    if crate::ops::variables(&fc)
        .iter()
        .any(|v| v != var && !crate::sym::is_constant_symbol(v))
    {
        return Precise::Unknown("free variables besides the integration variable");
    }
    let Ok(tape_f) = compile(&fc) else {
        return Precise::Unknown("integrand not numerically compilable");
    };
    // Fourth symbolic derivative for the Simpson remainder.
    let mut d = fc;
    for _ in 0..4 {
        d = crate::norm::simplify_core(&crate::diff::derivative(&d, var));
    }
    let Ok(tape_d4) = compile(&d) else {
        return Precise::Unknown("fourth derivative not numerically compilable");
    };

    let max_segs = crate::limits::current().max_quadrature_segments;
    let mut heap: BinaryHeap<Seg> = BinaryHeap::new();
    let mut pending: Vec<(f64, f64, u32)> = vec![(lo, hi, 0)];
    let mut segs = 0usize;
    let (mut sum_val, mut sum_err) = (0.0f64, 0.0f64);
    let push = |seg: Seg,
                heap: &mut BinaryHeap<Seg>,
                sum_val: &mut f64,
                sum_err: &mut f64| {
        *sum_val += seg.val;
        *sum_err += seg.err();
        heap.push(seg);
    };
    loop {
        // Materialize pending intervals (splitting further on eval failure).
        while let Some((l, h, depth)) = pending.pop() {
            segs += 1;
            if segs > max_segs || depth > 60 {
                return Precise::Unknown("quadrature budget exhausted");
            }
            match eval_segment(&tape_f, &tape_d4, l, h, depth) {
                Some(seg) => push(seg, &mut heap, &mut sum_val, &mut sum_err),
                None => {
                    let m = l + (h - l) / 2.0;
                    if m <= l || m >= h {
                        return Precise::Unknown("integrand not evaluable on a minimal segment");
                    }
                    pending.push((l, m, depth + 1));
                    pending.push((m, h, depth + 1));
                }
            }
        }
        let tol = 0.5 * 10f64.powi(1 - digits as i32) * sum_val.abs().max(f64::MIN_POSITIVE);
        if sum_err <= tol {
            // The incremental tracker cancels catastrophically when huge
            // segment bounds (near-poles) are pushed and popped — a spurious
            // "done" here once produced 0 ± 10⁸ dressed as an answer. Any
            // break candidate must survive an exact re-summation; on drift,
            // resync and keep splitting.
            let (v, e) = heap
                .iter()
                .fold((0.0f64, 0.0f64), |(v, e), s| (v + s.val, e + s.err()));
            sum_val = v;
            sum_err = e;
            let tol = 0.5 * 10f64.powi(1 - digits as i32) * v.abs().max(f64::MIN_POSITIVE);
            if e <= tol {
                if v.abs() > 2.0 * e {
                    break;
                }
                // Certified small, but indistinguishable from zero at a
                // *relative* digit target.
                return Precise::Unknown(
                    "integral not resolvable from zero at the requested digits",
                );
            }
        }
        let Some(worst) = heap.pop() else {
            return Precise::Unknown("quadrature budget exhausted");
        };
        sum_val -= worst.val;
        sum_err -= worst.err();
        let m = worst.lo + (worst.hi - worst.lo) / 2.0;
        if m <= worst.lo || m >= worst.hi {
            return Precise::Unknown("quadrature stalled at f64 resolution");
        }
        pending.push((worst.lo, m, worst.depth + 1));
        pending.push((m, worst.hi, worst.depth + 1));
        segs -= 1; // the parent is replaced by its two halves
    }

    // Re-sum in order for a tight final value (compensated).
    let mut segments: Vec<Seg> = heap.into_vec();
    segments.sort_by(|x, y| x.lo.total_cmp(&y.lo));
    let (mut total, mut comp, mut err_total, mut abs_total) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for s in &segments {
        let y = s.val - comp;
        let t = total + y;
        comp = (t - total) - y;
        total = t;
        err_total += s.err();
        abs_total += s.val.abs();
    }
    err_total += 4.0 * EPS * abs_total; // compensated-summation slack
    // Belt and braces: the packaged answer must actually meet the digit
    // target — never a technically-±1-ulp but uselessly wide result.
    if err_total > 0.5 * 10f64.powi(1 - digits as i32) * total.abs().max(f64::MIN_POSITIVE) {
        return Precise::Unknown("quadrature error target not met");
    }
    let total = if negate { -total } else { total };
    if needed_bits(digits) > crate::limits::current().max_eval_precision_bits {
        return Precise::Unknown("digits beyond max_eval_precision_bits");
    }
    // ±1 ulp packaging: scale so the certified error fits under one ulp.
    let scale = err_total.max(f64::MIN_POSITIVE).log2().ceil() as i32 + 1;
    match MpFix::from_f64(total, scale) {
        Some(m) => Precise::Bounded(m),
        None => Precise::Unknown("non-finite quadrature result"),
    }
}

fn endpoint(e: &Expr) -> Option<f64> {
    match super::evaluate_to_precision(e, 17) {
        Precise::Exact(n) => {
            let v = n.to_f64();
            v.is_finite().then_some(v)
        }
        p => p.to_f64().filter(|v| v.is_finite()),
    }
}
