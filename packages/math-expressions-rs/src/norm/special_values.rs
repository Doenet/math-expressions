//! Trig / exp / log special-value folding and parity.
//!
//! [`fold_special_values`] is an *unconditionally sound* rewrite pass, applied
//! bottom-up to a fixpoint. It is independent of the oracle-compatible
//! `simplify`; `exact::is_zero` uses it as a pre-pass. Three families:
//!
//! * **Lattice values** — sin/cos/tan/cot/sec/csc at rational multiples of π on
//!   the π/12 lattice, via the tested tables in [`crate::exact`]
//!   (`sin(2π) → 0`, `cos(π/3) → 1/2`, `sec(π/4) → √2`).
//! * **Parity + π-shift** — `sin(−u) → −sin u`, `cos(−u) → cos u`, and
//!   `f(u + kπ)` reduction for integer `k` (`sin(x + 2π) → sin x`,
//!   `tan(x + π) → tan x`).
//! * **exp/log inverses** — `e^{ln u} → u` (sound for `u ≠ 0`); `ln(e^u) → u`
//!   gated to a decidable real `u`; `ln 1 → 0`, `ln e → 1`, `e^0 → 1`.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, Zero};

use crate::expr::{Expr, MathConst};
use crate::norm::syntactic::map_children;

const TRIG: &[&str] = &["sin", "cos", "tan", "cot", "sec", "csc"];

/// Fold trig/exp/log special values and normalize parity, to a bounded
/// fixpoint. The input and output are canonical.
pub fn fold_special_values(e: &Expr) -> Expr {
    let mut cur = crate::norm::canonicalize(e);
    for _ in 0..8 {
        let next = crate::norm::canonicalize(&fold_once(&cur));
        if next == cur {
            break;
        }
        cur = next;
    }
    cur
}

fn fold_once(e: &Expr) -> Expr {
    let e = map_children(e, fold_once);
    fold_node(&e)
}

fn fold_node(e: &Expr) -> Expr {
    match e {
        Expr::Apply(head, args) => {
            if let (Expr::Sym(s), [arg]) = (&**head, args.as_slice()) {
                let name = s.name();
                if TRIG.contains(&name.as_str()) {
                    return fold_trig(&name, arg).unwrap_or_else(|| e.clone());
                }
                match name.as_str() {
                    "exp" => return fold_exp(arg).unwrap_or_else(|| e.clone()),
                    "log" | "ln" => return fold_log(arg).unwrap_or_else(|| e.clone()),
                    _ => {}
                }
            }
            e.clone()
        }
        // e^{ln u} → u.
        Expr::Pow(b, x) if is_e(b) => log_arg(x).unwrap_or_else(|| e.clone()),
        _ => e.clone(),
    }
}

// ---------------- trig ----------------

fn fold_trig(name: &str, arg: &Expr) -> Option<Expr> {
    let (sign, new_arg) = normalize_trig_arg(name, arg);
    let value = crate::exact::trig_special_value(name, &new_arg);
    let arg_changed = new_arg != canon(arg);
    if value.is_none() && sign > 0 && !arg_changed {
        return None; // nothing folded
    }
    let core = value.unwrap_or_else(|| apply(name, new_arg));
    Some(if sign < 0 { negate(&core) } else { core })
}

/// Pull the sign out of a negative-leading argument (parity) and drop integer
/// multiples of π (periodicity). Returns `(sign, reduced_arg)`.
fn normalize_trig_arg(name: &str, arg: &Expr) -> (i32, Expr) {
    let mut sign = 1;
    let mut a = canon(arg);
    if neg_leading(&a) {
        a = negate(&a);
        if is_odd_fn(name) {
            sign = -sign;
        }
    }
    let (pi_coeff, rest) = split_pi(&a);
    if pi_coeff.is_integer() && !pi_coeff.is_zero() {
        let k = pi_coeff.to_integer();
        let k_is_odd = (&k % 2i32).abs() == BigInt::from(1);
        // sin/cos/sec/csc have period 2π (odd k flips sign); tan/cot have
        // period π (any integer k drops out with no sign change).
        if matches!(name, "sin" | "cos" | "sec" | "csc") && k_is_odd {
            sign = -sign;
        }
        a = rest;
    }
    (sign, a)
}

/// sin, tan, csc, cot are odd; cos, sec are even.
fn is_odd_fn(name: &str) -> bool {
    matches!(name, "sin" | "tan" | "csc" | "cot")
}

/// Split `e` into `(q, rest)` with `e = q·π + rest`, gathering every summand
/// that is a rational multiple of π into `q`.
fn split_pi(e: &Expr) -> (BigRational, Expr) {
    let terms: Vec<Expr> = match e {
        Expr::Add(ts) => ts.clone(),
        other => vec![other.clone()],
    };
    let mut coeff = BigRational::zero();
    let mut rest = Vec::new();
    for t in terms {
        match pi_multiple(&t) {
            Some(q) => coeff += q,
            None => rest.push(t),
        }
    }
    (coeff, canon(&crate::norm::add(rest)))
}

/// The rational `q` when `e = q·π` (`π`, `3π`, `-π/2`, …), else `None`.
fn pi_multiple(e: &Expr) -> Option<BigRational> {
    if is_pi(e) {
        return Some(BigRational::from_integer(BigInt::from(1)));
    }
    let Expr::Mul(fs) = e else { return None };
    let mut coeff = BigRational::from_integer(BigInt::from(1));
    let mut saw_pi = false;
    for f in fs {
        if is_pi(f) {
            if saw_pi {
                return None; // π² is not a rational multiple of π
            }
            saw_pi = true;
        } else if let Expr::Num(n) = f {
            coeff *= n.to_bigrational()?;
        } else {
            return None;
        }
    }
    saw_pi.then_some(coeff)
}

// ---------------- exp / log ----------------

fn fold_exp(arg: &Expr) -> Option<Expr> {
    // exp(ln u) = u  (u ≠ 0).
    if let Some(u) = log_arg_apply(arg) {
        return Some(u);
    }
    // exp(0) = 1.
    is_zero_expr(arg).then(|| Expr::int(1))
}

fn fold_log(arg: &Expr) -> Option<Expr> {
    // ln(exp u) = u and ln(e^u) = u — gated to a decidable real u (S5 does
    // general realness; here we only fold when u evaluates in the exact tower,
    // which is always real).
    let inner = exp_arg(arg);
    if let Some(u) = inner {
        if crate::exact::exact_eval(&u).is_some() {
            return Some(u);
        }
    }
    if is_e(arg) {
        return Some(Expr::int(1)); // ln e = 1
    }
    if is_one_expr(arg) {
        return Some(Expr::int(0)); // ln 1 = 0
    }
    None
}

/// `u` when `x` is `ln u` / `log u` (an `Apply`), for the `e^{ln u}` rewrite.
fn log_arg(x: &Expr) -> Option<Expr> {
    log_arg_apply(x)
}

fn log_arg_apply(e: &Expr) -> Option<Expr> {
    if let Expr::Apply(head, args) = e {
        if let (Expr::Sym(s), [u]) = (&**head, args.as_slice()) {
            if matches!(s.name().as_str(), "log" | "ln") {
                return Some(u.clone());
            }
        }
    }
    None
}

/// `u` when `e` is `exp u` (an `Apply`) or `e^u` (a `Pow` with base e).
fn exp_arg(e: &Expr) -> Option<Expr> {
    if let Expr::Apply(head, args) = e {
        if let (Expr::Sym(s), [u]) = (&**head, args.as_slice()) {
            if s.name() == "exp" {
                return Some(u.clone());
            }
        }
    }
    if let Expr::Pow(b, u) = e {
        if is_e(b) {
            return Some((**u).clone());
        }
    }
    None
}

// ---------------- small helpers ----------------

fn canon(e: &Expr) -> Expr {
    crate::norm::canonicalize(e)
}

fn apply(name: &str, arg: Expr) -> Expr {
    Expr::Apply(Box::new(Expr::sym(name)), vec![arg])
}

fn negate(e: &Expr) -> Expr {
    canon(&crate::norm::mul(vec![Expr::int(-1), e.clone()]))
}

/// Heuristic "is this expression negative-leading" test for parity extraction.
fn neg_leading(e: &Expr) -> bool {
    match e {
        Expr::Num(n) => n.to_bigrational().is_some_and(|q| q.is_negative()),
        Expr::Neg(_) => true,
        Expr::Mul(fs) => fs.first().is_some_and(neg_leading),
        Expr::Add(ts) => ts.first().is_some_and(neg_leading),
        _ => false,
    }
}

fn is_pi(e: &Expr) -> bool {
    matches!(e, Expr::Const(MathConst::Pi)) || matches!(e, Expr::Sym(s) if s.name() == "pi")
}

fn is_e(e: &Expr) -> bool {
    matches!(e, Expr::Const(MathConst::E)) || matches!(e, Expr::Sym(s) if s.name() == "e")
}

fn is_zero_expr(e: &Expr) -> bool {
    matches!(canon(e), Expr::Num(n) if n.is_zero())
}

fn is_one_expr(e: &Expr) -> bool {
    matches!(canon(e), Expr::Num(n) if n.to_bigrational().is_some_and(|q| q == BigRational::from_integer(BigInt::from(1))))
}
