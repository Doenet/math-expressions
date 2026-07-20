//! Function kernels (ARBITRARY_PERCISION_PLAN §6): one registry row per
//! function, with the Tier-0 obligations (f64 value, derivative for error
//! propagation and precision planning, domain guard) and — for the P2 set —
//! an `MpFix` kernel meeting the ±1 ulp contract.
//!
//! The series kernels are ports of the `stack_computable` prototype's
//! `approximate.rs` (which carries realistic's ulp-error accounting):
//! `calc_precision = p − ⌈log2(2·terms)⌉ − 4`, truncation threshold
//! `2^(p − 4 − calc_precision)`. Argument reductions: `exp` by `ln 2`
//! (`e^x = 2^k · e^r`, an exact scale shift), `ln` by mantissa normalization
//! to [0.75, 1.5). Constants: π by Machin, `ln 2` by `2·atanh(1/3)`, `e` by
//! `exp(1)` — cached per thread at the finest scale computed so far.

use super::fix::{div_round, shift_round, MpFix};
use num_bigint::BigInt;
use num_complex::Complex64;
use num_traits::{Signed, ToPrimitive, Zero};
use std::cell::RefCell;
use std::collections::HashMap;

/// One function's precise-evaluation obligations. Rows live on
/// `FnDef::kernel` in `crate::functions` (one place per function); the
/// runtime array indexed by `Op::Call(u32)` is derived from the function
/// registry by [`registry`].
pub struct FnKernel {
    pub f: fn(f64) -> f64,
    /// Derivative (for Tier-0 error propagation and the planning pass).
    pub df: fn(f64) -> f64,
    /// Tier-0 domain guard: `false` ⇒ escalate to Tier 2 (which may also
    /// decline).
    pub domain: fn(f64) -> bool,
    /// The Tier-2 kernel, when ported (P2: sqrt/exp/ln/abs).
    pub fix: Option<FixId>,
    /// Complex principal-branch value (P4 Tier 0).
    pub cf: fn(Complex64) -> Complex64,
    /// |f′(z)| for complex error propagation and planning.
    pub cdfm: fn(Complex64) -> f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixId {
    Sqrt,
    Exp,
    Ln,
    Abs,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Log10,
}

/// The kernel rows of every registered function, in `functions::ALL`
/// order — the id space of `Op::Call(u32)`. Ids are only meaningful within
/// a run (tapes are compiled per evaluation), so registry order changes are
/// harmless.
pub fn registry() -> &'static [&'static FnKernel] {
    static R: std::sync::OnceLock<Vec<&'static FnKernel>> = std::sync::OnceLock::new();
    R.get_or_init(|| {
        crate::functions::ALL
            .iter()
            .filter_map(|d| d.kernel)
            .collect()
    })
}

/// Kernel id for a function spelling (name or alias, e.g. `arcsin`, `ln`).
pub fn lookup(name: &str) -> Option<u32> {
    static INDEX: std::sync::OnceLock<HashMap<&'static str, u32>> = std::sync::OnceLock::new();
    INDEX
        .get_or_init(|| {
            let mut m = HashMap::new();
            let mut id = 0u32;
            for def in crate::functions::ALL {
                if def.kernel.is_some() {
                    for key in std::iter::once(&def.name).chain(def.aliases) {
                        m.insert(*key, id);
                    }
                    id += 1;
                }
            }
            m
        })
        .get(name)
        .copied()
}

/// A per-run operation budget (ticked by every series iteration).
pub struct Budget {
    pub remaining: i64,
}

impl Budget {
    pub fn tick(&mut self) -> bool {
        self.remaining -= 1;
        self.remaining >= 0
    }
}

fn bound_log2(n: i64) -> i32 {
    (64 - n.unsigned_abs().max(1).leading_zeros()) as i32
}

// ---- Tier-2 kernels ----

/// √v at scale `s` (±1 ulp): take the argument at scale 2s and use the exact
/// integer square root (the prototype's Newton-free kernel).
pub fn sqrt_fix(x: &MpFix, s: i32, _budget: &mut Budget) -> Option<MpFix> {
    if x.mant.is_negative() {
        return None;
    }
    if x.mant.is_zero() {
        return Some(MpFix::zero(s));
    }
    let arg = x.at_scale(2 * s);
    if arg.mant.is_negative() {
        return None;
    }
    Some(MpFix {
        mant: arg.mant.sqrt(),
        scale: s,
    })
}

/// e^v at scale `s`: reduce by ln 2 (`e^v = 2^k·e^r`, |r| ≤ 0.35), series on
/// the remainder, exact scale shift by k.
pub fn exp_fix(x: &MpFix, s: i32, budget: &mut Budget) -> Option<MpFix> {
    let v = x.to_f64();
    if !v.is_finite() {
        return None;
    }
    let k = (v / std::f64::consts::LN_2).round();
    if k.abs() > 1e9 {
        return None; // magnitude beyond any sane precision budget
    }
    let k = k as i64;
    // Result of the series part e^r ∈ [0.7, 1.42]; compute it at scale
    // s_r = s − k (so the final exact shift by k lands on s), with guard.
    // A |k| beyond the precision budget would materialize gigantic series
    // scales — refuse (the planner guards the planned path; this guards the
    // kernel itself, e.g. when reached through the complex sweep).
    let cap = 8 * i64::from(crate::resource_limits::current().max_eval_precision_bits);
    if (i64::from(s) - k).abs() > cap || k.abs() > cap {
        return None;
    }
    let s_r = i32::try_from(i64::from(s) - k).ok()?;
    let p = s_r - 4;
    let op_prec = p - 3;
    // r = v − k·ln2 at op_prec (|r| ≤ ln2/2 < 0.35 — the series contract).
    let r_mant = if k == 0 {
        x.at_scale(op_prec).mant
    } else {
        // ln2 fine enough that |k|·(½ ulp of ln2) stays below ½ ulp of r.
        let ln2 = const_ln2(op_prec - 8 - bound_log2(k));
        let kl = BigInt::from(k) * &ln2.mant;
        let xa = x.at_scale(ln2.scale);
        shift_round(&(&xa.mant - &kl), ln2.scale - op_prec)
    };
    let series = exp_series(&r_mant, op_prec, p, budget)?;
    let out = MpFix {
        mant: series,
        scale: p,
    }
    .rescale(s_r);
    Some(MpFix {
        mant: out.mant,
        scale: i32::try_from(i64::from(out.scale) + k).ok()?,
    })
}

/// The prototype's `PrescaledExp` series: argument `op_appr·2^op_prec` with
/// |value| < 0.5; result at scale `p` (±1 ulp).
fn exp_series(op_appr: &BigInt, op_prec: i32, p: i32, budget: &mut Budget) -> Option<BigInt> {
    if p >= 1 {
        return Some(BigInt::zero());
    }
    let iterations_needed = i64::from(-p) / 2 + 2;
    let calc_precision = p - bound_log2(2 * iterations_needed) - 4;
    let scaled_1 = BigInt::from(1) << (-calc_precision) as u32;
    let max_trunc_error = BigInt::from(1) << (p - 4 - calc_precision) as u32;
    let mut current_term = scaled_1.clone();
    let mut sum = scaled_1;
    let mut n = BigInt::zero();
    while current_term.abs() > max_trunc_error {
        if !budget.tick() {
            return None;
        }
        n += 1;
        current_term = shift_round(&(&current_term * op_appr), op_prec);
        current_term = div_round(&current_term, &n);
        sum += &current_term;
    }
    Some(shift_round(&sum, calc_precision - p))
}

/// ln v at scale `s` (v > 0): normalize `v = t·2^k` with t ∈ [0.75, 1.5),
/// then `ln v = k·ln 2 + ln1p(t − 1)`.
pub fn ln_fix(x: &MpFix, s: i32, budget: &mut Budget) -> Option<MpFix> {
    if !x.mant.is_positive() {
        return None;
    }
    let msb = x.msb()?; // v ∈ [2^(msb−1), 2^msb)
    let mut k = msb - 1;
    // t = v·2^−k ∈ [1, 2); pull down to [0.75, 1.5): t ≥ 1.5 iff the top two
    // mantissa bits are `11` (magnitude-safe, unlike an f64 check).
    let bits = x.mant.bits();
    if bits >= 2 && (&x.mant >> (bits - 2)).to_i64() == Some(3) {
        k += 1;
    }
    let p = s - 4;
    let op_prec = p - 3;
    // u = t − 1 at op_prec: t = mant·2^(scale − k).
    let t = MpFix {
        mant: x.mant.clone(),
        scale: i32::try_from(i64::from(x.scale) - k).ok()?,
    };
    let one = BigInt::from(1) << (-op_prec) as u32;
    let u = &t.at_scale(op_prec).mant - &one;
    let series = ln1p_series(&u, op_prec, p, budget)?;
    let mut result = MpFix {
        mant: series,
        scale: p,
    };
    if k != 0 {
        let ln2 = const_ln2(p - 64 - bound_log2(k));
        let kln2 = MpFix {
            mant: BigInt::from(k) * &ln2.mant,
            scale: ln2.scale,
        }
        .rescale(p);
        result = MpFix {
            mant: &result.mant + &kln2.mant,
            scale: p,
        };
    }
    Some(result.rescale(s))
}

/// The prototype's `PrescaledLn` series: `ln(1+u)` for |u| < 0.5.
fn ln1p_series(u: &BigInt, op_prec: i32, p: i32, budget: &mut Budget) -> Option<BigInt> {
    if p >= 0 {
        return Some(BigInt::zero());
    }
    let iterations_needed = i64::from(-p);
    let calc_precision = p - bound_log2(2 * iterations_needed) - 4;
    let max_trunc_error = BigInt::from(1) << (p - 4 - calc_precision) as u32;
    let mut x_nth = shift_round(u, op_prec - calc_precision);
    let mut current_term = x_nth.clone();
    let mut sum = current_term.clone();
    let mut n: i64 = 1;
    let mut sign: i64 = 1;
    while current_term.abs() > max_trunc_error {
        if !budget.tick() {
            return None;
        }
        n += 1;
        sign = -sign;
        x_nth = shift_round(&(&x_nth * u), op_prec);
        current_term = div_round(&x_nth, &BigInt::from(n * sign));
        sum += &current_term;
    }
    Some(shift_round(&sum, calc_precision - p))
}

// ---- constants (thread-local high-water cache) ----

thread_local! {
    static CONSTS: RefCell<HashMap<&'static str, MpFix>> = RefCell::new(HashMap::new());
}

fn cached(name: &'static str, scale: i32, compute: impl Fn(i32) -> MpFix) -> MpFix {
    // The borrow must not span `compute` — `const_e` computes through
    // `exp_fix`, which itself asks the cache for ln 2 (re-borrow).
    let hit = CONSTS.with(|c| {
        c.borrow()
            .get(name)
            .filter(|v| v.scale <= scale)
            .cloned()
    });
    if let Some(v) = hit {
        return v.rescale(scale);
    }
    let fresh = compute(scale - 32); // slack so nearby requests hit the cache
    let out = fresh.rescale(scale);
    CONSTS.with(|c| c.borrow_mut().insert(name, fresh));
    out
}

/// π at scale `s` via Machin: `π = 16·atan(1/5) − 4·atan(1/239)`.
pub fn const_pi(s: i32) -> MpFix {
    cached("pi", s, |s| {
        let p = s - 8;
        let mut budget = Budget {
            remaining: i64::from(crate::resource_limits::current().max_series_terms),
        };
        let a5 = atan_recip(5, p, &mut budget).unwrap_or_else(BigInt::zero);
        let a239 = atan_recip(239, p, &mut budget).unwrap_or_else(BigInt::zero);
        MpFix {
            mant: a5 * 16 - a239 * 4,
            scale: p,
        }
    })
}

/// ln 2 at scale `s` via `2·atanh(1/3) = 2·Σ 1/((2j+1)·3^(2j+1))`.
pub fn const_ln2(s: i32) -> MpFix {
    cached("ln2", s, |s| {
        let p = s - 8;
        let scaled_1 = BigInt::from(1) << (-p) as u32;
        let mut power: BigInt = &scaled_1 / 3;
        let mut sum = power.clone();
        let mut j: i64 = 0;
        let mut budget = i64::from(crate::resource_limits::current().max_series_terms);
        while !power.is_zero() && budget > 0 {
            budget -= 1;
            j += 1;
            power /= 9;
            if power.is_zero() {
                break;
            }
            sum += &power / (2 * j + 1);
        }
        MpFix {
            mant: sum * 2,
            scale: p,
        }
    })
}

/// e at scale `s` (`exp(1)`).
pub fn const_e(s: i32) -> MpFix {
    cached("e", s, |s| {
        let mut budget = Budget {
            remaining: i64::from(crate::resource_limits::current().max_series_terms),
        };
        let one = MpFix {
            mant: BigInt::from(1) << 64,
            scale: -64,
        };
        exp_fix(&one, s, &mut budget).unwrap_or_else(|| MpFix::zero(s))
    })
}

/// `atan(1/n)` at scale `p` (realistic's `IntegralAtan`): integer-only
/// alternating series `Σ (−1)^j / ((2j+1)·n^(2j+1))`.
fn atan_recip(n: i64, p: i32, budget: &mut Budget) -> Option<BigInt> {
    let scaled_1 = BigInt::from(1) << (-p) as u32;
    let big_n_sq = BigInt::from(n) * n;
    let mut power: BigInt = &scaled_1 / n;
    let mut sum = power.clone();
    let mut j: i64 = 0;
    let mut sign: i64 = 1;
    while !power.is_zero() {
        if !budget.tick() {
            return None;
        }
        j += 1;
        sign = -sign;
        power = &power / &big_n_sq;
        if power.is_zero() {
            break;
        }
        sum += div_round(&power, &BigInt::from((2 * j + 1) * sign));
    }
    Some(sum)
}

/// Estimated log2 magnitude of a kernel's derivative at `x` — the planning
/// pass's transfer factor when Tier 0 recorded a usable value.
pub fn dlog2(id: u32, x: f64) -> f64 {
    let d = (registry()[id as usize].df)(x);
    if d == 0.0 || !d.is_finite() {
        0.0
    } else {
        d.abs().log2()
    }
}

/// Fallback msb estimate of `f(x)` when Tier 0 overflowed (log-domain).
pub fn result_msb_estimate(id: u32, arg: f64) -> f64 {
    match registry()[id as usize].fix {
        Some(FixId::Exp) => arg / std::f64::consts::LN_2,
        Some(FixId::Sqrt) => arg.abs().log2().max(0.0) / 2.0,
        _ => 4.0,
    }
}

pub fn to_f64_checked(x: &MpFix) -> Option<f64> {
    let v = x.to_f64();
    v.is_finite().then_some(v)
}

// ---- P3: trigonometric / inverse-trig / hyperbolic / log10 kernels ----

/// π/2 at scale `s`.
pub fn const_half_pi(s: i32) -> MpFix {
    let pi = const_pi(s - 1);
    MpFix {
        mant: pi.mant,
        scale: pi.scale - 1,
    }
    .rescale(s)
}

/// ln 10 at scale `s` (for log10).
pub fn const_ln10(s: i32) -> MpFix {
    cached("ln10", s, |s| {
        let mut budget = Budget {
            remaining: i64::from(crate::resource_limits::current().max_series_terms),
        };
        let ten = MpFix {
            mant: BigInt::from(10) << 32,
            scale: -32,
        };
        ln_fix(&ten, s, &mut budget).unwrap_or_else(|| MpFix::zero(s))
    })
}

/// Reduce a trig argument: `x = k·(π/2) + r` with `|r| ≤ π/4 + ulp`,
/// returning `r` at scale `s` and the quadrant `k mod 4`. `None` when the
/// argument's magnitude exceeds `max_trig_arg_bits` — the absolute-precision
/// contract means the planner already gave `x` enough mantissa to resolve
/// the reduction, so no Payne–Hanek machinery is needed below the cap.
fn trig_reduce(x: &MpFix, s: i32, _budget: &mut Budget) -> Option<(MpFix, i64)> {
    let msb = match x.msb() {
        None => return Some((x.at_scale(s.min(x.scale)), 0)),
        Some(m) => m,
    };
    if msb > crate::resource_limits::current().max_trig_arg_bits {
        return None;
    }
    if msb <= 0 {
        // |x| < 1 < π/2: no reduction.
        return Some((x.at_scale(s.min(x.scale)), 0));
    }
    // k = round(x / (π/2)) as a BigInt (may be huge); π/2 fine enough that
    // |k|·ulp(π/2) stays below ½ ulp of r.
    let hp_scale = s - i32::try_from(msb).ok()? - 8;
    let hp = const_half_pi(hp_scale);
    let xa = x.at_scale(hp_scale);
    let k = div_round(&xa.mant, &hp.mant);
    let r_mant = &xa.mant - &k * &hp.mant;
    let r = MpFix {
        mant: r_mant,
        scale: hp_scale,
    }
    .rescale(s);
    let four = BigInt::from(4);
    let mut q = (&k % &four).to_i64().unwrap_or(0);
    if q < 0 {
        q += 4;
    }
    Some((r, q))
}

/// The prototype's `PrescaledCos` series: cos of |r| < 1, result at `p`.
fn cos_series(op_appr: &BigInt, op_prec: i32, p: i32, budget: &mut Budget) -> Option<BigInt> {
    if p >= 1 {
        return Some(BigInt::from(1) << (1 - p).max(0) as u32 >> 1);
    }
    let iterations_needed = i64::from(-p) / 2 + 4;
    let calc_precision = p - bound_log2(2 * iterations_needed) - 4;
    let max_trunc_error = BigInt::from(1) << (p - 4 - calc_precision) as u32;
    let mut n: i64 = 0;
    let mut current_term = BigInt::from(1) << (-calc_precision) as u32;
    let mut sum = current_term.clone();
    while current_term.abs() > max_trunc_error {
        if !budget.tick() {
            return None;
        }
        n += 2;
        current_term = shift_round(&(&current_term * op_appr), op_prec);
        current_term = shift_round(&(&current_term * op_appr), op_prec);
        current_term = div_round(&current_term, &BigInt::from(-n * (n - 1)));
        sum += &current_term;
    }
    Some(shift_round(&sum, calc_precision - p))
}

/// sin of |r| < 1 (odd companion of `cos_series`), result at `p`.
fn sin_series(op_appr: &BigInt, op_prec: i32, p: i32, budget: &mut Budget) -> Option<BigInt> {
    if p >= 1 {
        return Some(BigInt::zero());
    }
    let iterations_needed = i64::from(-p) / 2 + 4;
    let calc_precision = p - bound_log2(2 * iterations_needed) - 4;
    let max_trunc_error = BigInt::from(1) << (p - 4 - calc_precision) as u32;
    let mut n: i64 = 1;
    let mut current_term = shift_round(op_appr, op_prec - calc_precision);
    let mut sum = current_term.clone();
    while current_term.abs() > max_trunc_error {
        if !budget.tick() {
            return None;
        }
        n += 2;
        current_term = shift_round(&(&current_term * op_appr), op_prec);
        current_term = shift_round(&(&current_term * op_appr), op_prec);
        current_term = div_round(&current_term, &BigInt::from(-n * (n - 1)));
        sum += &current_term;
    }
    Some(shift_round(&sum, calc_precision - p))
}

/// (sin r, cos r) of the *reduced* argument at scale `p`.
fn sin_cos_reduced(r: &MpFix, p: i32, budget: &mut Budget) -> Option<(MpFix, MpFix)> {
    let op_prec = p - 3;
    let arg = r.at_scale(op_prec.min(r.scale)).at_scale(op_prec);
    let s = sin_series(&arg.mant, op_prec, p, budget)?;
    let c = cos_series(&arg.mant, op_prec, p, budget)?;
    Some((
        MpFix { mant: s, scale: p },
        MpFix { mant: c, scale: p },
    ))
}

pub fn sin_fix(x: &MpFix, s: i32, budget: &mut Budget) -> Option<MpFix> {
    let p = s - 4;
    let (r, q) = trig_reduce(x, p - 2, budget)?;
    let (sr, cr) = sin_cos_reduced(&r, p, budget)?;
    let out = match q {
        0 => sr,
        1 => cr,
        2 => sr.neg(),
        _ => cr.neg(),
    };
    Some(out.rescale(s))
}

pub fn cos_fix(x: &MpFix, s: i32, budget: &mut Budget) -> Option<MpFix> {
    let p = s - 4;
    let (r, q) = trig_reduce(x, p - 2, budget)?;
    let (sr, cr) = sin_cos_reduced(&r, p, budget)?;
    let out = match q {
        0 => cr,
        1 => sr.neg(),
        2 => cr.neg(),
        _ => sr,
    };
    Some(out.rescale(s))
}

pub fn tan_fix(x: &MpFix, s: i32, budget: &mut Budget) -> Option<MpFix> {
    // sin/cos at a guard fine enough for the quotient's magnitude.
    let g = s - 24;
    let sn = sin_fix(x, g, budget)?;
    let cs = cos_fix(x, g, budget)?;
    div_fix(&sn, &cs, s)
}

/// a / b at scale `s` (rounded division of aligned mantissas).
pub fn div_fix(a: &MpFix, b: &MpFix, s: i32) -> Option<MpFix> {
    if b.mant.is_zero() {
        return None;
    }
    // a/b = (a.mant / b.mant) · 2^(a.scale − b.scale); deliver at s.
    let sh = i64::from(a.scale) - i64::from(b.scale) - i64::from(s);
    let max_sh = 8 * i64::from(crate::resource_limits::current().max_eval_precision_bits);
    if sh.abs() > max_sh {
        return None;
    }
    let num = if sh >= 0 {
        a.mant.clone() << u32::try_from(sh).ok()?
    } else {
        // Losing bits of a is fine: they are below b's resolution at s.
        shift_round(&a.mant, i32::try_from(sh).ok()?)
    };
    Some(MpFix {
        mant: div_round(&num, &b.mant),
        scale: s,
    })
}

/// atan by halving reductions to |x| < 1/4 then the alternating odd series;
/// |x| > 1 via `±π/2 − atan(1/x)`.
pub fn atan_fix(x: &MpFix, s: i32, budget: &mut Budget) -> Option<MpFix> {
    if x.mant.is_zero() {
        return Some(MpFix::zero(s));
    }
    let g = s - 16;
    if x.msb().unwrap_or(0) > 1 {
        // |x| ≥ 2: atan(x) = sign(x)·π/2 − atan(1/x). (|x| ∈ [1/4, 2) is
        // handled by the halving loop — recursing on 1/x at |x| = 1 would
        // never terminate.)
        let one = MpFix {
            mant: BigInt::from(1) << 32,
            scale: -32,
        };
        let recip = div_fix(&one, x, g - 4)?;
        let inner = atan_fix(&recip, g, budget)?;
        let hp = const_half_pi(g);
        let signed_hp = if x.mant.is_negative() { hp.neg() } else { hp };
        return Some(
            MpFix {
                mant: &signed_hp.mant - &inner.at_scale(g).mant,
                scale: g,
            }
            .rescale(s),
        );
    }
    // Halving: atan(x) = 2·atan(x / (1 + sqrt(1 + x²))), until |x| < 1/4.
    let mut v = x.at_scale(g.min(x.scale));
    let mut halvings = 0u32;
    while v.msb().unwrap_or(i64::MIN) > -2 && halvings < 8 {
        if !budget.tick() {
            return None;
        }
        // sqrt at output scale g−8 needs its argument at 2(g−8).
        let arg_scale = 2 * (g - 8);
        let x2 = mul_fix(&v, &v, arg_scale)?;
        let one_plus = MpFix {
            mant: &x2.at_scale(arg_scale).mant + (BigInt::from(1) << (-arg_scale) as u32),
            scale: arg_scale,
        };
        let root = sqrt_fix(&one_plus, g - 8, budget)?;
        let denom = MpFix {
            mant: &root.mant + (BigInt::from(1) << (-root.scale) as u32),
            scale: root.scale,
        };
        v = div_fix(&v, &denom, g)?;
        halvings += 1;
    }
    let p = g - 2;
    let op_prec = p - 3;
    let arg = v.at_scale(op_prec.min(v.scale)).at_scale(op_prec);
    let series = atan_series(&arg.mant, op_prec, p, budget)?;
    // ×2^halvings is an exact scale shift.
    Some(
        MpFix {
            mant: series,
            scale: p + halvings as i32,
        }
        .rescale(s),
    )
}

/// Alternating odd series Σ (−1)^j x^(2j+1)/(2j+1) for |x| < 1/4.
fn atan_series(op_appr: &BigInt, op_prec: i32, p: i32, budget: &mut Budget) -> Option<BigInt> {
    if p >= 0 {
        return Some(BigInt::zero());
    }
    let iterations_needed = i64::from(-p) / 4 + 4;
    let calc_precision = p - bound_log2(2 * iterations_needed) - 4;
    let max_trunc_error = BigInt::from(1) << (p - 4 - calc_precision) as u32;
    let mut x_pow = shift_round(op_appr, op_prec - calc_precision);
    let mut sum = x_pow.clone();
    let mut current_term = x_pow.clone();
    let mut n: i64 = 1;
    let mut sign: i64 = 1;
    while current_term.abs() > max_trunc_error {
        if !budget.tick() {
            return None;
        }
        n += 2;
        sign = -sign;
        x_pow = shift_round(&(&x_pow * op_appr), op_prec);
        x_pow = shift_round(&(&x_pow * op_appr), op_prec);
        current_term = div_round(&x_pow, &BigInt::from(n * sign));
        sum += &current_term;
    }
    Some(shift_round(&sum, calc_precision - p))
}

pub fn mul_fix(a: &MpFix, b: &MpFix, s: i32) -> Option<MpFix> {
    let scale = i64::from(a.scale) + i64::from(b.scale);
    let m = MpFix {
        mant: &a.mant * &b.mant,
        scale: i32::try_from(scale).ok()?,
    };
    Some(if m.scale >= s { m } else { m.rescale(s) })
}

/// asin via `atan(x/√(1−x²))`; |x| within an ulp of 1 resolves to ±π/2.
pub fn asin_fix(x: &MpFix, s: i32, budget: &mut Budget) -> Option<MpFix> {
    let g = s - 16;
    let arg_scale = 2 * (g - 8);
    let x2 = mul_fix(x, x, arg_scale)?;
    let t = MpFix {
        mant: (BigInt::from(1) << (-arg_scale) as u32) - &x2.at_scale(arg_scale).mant,
        scale: arg_scale,
    };
    if t.mant.is_negative() {
        return None; // |x| > 1
    }
    if t.mant.is_zero() {
        let hp = const_half_pi(s);
        return Some(if x.mant.is_negative() { hp.neg() } else { hp });
    }
    let root = sqrt_fix(&t, g - 8, budget)?;
    let ratio = div_fix(x, &root, g)?;
    atan_fix(&ratio, s, budget)
}

pub fn acos_fix(x: &MpFix, s: i32, budget: &mut Budget) -> Option<MpFix> {
    let g = s - 8;
    let asin = asin_fix(x, g, budget)?;
    let hp = const_half_pi(g);
    Some(
        MpFix {
            mant: &hp.mant - &asin.at_scale(g).mant,
            scale: g,
        }
        .rescale(s),
    )
}

pub fn sinh_fix(x: &MpFix, s: i32, budget: &mut Budget) -> Option<MpFix> {
    let g = s - 8;
    let ep = exp_fix(x, g, budget)?;
    let en = exp_fix(&x.neg(), g, budget)?;
    let common = ep.scale.max(en.scale);
    Some(
        MpFix {
            mant: &ep.rescale(common.max(ep.scale)).mant - &en.rescale(common.max(en.scale)).mant,
            scale: common - 1, // ÷2 = one scale step finer, exact
        }
        .at_scale(s.max(common - 1)),
    )
}

pub fn cosh_fix(x: &MpFix, s: i32, budget: &mut Budget) -> Option<MpFix> {
    let g = s - 8;
    let ep = exp_fix(x, g, budget)?;
    let en = exp_fix(&x.neg(), g, budget)?;
    let common = ep.scale.max(en.scale);
    Some(
        MpFix {
            mant: &ep.rescale(common.max(ep.scale)).mant + &en.rescale(common.max(en.scale)).mant,
            scale: common - 1, // ÷2 exact
        }
        .at_scale(s.max(common - 1)),
    )
}

pub fn tanh_fix(x: &MpFix, s: i32, budget: &mut Budget) -> Option<MpFix> {
    let g = s - 24;
    let sh = sinh_fix(x, g, budget)?;
    let ch = cosh_fix(x, g, budget)?;
    div_fix(&sh, &ch, s)
}

pub fn log10_fix(x: &MpFix, s: i32, budget: &mut Budget) -> Option<MpFix> {
    let g = s - 16;
    let ln = ln_fix(x, g, budget)?;
    let ln10 = const_ln10(g - 8);
    div_fix(&ln, &ln10, s)
}

