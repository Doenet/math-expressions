//! Exact-constant evaluation and the certified zero-equivalence service.
//!
//! Two public items:
//!
//! * [`Exact`] / [`exact_eval`] — a rigorous evaluator for real constants over
//!   the field ℚ adjoined with surds (√ of nonnegative rationals), π and e as
//!   transcendental generators, and the trig/exp/log special values that land
//!   in that field. It only ever returns a value it can *prove* correct;
//!   anything outside the tower yields `None`.
//! * [`is_zero`] — `is_zero(e, a) -> Tri` (`Some(true)` = certified zero,
//!   `Some(false)` = certified nonzero, `None` = undecided). Soundness is the
//!   invariant: it never answers `Some(_)` unless the answer is certain.
//!
//! ## Why the representation is sound
//!
//! A value is a ℚ-linear combination of the basis monomials `π^i · e^j · √r`
//! where `r` is a squarefree positive integer (`r = 1` ⇒ no surd). Those
//! monomials are linearly independent over ℚ: the surds `{√r : r squarefree}`
//! are ℚ-linearly independent (a standard result), and π, e are transcendental
//! (their algebraic independence from the surds — and, as every CAS assumes,
//! from each other — is taken as given). Hence the combination is zero **iff**
//! every coefficient is zero, which is exactly [`Exact::is_zero`].
//!
//! Multiplication stays in the ring because `√r · √s` re-normalizes to a
//! rational multiple of a single surd (`√12 = 2√3`); see [`mul_surd`].

use std::collections::BTreeMap;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::assumptions::{Assumptions, Tri};
use crate::expr::{Expr, MathConst};
use crate::num::Number;

/// A basis monomial `π^pi · e^e · √rad`. `rad` is a squarefree integer ≥ 1
/// (`rad == 1` means no surd factor).
type Mono = (u32, u32, BigInt);

/// An element of ℚ[π, e] ⊗ (ℚ-span of surds), as a sparse map from basis
/// monomial to rational coefficient. Zero coefficients are pruned, so the
/// value is zero exactly when the map is empty.
#[derive(Clone, Debug, Default)]
pub struct Exact {
    terms: BTreeMap<Mono, BigRational>,
}

fn br_int(n: i64) -> BigRational {
    BigRational::from_integer(BigInt::from(n))
}

impl Exact {
    fn zero() -> Exact {
        Exact::default()
    }

    /// The rational `q` (as `q · π^0 e^0 √1`).
    fn rat(q: BigRational) -> Exact {
        let mut t = BTreeMap::new();
        if !q.is_zero() {
            t.insert((0, 0, BigInt::one()), q);
        }
        Exact { terms: t }
    }

    /// A single monomial `coeff · π^pi · e^e · √rad` (rad already squarefree).
    fn mono(coeff: BigRational, pi: u32, e: u32, rad: BigInt) -> Exact {
        let mut t = BTreeMap::new();
        if !coeff.is_zero() {
            t.insert((pi, e, rad), coeff);
        }
        Exact { terms: t }
    }

    /// `coeff · √rad` for a rad that is **already squarefree** (the lattice
    /// tables pass literals 2/3/6; `eval_sqrt` passes the squarefree part it
    /// just computed). No factoring, no panic path — callers that hold a
    /// possibly-square-divisible radicand must go through [`squarefree_part`]
    /// first.
    fn surd(coeff: BigRational, squarefree_rad: u128) -> Exact {
        Exact::mono(coeff, 0, 0, BigInt::from(squarefree_rad))
    }

    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    fn add(&self, other: &Exact) -> Exact {
        let mut terms = self.terms.clone();
        for (k, v) in &other.terms {
            let e = terms.entry(k.clone()).or_insert_with(BigRational::zero);
            *e += v;
            if e.is_zero() {
                terms.remove(k);
            }
        }
        Exact { terms }
    }

    fn neg(&self) -> Exact {
        Exact {
            terms: self.terms.iter().map(|(k, v)| (k.clone(), -v)).collect(),
        }
    }

    fn mul(&self, other: &Exact, budget: &mut i64) -> Option<Exact> {
        let mut acc = Exact::zero();
        for ((p1, e1, r1), c1) in &self.terms {
            for ((p2, e2, r2), c2) in &other.terms {
                spend(budget)?;
                let (coeff, rad) = mul_surd(c1 * c2, r1, r2)?;
                let mono = Exact::mono(coeff, p1 + p2, e1 + e2, rad);
                acc = acc.add(&mono);
            }
        }
        Some(acc)
    }

    /// This value as a plain rational, if it has no π/e/surd part.
    fn as_rational(&self) -> Option<BigRational> {
        match self.terms.len() {
            0 => Some(BigRational::zero()),
            1 => {
                let ((p, e, r), c) = self.terms.iter().next().unwrap();
                (*p == 0 && *e == 0 && r.is_one()).then(|| c.clone())
            }
            _ => None,
        }
    }

    /// The rational `p` such that `self == p · π` (only the `π^1` monomial),
    /// or `0` when `self` is zero. `None` if any other component is present.
    fn as_pi_multiple(&self) -> Option<BigRational> {
        let mut p = BigRational::zero();
        for ((pi, e, r), c) in &self.terms {
            if *pi == 1 && *e == 0 && r.is_one() {
                p = c.clone();
            } else {
                return None;
            }
        }
        Some(p)
    }

    /// `1/self` when `self` is a nonzero pure rational or a single surd term
    /// `c·√r`; otherwise `None` (general field inversion is not implemented).
    fn inverse(&self) -> Option<Exact> {
        if let Some(q) = self.as_rational() {
            return (!q.is_zero()).then(|| Exact::rat(BigRational::one() / q));
        }
        if self.terms.len() == 1 {
            let ((p, e, r), c) = self.terms.iter().next().unwrap();
            if *p == 0 && *e == 0 && !r.is_one() && !c.is_zero() {
                // 1/(c·√r) = √r / (c·r)
                let denom = c * BigRational::from_integer(r.clone());
                return Some(Exact::mono(BigRational::one() / denom, 0, 0, r.clone()));
            }
        }
        None
    }

    /// This value as a canonical expression — the inverse direction of
    /// [`exact_eval`], used to emit exact special values such as
    /// `sin(pi/6) → 1/2`, `sec(pi/4) → sqrt(2)`, etc.
    pub(crate) fn to_expr(&self) -> Expr {
        if self.terms.is_empty() {
            return Expr::int(0);
        }
        let mut terms = Vec::with_capacity(self.terms.len());
        for ((pi, e, rad), c) in &self.terms {
            let mut factors: Vec<Expr> = vec![Expr::Num(Number::from_bigrational(c.clone()))];
            // π/e are emitted as `Sym` — the canonical spelling. (`Const(Pi)`
            // exists as a variant, but the parsers only produce `Sym`, and
            // canonicalize unifies `Const(Pi/E/I)` → `Sym`; minting `Sym`
            // directly keeps this output canonical without a re-pass.)
            if *pi > 0 {
                factors.push(crate::norm::pow(
                    Expr::sym("pi"),
                    Expr::int(i64::from(*pi)),
                ));
            }
            if *e > 0 {
                factors.push(crate::norm::pow(
                    Expr::sym("e"),
                    Expr::int(i64::from(*e)),
                ));
            }
            if !rad.is_one() {
                let radn = Expr::Num(Number::from_bigrational(BigRational::from_integer(rad.clone())));
                let half = Expr::Num(Number::from_bigrational(BigRational::new(
                    BigInt::one(),
                    BigInt::from(2),
                )));
                factors.push(crate::norm::pow(radn, half));
            }
            terms.push(crate::norm::mul(factors));
        }
        crate::norm::canonicalize(&crate::norm::add(terms))
    }

    fn pow_int(&self, k: i64, budget: &mut i64) -> Option<Exact> {
        if k == 0 {
            return Some(Exact::rat(BigRational::one()));
        }
        let base = if k < 0 { self.inverse()? } else { self.clone() };
        let mut acc = Exact::rat(BigRational::one());
        for _ in 0..k.unsigned_abs() {
            spend(budget)?;
            acc = acc.mul(&base, budget)?;
        }
        Some(acc)
    }
}

/// `coeff · √r1 · √r2` re-normalized to `(coeff', r')` with `r'` squarefree.
fn mul_surd(coeff: BigRational, r1: &BigInt, r2: &BigInt) -> Option<(BigRational, BigInt)> {
    if r1.is_one() {
        return Some((coeff, r2.clone()));
    }
    if r2.is_one() {
        return Some((coeff, r1.clone()));
    }
    let prod = (r1 * r2).to_u128()?;
    let (s, f) = squarefree_part(prod)?;
    Some((coeff * BigRational::from_integer(BigInt::from(s)), BigInt::from(f)))
}

/// Write `m = s²·f` with `f` squarefree; return `(s, f)`. `None` if `m` is 0
/// or its square-factoring would exceed the trial-division budget.
fn squarefree_part(mut m: u128) -> Option<(u128, u128)> {
    if m == 0 {
        return None;
    }
    let mut s: u128 = 1;
    let mut d: u128 = 2;
    while let Some(dd) = d.checked_mul(d) {
        if dd > m {
            break;
        }
        while m.is_multiple_of(dd) {
            m /= dd;
            s = s.checked_mul(d)?;
        }
        d += 1;
        // Cap trial division so a large near-prime radicand can't stall us —
        // §7f-governed like every other unpredictable-cost bound.
        if d > u128::from(crate::resource_limits::current().max_squarefree_trial_divisor) {
            return None;
        }
    }
    Some((s, m))
}

fn spend(budget: &mut i64) -> Option<()> {
    *budget -= 1;
    (*budget >= 0).then_some(())
}

fn apply1<'a>(e: &'a Expr, name: &str) -> Option<&'a Expr> {
    if let Expr::Apply(head, args) = e {
        if let (Expr::Sym(s), [u]) = (&**head, args.as_slice()) {
            if s.name() == name {
                return Some(u);
            }
        }
    }
    None
}

fn is_e(e: &Expr) -> bool {
    matches!(e, Expr::Const(MathConst::E)) || matches!(e, Expr::Sym(s) if s.name() == "e")
}

fn is_pi(e: &Expr) -> bool {
    matches!(e, Expr::Const(MathConst::Pi)) || matches!(e, Expr::Sym(s) if s.name() == "pi")
}

/// Evaluate `e` to an [`Exact`] value, or `None` if it falls outside the tower.
pub fn exact_eval(e: &Expr) -> Option<Exact> {
    let mut budget = crate::resource_limits::current().max_exact_eval_ops;
    eval(e, &mut budget)
}

fn eval(e: &Expr, budget: &mut i64) -> Option<Exact> {
    spend(budget)?;
    Some(match e {
        Expr::Num(n) => Exact::rat(n.to_bigrational()?),
        _ if is_pi(e) => Exact::mono(BigRational::one(), 1, 0, BigInt::one()),
        _ if is_e(e) => Exact::mono(BigRational::one(), 0, 1, BigInt::one()),
        Expr::Add(ts) => {
            let mut acc = Exact::zero();
            for t in ts {
                acc = acc.add(&eval(t, budget)?);
            }
            acc
        }
        Expr::Mul(fs) => {
            let mut acc = Exact::rat(BigRational::one());
            for f in fs {
                let v = eval(f, budget)?;
                acc = acc.mul(&v, budget)?;
            }
            acc
        }
        Expr::Pow(b, k) => eval_pow(b, k, budget)?,
        Expr::Apply(..) => eval_apply(e, budget)?,
        _ => return None,
    })
}

fn eval_pow(b: &Expr, k: &Expr, budget: &mut i64) -> Option<Exact> {
    // e^x
    if is_e(b) {
        return eval_exp(k, budget);
    }
    let Expr::Num(n) = k else { return None };
    // Integer exponent.
    if let Some(i) = n.to_bigrational().and_then(|q| q.is_integer().then(|| q.to_integer())) {
        return eval(b, budget)?.pow_int(i.to_i64()?, budget);
    }
    // Half-integer exponent ⇒ (inverse) square root of a nonnegative rational.
    let q = n.to_bigrational()?;
    let two = BigRational::from_integer(BigInt::from(2));
    if q == BigRational::one() / &two {
        return eval_sqrt(b, budget);
    }
    if q == -BigRational::one() / &two {
        return eval_sqrt(b, budget)?.inverse();
    }
    None
}

fn eval_sqrt(arg: &Expr, budget: &mut i64) -> Option<Exact> {
    let q = eval(arg, budget)?.as_rational()?;
    if q.is_negative() {
        return None; // complex — out of the real tower
    }
    if q.is_zero() {
        return Some(Exact::zero());
    }
    // √(n/d) = √(n·d)/d.
    let n = q.numer().to_u128()?;
    let d = q.denom().to_u128()?;
    let (s, f) = squarefree_part(n.checked_mul(d)?)?;
    let coeff = BigRational::new(BigInt::from(s), BigInt::from(d));
    Some(Exact::surd(coeff, f))
}

fn eval_exp(arg: &Expr, budget: &mut i64) -> Option<Exact> {
    // e^{ln u} = u.
    if let Some(u) = apply1(arg, "log").or_else(|| apply1(arg, "ln")) {
        return eval(u, budget);
    }
    let v = eval(arg, budget)?;
    (v.as_rational()? == BigRational::zero()).then(|| Exact::rat(BigRational::one()))
}

fn eval_log(arg: &Expr, budget: &mut i64) -> Option<Exact> {
    // ln(e^u) = u.
    if let Some(u) = apply1(arg, "exp") {
        return eval(u, budget);
    }
    if let Expr::Pow(b, x) = arg {
        if is_e(b) {
            return eval(x, budget);
        }
    }
    if is_e(arg) {
        return Some(Exact::rat(BigRational::one()));
    }
    let v = eval(arg, budget)?;
    (v.as_rational()? == BigRational::one()).then(Exact::zero)
}

fn eval_apply(e: &Expr, budget: &mut i64) -> Option<Exact> {
    let Expr::Apply(head, args) = e else { return None };
    let (Expr::Sym(s), [arg]) = (&**head, args.as_slice()) else {
        return None;
    };
    let name = s.name();
    match name.as_str() {
        "sin" | "cos" | "tan" => eval_trig(&name, arg, budget),
        "sqrt" => eval_sqrt(arg, budget),
        "exp" => eval_exp(arg, budget),
        "log" | "ln" => eval_log(arg, budget),
        "abs" => Some(Exact::rat(eval(arg, budget)?.as_rational()?.abs())),
        // These fold only at their known zero: sinh/tanh/asin/atan(0)=0.
        "sinh" | "tanh" | "asin" | "atan" => {
            (eval(arg, budget)?.as_rational()? == BigRational::zero()).then(Exact::zero)
        }
        "cosh" => (eval(arg, budget)?.as_rational()? == BigRational::zero())
            .then(|| Exact::rat(BigRational::one())),
        _ => None,
    }
}

/// sin/cos/tan at a rational multiple of π on the π/12 lattice (covers the
/// kπ/6 and kπ/4 lattices). Returns `None` for arguments off the lattice or,
/// for tan, at a pole.
fn eval_trig(name: &str, arg: &Expr, budget: &mut i64) -> Option<Exact> {
    let p = eval(arg, budget)?.as_pi_multiple()?;
    // Angle in units of π/12: t = 12·p, must be an integer.
    let t = p * BigRational::from_integer(BigInt::from(12));
    if !t.is_integer() {
        return None;
    }
    let ti = t.to_integer();
    let idx = |modulus: i64| -> usize {
        let m = BigInt::from(modulus);
        (((ti.clone() % &m) + &m) % &m).to_i64().unwrap() as usize
    };
    match name {
        "sin" => Some(sin_lattice(idx(24))),
        "cos" => Some(sin_lattice((idx(24) + 6) % 24)), // cos θ = sin(θ + 90°)
        "tan" => tan_lattice(idx(12)),
        _ => None,
    }
}

/// The exact value of `name(arg)` when `arg` is a rational multiple of π on the
/// π/12 lattice (`sin(pi/6) → 1/2`), as a canonical expression, or `None` off
/// the lattice, at a pole, or when the reciprocal value falls outside the
/// single-term inversion supported here.
pub(crate) fn trig_special_value(name: &str, arg: &Expr) -> Option<Expr> {
    let mut budget = crate::resource_limits::current().max_exact_eval_ops;
    let v = match name {
        "sin" | "cos" | "tan" => eval_trig(name, arg, &mut budget)?,
        "cot" => eval_trig("tan", arg, &mut budget)?.inverse()?,
        "sec" => eval_trig("cos", arg, &mut budget)?.inverse()?,
        "csc" => eval_trig("sin", arg, &mut budget)?.inverse()?,
        _ => return None,
    };
    Some(v.to_expr())
}

/// sin at k·15°, k ∈ 0..24. Uses the 0..12 table and sin(θ+180°) = −sin θ.
fn sin_lattice(k: usize) -> Exact {
    if k >= 12 {
        return sin_lattice(k - 12).neg();
    }
    let q = |a, b| BigRational::new(BigInt::from(a), BigInt::from(b));
    // (√6 ± √2)/4
    let s6p2 = Exact::surd(q(1, 4), 6).add(&Exact::surd(q(1, 4), 2));
    let s6m2 = Exact::surd(q(1, 4), 6).add(&Exact::surd(q(-1, 4), 2));
    match k {
        0 => Exact::zero(),
        1 => s6m2,
        2 => Exact::rat(q(1, 2)),
        3 => Exact::surd(q(1, 2), 2),
        4 => Exact::surd(q(1, 2), 3),
        5 => s6p2,
        6 => Exact::rat(BigRational::one()),
        7 => s6p2,
        8 => Exact::surd(q(1, 2), 3),
        9 => Exact::surd(q(1, 2), 2),
        10 => Exact::rat(q(1, 2)),
        11 => s6m2,
        _ => unreachable!(),
    }
}

/// tan at k·15°, k ∈ 0..12 (tan has period π = 12 units). `None` at the pole.
fn tan_lattice(k: usize) -> Option<Exact> {
    let q = |a, b| BigRational::new(BigInt::from(a), BigInt::from(b));
    Some(match k {
        0 => Exact::zero(),
        1 => Exact::rat(br_int(2)).add(&Exact::surd(br_int(-1), 3)), // 2 − √3
        2 => Exact::surd(q(1, 3), 3),                                // √3/3
        3 => Exact::rat(BigRational::one()),
        4 => Exact::surd(BigRational::one(), 3), // √3
        5 => Exact::rat(br_int(2)).add(&Exact::surd(BigRational::one(), 3)), // 2 + √3
        6 => return None,                        // pole (90°)
        7 => Exact::rat(br_int(-2)).add(&Exact::surd(br_int(-1), 3)), // −(2 + √3)
        8 => Exact::surd(br_int(-1), 3),         // −√3
        9 => Exact::rat(br_int(-1)),
        10 => Exact::surd(q(-1, 3), 3), // −√3/3
        11 => Exact::rat(br_int(-2)).add(&Exact::surd(BigRational::one(), 3)), // −(2 − √3)
        _ => unreachable!(),
    })
}

// ===================== the zero-equivalence service =====================

/// Certified test for `e ≡ 0`: `Some(true)` = provably zero, `Some(false)` =
/// provably nonzero, `None` = undecided.
///
/// The `_a` assumptions are accepted for forward compatibility (sign/realness
/// reasoning is not yet implemented) but not yet consulted.
pub fn is_zero(e: &Expr, _a: &Assumptions) -> Tri {
    let c = crate::norm::canonicalize(&crate::norm::expand(e));
    let vars = free_vars(&c);
    if let Some(v) = certify_canonical(&c, &vars) {
        return Some(v);
    }
    if vars.is_empty() {
        None
    } else {
        // (c) certified refuter: a single certified-nonzero sample proves the
        // expression is not identically zero. Sampling can never *confirm*
        // zero, so this stage only ever yields `Some(false)` or `None`.
        refute_by_sampling(&c, &vars)
    }
}

/// Accept-only fast path of [`is_zero`]: `true` iff `e` is *certified*
/// identically zero by the exact stages alone — no numeric sampling. `false`
/// means "not certified", **not** "nonzero". The right gate for callers with
/// their own cheaper rejection test (e.g. the integration gate), since the
/// sampling refuter burns its full arbitrary-precision budget precisely when
/// the expression *is* zero.
pub(crate) fn certified_zero(e: &Expr, _a: &Assumptions) -> bool {
    let c = crate::norm::canonicalize(&crate::norm::expand(e));
    let vars = free_vars(&c);
    certify_canonical(&c, &vars) == Some(true)
}

/// The non-sampling certification pipeline on a canonical, expanded input:
/// (a) structural cancellation, (d) rational normal form, (b) exact constant
/// evaluation (variable-free only — the one stage that can also certify
/// *nonzero*), then the RootOf reducer. `None` = undecided.
fn certify_canonical(c: &Expr, vars: &[String]) -> Tri {
    // (a) structural: expand + canonicalize caught polynomial identities.
    if matches!(c, Expr::Num(n) if n.is_zero()) {
        return Some(true);
    }
    // (d) rational-function normalization (S2): a rational identity whose
    // combined numerator cancels to zero is certified zero — this decides
    // `1/(x+1) + 1/(x-1) - 2x/(x²-1)` and the like, treating opaque kernels as
    // independent indeterminates (sound: never `true` for a nonzero value).
    if crate::ratform::is_identically_zero(c) {
        return Some(true);
    }
    if vars.is_empty() {
        // (b) exact constant evaluation. Failure falls through to the RootOf
        // decider, then to Unknown (adversarial almost-zeros land there —
        // never a wrong `Some(true)`).
        if let Some(v) = exact_eval(c) {
            return Some(v.is_zero());
        }
        return rootof_is_zero(c);
    }
    None
}

/// The non-constant free variable names of `c`.
fn free_vars(c: &Expr) -> Vec<String> {
    crate::ops::variables(c)
        .into_iter()
        .filter(|v| !crate::sym::is_constant_symbol(v))
        .collect()
}

/// Deterministic "random" rational sample points, chosen to dodge common
/// removable structure (small integers, halves) and singularities at 0.
const SAMPLE_POINTS: &[(i64, i64)] = &[
    (7, 3),
    (-11, 5),
    (13, 4),
    (2, 7),
    (-17, 6),
    (23, 8),
];

fn refute_by_sampling(e: &Expr, vars: &[String]) -> Tri {
    use std::collections::HashMap;
    for round in 0..SAMPLE_POINTS.len() {
        let subs: HashMap<String, Expr> = vars
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let (n, d) = SAMPLE_POINTS[(round + i) % SAMPLE_POINTS.len()];
                (v.clone(), Expr::Num(Number::from_bigrational(BigRational::new(n.into(), d.into()))))
            })
            .collect();
        let at = crate::ops::substitute(e, &subs);
        match crate::precise::evaluate_to_precision(&at, 15) {
            crate::precise::Precise::Exact(n) if !n.is_zero() => return Some(false),
            crate::precise::Precise::Bounded(m) if certified_nonzero(&m) => return Some(false),
            _ => {}
        }
    }
    None
}

/// The ±1-ulp arbitrary-precision contract, via the single shared test on
/// `MpFix` (see `MpFix::excludes_zero`).
fn certified_nonzero(m: &crate::precise::fix::MpFix) -> bool {
    m.excludes_zero()
}

// ===================== RootOf algebraic identities =====================

/// Certified zero test for an expression built from rationals and a *single*
/// `RootOf` leaf `α` by `+`, `*`, and nonnegative integer powers.
///
/// The value is folded into `ℚ[t]/(p)`, where `p` is `α`'s defining
/// polynomial, by reducing modulo `p` at every step. If the result is the zero
/// polynomial then the value is `q(t)·p(t)` for some `q`, so it vanishes at
/// `α` (`p(α) = 0`) — sound **regardless of whether `p` is irreducible**. The
/// converse needs irreducibility, so a nonzero remainder yields `None`, never
/// `Some(false)`.
fn rootof_is_zero(e: &Expr) -> Tri {
    let root = unique_rootof(e)?;
    let Expr::RootOf { poly, .. } = &root else {
        return None;
    };
    let p = crate::rootof::coeffs_to_upoly(poly)?;
    if crate::upoly::degree(&p) == 0 {
        return None;
    }
    let mut budget = crate::resource_limits::current().max_exact_eval_ops;
    let r = fold_rootof(e, &p, &mut budget)?;
    crate::upoly::is_zero(&r).then_some(true)
}

/// The one distinct `RootOf` leaf in `e`, or `None` if there are none or more
/// than one (a compositum of distinct algebraics is not supported).
fn unique_rootof(e: &Expr) -> Option<Expr> {
    fn walk(e: &Expr, found: &mut Option<Expr>, multiple: &mut bool) {
        if let Expr::RootOf { .. } = e {
            match found {
                None => *found = Some(e.clone()),
                Some(prev) if prev == e => {}
                Some(_) => *multiple = true,
            }
        }
        for c in e.children() {
            walk(c, found, multiple);
        }
    }
    let (mut found, mut multiple) = (None, false);
    walk(e, &mut found, &mut multiple);
    (!multiple).then_some(found).flatten()
}

/// Fold `e` into its coefficient vector in `ℚ[t]/(p)` (low → high), reducing
/// modulo `p` after each operation.
fn fold_rootof(e: &Expr, p: &[BigRational], budget: &mut i64) -> Option<Vec<BigRational>> {
    spend(budget)?;
    let reduce = |a: Vec<BigRational>| crate::upoly::divrem(&a, p).1;
    Some(match e {
        Expr::Num(n) => vec![n.to_bigrational()?],
        Expr::RootOf { .. } => reduce(vec![BigRational::zero(), BigRational::one()]),
        Expr::Add(ts) => {
            let mut acc = Vec::new();
            for t in ts {
                acc = crate::upoly::add_p(&acc, &fold_rootof(t, p, budget)?);
            }
            reduce(acc)
        }
        Expr::Mul(fs) => {
            let mut acc = vec![BigRational::one()];
            for f in fs {
                acc = reduce(crate::upoly::mul(&acc, &fold_rootof(f, p, budget)?));
            }
            acc
        }
        Expr::Pow(b, k) => {
            let Expr::Num(n) = &**k else { return None };
            let ki = n
                .to_bigrational()
                .and_then(|q| q.is_integer().then(|| q.to_integer()))?
                .to_i64()?;
            if ki < 0 {
                return None;
            }
            let base = fold_rootof(b, p, budget)?;
            let mut acc = vec![BigRational::one()];
            for _ in 0..ki {
                spend(budget)?;
                acc = reduce(crate::upoly::mul(&acc, &base));
            }
            acc
        }
        _ => return None,
    })
}
