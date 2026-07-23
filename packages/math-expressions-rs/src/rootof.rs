//! `Expr`-level helpers for the `RootOf` leaf: canonical construction, the
//! display/serialization form, power reduction in ℚ[t]/(p), and cached
//! numeric evaluation.

use crate::expr::Expr;
use crate::num::Number;
use crate::upoly::{self, UPoly};
use num_complex::Complex64;
use num_rational::BigRational;
use num_traits::{One, Zero};
use std::cell::RefCell;
use std::collections::HashMap;

/// Canonical constructor: trims, takes the squarefree radical (same distinct
/// roots, so the index is preserved), normalizes to primitive integer
/// coefficients with positive leading coefficient, and bounds-checks the
/// degree and index. `None` = not a representable root (caller keeps its
/// unevaluated form).
pub(crate) fn make_rootof(coeffs: &[BigRational], index: u32) -> Option<Expr> {
    let mut p = coeffs.to_vec();
    upoly::trim(&mut p);
    if upoly::degree(&p) < 1 {
        return None;
    }
    let g = upoly::gcd(&p, &upoly::derivative(&p));
    if upoly::degree(&g) >= 1 {
        p = upoly::divrem(&p, &g).0;
    }
    let d = upoly::degree(&p);
    if d < 1 || d > crate::resource_limits::current().max_rootof_degree || (index as usize) >= d {
        return None;
    }
    let ints = upoly::to_primitive_int(&p);
    Some(Expr::RootOf {
        poly: ints
            .into_iter()
            .map(Number::from_bigint)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        index,
    })
}

pub(crate) fn coeffs_to_upoly(poly: &[Number]) -> Option<UPoly> {
    poly.iter().map(|n| n.to_bigrational()).collect()
}

/// The polynomial as a display tree in `var`, highest degree first.
pub(crate) fn poly_display(poly: &[Number], var: &str) -> Expr {
    let t = Expr::sym(var);
    let mut terms = Vec::new();
    for (i, c) in poly.iter().enumerate().rev() {
        if c.is_zero() {
            continue;
        }
        let base = match i {
            0 => None,
            1 => Some(t.clone()),
            _ => Some(Expr::Pow(
                Box::new(t.clone()),
                Box::new(Expr::Num(Number::Int(i as i64))),
            )),
        };
        let term = match base {
            None => Expr::Num(c.clone()),
            Some(b) => {
                if c.is_one() {
                    b
                } else if c.neg().is_one() {
                    Expr::Neg(Box::new(b))
                } else {
                    Expr::Mul(vec![Expr::Num(c.clone()), b])
                }
            }
        };
        terms.push(term);
    }
    match terms.len() {
        0 => Expr::int(0),
        1 => terms.pop().unwrap(),
        _ => Expr::Add(terms),
    }
}

/// The function-application spelling `rootof(p(t), k)` — shared by the text
/// renderer and the JS-tree serializer (whose round trip re-canonicalizes it
/// back into the leaf).
pub(crate) fn as_apply(poly: &[Number], index: u32) -> Expr {
    Expr::Apply(
        Box::new(Expr::sym("rootof")),
        vec![
            poly_display(poly, "t"),
            Expr::Num(Number::Int(i64::from(index))),
        ],
    )
}

/// Recognize a canonicalized `rootof(p, k)` application (any single variable
/// name in `p`). `None` = leave the application unevaluated.
pub(crate) fn from_apply_args(args: &[Expr]) -> Option<Expr> {
    let [p, k] = args else { return None };
    let Expr::Num(Number::Int(k)) = k else {
        return None;
    };
    if *k < 0 {
        return None;
    }
    let vars = crate::ops::variables(p);
    let [var] = vars.as_slice() else { return None };
    let coeffs = expr_to_upoly(p, var)?;
    make_rootof(&coeffs, u32::try_from(*k).ok()?)
}

/// Dense coefficients of a *canonical* single-variable polynomial tree.
fn expr_to_upoly(e: &Expr, var: &str) -> Option<UPoly> {
    fn term(e: &Expr, var: &str) -> Option<(usize, BigRational)> {
        match e {
            Expr::Num(n) => Some((0, n.to_bigrational()?)),
            Expr::Sym(s) if s.name() == var => Some((1, BigRational::one())),
            Expr::Pow(b, x) => match (&**b, &**x) {
                (Expr::Sym(s), Expr::Num(Number::Int(k))) if s.name() == var && *k >= 1 => {
                    Some((*k as usize, BigRational::one()))
                }
                _ => None,
            },
            Expr::Mul(fs) => {
                let mut deg = 0usize;
                let mut coeff = BigRational::one();
                for f in fs {
                    let (d, c) = term(f, var)?;
                    deg += d;
                    coeff *= c;
                }
                Some((deg, coeff))
            }
            _ => None,
        }
    }
    let terms: Vec<&Expr> = match e {
        Expr::Add(ts) => ts.iter().collect(),
        other => vec![other],
    };
    let mut out: UPoly = Vec::new();
    for t in terms {
        let (d, c) = term(t, var)?;
        if out.len() <= d {
            out.resize(d + 1, BigRational::zero());
        }
        out[d] += c;
    }
    upoly::trim(&mut out);
    Some(out)
}

/// Power reduction: `RootOf(p,k)^n` as a polynomial of degree < deg p in
/// the same root. `None` = no reduction applies (0 ≤ n < deg p) or the
/// negative power doesn't exist. Built with the canonical smart constructors.
pub(crate) fn power_reduced(root: &Expr, n: i64) -> Option<Expr> {
    let Expr::RootOf { poly, .. } = root else {
        return None;
    };
    let p = coeffs_to_upoly(poly)?;
    let d = upoly::degree(&p);
    let r = if n >= 0 {
        if (n as usize) < d {
            return None;
        }
        upoly::power_mod(n as u64, &p)
    } else {
        // t is invertible in ℚ[t]/(p): the canonical form has p(0) ≠ 0
        // (a zero constant coefficient would make p divisible by t, and 0
        // is extracted as a rational root before a RootOf forms). Still,
        // guard rather than assume.
        if p[0].is_zero() {
            return None;
        }
        let mut inv: UPoly = (1..=d).map(|i| -(&p[i] / &p[0])).collect();
        upoly::trim(&mut inv);
        let mut acc: UPoly = vec![BigRational::one()];
        let mut base = inv;
        let mut k = n.unsigned_abs();
        while k > 0 {
            if k & 1 == 1 {
                acc = upoly::divrem(&upoly::mul(&acc, &base), &p).1;
            }
            base = upoly::divrem(&upoly::mul(&base, &base), &p).1;
            k >>= 1;
        }
        acc
    };
    Some(upoly_in_root(&r, root))
}

/// `c₀ + c₁·r + … + c_{d−1}·r^{d−1}` through the canonical constructors.
pub(crate) fn upoly_in_root(coeffs: &[BigRational], root: &Expr) -> Expr {
    let terms: Vec<Expr> = coeffs
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.is_zero())
        .map(|(i, c)| {
            let num = Expr::Num(Number::from_bigrational(c.clone()));
            match i {
                0 => num,
                1 => crate::norm::mul(vec![num, root.clone()]),
                _ => crate::norm::mul(vec![
                    num,
                    Expr::Pow(
                        Box::new(root.clone()),
                        Box::new(Expr::Num(Number::Int(i as i64))),
                    ),
                ]),
            }
        })
        .collect();
    crate::norm::add(terms)
}

/// Entry cap for each thread-local memo below. The caches are keyed by the
/// full coefficient vector and never expire, so without a cap an adversarial
/// stream of distinct polynomials (a long-lived grading worker) grows memory
/// monotonically. On overflow the whole cache is dropped — entries are pure
/// memos, recomputable at the cost of one isolation.
const CACHE_CAP: usize = 1024;

fn insert_capped<K: std::hash::Hash + Eq, V>(map: &mut HashMap<K, V>, key: K, value: V) {
    if map.len() >= CACHE_CAP {
        map.clear();
    }
    map.insert(key, value);
}

#[cfg(test)]
mod tests {
    use super::{insert_capped, CACHE_CAP};
    use std::collections::HashMap;

    /// The memo cap (ARCHITECTURE_REVIEW §8): an unbounded stream of distinct
    /// polynomials must not grow the cache without limit. At capacity the whole
    /// map is dropped, then the new entry is inserted — so memory is bounded and
    /// the fresh key is always retrievable.
    #[test]
    fn cache_cap_drops_whole_map_on_overflow() {
        let mut m: HashMap<i32, i32> = HashMap::new();
        for i in 0..CACHE_CAP as i32 {
            insert_capped(&mut m, i, i);
        }
        assert_eq!(m.len(), CACHE_CAP, "fills exactly to the cap");
        // One more entry overflows: the map is cleared, then the new key added.
        insert_capped(&mut m, -1, 42);
        assert_eq!(m.len(), 1, "cleared on overflow");
        assert_eq!(m.get(&-1), Some(&42), "the overflowing key survives");
    }
}

thread_local! {
    /// Ordered numeric roots per canonical polynomial, computed once
    /// (isolation is the expensive part). `None` is cached too: a poly whose
    /// ordering can't be certified stays uncertifiable.
    static ROOT_CACHE: RefCell<HashMap<Vec<Number>, Option<Vec<Complex64>>>> =
        RefCell::new(HashMap::new());
}

/// The `index`-th root numerically (canonical order), for the samplers.
pub(crate) fn numeric_root(poly: &[Number], index: u32) -> Option<Complex64> {
    let key: Vec<Number> = poly.to_vec();
    let cached = ROOT_CACHE.with(|c| c.borrow().get(&key).cloned());
    let roots = match cached {
        Some(r) => r,
        None => {
            let computed = coeffs_to_upoly(poly).and_then(|p| upoly::all_roots_ordered(&p));
            ROOT_CACHE.with(|c| insert_capped(&mut c.borrow_mut(), key, computed.clone()));
            computed
        }
    }?;
    roots.get(index as usize).copied()
}

// ---- arbitrary-precision refinement (ARBITRARY_PERCISION_PLAN §2d hook) ----

use crate::precise::complex::{self, CFix};
use crate::precise::fix::{div_round, MpFix};
use crate::precise::kernels::Budget;
use num_bigint::BigInt;
use num_traits::Signed;

type IsoIntervals = Option<Vec<(BigRational, BigRational)>>;

thread_local! {
    static ISO_CACHE: RefCell<HashMap<Vec<Number>, IsoIntervals>> = RefCell::new(HashMap::new());
}

fn isolating_intervals(poly: &[Number]) -> IsoIntervals {
    let key: Vec<Number> = poly.to_vec();
    let cached = ISO_CACHE.with(|c| c.borrow().get(&key).cloned());
    match cached {
        Some(v) => v,
        None => {
            let computed = coeffs_to_upoly(poly).and_then(|p| upoly::isolate_real_roots(&p));
            ISO_CACHE.with(|c| insert_capped(&mut c.borrow_mut(), key, computed.clone()));
            computed
        }
    }
}

/// 2^k as a rational.
fn pow2_rational(k: i32) -> BigRational {
    if k >= 0 {
        BigRational::from_integer(BigInt::from(1) << k as u32)
    } else {
        BigRational::new(BigInt::from(1), BigInt::from(1) << (-k) as u32)
    }
}

/// Round a rational to the dyadic grid `2^scale` (half away from zero).
fn round_dyadic(q: &BigRational, scale: i32) -> BigRational {
    let scaled = q / pow2_rational(scale);
    let n = div_round(scaled.numer(), scaled.denom());
    BigRational::from_integer(n) * pow2_rational(scale)
}

fn mpfix_from_rational(q: &BigRational, scale: i32) -> Option<MpFix> {
    let scaled = q / pow2_rational(scale);
    Some(MpFix {
        mant: div_round(scaled.numer(), scaled.denom()),
        scale,
    })
}

/// Refine the `index`-th root to `±1 ulp` at `target_scale`, when that root
/// is real: Newton on dyadic rationals from the f64 seed, then an **exact**
/// sign-change certificate at `x ± ulp/4` (coefficients are exact, so the
/// certificate is unconditional). `None` if the root is complex (caller
/// escalates to the complex tier) or certification fails.
pub(crate) fn refine_real(poly: &[Number], index: u32, target_scale: i32) -> Option<MpFix> {
    let p = coeffs_to_upoly(poly)?;
    let intervals = isolating_intervals(poly)?;
    let idx = index as usize;
    if idx >= intervals.len() {
        return None; // complex root — not this tier's job
    }
    let seed = numeric_root(poly, index)?.re;
    let dp = upoly::derivative(&p);
    let lim = crate::resource_limits::current();
    if i64::from(-target_scale) > 4 * i64::from(lim.max_eval_precision_bits) {
        return None;
    }

    for extra_guard in [8i32, 64] {
        // Precision ladder: double the working bits each Newton step.
        let final_scale = target_scale.saturating_sub(extra_guard);
        let mut x = round_dyadic(
            &BigRational::from_float(seed).unwrap_or_default(),
            -52,
        );
        let mut s = -52i32;
        let mut steps = 0;
        loop {
            s = if s <= final_scale {
                final_scale
            } else {
                s.saturating_mul(2).max(final_scale)
            };
            let fx = upoly::eval_rat(&p, &x);
            if fx.is_zero() {
                return mpfix_from_rational(&x, target_scale);
            }
            let dfx = upoly::eval_rat(&dp, &x);
            if dfx.is_zero() {
                break; // Newton undefined; try more guard or give up
            }
            x = round_dyadic(&(&x - fx / dfx), s);
            steps += 1;
            if s == final_scale || steps > 64 {
                break;
            }
        }
        // Newton to convergence at final precision (the ladder above only
        // guarantees fast convergence from an accurate seed; a crude seed —
        // e.g. from an astronomically wide Cauchy-bound interval — needs
        // the extra iterations), then certify.
        for _ in 0..64 {
            let fx = upoly::eval_rat(&p, &x);
            if fx.is_zero() {
                break;
            }
            let dfx = upoly::eval_rat(&dp, &x);
            if dfx.is_zero() {
                break;
            }
            let next = round_dyadic(&(&x - fx / dfx), final_scale);
            let moved = (&next - &x).abs() > pow2_rational(final_scale + 1);
            x = next;
            if !moved {
                break;
            }
        }
        let h = pow2_rational(target_scale - 2);
        let lo = upoly::eval_rat(&p, &(&x - &h));
        let hi = upoly::eval_rat(&p, &(&x + &h));
        let sgn = |v: &BigRational| -> i8 {
            if v.is_zero() {
                0
            } else if v.is_positive() {
                1
            } else {
                -1
            }
        };
        if sgn(&lo) * sgn(&hi) <= 0 {
            // Root certified inside (x − ulp/4, x + ulp/4).
            return mpfix_from_rational(&x, target_scale);
        }
    }
    None
}

/// Complex Horner at working scale `s`: p(z) with exact integer coefficients.
fn horner_cfix(coeffs: &[Number], z: &CFix, s: i32) -> Option<CFix> {
    let cs = s.min(0) - 4;
    let mut acc = CFix::real(MpFix::from_number(coeffs.last()?, cs)?);
    for c in coeffs.iter().rev().skip(1) {
        acc = complex::cmul(&acc, z, s)?;
        let cc = CFix::real(MpFix::from_number(c, cs)?);
        acc = complex::cadd(&[&acc, &cc], s.min(acc.re.scale));
    }
    Some(acc)
}

fn cfix_from_c64(z: Complex64, scale: i32) -> Option<CFix> {
    Some(CFix {
        re: MpFix::from_f64(z.re, scale)?,
        im: MpFix::from_f64(z.im, scale)?,
    })
}

fn cfix_msb(z: &CFix) -> Option<i64> {
    match (z.re.msb(), z.im.msb()) {
        (Some(a), Some(b)) => Some(a.max(b) + 1),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    }
}

/// Refine the `index`-th root at `target_scale` in the complex tier: CFix
/// Newton from the certified-order f64 seed on a doubling precision ladder,
/// accepted only when the rigorous simple-root bound
/// `|z − root| ≤ n·|p(z)|/|p′(z)|` (Horner rounding folded in) is below the
/// target ulp. Real roots delegate to [`refine_real`].
pub(crate) fn refine_complex(
    poly: &[Number],
    index: u32,
    target_scale: i32,
    budget: &mut Budget,
) -> Option<CFix> {
    let seed = numeric_root(poly, index)?;
    if seed.im == 0.0 {
        return refine_real(poly, index, target_scale).map(CFix::real);
    }
    let deg = poly.len().saturating_sub(1);
    if deg < 2 {
        return None;
    }
    let dcoeffs: Vec<Number> = poly
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, c)| c.mul(&Number::Int(i as i64)))
        .collect();
    // log2 of n plus Horner-rounding slack (≤ 2·deg+2 ulps per evaluation).
    let slack_bits = 64 - (4 * deg as u64 + 4).leading_zeros();
    // f64 estimate of |p′| near the root fixes the working scale.
    let dpf: Complex64 = {
        let mut acc = Complex64::ZERO;
        for c in dcoeffs.iter().rev() {
            acc = acc * seed + Complex64::new(c.to_f64(), 0.0);
        }
        acc
    };
    if !dpf.norm().is_finite() || dpf.norm() == 0.0 {
        return None;
    }
    let dp_msb = dpf.norm().log2().ceil() as i32;
    let lim = crate::resource_limits::current();
    let w = target_scale
        .saturating_add(dp_msb)
        .saturating_sub(2 * slack_bits as i32 + 16);
    if i64::from(-w) > 4 * i64::from(lim.max_eval_precision_bits) {
        return None;
    }

    let mut z = cfix_from_c64(seed, -52)?;
    let mut s = -52i32;
    loop {
        if !budget.tick() {
            return None;
        }
        s = if s <= w { w } else { s.saturating_mul(2).max(w) };
        let work = s - 8;
        let pz = horner_cfix(poly, &z, work)?;
        let dpz = horner_cfix(&dcoeffs, &z, work)?;
        let step = complex::cdiv(&pz, &dpz, s, budget)?;
        z = complex::cadd(&[&z, &step.neg()], s);
        if s == w {
            break;
        }
    }
    // Certificate: n·|p(z)|/|p′(z)| ≤ 2^(target−2), with Horner rounding
    // absorbed by treating |p(z)| as at least a few ulps of the work scale.
    let work = w - 8;
    let pz = horner_cfix(poly, &z, work)?;
    let dpz = horner_cfix(&dcoeffs, &z, work)?;
    let p_log2 = cfix_msb(&pz)
        .unwrap_or(i64::from(work))
        .max(i64::from(work) + i64::from(slack_bits));
    let dp_log2 = cfix_msb(&dpz)?;
    if dp_log2 <= i64::from(work) + i64::from(slack_bits) + 4 {
        return None; // derivative not resolved — near-multiple root
    }
    let bound_log2 = p_log2 + i64::from(slack_bits) - dp_log2;
    if bound_log2 <= i64::from(target_scale) - 2 {
        Some(z.rescale(target_scale))
    } else {
        None
    }
}
