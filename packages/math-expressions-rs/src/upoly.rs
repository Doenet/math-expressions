//! Dense univariate ℚ[t] utilities for the `RootOf` pipeline
//! (MATRIX_PLAN.md §2c) and the quotient-ring elimination of §3.
//!
//! Polynomials are dense coefficient vectors, low → high, with no trailing
//! zeros (the zero polynomial is the empty vector). Everything here is exact
//! rational arithmetic except the final f64 root approximations, whose
//! *ordering* is certified against exact Sturm counts — ambiguity is a `None`,
//! never a guess (§7f philosophy).

use num_bigint::BigInt;
use num_complex::Complex64;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

pub(crate) type UPoly = Vec<BigRational>;

pub(crate) fn trim(p: &mut UPoly) {
    while p.last().is_some_and(|c| c.is_zero()) {
        p.pop();
    }
}

pub(crate) fn degree(p: &[BigRational]) -> usize {
    p.len().saturating_sub(1)
}

pub(crate) fn is_zero(p: &[BigRational]) -> bool {
    p.is_empty()
}

pub(crate) fn add_p(a: &[BigRational], b: &[BigRational]) -> UPoly {
    let mut out = vec![BigRational::zero(); a.len().max(b.len())];
    for (i, c) in a.iter().enumerate() {
        out[i] += c;
    }
    for (i, c) in b.iter().enumerate() {
        out[i] += c;
    }
    trim(&mut out);
    out
}

pub(crate) fn sub(a: &[BigRational], b: &[BigRational]) -> UPoly {
    let mut out = vec![BigRational::zero(); a.len().max(b.len())];
    for (i, c) in a.iter().enumerate() {
        out[i] += c;
    }
    for (i, c) in b.iter().enumerate() {
        out[i] -= c;
    }
    trim(&mut out);
    out
}

pub(crate) fn mul(a: &[BigRational], b: &[BigRational]) -> UPoly {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![BigRational::zero(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        if x.is_zero() {
            continue;
        }
        for (j, y) in b.iter().enumerate() {
            out[i + j] += x * y;
        }
    }
    trim(&mut out);
    out
}

pub(crate) fn scale(a: &[BigRational], c: &BigRational) -> UPoly {
    if c.is_zero() {
        return Vec::new();
    }
    a.iter().map(|x| x * c).collect()
}

/// Euclidean division over ℚ: `a = q·b + r`, deg r < deg b. `b` nonzero.
pub(crate) fn divrem(a: &[BigRational], b: &[BigRational]) -> (UPoly, UPoly) {
    assert!(!b.is_empty(), "division by the zero polynomial");
    let mut r: UPoly = a.to_vec();
    trim(&mut r);
    let db = degree(b);
    let lc = &b[db];
    let mut q = vec![BigRational::zero(); r.len().saturating_sub(db)];
    while !r.is_empty() && degree(&r) >= db {
        let dr = degree(&r);
        let coeff = &r[dr] / lc;
        q[dr - db] = coeff.clone();
        for i in 0..=db {
            let delta = &b[i] * &coeff;
            r[dr - db + i] -= delta;
        }
        trim(&mut r);
    }
    trim(&mut q);
    (q, r)
}

pub(crate) fn derivative(p: &[BigRational]) -> UPoly {
    let mut out: UPoly = p
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, c)| c * BigRational::from_integer(BigInt::from(i)))
        .collect();
    trim(&mut out);
    out
}

pub(crate) fn monic(p: &[BigRational]) -> UPoly {
    if p.is_empty() {
        return Vec::new();
    }
    let lc = p[degree(p)].clone();
    p.iter().map(|c| c / &lc).collect()
}

/// Monic gcd over ℚ (Euclid).
pub(crate) fn gcd(a: &[BigRational], b: &[BigRational]) -> UPoly {
    let (mut x, mut y) = (a.to_vec(), b.to_vec());
    trim(&mut x);
    trim(&mut y);
    while !y.is_empty() {
        let (_, r) = divrem(&x, &y);
        x = y;
        y = r;
    }
    monic(&x)
}

/// Extended Euclid: returns `(g, s)` with `s·a ≡ g (mod f)`, `g` monic.
/// (`s` is reduced mod `f`.) Inverting `a` in ℚ[t]/(f) succeeds when
/// `deg g == 0`; a nontrivial `g` is a *discovered factor* of `f`.
pub(crate) fn xgcd_mod(a: &[BigRational], f: &[BigRational]) -> (UPoly, UPoly) {
    let (mut r0, mut r1) = (a.to_vec(), f.to_vec());
    trim(&mut r0);
    trim(&mut r1);
    let (mut s0, mut s1) = (vec![BigRational::one()], Vec::new());
    while !r1.is_empty() {
        let (q, r) = divrem(&r0, &r1);
        let s_next = sub(&s0, &mul(&q, &s1));
        r0 = r1;
        r1 = r;
        s0 = s1;
        s1 = s_next;
    }
    if r0.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let lc = r0[degree(&r0)].clone();
    let g = monic(&r0);
    let mut s = scale(&s0, &(BigRational::one() / lc));
    if f.len() > 1 {
        s = divrem(&s, f).1;
    }
    (g, s)
}

/// Yun's squarefree decomposition: `p = ∏ fᵢ^i` with each `fᵢ` squarefree and
/// pairwise coprime. Returns the (fᵢ, i) with deg fᵢ ≥ 1.
pub(crate) fn squarefree_decomposition(p: &[BigRational]) -> Vec<(UPoly, u32)> {
    if degree(p) == 0 {
        return Vec::new();
    }
    let dp = derivative(p);
    let g = gcd(p, &dp);
    if degree(&g) == 0 {
        return vec![(monic(p), 1)];
    }
    let mut out = Vec::new();
    let (mut c, _) = divrem(p, &g);
    let (dg, _) = divrem(&dp, &g);
    let mut d = sub(&dg, &derivative(&c));
    let mut i = 1u32;
    while degree(&c) >= 1 {
        let a = gcd(&c, &d);
        if degree(&a) >= 1 {
            out.push((monic(&a), i));
        }
        let (c2, _) = divrem(&c, &a);
        let (dd, _) = divrem(&d, &a);
        d = sub(&dd, &derivative(&c2));
        c = c2;
        i += 1;
    }
    out
}

/// Clear denominators and content: the unique integer-primitive form with a
/// positive leading coefficient (the `RootOf` canonical invariant).
pub(crate) fn to_primitive_int(p: &[BigRational]) -> Vec<BigInt> {
    if p.is_empty() {
        return Vec::new();
    }
    let mut l = BigInt::one();
    for c in p {
        l = lcm(&l, c.denom());
    }
    let mut ints: Vec<BigInt> = p
        .iter()
        .map(|c| (c * &BigRational::from_integer(l.clone())).to_integer())
        .collect();
    let mut g = BigInt::zero();
    for c in &ints {
        g = int_gcd(g, c.abs());
    }
    if !g.is_zero() && !g.is_one() {
        for c in &mut ints {
            *c = &*c / &g;
        }
    }
    if ints.last().is_some_and(|c| c.is_negative()) {
        for c in &mut ints {
            *c = -&*c;
        }
    }
    ints
}

fn int_gcd(mut a: BigInt, mut b: BigInt) -> BigInt {
    while !b.is_zero() {
        let r = &a % &b;
        a = b;
        b = r;
    }
    a.abs()
}

fn lcm(a: &BigInt, b: &BigInt) -> BigInt {
    if a.is_zero() || b.is_zero() {
        return BigInt::zero();
    }
    (a * b).abs() / int_gcd(a.clone(), b.clone())
}

pub(crate) fn eval_rat(p: &[BigRational], x: &BigRational) -> BigRational {
    let mut acc = BigRational::zero();
    for c in p.iter().rev() {
        acc = acc * x + c;
    }
    acc
}

pub(crate) fn eval_c64(p: &[f64], z: Complex64) -> Complex64 {
    let mut acc = Complex64::ZERO;
    for &c in p.iter().rev() {
        acc = acc * z + c;
    }
    acc
}

/// Rational roots of a squarefree polynomial by the rational-root theorem,
/// with the found roots deflated out. Divisor enumeration is capped: if the
/// end coefficients don't factor under the cap, no candidates are tried —
/// the roots then simply stay inside a `RootOf` (correct, less minimal —
/// plan decision 6).
pub(crate) fn rational_roots(p: &UPoly) -> (Vec<BigRational>, UPoly) {
    let mut p = p.clone();
    trim(&mut p);
    let mut roots = Vec::new();
    // A zero constant term means a root at 0 (squarefree ⇒ exactly one).
    if p.first().is_some_and(|c| c.is_zero()) {
        roots.push(BigRational::zero());
        p.remove(0);
    }
    if degree(&p) >= 1 {
        let ints = to_primitive_int(&p);
        let (a0, an) = (ints[0].abs(), ints[ints.len() - 1].abs());
        if let (Some(nums), Some(dens)) = (divisors_capped(&a0), divisors_capped(&an)) {
            let mut candidates: Vec<BigRational> = Vec::new();
            for n in &nums {
                for d in &dens {
                    let r = BigRational::new(n.clone(), d.clone());
                    candidates.push(r.clone());
                    candidates.push(-r);
                }
            }
            candidates.sort();
            candidates.dedup();
            for r in candidates {
                if degree(&p) < 1 {
                    break;
                }
                if eval_rat(&p, &r).is_zero() {
                    roots.push(r.clone());
                    // Deflate by (t − r); exact by construction.
                    let linear = vec![-r, BigRational::one()];
                    let (q, rem) = divrem(&p, &linear);
                    debug_assert!(rem.is_empty());
                    p = q;
                }
            }
        }
    }
    roots.sort();
    (roots, p)
}

/// All positive divisors of `n` (n ≥ 1), or `None` if `n` doesn't fully
/// factor with trial division under `max_trial_divisor`.
pub(crate) fn divisors_capped(n: &BigInt) -> Option<Vec<BigInt>> {
    let cap = crate::resource_limits::current().max_trial_divisor;
    let mut rest = n.clone();
    let mut factors: Vec<(BigInt, u32)> = Vec::new();
    let mut d = BigInt::from(2u32);
    while &d * &d <= rest {
        if d.to_u64().is_none_or(|v| v > cap) {
            return None;
        }
        let mut m = 0u32;
        while (&rest % &d).is_zero() {
            rest /= &d;
            m += 1;
        }
        if m > 0 {
            factors.push((d.clone(), m));
        }
        d += 1;
    }
    if rest > BigInt::one() {
        factors.push((rest, 1));
    }
    let mut divs = vec![BigInt::one()];
    for (f, m) in factors {
        let prev = divs.clone();
        let mut pw = BigInt::one();
        for _ in 0..m {
            pw = &pw * &f;
            divs.extend(prev.iter().map(|d| d * &pw));
        }
        if divs.len() > 4096 {
            return None;
        }
    }
    Some(divs)
}

// ---- Sturm sequences and certified real-root isolation ----

/// Divide by the positive content: primitive integer coefficients with the
/// SIGN PRESERVED (a positive scaling keeps every Sturm sign-variation
/// count intact, and bounds the otherwise-exponential Euclidean-PRS
/// coefficient growth).
fn primitive_signed(p: &[BigRational]) -> UPoly {
    if p.is_empty() {
        return Vec::new();
    }
    let mut l = BigInt::one();
    for c in p {
        l = lcm(&l, c.denom());
    }
    let mut ints: Vec<BigInt> = p
        .iter()
        .map(|c| (c * &BigRational::from_integer(l.clone())).to_integer())
        .collect();
    let mut g = BigInt::zero();
    for c in &ints {
        g = int_gcd(g, c.abs());
    }
    if !g.is_zero() && !g.is_one() {
        for c in &mut ints {
            *c = &*c / &g;
        }
    }
    ints.into_iter().map(BigRational::from_integer).collect()
}

fn sturm_chain(p: &[BigRational]) -> Vec<UPoly> {
    let mut chain = vec![primitive_signed(p), primitive_signed(&derivative(p))];
    loop {
        let n = chain.len();
        if chain[n - 1].is_empty() {
            chain.pop();
            break;
        }
        let (_, r) = divrem(&chain[n - 2], &chain[n - 1]);
        if r.is_empty() {
            break;
        }
        chain.push(primitive_signed(&scale(&r, &-BigRational::one())));
    }
    chain
}

fn sign_variations_at(chain: &[UPoly], x: &BigRational) -> u32 {
    let mut last = 0i8;
    let mut v = 0u32;
    for p in chain {
        let s = match eval_rat(p, x) {
            e if e.is_zero() => 0i8,
            e if e.is_positive() => 1,
            _ => -1,
        };
        if s != 0 {
            if last != 0 && s != last {
                v += 1;
            }
            last = s;
        }
    }
    v
}

/// Number of real roots in the half-open interval (a, b].
fn roots_in(chain: &[UPoly], a: &BigRational, b: &BigRational) -> u32 {
    sign_variations_at(chain, a).saturating_sub(sign_variations_at(chain, b))
}

/// Power-of-two Fujiwara root bound: every root has
/// |z| ≤ 2·max_k |a_{n−k}/a_n|^{1/k} ≤ 2^(1 + max_k ⌈log₂(r_k)/k⌉).
/// Unlike the Cauchy bound (1 + max|aᵢ/aₙ|), this stays tight for scaled
/// and ill-conditioned inputs — Wilkinson-20's Cauchy bound is ~2·10¹⁸
/// while its roots end at 20, and every wasted octave costs a full
/// Sturm-chain bisection level.
fn root_bound(p: &[BigRational]) -> BigRational {
    let d = degree(p);
    let lc = &p[d];
    let mut max_bits: i64 = 0;
    for k in 1..=d {
        let c = &p[d - k];
        if c.is_zero() {
            continue;
        }
        let r = (c / lc).abs();
        // log₂ r < numer.bits() − denom.bits() + 1, exactly computable.
        let bits = r.numer().bits() as i64 - r.denom().bits() as i64 + 1;
        let per = (bits + k as i64 - 1).div_euclid(k as i64).max(0);
        max_bits = max_bits.max(per);
    }
    let shift = u32::try_from(max_bits + 1).unwrap_or(u32::MAX / 2).min(1 << 20);
    BigRational::from_integer(BigInt::from(1) << shift)
}

/// Isolate all real roots of a squarefree `p`: disjoint half-open intervals
/// (lo, hi], ascending, exactly one root each. `None` if the bisection budget
/// (`max_isolation_bits`) runs out.
pub(crate) fn isolate_real_roots(p: &[BigRational]) -> Option<Vec<(BigRational, BigRational)>> {
    if degree(p) == 0 {
        return Some(Vec::new());
    }
    let chain = sturm_chain(p);
    let bound = root_bound(p);
    let lo = -bound.clone();
    let hi = bound;
    let total = roots_in(&chain, &lo, &hi);
    let mut budget = crate::resource_limits::current().max_isolation_bits;
    let mut work = vec![(lo, hi, total)];
    let mut done = Vec::new();
    while let Some((a, b, count)) = work.pop() {
        match count {
            0 => {}
            1 => done.push((a, b)),
            _ => {
                if budget == 0 {
                    return None;
                }
                budget -= 1;
                let mid = (&a + &b) / BigRational::from_integer(BigInt::from(2));
                let left = roots_in(&chain, &a, &mid);
                work.push((a, mid.clone(), left));
                work.push((mid, b, count - left));
            }
        }
    }
    done.sort_by(|x, y| x.0.cmp(&y.0));
    Some(done)
}

/// Shrink an isolating interval until its width is below `2^-bits` of its
/// scale, by Sturm-count bisection (endpoint-root-proof), then return the
/// midpoint as f64.
/// Shrink an isolating interval (a, b] (exactly one simple root inside) to
/// f64 resolution by sign bisection — one exact polynomial evaluation per
/// step, so wide Cauchy-bound intervals (2048 halvings ≈ 616 decimal orders
/// of magnitude) stay cheap.
pub(crate) fn refine_to_f64(p: &[BigRational], mut a: BigRational, mut b: BigRational) -> Option<f64> {
    let two = BigRational::from_integer(BigInt::from(2));
    let sgn = |v: &BigRational| -> i8 {
        if v.is_zero() {
            0
        } else if v.is_positive() {
            1
        } else {
            -1
        }
    };
    let sb = sgn(&eval_rat(p, &b));
    if sb == 0 {
        return b.to_f64();
    }
    // The left endpoint is open; if it happens to be a *different* root,
    // nudge inward until the sign certifies we are left of ours.
    let mut sa = sgn(&eval_rat(p, &a));
    if sa == 0 || sa == sb {
        let w = &b - &a;
        let mut step = w / &two;
        let mut found = false;
        for _ in 0..128 {
            let cand = &a + &step;
            let sc = sgn(&eval_rat(p, &cand));
            if sc == 0 {
                return cand.to_f64();
            }
            if sc == -sb {
                a = cand;
                sa = sc;
                found = true;
                break;
            }
            step /= &two;
        }
        if !found {
            return None;
        }
    }
    for _ in 0..2048 {
        let width = (b.clone() - a.clone()).abs();
        let scal = a.abs().max(b.abs()) + BigRational::one();
        // width < scale·2⁻⁵⁵ is beyond f64 resolution — stop.
        if width * BigRational::from_integer(BigInt::from(1u64 << 55)) < scal {
            break;
        }
        let mid = (&a + &b) / &two;
        match sgn(&eval_rat(p, &mid)) {
            0 => return mid.to_f64(),
            sc if sc == sa => a = mid,
            _ => b = mid,
        }
    }
    ((&a + &b) / two).to_f64()
}

/// All roots of the squarefree polynomial with the given rational
/// coefficients, in **canonical index order**: real roots ascending, then
/// complex roots grouped in conjugate pairs (negative imaginary part first),
/// pairs ordered by (re, |im|). `None` when f64 resolution can't certify the
/// ordering — never a guessed order.
pub(crate) fn all_roots_ordered(p: &[BigRational]) -> Option<Vec<Complex64>> {
    let deg = degree(p);
    if deg == 0 {
        return Some(Vec::new());
    }
    let intervals = isolate_real_roots(p)?;
    let mut reals = Vec::with_capacity(intervals.len());
    for (a, b) in intervals {
        reals.push(refine_to_f64(p, a, b)?);
    }
    let n_complex = deg - reals.len();
    if n_complex == 0 {
        return Some(reals.iter().map(|&r| Complex64::new(r, 0.0)).collect());
    }

    // Durand–Kerner for the full root set, then match away the certified
    // real roots; what remains must pair into conjugates.
    let monic_f64: Vec<f64> = {
        let m = monic(p);
        let v: Option<Vec<f64>> = m.iter().map(|c| c.to_f64()).collect();
        let v = v?;
        if v.iter().any(|c| !c.is_finite()) {
            return None;
        }
        v
    };
    let mut zs = durand_kerner(&monic_f64)?;
    // Remove the DK root nearest each certified real value.
    for &r in &reals {
        let target = Complex64::new(r, 0.0);
        let idx = nearest(&zs, target)?;
        zs.remove(idx);
    }
    let scale = 1.0 + zs.iter().fold(0.0f64, |m, z| m.max(z.norm()));
    let tol = 1e-8 * scale;
    // Every leftover root must be honestly complex.
    if zs.iter().any(|z| z.im.abs() < tol) {
        return None;
    }
    // Pair conjugates. Each pair is represented by its (im < 0) member and
    // MIRRORED EXACTLY for the (im > 0) mate — real coefficients guarantee
    // exact conjugate symmetry, and mirroring removes the independent DK
    // rounding noise between the two members.
    let negs: Vec<Complex64> = zs.iter().copied().filter(|z| z.im < 0.0).collect();
    let mut poss: Vec<Complex64> = zs.iter().copied().filter(|z| z.im > 0.0).collect();
    if negs.len() != poss.len() {
        return None;
    }
    let mut pairs: Vec<Complex64> = Vec::with_capacity(negs.len());
    for z in negs {
        let conj = z.conj();
        let idx = nearest(&poss, conj)?;
        if (poss[idx] - conj).norm() > tol {
            return None;
        }
        poss.remove(idx);
        pairs.push(z);
    }
    // Canonical pair order is (re, |im|), but the computed re carries f64
    // noise — pairs that genuinely share a real part (e.g. all on the
    // imaginary axis) must not be ordered by that noise. Cluster by re with
    // a certified gap dichotomy: gaps below tol/8 merge, gaps above 8·tol
    // split, anything between is ambiguous → refuse.
    pairs.sort_by(|a, b| a.re.total_cmp(&b.re));
    let mut clusters: Vec<Vec<Complex64>> = Vec::new();
    for z in pairs {
        match clusters.last_mut() {
            Some(cluster) => {
                let gap = z.re - cluster.last().unwrap().re;
                if gap <= tol / 8.0 {
                    cluster.push(z);
                } else if gap >= 8.0 * tol {
                    clusters.push(vec![z]);
                } else {
                    return None; // ambiguous re ordering at f64
                }
            }
            None => clusters.push(vec![z]),
        }
    }
    let mut out: Vec<Complex64> = reals.iter().map(|&r| Complex64::new(r, 0.0)).collect();
    for cluster in &mut clusters {
        // Within a cluster: by |im| (= −im for the negative member),
        // requiring certified separation between consecutive pairs.
        cluster.sort_by(|a, b| (-a.im).total_cmp(&(-b.im)));
        for w in cluster.windows(2) {
            if (w[1].im - w[0].im).abs() < tol {
                return None;
            }
        }
        for z in cluster {
            out.push(*z);
            out.push(z.conj());
        }
    }
    Some(out)
}

fn nearest(zs: &[Complex64], target: Complex64) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (i, z) in zs.iter().enumerate() {
        let d = (*z - target).norm();
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some((i, d));
        }
    }
    best.map(|(i, _)| i)
}

/// Durand–Kerner iteration on a monic squarefree polynomial. `None` when it
/// fails to converge (an iteration-count refusal, not a hang).
fn durand_kerner(monic_coeffs: &[f64]) -> Option<Vec<Complex64>> {
    let deg = monic_coeffs.len() - 1;
    // Fujiwara root bound: 2·max_k |c_{n−k}|^{1/k}. Unlike the naive
    // 1 + max|cᵢ|, it stays tight for scaled roots (t³ − 2·10³⁰ has its
    // roots at ~1.26·10¹⁰, not 10³⁰ — seeds must start near them or the
    // iteration cap is spent crossing twenty orders of magnitude).
    let radius = 2.0
        * (1..=deg)
            .map(|k| monic_coeffs[deg - k].abs().powf(1.0 / k as f64))
            .fold(f64::MIN_POSITIVE, f64::max);
    let seed = Complex64::new(0.4, 0.9);
    let mut zs: Vec<Complex64> = (0..deg)
        .map(|k| radius * seed.powu(k as u32 + 1) / seed.norm().powi(k as i32))
        .collect();
    for _ in 0..600 {
        let mut max_step = 0.0f64;
        for i in 0..deg {
            let mut denom = Complex64::ONE;
            for j in 0..deg {
                if i != j {
                    denom *= zs[i] - zs[j];
                }
            }
            if denom.norm() == 0.0 {
                // Coincident iterates: nudge and continue.
                zs[i] += Complex64::new(1e-6 * radius, 1e-6 * radius);
                continue;
            }
            let step = eval_c64(monic_coeffs, zs[i]) / denom;
            zs[i] -= step;
            max_step = max_step.max(step.norm());
        }
        if max_step < 1e-14 * radius {
            break;
        }
    }
    // Newton polish + residual check.
    let dcoeffs: Vec<f64> = monic_coeffs
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, c)| c * i as f64)
        .collect();
    for z in zs.iter_mut() {
        for _ in 0..3 {
            let d = eval_c64(&dcoeffs, *z);
            if d.norm() > 0.0 {
                *z -= eval_c64(monic_coeffs, *z) / d;
            }
        }
        if eval_c64(monic_coeffs, *z).norm() > 1e-6 * radius.powi(deg.min(30) as i32).max(1.0) {
            return None;
        }
    }
    Some(zs)
}

/// `t^n mod p` for the power-reduction rule (§2d): returns the dense
/// remainder coefficients (degree < deg p).
pub(crate) fn power_mod(n: u64, p: &[BigRational]) -> UPoly {
    let d = degree(p);
    if (n as usize) < d {
        let mut out = vec![BigRational::zero(); n as usize + 1];
        out[n as usize] = BigRational::one();
        return out;
    }
    // Square-and-multiply in ℚ[t]/(p).
    let reduce = |x: &UPoly| -> UPoly { divrem(x, p).1 };
    let mut base = reduce(&vec![BigRational::zero(), BigRational::one()]);
    let mut acc = vec![BigRational::one()];
    let mut k = n;
    while k > 0 {
        if k & 1 == 1 {
            acc = reduce(&mul(&acc, &base));
        }
        base = reduce(&mul(&base, &base));
        k >>= 1;
    }
    acc
}
