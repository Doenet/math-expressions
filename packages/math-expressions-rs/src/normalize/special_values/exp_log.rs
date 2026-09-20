//! `exp`/`log` inverses, the two constant values, and change of base.

use super::util::{apply, canon, is_e, is_one_expr, is_zero_expr};
use crate::expr::Expr;

pub(super) fn fold_exp(arg: &Expr) -> Option<Expr> {
    // exp(ln u) = u  (u ≠ 0).
    if let Some(u) = log_arg(arg) {
        return Some(u);
    }
    // exp(0) = 1.
    is_zero_expr(arg).then(|| Expr::int(1))
}

pub(super) fn fold_log(arg: &Expr) -> Option<Expr> {
    // ln(exp u) = u and ln(e^u) = u — gated to a decidable real u (S5 does
    // general realness; here we only fold when u evaluates in the exact tower,
    // which is always real).
    let inner = exp_arg(arg);
    if let Some(u) = inner {
        if crate::eval_exact::exact_eval(&u).is_some() {
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

/// `log_b(a) → log(a)/log(b)`, so a based logarithm reduces to the one form the
/// rest of the engine knows how to work with. Without it `log_b(a)` was inert:
/// it never combined with anything, and `log_b(a) − log(a)/log(b)` did not
/// simplify to zero.
///
/// Base 1 is the one base the rewrite may not take, because `log 1` is `0`:
/// it answered `log_1(5) → ∞` (and `log_1(1) → 1`, through the numeric pass
/// below) where there is no such logarithm at all — `1^y` is `1` for every `y`.
/// It folds to `NaN` with the crate's other undefined forms.
///
/// Declines when the numeric pass would produce an exact value instead
/// (`log_2(8)` is `3`, not `log 8 / log 2`). That pass runs *after* this one in
/// the `full_simplify` round, so the check has to happen here rather than being
/// left to ordering. Once rewritten the node is no longer an `Index`-headed
/// apply, so the surrounding fixpoint cannot re-enter it.
pub(super) fn change_of_base(arg: &Expr, base: &Expr) -> Option<Expr> {
    if is_one_expr(base) {
        return Some(Expr::Const(crate::expr::MathConst::NaN));
    }
    let head = Expr::Index(Box::new(Expr::sym("log")), Box::new(base.clone()));
    if crate::normalize::fold_apply::folds_to_a_number(&head, std::slice::from_ref(arg)) {
        return None;
    }
    Some(canon(&crate::normalize::mul(vec![
        apply("log", arg.clone()),
        crate::normalize::pow(apply("log", base.clone()), Expr::int(-1)),
    ])))
}

/// `u` when `e` is `ln u` / `log u` (an `Apply`) — both for `exp(ln u)` and for
/// the `e^{ln u}` spelling of the same rewrite.
pub(super) fn log_arg(e: &Expr) -> Option<Expr> {
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
