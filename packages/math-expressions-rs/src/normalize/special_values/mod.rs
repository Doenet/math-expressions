//! Trig / exp / log special-value folding and parity.
//!
//! [`fold_special_values`] is an *unconditionally sound* rewrite pass, applied
//! bottom-up to a fixpoint. It is a pass in its own right, *not* one of the
//! base rewrite clusters in [`simplify`](crate::normalize::simplify): the base
//! rounds never run it, and the public `simplify` reaches it only through the
//! `full_simplify` fixpoint driver.
//!
//! The dependency on [`crate::eval_exact`] runs one way — this pass *reads*
//! that module's constant tower and trig tables, and nothing in `eval_exact`
//! calls back here (`eval_exact::is_zero` goes `expand` → `canonicalize` →
//! structural / rational-normal-form / exact-constant stages). That asymmetry
//! is what lets `tests/simplify_corpus.rs` use `eval_exact::is_zero` as an
//! oracle for the folds performed here without the check becoming circular at
//! the pass level. Four families:
//!
//! * **Lattice values** — sin/cos/tan/cot/sec/csc at rational multiples of π on
//!   the π/12 and π/10 lattices, via the tested tables in [`crate::eval_exact`]
//!   (`sin(2π) → 0`, `cos(π/3) → 1/2`, `sec(π/4) → √2`, `cos(π/5) → (1+√5)/4`),
//!   and the inverse functions at the values those tables take
//!   (`asin(1) → π/2`, `acos((1+√5)/4) → π/5`).
//! * **Parity + π-shift** — `sin(−u) → −sin u`, `cos(−u) → cos u`, and
//!   `f(u + kπ)` reduction for integer `k` (`sin(x + 2π) → sin x`,
//!   `tan(x + π) → tan x`); on the inverse side `asin(−u) → −asin u` and
//!   `acos(−u) → π − acos u`.
//! * **Inverse compositions** — `f(g⁻¹(u))` for any of the 36 trig/inverse-trig
//!   pairs (`sin(asin u) → u`, `cos(asin u) → √(1−u²)`, `sec(atan u) → √(1+u²)`),
//!   and the complementary sums `asin u + acos u → π/2`, `acsc u + asec u → π/2`.
//! * **exp/log inverses** — `e^{ln u} → u` (sound for `u ≠ 0`); `ln(e^u) → u`
//!   gated to a decidable real `u`; `ln 1 → 0`, `ln e → 1`, `e^0 → 1`.
//!
//! The composition in the *other* direction — `asin(sin x) → x`, and the rest
//! of that family — is not here and cannot be: it is true only on the principal
//! branch (`asin(sin 3) = π − 3`), so it needs a range assumption on `x` rather
//! than an unconditional rewrite.
//!
//! Barrel module. One submodule per family, over a shared vocabulary:
//!
//! - [`trig`]         — the forward functions, their parity and π-periodicity
//! - [`inverse_trig`] — the inverse functions, their parity, the compositions
//!   with the forward ones, and the complementary sums
//! - [`exp_log`]      — the exp/log inverses and change of base
//! - [`util`]         — the trig name tables and the shared small predicates

mod exp_log;
mod inverse_trig;
mod trig;
mod util;

use crate::expr::map_children;
use crate::expr::Expr;

use exp_log::{change_of_base, fold_exp, fold_log, log_arg};
use inverse_trig::{fold_complementary, fold_inverse_trig};
use trig::fold_trig;
use util::{is_e, INVERSE_TRIG, TRIG};

/// Fold trig/exp/log special values and normalize parity, to a bounded
/// fixpoint. The input and output are canonical.
pub fn fold_special_values(e: &Expr) -> Expr {
    let mut cur = crate::normalize::canonicalize(e);
    for _ in 0..8 {
        let next = crate::normalize::canonicalize(&fold_once(&cur));
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
                if INVERSE_TRIG.contains(&name.as_str()) {
                    return fold_inverse_trig(&name, arg).unwrap_or_else(|| e.clone());
                }
                match name.as_str() {
                    "exp" => return fold_exp(arg).unwrap_or_else(|| e.clone()),
                    "log" | "ln" => return fold_log(arg).unwrap_or_else(|| e.clone()),
                    _ => {}
                }
            }
            // `log_b(a)` — both parsers spell a based logarithm as an `Index`
            // head, not a two-argument apply.
            if let (Expr::Index(f, base), [arg]) = (&**head, args.as_slice()) {
                if matches!(&**f, Expr::Sym(s) if s.name() == "log") {
                    return change_of_base(arg, base).unwrap_or_else(|| e.clone());
                }
            }
            e.clone()
        }
        // e^{ln u} → u.
        Expr::Pow(b, x) if is_e(b) => log_arg(x).unwrap_or_else(|| e.clone()),
        // i^n → {1, i, −1, −i}.
        Expr::Pow(b, x) => fold_imaginary_power(b, x).unwrap_or_else(|| e.clone()),
        Expr::Add(ts) => fold_complementary(ts).unwrap_or_else(|| e.clone()),
        _ => e.clone(),
    }
}

/// `i^n → {1, i, −1, −i}` for an integer exponent (`n mod 4`). Unconditionally
/// sound: the imaginary unit's integer powers are exact, with no branch cut to
/// choose. `i` is a symbol here (`canonicalize` turns `Const(I)` into
/// `Sym("i")`), so without this rewrite its integer powers stay symbolic —
/// `equals` already knows `i² = −1` numerically, so only `simplify`/`.tree`
/// display was affected. DoenetML open item 9 (the unambiguous half; the root
/// cases await the corpus). `n mod 4` is Euclidean so `i^{-1} = −i` folds too.
pub(crate) fn fold_imaginary_power(base: &Expr, exp: &Expr) -> Option<Expr> {
    if !util::is_i(base) {
        return None;
    }
    let crate::expr::Expr::Num(crate::num::Number::Int(n)) = exp else {
        return None;
    };
    Some(match n.rem_euclid(4) {
        0 => Expr::int(1),
        1 => Expr::sym("i"),
        2 => Expr::int(-1),
        _ => Expr::Neg(Box::new(Expr::sym("i"))),
    })
}
