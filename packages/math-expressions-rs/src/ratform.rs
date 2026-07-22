//! Rational-function normal form.
//!
//! [`together`] / [`cancel`] put an expression over a single common
//! denominator and reduce numerator and denominator to lowest terms with the
//! multivariate polynomial GCD ([`crate::poly`]). Non-rational subtrees —
//! `sin x`, `√x`, `π`, `RootOf` leaves, … — are treated as **opaque kernels**:
//! each distinct one is replaced by a fresh indeterminate before the polynomial
//! arithmetic and restored afterwards. This is the SymPy trick that lets
//! rational normalization apply *underneath* any function, so
//! `1/sin(x) + 1/sin(x)` becomes `2/sin(x)` without the simplifier ever needing
//! to understand `sin`.
//!
//! Soundness of kernelization: distinct kernels become independent
//! indeterminates, so any identity we prove holds for *all* kernel values — a
//! sufficient (never necessary) condition for the real identity. Hence
//! [`is_identically_zero`] only ever yields `true` for a genuine zero.

use std::collections::{BTreeSet, HashMap};

use num_traits::One;

use crate::expr::Expr;
use crate::num::Number;
use crate::poly;

/// Maximum distinct indeterminates (real variables + kernels) the dense
/// recursive polynomial model will accept before we bail to the unchanged
/// form. Guards the 2ᵏ blow-up of `k` independent linear denominators (e.g.
/// `∑ 1/(xᵢ+1)`), which the per-variable degree cap alone does not catch.
const MAX_INDETERMINATES: usize = 6;

/// Combine `e` into a single reduced fraction `num/den` in lowest terms, with
/// opaque kernels held fixed. Returns `e` canonicalized unchanged when the
/// rational structure is too large to normalize within the caps.
pub fn together(e: &Expr) -> Expr {
    match rational_normal(e) {
        Some((num, den)) if is_one_expr(&den) => num,
        Some((num, den)) => crate::norm::canonicalize(&Expr::Div(Box::new(num), Box::new(den))),
        None => crate::norm::canonicalize(e),
    }
}

/// Cancel the common factors of a ratio, reducing it to lowest terms. For this
/// normal form it coincides with [`together`] (both return the coprime single
/// fraction); kept as a separate name to mirror the CAS vocabulary and to give
/// callers that only want cancellation a clear entry point.
pub fn cancel(e: &Expr) -> Expr {
    together(e)
}

/// Certified test used by `exact::is_zero` stage (d): `true` iff `e` normalizes
/// to a zero numerator over a nonzero denominator, i.e. `e ≡ 0` as a rational
/// function in its variables and kernels. Never `true` for a non-zero `e`.
pub(crate) fn is_identically_zero(e: &Expr) -> bool {
    matches!(rational_normal(e), Some((num, _)) if is_zero_expr(&num))
}

/// The reduced `(numerator, denominator)` pair as canonical, kernel-restored
/// expressions, or `None` when the input is outside the caps.
fn rational_normal(e: &Expr) -> Option<(Expr, Expr)> {
    let canon = crate::norm::canonicalize(e);

    // Replace opaque (non-rational) subtrees with fresh kernel symbols.
    let mut kernels = Kernels::default();
    let ke = kernelize(&canon, &mut kernels);

    let vars = indeterminates(&ke);
    if vars.is_empty() || vars.len() > MAX_INDETERMINATES {
        return None;
    }

    // (num, den) as polynomial expressions over `vars` (+ kernels).
    let (num_e, den_e) = rational_parts(&ke)?;
    if is_zero_expr(&den_e) {
        return None; // 0 denominator — undefined, refuse
    }
    let pn = poly::expr_to_poly(&crate::norm::canonicalize(&num_e), &vars)?;
    let pd = poly::expr_to_poly(&crate::norm::canonicalize(&den_e), &vars)?;

    // Cancel gcd, then normalize rational content onto the numerator (so
    // `(2x+4)/2` → `x+2`, matching `reduce_rational`).
    let g = poly::gcd(&pn, &pd, vars.len())?;
    let (pn, pd) = if poly::is_trivial(&g) {
        (pn, pd)
    } else {
        (
            poly::exact_div_top(&pn, &g, vars.len())?,
            poly::exact_div_top(&pd, &g, vars.len())?,
        )
    };
    let (cn, pn) = poly::strip_rational_content(&pn);
    let (cd, pd) = poly::strip_rational_content(&pd);
    let scalar = Expr::Num(Number::from_bigrational(cn / cd));

    let num = crate::norm::mul(vec![scalar, poly::poly_to_expr(&pn, &vars)]);
    let den = poly::poly_to_expr(&pd, &vars);

    // Restore the kernels and canonicalize both halves.
    let num = crate::norm::canonicalize(&kernels.restore(&num));
    let den = crate::norm::canonicalize(&kernels.restore(&den));
    Some((num, den))
}

/// `(numerator, denominator)` of a *kernelized* expression as polynomial trees
/// (no `Div`, no negative powers). Builds them with the canonical constructors
/// so the result feeds straight into `expr_to_poly`. `None` on a non-rational
/// node (should not occur post-kernelization) or a term-count breach.
fn rational_parts(e: &Expr) -> Option<(Expr, Expr)> {
    let one = || Expr::int(1);
    Some(match e {
        Expr::Num(_) | Expr::Sym(_) => (e.clone(), one()),
        Expr::Neg(a) => {
            let (n, d) = rational_parts(a)?;
            (neg(n), d)
        }
        Expr::Add(ts) => {
            if ts.len() > crate::resource_limits::current().max_ratform_terms {
                return None;
            }
            let mut acc = (Expr::int(0), one());
            for t in ts {
                let (n, d) = rational_parts(t)?;
                // acc = (acc.n·d + n·acc.d) / (acc.d·d)
                let num = crate::norm::add(vec![
                    crate::norm::mul(vec![acc.0, d.clone()]),
                    crate::norm::mul(vec![n, acc.1.clone()]),
                ]);
                let den = crate::norm::mul(vec![acc.1, d]);
                acc = (num, den);
            }
            acc
        }
        Expr::Mul(fs) => {
            let (mut num, mut den) = (one(), one());
            for f in fs {
                let (n, d) = rational_parts(f)?;
                num = crate::norm::mul(vec![num, n]);
                den = crate::norm::mul(vec![den, d]);
            }
            (num, den)
        }
        Expr::Div(a, b) => {
            let (na, da) = rational_parts(a)?;
            let (nb, db) = rational_parts(b)?;
            (crate::norm::mul(vec![na, db]), crate::norm::mul(vec![da, nb]))
        }
        Expr::Pow(b, k) => {
            let Expr::Num(Number::Int(k)) = &**k else {
                return None;
            };
            let (n, d) = rational_parts(b)?;
            if *k >= 0 {
                (pow_int(n, *k), pow_int(d, *k))
            } else {
                (pow_int(d, -*k), pow_int(n, -*k))
            }
        }
        _ => return None,
    })
}

fn neg(x: Expr) -> Expr {
    crate::norm::mul(vec![Expr::int(-1), x])
}

fn pow_int(base: Expr, k: i64) -> Expr {
    crate::norm::pow(base, Expr::int(k))
}

fn is_zero_expr(e: &Expr) -> bool {
    matches!(crate::norm::canonicalize(e), Expr::Num(n) if n.is_zero())
}

fn is_one_expr(e: &Expr) -> bool {
    matches!(crate::norm::canonicalize(e), Expr::Num(n) if n.to_bigrational().is_some_and(|q| q.is_one()))
}

// ---------------- opaque kernels ----------------

/// Distinct opaque subtrees, each assigned a fresh symbol name `$k{n}` (the `$`
/// prefix cannot appear in parsed input, so there is no collision with a real
/// variable). Deduplicated by canonical structural equality.
#[derive(Default)]
struct Kernels {
    map: Vec<(String, Expr)>, // name → original subtree
}

impl Kernels {
    fn intern(&mut self, e: &Expr) -> Expr {
        if let Some((name, _)) = self.map.iter().find(|(_, k)| k == e) {
            return Expr::sym(name);
        }
        let name = format!("$k{}", self.map.len());
        self.map.push((name.clone(), e.clone()));
        Expr::sym(&name)
    }

    fn restore(&self, e: &Expr) -> Expr {
        if self.map.is_empty() {
            return e.clone();
        }
        let subs: HashMap<String, Expr> = self.map.iter().cloned().collect();
        crate::ops::substitute(e, &subs)
    }
}

/// Replace every maximal non-rational subtree of a *canonical* expression with
/// a fresh kernel symbol. The rational skeleton (`+ − · / ^ℤ`, numbers, and
/// ordinary variables) is preserved; constants (`π`, `e`) and every function
/// application or non-integer power become kernels.
fn kernelize(e: &Expr, kernels: &mut Kernels) -> Expr {
    match e {
        Expr::Num(_) => e.clone(),
        Expr::Sym(s) if !crate::sym::is_constant_symbol(&s.name()) => e.clone(),
        Expr::Add(ts) => Expr::Add(ts.iter().map(|t| kernelize(t, kernels)).collect()),
        Expr::Mul(fs) => Expr::Mul(fs.iter().map(|f| kernelize(f, kernels)).collect()),
        Expr::Neg(a) => Expr::Neg(Box::new(kernelize(a, kernels))),
        Expr::Div(a, b) => Expr::Div(
            Box::new(kernelize(a, kernels)),
            Box::new(kernelize(b, kernels)),
        ),
        Expr::Pow(b, k) if matches!(&**k, Expr::Num(Number::Int(_))) => {
            Expr::Pow(Box::new(kernelize(b, kernels)), k.clone())
        }
        // Constants, functions, non-integer powers, RootOf, relations, … are
        // opaque.
        _ => kernels.intern(e),
    }
}

/// The distinct indeterminate names (real variables + kernels) in a kernelized
/// expression, in a fixed (sorted) order.
fn indeterminates(e: &Expr) -> Vec<String> {
    let mut set = BTreeSet::new();
    fn walk(e: &Expr, set: &mut BTreeSet<String>) {
        if let Expr::Sym(s) = e {
            let name = s.name();
            if !crate::sym::is_constant_symbol(&name) {
                set.insert(name);
            }
        }
        for c in e.children() {
            walk(c, set);
        }
    }
    walk(e, &mut set);
    set.into_iter().collect()
}
