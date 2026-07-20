//! Divergence classification for definite integrals (DIVERGENCE_PLAN.md).
//!
//! Three honest answers instead of one: a certified `Value` (proper, or
//! tail-bounded improper), a certified `Divergent` (backed by an exact
//! rational decision or a mean-value comparison certificate — never a
//! numeric heuristic), or `Unknown`.
//!
//! Tier D1: rational integrands are decided *exactly* (Sturm isolation of
//! denominator roots — a complete decision procedure).
//! Tier D2: kernel-built integrands get certificate search — the MVT
//! comparison (`|D(x)| ≤ sup|D′|·|x−ρ|` below a certified sign change ⇒
//! `∫ N/Dˢ` diverges for s ≥ 1 with N sign-definite) plus exact-point
//! probing for even-order zeros at closed-form points.
//! Tier D3: convergence certificates (`|f| ≤ M·|x−ρ|^{−β}`, β < 1, via the
//! lower MVT bound) turn convergent improper integrals into certified
//! values by excising the singular cells with rigorous tail bounds.

use super::quad::{adaptive_quadrature, compile_pair, endpoint, interval_eval, package, Iv};
use super::tape::CompiledExpr;
use super::Precise;
use crate::expr::Expr;
use crate::num::Number;
use num_rational::BigRational;
use num_traits::{Signed, ToPrimitive, Zero};
use std::collections::HashMap;

/// The three-way verdict of [`integrate_analyzed`].
#[derive(Debug)]
pub enum IntegralVerdict {
    /// Certified digits (proper, or tail-bounded improper).
    Value(Precise),
    /// Certified divergent, with the singular points that prove it.
    Divergent { at: Vec<SingularPoint> },
    Unknown(String),
}

/// A singular point backing a `Divergent` verdict.
#[derive(Debug, Clone)]
pub struct SingularPoint {
    /// f64 location (bracket midpoint or the exact point's value).
    pub location: f64,
    /// Exact location where one is known (rational, π-multiple, `RootOf`).
    pub exact: Option<Expr>,
}

// ================= structural decomposition =================

/// One divisor occurrence: the integrand is `N · D^(−s)` (s > 0), with `N`
/// the product of every other canonical factor (including trig-quotient
/// rewrites: `tan u` contributes divisor `cos u` and numerator `sin u`).
struct Divisor {
    d: Expr,
    s: f64,
    n: Expr,
}

/// A `ln`/`log` factor `ln(u)` (singular where u → 0; always integrable —
/// relevant to the convergence certificates only).
struct LogFactor {
    u: Expr,
}

fn split_factors(fc: &Expr) -> Vec<Expr> {
    match fc {
        Expr::Mul(fs) => fs.clone(),
        other => vec![other.clone()],
    }
}

fn neg_exponent(n: &Number) -> Option<f64> {
    let v = n.to_f64();
    (v < 0.0 && v.is_finite()).then(|| -v)
}

fn apply_name(e: &Expr) -> Option<(String, &Expr)> {
    if let Expr::Apply(head, args) = e {
        if let (Expr::Sym(s), [u]) = (&**head, args.as_slice()) {
            return Some((s.name(), u));
        }
    }
    None
}

fn mk_apply(name: &str, arg: Expr) -> Expr {
    Expr::Apply(Box::new(Expr::sym(name)), vec![arg])
}

/// Decompose one canonical factor into (divisor, s, numerator-contribution).
fn factor_divisor(f: &Expr) -> Option<(Expr, f64, Option<Expr>)> {
    match f {
        Expr::Pow(base, k) => {
            let Expr::Num(n) = &**k else { return None };
            let s = neg_exponent(n)?;
            if let Some((name, u)) = apply_name(base) {
                match name.as_str() {
                    // sqrt(D)^(−s) = D^(−s/2)
                    "sqrt" => return Some((u.clone(), s / 2.0, None)),
                    // tan^(−s) = cot^s: divisor sin, numerator cos^s
                    "tan" => {
                        return Some((
                            mk_apply("sin", u.clone()),
                            s,
                            Some(crate::norm::pow(
                                mk_apply("cos", u.clone()),
                                Expr::Num(n.neg()),
                            )),
                        ))
                    }
                    "cot" => {
                        return Some((
                            mk_apply("cos", u.clone()),
                            s,
                            Some(crate::norm::pow(
                                mk_apply("sin", u.clone()),
                                Expr::Num(n.neg()),
                            )),
                        ))
                    }
                    _ => {}
                }
            }
            Some((( **base).clone(), s, None))
        }
        Expr::Apply(..) => {
            let (name, u) = apply_name(f)?;
            match name.as_str() {
                "tan" => Some((mk_apply("cos", u.clone()), 1.0, Some(mk_apply("sin", u.clone())))),
                "cot" => Some((mk_apply("sin", u.clone()), 1.0, Some(mk_apply("cos", u.clone())))),
                "sec" => Some((mk_apply("cos", u.clone()), 1.0, None)),
                "csc" => Some((mk_apply("sin", u.clone()), 1.0, None)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Positive-power tan/cot factors also hide divisors (tan²u = sin²u/cos²u).
fn positive_trig_divisor(f: &Expr) -> Option<(Expr, f64, Expr)> {
    if let Expr::Pow(base, k) = f {
        if let (Some((name, u)), Expr::Num(n)) = (apply_name(base), &**k) {
            let v = n.to_f64();
            if v > 0.0 && v.is_finite() {
                match name.as_str() {
                    "tan" => {
                        return Some((
                            mk_apply("cos", u.clone()),
                            v,
                            crate::norm::pow(mk_apply("sin", u.clone()), Expr::Num(n.clone())),
                        ))
                    }
                    "cot" => {
                        return Some((
                            mk_apply("sin", u.clone()),
                            v,
                            crate::norm::pow(mk_apply("cos", u.clone()), Expr::Num(n.clone())),
                        ))
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

fn collect_structure(fc: &Expr) -> (Vec<Divisor>, Vec<LogFactor>) {
    let factors = split_factors(fc);
    let mut divisors = Vec::new();
    let mut logs = Vec::new();
    for (i, f) in factors.iter().enumerate() {
        let rest = |extra: Option<Expr>| -> Expr {
            let mut others: Vec<Expr> = factors
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, g)| g.clone())
                .collect();
            if let Some(e) = extra {
                others.push(e);
            }
            if others.is_empty() {
                Expr::Num(Number::Int(1)) // empty product: N ≡ 1
            } else {
                crate::norm::mul(others)
            }
        };
        if let Some((d, s, ncontrib)) = factor_divisor(f) {
            divisors.push(Divisor {
                d: crate::norm::canonicalize(&d),
                s,
                n: rest(ncontrib),
            });
        } else if let Some((d, s, ncontrib)) = positive_trig_divisor(f) {
            divisors.push(Divisor {
                d: crate::norm::canonicalize(&d),
                s,
                n: rest(Some(ncontrib)),
            });
        } else if let Some((name, u)) = apply_name(f) {
            if name == "ln" || name == "log" {
                logs.push(LogFactor { u: u.clone() });
            }
        }
    }
    (divisors, logs)
}

// ================= certified evaluation helpers =================

/// Certified sign of a compiled expression at `x` (`None` = |value| ≤ err).
fn cert_sign(tape: &CompiledExpr, x: f64) -> Option<i8> {
    let (v, e) = tape.eval_f64(&[x])?;
    (v.abs() > e).then_some(if v > 0.0 { 1 } else { -1 })
}

/// An exact value `q + p·π` (both rational) — the ground field for the
/// exact-point probes. Trig/exp/ln fold only at their exactly-known points,
/// so every answer is rigorous (DIVERGENCE_PLAN decision 3).
#[derive(Clone, Debug)]
struct PiLin {
    q: BigRational,
    p: BigRational,
}

impl PiLin {
    fn rat(q: BigRational) -> PiLin {
        PiLin {
            q,
            p: BigRational::zero(),
        }
    }
    fn is_zero(&self) -> bool {
        self.q.is_zero() && self.p.is_zero()
    }
    fn pure_rat(&self) -> Option<&BigRational> {
        self.p.is_zero().then_some(&self.q)
    }
}

fn exact_eval(e: &Expr) -> Option<PiLin> {
    Some(match e {
        Expr::Num(n) => PiLin::rat(n.to_bigrational()?),
        Expr::Const(crate::expr::MathConst::Pi) => PiLin {
            q: BigRational::zero(),
            p: BigRational::from_integer(1.into()),
        },
        Expr::Sym(sym) if sym.name() == "pi" => PiLin {
            q: BigRational::zero(),
            p: BigRational::from_integer(1.into()),
        },
        Expr::Add(ts) => {
            let mut acc = PiLin::rat(BigRational::zero());
            for t in ts {
                let v = exact_eval(t)?;
                acc.q += v.q;
                acc.p += v.p;
            }
            acc
        }
        Expr::Mul(fs) => {
            let mut acc = PiLin::rat(BigRational::from_integer(1.into()));
            for f in fs {
                let v = exact_eval(f)?;
                // (q1 + p1π)(q2 + p2π): a π² term is out of the field.
                if !acc.p.is_zero() && !v.p.is_zero() {
                    return None;
                }
                acc = PiLin {
                    q: &acc.q * &v.q,
                    p: &acc.p * &v.q + &acc.q * &v.p,
                };
            }
            acc
        }
        Expr::Pow(b, k) => {
            let Expr::Num(Number::Int(k)) = &**k else {
                return None;
            };
            let v = exact_eval(b)?;
            let q = v.pure_rat()?.clone();
            if *k < 0 && q.is_zero() {
                return None;
            }
            let mut acc = BigRational::from_integer(1.into());
            for _ in 0..k.unsigned_abs().min(64) {
                acc *= &q;
            }
            if k.unsigned_abs() > 64 {
                return None;
            }
            PiLin::rat(if *k < 0 {
                BigRational::from_integer(1.into()) / acc
            } else {
                acc
            })
        }
        Expr::Apply(..) => {
            let (name, u) = apply_name(e)?;
            let v = exact_eval(u)?;
            // Quarter-turn index for trig at q = 0, p = k/2.
            let quarter = || -> Option<i64> {
                if !v.q.is_zero() {
                    return None;
                }
                let two_p = &v.p * BigRational::from_integer(2.into());
                two_p.is_integer().then(|| {
                    (two_p.to_integer() % 4i64 + 4i64) % 4i64
                })?.to_i64()
            };
            let zero = BigRational::zero;
            let one = || BigRational::from_integer(1.into());
            match name.as_str() {
                "sin" => match quarter() {
                    Some(0) | Some(2) => PiLin::rat(zero()),
                    Some(1) => PiLin::rat(one()),
                    Some(3) => PiLin::rat(-one()),
                    None => return None,
                    _ => return None,
                },
                "cos" => match quarter() {
                    Some(0) => PiLin::rat(one()),
                    Some(2) => PiLin::rat(-one()),
                    Some(1) | Some(3) => PiLin::rat(zero()),
                    None => return None,
                    _ => return None,
                },
                "tan" => match quarter() {
                    Some(0) | Some(2) => PiLin::rat(zero()),
                    _ => return None, // pole or unknown
                },
                "sqrt" => {
                    let q = v.pure_rat()?;
                    if q.is_negative() {
                        return None;
                    }
                    PiLin::rat(rational_sqrt_exact(q)?)
                }
                "exp" => {
                    if v.is_zero() {
                        PiLin::rat(one())
                    } else {
                        return None;
                    }
                }
                "ln" | "log" => {
                    if v.pure_rat()? == &one() {
                        PiLin::rat(zero())
                    } else {
                        return None;
                    }
                }
                "sinh" | "tanh" | "asin" | "atan" => {
                    if v.is_zero() {
                        PiLin::rat(zero())
                    } else {
                        return None;
                    }
                }
                "cosh" => {
                    if v.is_zero() {
                        PiLin::rat(one())
                    } else {
                        return None;
                    }
                }
                "abs" => PiLin::rat(v.pure_rat()?.abs()),
                _ => return None,
            }
        }
        _ => return None,
    })
}

fn rational_sqrt_exact(r: &BigRational) -> Option<BigRational> {
    let (n, d) = (r.numer(), r.denom());
    let (sn, sd) = (n.sqrt(), d.sqrt());
    (&sn * &sn == *n && &sd * &sd == *d).then(|| BigRational::new(sn, sd))
}

fn subst_point(e: &Expr, var: &str, pt: &Expr) -> Expr {
    let subs = HashMap::from([(var.to_string(), pt.clone())]);
    crate::ops::substitute(e, &subs)
}

/// Is `e(pt)` *exactly* zero? Decided by the exact ℚ+ℚπ evaluator (the
/// numeric tiers can never certify a true zero).
fn exactly_zero_at(e: &Expr, var: &str, pt: &Expr) -> bool {
    let sub = crate::norm::canonicalize(&subst_point(e, var, pt));
    exact_eval(&sub).map(|v| v.is_zero()).unwrap_or(false)
}

/// Is `e(pt)` certified nonzero? Exact evaluation first; else the ±1-ulp
/// contract of the arbitrary-precision path (|mant| ≥ 2 excludes 0).
fn certified_nonzero_at(e: &Expr, var: &str, pt: &Expr) -> bool {
    let sub = crate::norm::canonicalize(&subst_point(e, var, pt));
    if let Some(v) = exact_eval(&sub) {
        return !v.is_zero();
    }
    match super::evaluate_to_precision(&sub, 12) {
        Precise::Exact(n) => !n.is_zero(),
        Precise::Bounded(m) => m.mant.abs() > num_bigint::BigInt::from(1),
        Precise::Complex { .. } | Precise::Unknown(_) => false,
    }
}

fn exact_number_from_f64(v: f64) -> Option<Number> {
    BigRational::from_float(v).map(Number::from_bigrational)
}

// ================= zero location =================

#[derive(Clone, Debug)]
enum ZeroKind {
    /// Certified sign change of D across the cell.
    SignChange,
    /// D vanishes exactly at this closed-form point (with its f64 value).
    ExactPoint(Expr, f64),
}

#[derive(Clone, Debug)]
struct ZeroCell {
    lo: f64,
    hi: f64,
    kind: ZeroKind,
}

/// Locate certified zeros / ambiguous cells of `d` in [lo, hi].
/// `None` = the divisor tape can't evaluate (unknown function inside).
/// (certified zero cells, ambiguous-sign cells).
type LocatedZeros = (Vec<ZeroCell>, Vec<(f64, f64)>);

fn locate_zeros(d: &Expr, var: &str, lo: f64, hi: f64) -> Option<LocatedZeros> {
    let lim = crate::limits::current();
    let tape = super::tape::compile(d).ok()?;
    if tape.vars().iter().any(|v| v != var) {
        return None;
    }
    let span = hi - lo;
    let mut cells: Vec<ZeroCell> = Vec::new();
    let mut ambiguous: Vec<(f64, f64)> = Vec::new();

    // Exact endpoint zeros first.
    for (ep, other) in [(lo, lo + span / 8.0), (hi, hi - span / 8.0)] {
        if let Some(n) = exact_number_from_f64(ep) {
            let pt = Expr::Num(n);
            if exactly_zero_at(d, var, &pt) {
                cells.push(ZeroCell {
                    lo: ep.min(other),
                    hi: ep.max(other),
                    kind: ZeroKind::ExactPoint(pt, ep),
                });
            }
        }
    }

    // Uniform grid scan for interior sign structure.
    const GRID: usize = 96;
    let mut budget = lim.max_certificate_bisections as i64;
    let mut signs: Vec<Option<i8>> = Vec::with_capacity(GRID + 1);
    for i in 0..=GRID {
        let x = lo + span * i as f64 / GRID as f64;
        signs.push(cert_sign(&tape, x));
    }
    for i in 0..GRID {
        let (l, r) = (
            lo + span * i as f64 / GRID as f64,
            lo + span * (i + 1) as f64 / GRID as f64,
        );
        match (signs[i], signs[i + 1]) {
            (Some(sl), Some(sr)) if sl != sr => {
                // Refine the bracket by certified bisection.
                let (mut bl, mut br, mut sbl) = (l, r, sl);
                for _ in 0..48 {
                    if budget <= 0 {
                        break;
                    }
                    budget -= 1;
                    let m = bl + (br - bl) / 2.0;
                    if m <= bl || m >= br {
                        break;
                    }
                    match cert_sign(&tape, m) {
                        Some(sm) if sm == sbl => bl = m,
                        Some(_) => br = m,
                        None => break, // sign fades near the zero: bracket is tight enough
                    }
                }
                let _ = &mut sbl;
                cells.push(ZeroCell {
                    lo: bl,
                    hi: br,
                    kind: ZeroKind::SignChange,
                });
            }
            (None, _) | (_, None) => {
                // Uncertifiable sign: an even-order zero or a near-zero.
                if signs[i].is_none() {
                    ambiguous.push((l - span / GRID as f64, r));
                } else {
                    ambiguous.push((l, r + span / GRID as f64));
                }
            }
            _ => {}
        }
        if cells.len() + ambiguous.len() > lim.max_singularity_candidates {
            return None; // candidate explosion — refuse classification
        }
    }
    // Merge adjacent ambiguous cells.
    ambiguous.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (l, r) in ambiguous {
        match merged.last_mut() {
            Some(last) if l <= last.1 => last.1 = last.1.max(r),
            _ => merged.push((l, r)),
        }
    }
    Some((cells, merged))
}

// ================= D2 divergence certificates =================

/// The MVT comparison certificate on a cell with a certified zero of D:
/// `|D| ≤ sup|D′|·dist` ⇒ `∫ |N|/|D|^s = ∞` when s ≥ 1 and N sign-definite.
fn mvt_certificate(div: &Divisor, var: &str, cell: &ZeroCell) -> bool {
    if div.s < 1.0 {
        return false;
    }
    let dprime = crate::norm::canonicalize(&crate::diff::derivative(&div.d, var));
    let Ok(dp_tape) = super::tape::compile(&dprime) else {
        return false;
    };
    let Ok(n_tape) = super::tape::compile(&crate::norm::canonicalize(&div.n)) else {
        return false;
    };
    let iv = Iv {
        lo: cell.lo,
        hi: cell.hi,
    };
    // sup |D′| finite on the cell.
    let Some(dp_iv) = interval_eval(&dp_tape, iv) else {
        return false;
    };
    if !dp_iv.mag().is_finite() {
        return false;
    }
    // N sign-definite (bounded away from zero) on the cell.
    let Some(n_iv) = interval_eval(&n_tape, iv) else {
        return false;
    };
    n_iv.lo > 0.0 || n_iv.hi < 0.0
}

/// Exact-point probing (plan §3c): at a closed-form point, decide the zero
/// order of D exactly; `m·s ≥ 1` with N nonzero ⇒ divergent (Taylor bound,
/// valid because the lower derivatives vanish *exactly* at the point).
fn exact_point_certificate(
    div: &Divisor,
    var: &str,
    pt: &Expr,
    pt_f64: f64,
    cell_w: f64,
) -> bool {
    if !exactly_zero_at(&div.d, var, pt) {
        return false;
    }
    // Order of the zero: first certified-nonvanishing derivative (≤ 4).
    let mut dj = div.d.clone();
    let mut m = 0usize;
    for j in 1..=4 {
        dj = crate::norm::canonicalize(&crate::diff::derivative(&dj, var));
        if exactly_zero_at(&dj, var, pt) {
            continue;
        }
        if certified_nonzero_at(&dj, var, pt) {
            m = j;
            break;
        }
        return false; // can't resolve the derivative — no certificate
    }
    if m == 0 || (m as f64) * div.s < 1.0 {
        return false;
    }
    // sup |D^{(m)}| finite and N sign-definite on a small cell around pt.
    let Ok(dm_tape) = super::tape::compile(&dj) else {
        return false;
    };
    let Ok(n_tape) = super::tape::compile(&crate::norm::canonicalize(&div.n)) else {
        return false;
    };
    let mut w = cell_w.max(1e-12);
    for _ in 0..20 {
        let iv = Iv {
            lo: pt_f64 - w,
            hi: pt_f64 + w,
        };
        if let (Some(dm_iv), Some(n_iv)) = (interval_eval(&dm_tape, iv), interval_eval(&n_tape, iv))
        {
            if dm_iv.mag().is_finite() && (n_iv.lo > 0.0 || n_iv.hi < 0.0) {
                return true;
            }
        }
        w /= 4.0;
    }
    false
}

/// Closed-form candidate points inside a cell: low-denominator rationals and
/// π/2-lattice points.
fn closed_form_candidates(lo: f64, hi: f64) -> Vec<(Expr, f64)> {
    let mid = lo + (hi - lo) / 2.0;
    let half = (hi - lo) / 2.0;
    let mut out = Vec::new();
    for q in 1..=64u32 {
        let p = (mid * q as f64).round();
        let v = p / q as f64;
        if (v - mid).abs() <= half && p.abs() < 1e15 {
            if let Some(n) = exact_number_from_f64(v).filter(|n| {
                // keep only genuinely low-denominator snaps (v = p/q exactly)
                *n == Number::rat(p as i64, q as i64)
            }) {
                out.push((Expr::Num(n), v));
                break;
            }
        }
    }
    // k·π/2 lattice.
    let k = (mid / (std::f64::consts::PI / 2.0)).round();
    let v = k * std::f64::consts::PI / 2.0;
    if (v - mid).abs() <= half && k.abs() < 1e12 && k != 0.0 {
        let pt = crate::norm::canonicalize(&Expr::Mul(vec![
            Expr::Num(Number::rat(k as i64, 2)),
            Expr::sym("pi"),
        ]));
        out.push((pt, v));
    }
    out
}

// ================= D1: exact rational decision =================

/// `Some(poles)` when the integrand is rational over ℚ and the decision is
/// exact (empty vec = provably no poles in [lo, hi]); `None` = not rational.
fn rational_poles(fc: &Expr, var: &str, lo: f64, hi: f64) -> Option<Vec<SingularPoint>> {
    let (_, den) = crate::integrate::rational::expr_to_ratfun(fc, var)?;
    if crate::upoly::degree(&den) == 0 {
        return Some(Vec::new());
    }
    let lo_r = BigRational::from_float(lo)?;
    let hi_r = BigRational::from_float(hi)?;
    let mut poles = Vec::new();
    // Endpoint zeros, exactly.
    for (ep_r, ep) in [(lo_r.clone(), lo), (hi_r.clone(), hi)] {
        if crate::upoly::eval_rat(&den, &ep_r).is_zero() {
            poles.push(SingularPoint {
                location: ep,
                exact: Some(Expr::Num(Number::from_bigrational(ep_r.clone()))),
            });
        }
    }
    // Interior roots via exact isolation.
    let radical = {
        let g = crate::upoly::gcd(&den, &crate::upoly::derivative(&den));
        if crate::upoly::degree(&g) >= 1 {
            crate::upoly::divrem(&den, &g).0
        } else {
            den.clone()
        }
    };
    let intervals = crate::upoly::isolate_real_roots(&radical)?;
    for (idx, (a, b)) in intervals.iter().enumerate() {
        // The isolating interval (a, b] holds exactly one root ρ. Its left
        // endpoint may itself be a *different* root (f(a) = 0), so all
        // bisection orients by the sign at b: on (a, ρ) the sign is the
        // opposite of sign(f(b)), flipping exactly once at ρ.
        let (mut a, mut b) = (a.clone(), b.clone());
        let fb = crate::upoly::eval_rat(&radical, &b);
        if fb.is_zero() {
            // ρ = b exactly (a rational root): membership is immediate.
            if b >= lo_r && b <= hi_r {
                let loc = b.to_f64().unwrap_or(f64::NAN);
                poles.push(SingularPoint {
                    location: loc,
                    exact: Some(Expr::Num(Number::from_bigrational(b.clone()))),
                });
            }
            continue;
        }
        let sign_b = if fb.is_positive() { 1i8 } else { -1 };
        let bisect = |a: &mut BigRational, b: &mut BigRational| -> Option<BigRational> {
            // One sign-oriented bisection step; Some(root) on an exact hit.
            let mid = (&*a + &*b) / BigRational::from_integer(2.into());
            let fm = crate::upoly::eval_rat(&radical, &mid);
            if fm.is_zero() {
                return Some(mid);
            }
            let sm = if fm.is_positive() { 1i8 } else { -1 };
            if sm == sign_b {
                *b = mid; // mid is right of ρ
            } else {
                *a = mid; // mid is left of ρ
            }
            None
        };
        let mut exact_root: Option<BigRational> = None;
        let mut decided: Option<bool> = None;
        for _ in 0..256 {
            if b <= lo_r || a >= hi_r {
                decided = Some(false);
                break;
            }
            if a >= lo_r && b <= hi_r {
                decided = Some(true);
                break;
            }
            if let Some(r) = bisect(&mut a, &mut b) {
                exact_root = Some(r);
                break;
            }
        }
        if let Some(r) = exact_root {
            if r >= lo_r && r <= hi_r {
                poles.push(SingularPoint {
                    location: r.to_f64().unwrap_or(f64::NAN),
                    exact: Some(Expr::Num(Number::from_bigrational(r))),
                });
            }
            continue;
        }
        if decided != Some(true) {
            continue;
        }
        // Shrink tightly (exact sign bisection) so the rational snap below
        // scans at most a couple of candidates per denominator.
        for _ in 0..80 {
            if bisect(&mut a, &mut b).is_some() {
                break;
            }
        }
        let location = crate::upoly::refine_to_f64(&radical, a.clone(), b.clone())
            .unwrap_or_else(|| ((&a + &b) / BigRational::from_integer(2.into())).to_f64().unwrap_or(f64::NAN));
        // Exact form: a low-denominator rational root, else RootOf(radical, idx)
        // (real roots come first in canonical index order, ascending).
        let exact = rational_root_in(&radical, &a, &b)
            .map(|r| Expr::Num(Number::from_bigrational(r)))
            .or_else(|| crate::rootof::make_rootof(&radical, idx as u32));
        poles.push(SingularPoint { location, exact });
    }
    Some(poles)
}

fn rational_root_in(p: &[BigRational], a: &BigRational, b: &BigRational) -> Option<BigRational> {
    for q in 1..=1024i64 {
        let qa = a * BigRational::from_integer(q.into());
        let qb = b * BigRational::from_integer(q.into());
        let mut n = qa.ceil().to_integer();
        // n ranges over the integers with n/q ∈ [a, b], i.e. n ∈ [q·a, q·b].
        while BigRational::from_integer(n.clone()) <= qb {
            let cand = BigRational::new(n.clone(), q.into());
            if &cand > a && crate::upoly::eval_rat(p, &cand).is_zero() {
                return Some(cand);
            }
            n += 1;
        }
    }
    None
}

// ================= classification driver =================

struct Classified {
    divergent: Vec<SingularPoint>,
    /// Certified singular-but-not-divergent cells (candidates for §5).
    singular_cells: Vec<SingularCell>,
    /// Candidate cells with no certificate either way.
    unresolved: bool,
}

/// A cell to excise for improper evaluation, with what vanishes there.
struct SingularCell {
    center: f64,
    /// (divisor index, order m) for divisors with certified simple/exact
    /// zeros here.
    vanishing: Vec<(usize, usize)>,
    /// Log factors vanishing here (index into logs).
    vanishing_logs: Vec<usize>,
    /// Exact point when known (order certified exactly).
    exact: Option<Expr>,
}

fn classify(fc: &Expr, var: &str, lo: f64, hi: f64) -> Result<Classified, String> {
    let (divisors, logs) = collect_structure(fc);
    let mut out = Classified {
        divergent: Vec::new(),
        singular_cells: Vec::new(),
        unresolved: false,
    };
    for (di, div) in divisors.iter().enumerate() {
        let Some((cells, ambiguous)) = locate_zeros(&div.d, var, lo, hi) else {
            out.unresolved = true;
            continue;
        };
        for cell in &cells {
            match &cell.kind {
                ZeroKind::SignChange => {
                    if mvt_certificate(div, var, cell) {
                        out.divergent.push(SingularPoint {
                            location: cell.lo + (cell.hi - cell.lo) / 2.0,
                            exact: None,
                        });
                    } else if div.s < 1.0 {
                        out.singular_cells.push(SingularCell {
                            center: cell.lo + (cell.hi - cell.lo) / 2.0,
                            vanishing: vec![(di, 1)],
                            vanishing_logs: Vec::new(),
                            exact: None,
                        });
                    } else {
                        out.unresolved = true;
                    }
                }
                ZeroKind::ExactPoint(pt, ptf) => {
                    if exact_point_certificate(div, var, pt, *ptf, cell.hi - cell.lo) {
                        out.divergent.push(SingularPoint {
                            location: *ptf,
                            exact: Some(pt.clone()),
                        });
                    } else if let Some(m) = exact_zero_order(&div.d, var, pt) {
                        if (m as f64) * div.s < 1.0 {
                            out.singular_cells.push(SingularCell {
                                center: *ptf,
                                vanishing: vec![(di, m)],
                                vanishing_logs: Vec::new(),
                                exact: Some(pt.clone()),
                            });
                        } else {
                            out.unresolved = true;
                        }
                    } else {
                        out.unresolved = true;
                    }
                }
            }
        }
        for (al, ah) in ambiguous {
            // Even-order zero or near-zero: try exact-point probing.
            let mut resolved = false;
            for (pt, ptf) in closed_form_candidates(al, ah) {
                if exact_point_certificate(div, var, &pt, ptf, ah - al) {
                    out.divergent.push(SingularPoint {
                        location: ptf,
                        exact: Some(pt),
                    });
                    resolved = true;
                    break;
                }
                if let Some(m) = exact_zero_order(&div.d, var, &pt) {
                    if (m as f64) * div.s < 1.0 {
                        out.singular_cells.push(SingularCell {
                            center: ptf,
                            vanishing: vec![(di, m)],
                            vanishing_logs: Vec::new(),
                            exact: Some(pt),
                        });
                        resolved = true;
                        break;
                    }
                }
            }
            if !resolved {
                // Could be a smooth near-zero (fine for plain quadrature) or
                // an uncertifiable singularity; plain quadrature decides.
                out.unresolved = true;
            }
        }
    }
    // Log-factor singularities (u → 0): always integrable; locate for §5.
    for (li, lf) in logs.iter().enumerate() {
        let Some((cells, ambiguous)) = locate_zeros(&lf.u, var, lo, hi) else {
            out.unresolved = true;
            continue;
        };
        for (al, ah) in ambiguous {
            // Same probing as the divisor path: an exact simple zero of u at
            // a closed-form point is a certified (integrable) log cell.
            let mut resolved = false;
            for (pt, ptf) in closed_form_candidates(al, ah) {
                if exact_zero_order(&lf.u, var, &pt) == Some(1) {
                    out.singular_cells.push(SingularCell {
                        center: ptf,
                        vanishing: Vec::new(),
                        vanishing_logs: vec![li],
                        exact: Some(pt),
                    });
                    resolved = true;
                    break;
                }
            }
            if !resolved {
                out.unresolved = true;
            }
        }
        for cell in cells {
            let (center, exact) = match cell.kind {
                ZeroKind::SignChange => (cell.lo + (cell.hi - cell.lo) / 2.0, None),
                ZeroKind::ExactPoint(pt, ptf) => (ptf, Some(pt)),
            };
            out.singular_cells.push(SingularCell {
                center,
                vanishing: Vec::new(),
                vanishing_logs: vec![li],
                exact,
            });
        }
    }
    // Merge singular cells that share a location.
    out.singular_cells.sort_by(|a, b| a.center.total_cmp(&b.center));
    let mut merged: Vec<SingularCell> = Vec::new();
    let close = (hi - lo) * 1e-9;
    for c in out.singular_cells.drain(..) {
        match merged.last_mut() {
            Some(last) if (c.center - last.center).abs() <= close => {
                last.vanishing.extend(c.vanishing);
                last.vanishing.sort_unstable();
                last.vanishing.dedup_by_key(|&mut (di, _)| di);
                last.vanishing_logs.extend(c.vanishing_logs);
                last.vanishing_logs.sort_unstable();
                last.vanishing_logs.dedup();
                if last.exact.is_none() {
                    last.exact = c.exact;
                }
            }
            _ => merged.push(c),
        }
    }
    out.singular_cells = merged;
    Ok(out)
}

/// The exact order of D's zero at a closed-form point (`None` = not an
/// exact zero, or unresolvable within 4 derivatives).
fn exact_zero_order(d: &Expr, var: &str, pt: &Expr) -> Option<usize> {
    if !exactly_zero_at(d, var, pt) {
        return None;
    }
    let mut dj = d.clone();
    for j in 1..=4 {
        dj = crate::norm::canonicalize(&crate::diff::derivative(&dj, var));
        if exactly_zero_at(&dj, var, pt) {
            continue;
        }
        return certified_nonzero_at(&dj, var, pt).then_some(j);
    }
    None
}

// ================= the public front ends =================

/// Quick divergence screen used by `integrate_to_precision` (D1 + the D2
/// certificates; no improper machinery). `fc` must be simplified canonical.
pub(crate) fn is_certified_divergent(fc: &Expr, var: &str, lo: f64, hi: f64) -> bool {
    if let Some(poles) = rational_poles(fc, var, lo, hi) {
        return !poles.is_empty();
    }
    match classify(fc, var, lo, hi) {
        Ok(c) => !c.divergent.is_empty(),
        Err(_) => false,
    }
}

/// Full three-way analysis (DIVERGENCE_PLAN §1).
pub fn integrate_analyzed(
    f: &Expr,
    var: &str,
    a: &Expr,
    b: &Expr,
    digits: usize,
) -> IntegralVerdict {
    let digits = digits.clamp(1, 13);
    let (Some(lo0), Some(hi0)) = (endpoint(a), endpoint(b)) else {
        return IntegralVerdict::Unknown("endpoint not a finite constant".into());
    };
    if lo0 == hi0 {
        return IntegralVerdict::Value(Precise::Exact(Number::Int(0)));
    }
    let (lo, hi, negate) = if lo0 < hi0 {
        (lo0, hi0, false)
    } else {
        (hi0, lo0, true)
    };
    let fc = crate::norm::simplify_core(f);
    if crate::ops::variables(&fc)
        .iter()
        .any(|v| v != var && !crate::sym::is_constant_symbol(v))
    {
        return IntegralVerdict::Unknown("free variables besides the integration variable".into());
    }

    // Tier D1: rational — exact decision.
    if let Some(poles) = rational_poles(&fc, var, lo, hi) {
        if !poles.is_empty() {
            return IntegralVerdict::Divergent { at: poles };
        }
        return plain_value(&fc, var, lo, hi, digits, negate);
    }

    // Tier D2/D3.
    let classified = match classify(&fc, var, lo, hi) {
        Ok(c) => c,
        Err(e) => return IntegralVerdict::Unknown(e),
    };
    if !classified.divergent.is_empty() {
        return IntegralVerdict::Divergent {
            at: classified.divergent,
        };
    }
    if classified.singular_cells.is_empty() {
        // Nothing singular found (unresolved candidates included: the plain
        // quadrature is the honest arbiter for those).
        return plain_value(&fc, var, lo, hi, digits, negate);
    }
    if classified.unresolved {
        return IntegralVerdict::Unknown(
            "singularity candidates without certificates alongside certified ones".into(),
        );
    }
    improper_value(&fc, var, lo, hi, digits, negate, &classified)
}

fn plain_value(fc: &Expr, var: &str, lo: f64, hi: f64, digits: usize, negate: bool) -> IntegralVerdict {
    let (tape_f, tape_d4) = match compile_pair(fc, var) {
        Ok(t) => t,
        Err(why) => return IntegralVerdict::Unknown(why.into()),
    };
    match adaptive_quadrature(&tape_f, &tape_d4, lo, hi, digits, 0.0) {
        Ok((v, e)) => match package(v, e, digits, negate) {
            Precise::Unknown(w) => IntegralVerdict::Unknown(w.into()),
            p => IntegralVerdict::Value(p),
        },
        Err(why) => IntegralVerdict::Unknown(why.into()),
    }
}

// ================= D3: tail-bounded improper evaluation =================

/// Certified tail bound for excising `[c−w, c+w]` (clipped to [lo,hi]):
/// `|f| ≤ A·∏Kᵢ^{−sᵢ}·|x−ρ|^{−β}·(C₁ + C₂|x−ρ|^{−γ})` integrated over the
/// cell, from lower MVT/Taylor bounds on each vanishing divisor and the
/// `|ln t| ≤ (eγ)⁻¹ t^{−γ}` inequality per vanishing log factor.
fn cell_tail_bound(
    fc: &Expr,
    var: &str,
    cell: &SingularCell,
    w: f64,
    lo: f64,
    hi: f64,
) -> Option<f64> {
    let (divisors, logs) = collect_structure(fc);
    let cl = (cell.center - w).max(lo);
    let ch = (cell.center + w).min(hi);
    let iv = Iv { lo: cl, hi: ch };
    let width = ch - cl;
    if width <= 0.0 {
        return Some(0.0);
    }

    let mut beta = 0.0f64;
    let mut coeff = 1.0f64; // A·∏ Kᵢ^{−sᵢ·mᵢ-adjusted}
    // Vanishing divisors: lower bounds |D| ≥ K·|x−ρ|^m.
    for &(di, m) in &cell.vanishing {
        let div = divisors.get(di)?;
        beta += div.s * m as f64;
        // K from the m-th derivative, sign-definite on the cell (for m = 1
        // this is the plain lower MVT bound; for exact points the lower
        // derivatives vanish exactly, making the Taylor form valid).
        let mut dm = div.d.clone();
        for _ in 0..m {
            dm = crate::norm::canonicalize(&crate::diff::derivative(&dm, var));
        }
        let dm_tape = super::tape::compile(&dm).ok()?;
        let dm_iv = interval_eval(&dm_tape, iv)?;
        if !(dm_iv.lo > 0.0 || dm_iv.hi < 0.0) {
            return None;
        }
        let k = dm_iv.lo.abs().min(dm_iv.hi.abs()) / factorial(m);
        if k.is_nan() || k <= 0.0 {
            return None;
        }
        coeff *= k.powf(-div.s);
    }
    if beta >= 1.0 {
        return None;
    }
    // At most one vanishing log factor (plan deviation note).
    if cell.vanishing_logs.len() > 1 {
        return None;
    }
    let gamma = if cell.vanishing_logs.is_empty() {
        0.0
    } else {
        (1.0 - beta) / 2.0
    };
    let (mut c1, mut c2) = (1.0f64, 0.0f64);
    if let Some(&li) = cell.vanishing_logs.first() {
        let lf = logs.get(li)?;
        // u has a certified simple zero here: K_u|x−ρ| ≤ |u| ≤ M_u|x−ρ|.
        let du = crate::norm::canonicalize(&crate::diff::derivative(&lf.u, var));
        let du_tape = super::tape::compile(&du).ok()?;
        let du_iv = interval_eval(&du_tape, iv)?;
        if !(du_iv.lo > 0.0 || du_iv.hi < 0.0) {
            return None;
        }
        let ku = du_iv.lo.abs().min(du_iv.hi.abs());
        let mu = du_iv.mag();
        // |ln u| ≤ |ln(K_u·|x−ρ|)| + |ln(M_u·w / (K_u·|x−ρ|))|-slack — use
        // |ln u| ≤ |ln K_u| + |ln M_u·w| + (eγ)⁻¹|x−ρ|^{−γ}.
        c1 = ku.ln().abs() + (mu * width).abs().ln().abs() + 1.0;
        c2 = 1.0 / (std::f64::consts::E * gamma);
    }
    // A: sup of the integrand with the vanishing factors stripped.
    let mut kept: Vec<Expr> = Vec::new();
    let vanish_div: Vec<usize> = cell.vanishing.iter().map(|&(d, _)| d).collect();
    let factors = split_factors(fc);
    'outer: for f in &factors {
        for &di in &vanish_div {
            if factor_matches_divisor(f, &divisors[di]) {
                continue 'outer;
            }
        }
        if let Some((name, _)) = apply_name(f) {
            if (name == "ln" || name == "log") && !cell.vanishing_logs.is_empty() {
                continue;
            }
        }
        kept.push(f.clone());
    }
    let n_expr = if kept.is_empty() {
        Expr::Num(Number::Int(1))
    } else {
        crate::norm::mul(kept)
    };
    let n_tape = super::tape::compile(&crate::norm::canonicalize(&n_expr)).ok()?;
    let n_iv = interval_eval(&n_tape, iv)?;
    let a_sup = n_iv.mag();
    if !a_sup.is_finite() {
        return None;
    }

    // ∫_cell ≤ A·coeff·2·[C₁·w^{1−β}/(1−β) + C₂·w^{1−β−γ}/(1−β−γ)].
    let t1 = c1 * width.powf(1.0 - beta) / (1.0 - beta);
    let t2 = if c2 > 0.0 {
        c2 * width.powf(1.0 - beta - gamma) / (1.0 - beta - gamma)
    } else {
        0.0
    };
    let tail = a_sup * coeff * 2.0 * (t1 + t2) * (1.0 + 1e-12);
    tail.is_finite().then_some(tail)
}

fn factor_matches_divisor(f: &Expr, div: &Divisor) -> bool {
    match factor_divisor(f) {
        Some((d, _, _)) => crate::norm::canonicalize(&d) == div.d,
        None => positive_trig_divisor(f)
            .map(|(d, _, _)| crate::norm::canonicalize(&d) == div.d)
            .unwrap_or(false),
    }
}

fn factorial(m: usize) -> f64 {
    (1..=m).map(|i| i as f64).product::<f64>().max(1.0)
}

fn improper_value(
    fc: &Expr,
    var: &str,
    lo: f64,
    hi: f64,
    digits: usize,
    negate: bool,
    classified: &Classified,
) -> IntegralVerdict {
    let (tape_f, tape_d4) = match compile_pair(fc, var) {
        Ok(t) => t,
        Err(why) => return IntegralVerdict::Unknown(why.into()),
    };
    let span = hi - lo;
    let cells = &classified.singular_cells;
    let mut w = span / 64.0;
    for _ in 0..crate::limits::current().max_improper_refinements {
        // Certified tails at this width.
        let mut total_tail = 0.0f64;
        let mut ok = true;
        for cell in cells {
            match cell_tail_bound(fc, var, cell, w, lo, hi) {
                Some(t) => total_tail += t,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            w /= 4.0;
            continue;
        }
        // Complement pieces.
        let mut edges: Vec<(f64, f64)> = Vec::new();
        let mut cursor = lo;
        for cell in cells {
            let (cl, ch) = ((cell.center - w).max(lo), (cell.center + w).min(hi));
            if cl > cursor {
                edges.push((cursor, cl));
            }
            cursor = cursor.max(ch);
        }
        if cursor < hi {
            edges.push((cursor, hi));
        }
        let piece_tol = (total_tail / (edges.len().max(1) as f64)).max(f64::MIN_POSITIVE);
        let mut value = 0.0f64;
        let mut err = total_tail;
        let mut pieces_ok = true;
        // Pieces run one digit tighter than the total target so their
        // certified error is a small fraction of it (the tail takes the
        // rest); otherwise the sum sits exactly at the boundary.
        let piece_digits = (digits + 1).min(13);
        for &(pl, ph) in &edges {
            match adaptive_quadrature(&tape_f, &tape_d4, pl, ph, piece_digits, piece_tol) {
                Ok((v, e)) => {
                    value += v;
                    err += e;
                }
                Err(_) => {
                    pieces_ok = false;
                    break;
                }
            }
        }
        if pieces_ok {
            let target = 0.5 * 10f64.powi(1 - digits as i32) * value.abs().max(f64::MIN_POSITIVE);
            if err <= target && value.abs() > 2.0 * err {
                match package(value, err, digits, negate) {
                    Precise::Unknown(why) => return IntegralVerdict::Unknown(why.into()),
                    p => return IntegralVerdict::Value(p),
                }
            }
        }
        w /= 4.0;
        // f64 cliff: near a non-zero singular point, interval cancellation
        // (1 − x² within ~16ε of x = 1) makes narrower cells unresolvable —
        // shrinking further cannot succeed.
        let cliff = cells
            .iter()
            .map(|c| 64.0 * f64::EPSILON * c.center.abs().max(1e-3))
            .fold(0.0f64, f64::max);
        if w < cliff {
            return IntegralVerdict::Unknown(
                "improper tail below f64 resolution at the singular point (fewer digits may succeed)"
                    .into(),
            );
        }
    }
    IntegralVerdict::Unknown("improper-integral refinement budget exhausted".into())
}
