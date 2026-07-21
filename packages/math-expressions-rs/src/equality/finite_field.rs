//! Finite-field rejection filter (PORTING_PLAN.md §10 stage 2, ported from
//! `lib/expression/equality/finite-field.js` + `converters/ast-to-finite-field.js`).
//!
//! Both expressions are evaluated *exactly* in ℤ/pℤ for several primes `p`, with
//! variables bound to random field elements. Exact modular arithmetic has no
//! floating-point magnitude, so an additive/structural difference is always
//! caught — `e^(10x)` and `e^(10x)+C` evaluate to different field elements even
//! though the `+C` is negligible in floating point. This is the exact filter
//! that lets the complex sampler be *lenient* (accept branch-cut identities)
//! without also accepting those near-misses.
//!
//! The construction that makes exp/trig identities hold in the field: `e` is a
//! **primitive root** `g`, so `exp(x) = g^x` and `sin`/`cos` are the field
//! analogues via a 4th root of unity `i = g^((p-1)/4)`. `log` is *not* modelled
//! (returns NaN → this filter stays silent on log identities, leaving them to
//! the sampler). Anything the field can't evaluate (log, factorial, unknown
//! applications, `pi` for odd `p`) yields NaN and that prime is skipped.

use crate::eval::{free_symbols, is_opaque_atom, opaque_key};
use crate::expr::{Expr, MathConst};
use crate::num::Number;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;

/// Primes used by JS (`component_equals`). All are ≡ 1 (mod 4) so a 4th root of
/// unity exists for the trig construction.
const PRIMES: [i64; 9] = [1181, 1187, 1193, 1201, 1213, 1217, 1223, 1229, 1231];

/// Does the finite-field filter *definitively* find `a` and `b` unequal? Only a
/// `true` return is meaningful (reject); `false` means "undetermined" (the field
/// couldn't evaluate, or they agreed at every prime). Never confirms equality.
pub fn definitely_unequal(a: &Expr, b: &Expr) -> bool {
    // Collect the shared variable names once (opaque-atom keys are harmless — the
    // field NaNs unknown applications before any such lookup happens).
    let mut names = std::collections::BTreeSet::new();
    free_symbols(a, &mut names);
    free_symbols(b, &mut names);
    let names: Vec<String> = names.into_iter().collect();

    // A fixed seed keeps this reproducible (and off OS entropy, for wasm).
    let mut rng = SmallRng::seed_from_u64(0xF17E_F1E1_D0D0_0001);
    for &p in &PRIMES {
        let bindings: HashMap<String, i64> = names
            .iter()
            .map(|v| (v.clone(), rng.random_range(0..p)))
            .collect();
        let fa = eval(a, &bindings, p);
        let fb = eval(b, &bindings, p);
        if let (Some(va), Some(vb)) = (fa.usable(), fb.usable()) {
            // Values are multisets (a field sqrt has two roots), so the results
            // are only *definitely* unequal when they share no possible value —
            // if the sets overlap, some branch could make them equal.
            if va.iter().all(|x| !vb.contains(x)) {
                return true; // disjoint in ℤ/pℤ ⇒ definitely unequal
            }
        }
    }
    false
}

/// Evaluate `e` in ℤ/`modulus`ℤ with the given variable bindings — the port of
/// `me.finite_field_evaluate`. Returns the possible residues (more than one
/// when a field square root is involved), or `None` when the field cannot
/// represent the expression (`log`, `factorial`, an unknown application, or
/// `pi` for an odd modulus). Exponents are reduced mod φ(`modulus`), as in the
/// JS converter.
pub fn finite_field_evaluate(
    e: &Expr,
    bindings: &HashMap<String, i64>,
    modulus: i64,
) -> Option<Vec<i64>> {
    eval(e, bindings, modulus).usable()
}

/// A value in ℤ/`modulus`ℤ. Multivalued because a field square root has two
/// roots; NaN marks "this field can't represent it" (skip the prime).
#[derive(Clone)]
struct Ff {
    vals: Vec<i64>,
    modulus: i64,
    nan: bool,
}

impl Ff {
    fn num(v: i64, modulus: i64) -> Ff {
        Ff {
            vals: vec![v.rem_euclid(modulus)],
            modulus,
            nan: false,
        }
    }
    fn nan() -> Ff {
        Ff {
            vals: vec![],
            modulus: 1,
            nan: true,
        }
    }

    /// The comparable value set, sorted and deduped — `None` if NaN or empty, so
    /// the caller skips this prime (matches JS's `!isNaN && values.length > 0`).
    fn usable(&self) -> Option<Vec<i64>> {
        if self.nan || self.vals.is_empty() {
            return None;
        }
        let mut v = self.vals.clone();
        v.sort_unstable();
        v.dedup();
        Some(v)
    }

    /// Combine two values elementwise (Cartesian product over the value sets)
    /// under `op`, taken mod the given modulus. Mirrors JS `ZmodN.apply`.
    fn combine(&self, other: &Ff, modulus: i64, op: impl Fn(i64, i64) -> Option<i64>) -> Ff {
        if self.nan || other.nan {
            return Ff::nan();
        }
        let mut vals = Vec::new();
        for &x in &self.vals {
            for &y in &other.vals {
                match op(x, y) {
                    Some(r) => vals.push(r.rem_euclid(modulus)),
                    None => return Ff::nan(),
                }
            }
        }
        Ff {
            vals,
            modulus,
            nan: false,
        }
    }

    fn add(&self, o: &Ff) -> Ff {
        let m = gcd(self.modulus, o.modulus);
        self.combine(o, m, |x, y| Some(x + y))
    }
    fn sub(&self, o: &Ff) -> Ff {
        let m = gcd(self.modulus, o.modulus);
        self.combine(o, m, |x, y| Some(x - y))
    }
    fn mul(&self, o: &Ff) -> Ff {
        let m = gcd(self.modulus, o.modulus);
        self.combine(o, m, |x, y| Some(mul_mod(x, y, m)))
    }
    fn div(&self, o: &Ff) -> Ff {
        let m = gcd(self.modulus, o.modulus);
        // this / other, elementwise (numerator ÷ denominator).
        self.combine(o, m, |b, a| {
            if a != 0 && b % a == 0 {
                Some(b / a) // exact integer quotient (also covers pi/2 etc.)
            } else {
                inv_mod(a.rem_euclid(m), m).map(|ai| mul_mod(b.rem_euclid(m), ai, m))
            }
        })
    }
    fn neg(&self) -> Ff {
        if self.nan {
            return Ff::nan();
        }
        Ff {
            vals: self
                .vals
                .iter()
                .map(|x| (self.modulus - x) % self.modulus)
                .collect(),
            modulus: self.modulus,
            nan: false,
        }
    }

    /// `self ^ other`, where the exponent must live mod φ(self.modulus) — JS
    /// enforces this exactly, and NaNs otherwise.
    fn pow(&self, other: &Ff) -> Ff {
        if self.nan || other.nan {
            return Ff::nan();
        }
        if other.modulus != euler_phi(self.modulus) {
            return Ff::nan();
        }
        let m = self.modulus;
        self.combine(other, m, |x, y| Some(pow_mod(x.rem_euclid(m), y, m)))
    }

    fn sqrt(&self) -> Ff {
        if self.nan {
            return Ff::nan();
        }
        let m = self.modulus;
        let mut vals = Vec::new();
        for &x in &self.vals {
            vals.extend(sqrt_mod(x.rem_euclid(m), m));
        }
        Ff {
            vals,
            modulus: m,
            nan: false,
        }
    }
}

/// Evaluate `e` in ℤ/`modulus`ℤ at `bindings`. Exponent subtrees are evaluated
/// mod φ(modulus), as in the JS converter.
fn eval(e: &Expr, bindings: &HashMap<String, i64>, modulus: i64) -> Ff {
    // Opaque subtrees (subscripts `y_t`, primes, `OtherOp`, unknown applications)
    // are sampled as a single field variable keyed by structure — the same
    // treatment as the complex evaluator, so `e^(10 y_t)` and `e^(10 y_t)+C` are
    // caught. The key was bound in `definitely_unequal`.
    if is_opaque_atom(e) {
        return match bindings.get(&opaque_key(e)) {
            Some(&v) => Ff::num(v, modulus),
            None => Ff::nan(),
        };
    }
    match e {
        Expr::Num(n) => number(n, modulus),
        Expr::Const(MathConst::Pi) => pi(modulus),
        Expr::Const(MathConst::E) => e_const(modulus),
        Expr::Const(MathConst::I) => Ff::nan(),
        Expr::Const(_) => Ff::nan(),
        Expr::Sym(s) => match s.name().as_str() {
            "e" => e_const(modulus),
            "pi" => pi(modulus),
            "i" => Ff::nan(),
            name => match bindings.get(name) {
                Some(&v) => Ff::num(v, modulus),
                None => Ff::nan(),
            },
        },
        Expr::Add(xs) => {
            let mut acc = Ff::num(0, modulus);
            for x in xs {
                acc = acc.add(&eval(x, bindings, modulus));
            }
            acc
        }
        Expr::Mul(xs) => {
            let mut acc = Ff::num(1, modulus);
            for x in xs {
                acc = acc.mul(&eval(x, bindings, modulus));
            }
            acc
        }
        Expr::Neg(x) => eval(x, bindings, modulus).neg(),
        Expr::Div(a, b) => eval(a, bindings, modulus).div(&eval(b, bindings, modulus)),
        Expr::Pow(b, ex) => {
            let base = eval(b, bindings, modulus);
            // A negative-integer exponent is a reciprocal `1 / base^|k|`. Do it
            // as an explicit division so a zero base is a pole (NaN), not `0` —
            // `pow_mod(0, p-2, p)` would otherwise silently give 0. (Canonical
            // form encodes every `/` this way.)
            if let Expr::Num(Number::Int(k)) = ex.as_ref() {
                if *k < 0 {
                    // checked_neg: `-i64::MIN` would overflow-panic in debug.
                    let Some(abs) = k.checked_neg() else {
                        return Ff::nan();
                    };
                    let pow = base.pow(&Ff::num(abs, euler_phi(modulus)));
                    return Ff::num(1, modulus).div(&pow);
                }
            }
            // Exponent lives mod φ(modulus).
            let exp = eval(ex, bindings, euler_phi(modulus));
            // Pole-conservatism for a zero base with a non-literal exponent:
            // since nested-pow flattening, `1/x^a` canonicalizes to
            // `Pow(x, Mul(-1, a))`, so the reciprocal no longer announces
            // itself with a literal negative exponent — a sampled binding of 0
            // would silently give `0` where the true value is a pole. Any
            // symbolic exponent may represent a negative power, so treat
            // 0^symbolic as NaN (the prime is skipped; the filter only ever
            // gets *less* aggressive, which is the safe direction for a
            // rejection-only stage).
            if base.vals.contains(&0) && !matches!(ex.as_ref(), Expr::Num(Number::Int(k)) if *k >= 0)
            {
                return Ff::nan();
            }
            base.pow(&exp)
        }
        Expr::Apply(head, args) => eval_apply(head, args, bindings, modulus),
        _ => Ff::nan(),
    }
}

fn eval_apply(head: &Expr, args: &[Expr], bindings: &HashMap<String, i64>, modulus: i64) -> Ff {
    // A modified head `sin^n(x)` means `sin(x)^n` — apply the inner function,
    // then raise (exponent mod φ). This keeps `sin^2(x)+cos^2(x) = 1` a field
    // identity, so `f + sin^2(x)+cos^2(x)` differs from `f` by exactly 1.
    if let Expr::Pow(inner, exp) = head {
        let base = eval_apply(inner, args, bindings, modulus);
        let e = eval(exp, bindings, euler_phi(modulus));
        return base.pow(&e);
    }
    let Expr::Sym(s) = head else { return Ff::nan() };
    let [arg] = args else { return Ff::nan() };
    match s.name().as_str() {
        // exp(x) = g^x, exponent mod φ(p).
        "exp" => {
            let g = e_const(modulus);
            let x = eval(arg, bindings, euler_phi(modulus));
            g.pow(&x)
        }
        "sqrt" => eval(arg, bindings, modulus).sqrt(),
        "abs" => {
            let v = eval(arg, bindings, modulus);
            v.mul(&v).sqrt()
        }
        // Reciprocal trig in terms of the primary ones.
        "sec" => recip(cos(arg, bindings, modulus)),
        "csc" => recip(sin(arg, bindings, modulus)),
        "cot" => recip(tan(arg, bindings, modulus)),
        "tan" => tan(arg, bindings, modulus),
        "sin" => sin(arg, bindings, modulus),
        "cos" => cos(arg, bindings, modulus),
        // log, factorial, gamma, and unknown functions are not modelled.
        _ => Ff::nan(),
    }
}

fn recip(v: Ff) -> Ff {
    Ff::num(1, v.modulus).div(&v)
}

/// cos(x) = (g^x + g^(-x)) / 2, with the exponent mod φ(p).
fn cos(arg: &Expr, bindings: &HashMap<String, i64>, modulus: i64) -> Ff {
    let g = e_const(modulus);
    if g.nan {
        return Ff::nan();
    }
    let x = eval(arg, bindings, euler_phi(modulus));
    let gx = g.pow(&x);
    let gnx = g.pow(&x.neg());
    gx.add(&gnx).div(&Ff::num(2, modulus))
}

/// sin(x) = (g^x - g^(-x)) / (2i), where i = g^((p-1)/4) is a 4th root of unity.
fn sin(arg: &Expr, bindings: &HashMap<String, i64>, modulus: i64) -> Ff {
    let root = match primitive_root(modulus) {
        Some(r) => r,
        None => return Ff::nan(),
    };
    let phi = euler_phi(modulus);
    if phi % 4 != 0 {
        return Ff::nan();
    }
    let g = Ff::num(root, modulus);
    let i = Ff::num(pow_mod(root, phi / 4, modulus), modulus);
    let x = eval(arg, bindings, phi);
    let gx = g.pow(&x);
    let gnx = g.pow(&x.neg());
    gx.sub(&gnx).div(&i.add(&i))
}

fn tan(arg: &Expr, bindings: &HashMap<String, i64>, modulus: i64) -> Ff {
    sin(arg, bindings, modulus).div(&cos(arg, bindings, modulus))
}

fn number(n: &Number, modulus: i64) -> Ff {
    match n {
        Number::Int(v) => Ff::num(*v, modulus),
        // A large reduced denominator marks a decimal that is really a float
        // approximation of a transcendental (e.g. `3.141592653589793` = π); exact
        // field arithmetic would wrongly separate two such near-equal decimals, so
        // treat it as unrepresentable (skip). Genuine small fractions stay exact,
        // which is what lets the field reject `0.33 ≠ 1/3`. Mirrors JS's
        // `rationalApproximation` `approximate` flag.
        Number::Rat(num, den) if den.unsigned_abs() <= 1_000_000_000 => {
            Ff::num(*num, modulus).div(&Ff::num(*den, modulus))
        }
        // Big integers/rationals, high-precision decimals, and evaluation floats
        // are not modelled exactly.
        _ => Ff::nan(),
    }
}

fn e_const(modulus: i64) -> Ff {
    match primitive_root(modulus) {
        Some(g) => Ff::num(g, modulus),
        None => Ff::nan(),
    }
}

fn pi(modulus: i64) -> Ff {
    if modulus % 2 == 0 {
        Ff::num(modulus / 2, modulus)
    } else {
        Ff::nan()
    }
}

// ---- Number theory over i64 (moduli here are ≤ 1231, so products fit) ----

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

fn mul_mod(a: i64, b: i64, m: i64) -> i64 {
    (a.rem_euclid(m) * b.rem_euclid(m)).rem_euclid(m)
}

fn pow_mod(mut base: i64, mut exp: i64, m: i64) -> i64 {
    if m == 1 {
        return 0;
    }
    base = base.rem_euclid(m);
    let mut result = 1i64;
    while exp > 0 {
        if exp & 1 == 1 {
            result = mul_mod(result, base, m);
        }
        base = mul_mod(base, base, m);
        exp >>= 1;
    }
    result
}

/// Modular inverse via the extended Euclidean algorithm; `None` if `a` and `m`
/// are not coprime.
fn inv_mod(a: i64, m: i64) -> Option<i64> {
    let (mut old_r, mut r) = (a.rem_euclid(m), m);
    let (mut old_s, mut s) = (1i64, 0i64);
    while r != 0 {
        let q = old_r / r;
        (old_r, r) = (r, old_r - q * r);
        (old_s, s) = (s, old_s - q * s);
    }
    if old_r != 1 {
        None
    } else {
        Some(old_s.rem_euclid(m))
    }
}

/// Euler's totient. Fast path for our prime moduli (φ(p) = p−1); general
/// factorization otherwise (used for φ(p−1) when an exponent contains `e`).
fn euler_phi(mut n: i64) -> i64 {
    if n <= 1 {
        return 1;
    }
    let mut result = n;
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            while n % d == 0 {
                n /= d;
            }
            result -= result / d;
        }
        d += 1;
    }
    if n > 1 {
        result -= result / n;
    }
    result
}

/// Distinct prime factors of `n`.
fn prime_factors(mut n: i64) -> Vec<i64> {
    let mut fs = Vec::new();
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            fs.push(d);
            while n % d == 0 {
                n /= d;
            }
        }
        d += 1;
    }
    if n > 1 {
        fs.push(n);
    }
    fs
}

/// Smallest primitive root of a prime `p` (a generator of (ℤ/pℤ)*). `None` when
/// `p` is not prime with a primitive root (composite moduli that arise only in
/// rare exponent positions), which the caller treats as NaN.
fn primitive_root(p: i64) -> Option<i64> {
    if p < 2 {
        return None;
    }
    if p == 2 {
        return Some(1);
    }
    if !is_prime(p) {
        return None;
    }
    let phi = p - 1;
    let factors = prime_factors(phi);
    (2..p).find(|&g| factors.iter().all(|&q| pow_mod(g, phi / q, p) != 1))
}

fn is_prime(n: i64) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    true
}

/// Square roots of `a` mod prime `p` (0, 1, or 2 of them). Tonelli–Shanks, with
/// the `p ≡ 3 (mod 4)` shortcut; here `p ≡ 1 (mod 4)` so the general path runs.
fn sqrt_mod(a: i64, p: i64) -> Vec<i64> {
    let a = a.rem_euclid(p);
    if a == 0 {
        return vec![0];
    }
    if p == 2 {
        return vec![a % 2];
    }
    // Euler's criterion: a is a QR iff a^((p-1)/2) == 1.
    if pow_mod(a, (p - 1) / 2, p) != 1 {
        return vec![];
    }
    let r = if p % 4 == 3 {
        pow_mod(a, (p + 1) / 4, p)
    } else {
        tonelli_shanks(a, p)
    };
    let r2 = (p - r) % p;
    if r == r2 {
        vec![r]
    } else {
        vec![r, r2]
    }
}

fn tonelli_shanks(a: i64, p: i64) -> i64 {
    // Write p-1 = q * 2^s with q odd.
    let mut q = p - 1;
    let mut s = 0;
    while q % 2 == 0 {
        q /= 2;
        s += 1;
    }
    // Find a quadratic non-residue z.
    let mut z = 2;
    while pow_mod(z, (p - 1) / 2, p) != p - 1 {
        z += 1;
    }
    let mut m = s;
    let mut c = pow_mod(z, q, p);
    let mut t = pow_mod(a, q, p);
    let mut r = pow_mod(a, (q + 1) / 2, p);
    while t != 1 {
        // Least i in 0<i<m with t^(2^i) == 1.
        let mut i = 0;
        let mut t2 = t;
        while t2 != 1 {
            t2 = mul_mod(t2, t2, p);
            i += 1;
        }
        let b = pow_mod(c, 1 << (m - i - 1), p);
        m = i;
        c = mul_mod(b, b, p);
        t = mul_mod(t, c, p);
        r = mul_mod(r, b, p);
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_theory_primitives() {
        assert_eq!(pow_mod(2, 10, 1000), 24);
        assert_eq!(mul_mod(1230, 1230, 1231), 1); // (-1)^2
        assert_eq!(inv_mod(2, 1231), Some(616)); // 2*616 = 1232 ≡ 1
        assert_eq!(gcd(12, 18), 6);
        // 1231 is prime; a primitive root generates the whole group.
        let g = primitive_root(1231).unwrap();
        assert_eq!(euler_phi(1231), 1230);
        let mut seen = std::collections::HashSet::new();
        let mut x = 1;
        for _ in 0..1230 {
            x = mul_mod(x, g, 1231);
            seen.insert(x);
        }
        assert_eq!(seen.len(), 1230); // g is a generator
        assert!(primitive_root(1230).is_none()); // composite → none
    }

    #[test]
    fn modular_square_roots() {
        // 4 has roots ±2 mod 1231.
        let mut roots = sqrt_mod(4, 1231);
        roots.sort_unstable();
        assert_eq!(roots, vec![2, 1229]);
        // A non-residue has none; every claimed root squares back to a.
        for a in [1i64, 4, 9, 100, 555] {
            for r in sqrt_mod(a, 1231) {
                assert_eq!(mul_mod(r, r, 1231), a.rem_euclid(1231));
            }
        }
    }
}
