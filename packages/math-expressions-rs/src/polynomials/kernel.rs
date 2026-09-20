//! Opaque kernels: reading a non-polynomial expression as a polynomial anyway.
//!
//! [`multivariate`](super::multivariate) is a polynomial ring over ℚ in named
//! variables, so on its own it refuses `sin x`, `√x`, `π` and `e` — none of
//! them is a rational coefficient and none is a variable. Kernelization is the
//! standard answer (SymPy's): replace each distinct opaque subtree with a fresh
//! indeterminate, do the polynomial arithmetic, substitute back.
//!
//! **Why this is sound.** Distinct kernels become *independent* indeterminates,
//! so anything proved in the extended ring is a polynomial identity over all
//! kernel values — and identities survive specialization. A cancellation found
//! this way (`num = g·qn`, `den = g·qd`) therefore still holds when the kernels
//! resume being `sin x` or `π`.
//!
//! **What it costs is completeness, never correctness.** Relations *among* the
//! kernels are invisible: `sin²x + cos²x` does not become 1, and `i` behaves as
//! a free variable rather than a root of `t²+1`, so `(x²+1)/(x+i)` is left
//! alone instead of reducing to `x−i`. Missing a cancellation is the failure
//! mode; producing a wrong one is not. Whether a kernel is transcendental
//! decides only *how much* is missed — over `π` and `e`, which are
//! transcendental over ℚ, `ℚ[π] ≅ ℚ[t]` and nothing is missed at all.

use std::collections::{BTreeSet, HashMap};

use crate::expr::Expr;
use crate::num::Number;

/// Distinct opaque subtrees, each assigned a fresh symbol name `$k{n}` (the `$`
/// prefix cannot appear in parsed input, so there is no collision with a real
/// variable). Deduplicated by canonical structural equality.
///
/// One `Kernels` must cover every expression that will be compared or divided
/// against another — kernelizing a numerator and a denominator separately would
/// give the same `sin x` two different names, and the common factor would go
/// unseen.
#[derive(Default)]
pub(crate) struct Kernels {
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

    pub(crate) fn restore(&self, e: &Expr) -> Expr {
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
pub(crate) fn kernelize(e: &Expr, kernels: &mut Kernels) -> Expr {
    match e {
        Expr::Num(_) => e.clone(),
        Expr::Sym(s) if !crate::expr::sym::is_constant_symbol(&s.name()) => e.clone(),
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
pub(crate) fn indeterminates(e: &Expr) -> Vec<String> {
    let mut set = BTreeSet::new();
    fn walk(e: &Expr, set: &mut BTreeSet<String>) {
        if let Expr::Sym(s) = e {
            let name = s.name();
            if !crate::expr::sym::is_constant_symbol(&name) {
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
