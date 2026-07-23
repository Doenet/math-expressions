//! S1 (FULL_SIMPLIFY_PLAN) — certified `is_zero` service tests.
//!
//! `is_zero` returns `Some(true)` only when the expression is *certified*
//! identically zero, `Some(false)` only when *certified* nonzero, and `None`
//! (Unknown) otherwise. The invariant under test everywhere: it never lies.

use math_expressions::assumptions::Assumptions;
use math_expressions::exact::is_zero;
use math_expressions::TextToAst;

fn parse(s: &str) -> math_expressions::Expr {
    TextToAst::new(Default::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e:?}"))
}

fn z(s: &str) -> Option<bool> {
    is_zero(&parse(s), &Assumptions::new())
}

// ---------- the constant zoo: certified zero ----------

#[test]
fn trig_at_rational_pi_lattice() {
    assert_eq!(z("sin(2*pi)"), Some(true)); // quarter-turn (existing PiLin)
    assert_eq!(z("cos(pi/2)"), Some(true));
    assert_eq!(z("sin(pi)"), Some(true));
    assert_eq!(z("tan(pi)"), Some(true));
    // kπ/6 and kπ/4 lattices (the S1 extension beyond kπ/2):
    assert_eq!(z("cos(pi/3) - 1/2"), Some(true));
    assert_eq!(z("sin(pi/6) - 1/2"), Some(true));
    assert_eq!(z("tan(pi/4) - 1"), Some(true));
    assert_eq!(z("sin(pi/4) - sqrt(2)/2"), Some(true));
    assert_eq!(z("cos(pi/6) - sqrt(3)/2"), Some(true));
    assert_eq!(z("tan(pi/6) - sqrt(3)/3"), Some(true));
    // kπ/12 lattice (nested surds):
    assert_eq!(z("sin(pi/12) - (sqrt(6) - sqrt(2))/4"), Some(true));
    // periodicity reduction:
    assert_eq!(z("sin(13*pi/6) - 1/2"), Some(true));
    assert_eq!(z("cos(7*pi/3) - 1/2"), Some(true));
}

#[test]
fn surd_arithmetic() {
    assert_eq!(z("sqrt(8) - 2*sqrt(2)"), Some(true));
    assert_eq!(z("sqrt(2)*sqrt(2) - 2"), Some(true));
    assert_eq!(z("sqrt(12) - 2*sqrt(3)"), Some(true));
    assert_eq!(z("sqrt(2)*sqrt(3) - sqrt(6)"), Some(true));
    assert_eq!(z("sqrt(1/2) - sqrt(2)/2"), Some(true));
    assert_eq!(z("(sqrt(2) + 1)*(sqrt(2) - 1) - 1"), Some(true));
}

#[test]
fn exp_log_inverse() {
    assert_eq!(z("exp(ln(3)) - 3"), Some(true));
    assert_eq!(z("e^(ln(3)) - 3"), Some(true));
    assert_eq!(z("ln(exp(5)) - 5"), Some(true));
    assert_eq!(z("ln(1)"), Some(true));
    assert_eq!(z("exp(0) - 1"), Some(true));
    assert_eq!(z("ln(e) - 1"), Some(true));
}

#[test]
fn plain_rational_constants() {
    assert_eq!(z("1/2 + 1/3 - 5/6"), Some(true));
    assert_eq!(z("0"), Some(true));
    assert_eq!(z("2^10 - 1024"), Some(true));
    assert_eq!(z("3 - 3"), Some(true));
}

// ---------- certified nonzero ----------

#[test]
fn certified_nonzero() {
    assert_eq!(z("sqrt(8) - 3*sqrt(2)"), Some(false)); // = -sqrt(2)
    assert_eq!(z("cos(pi/3) - 1/3"), Some(false));
    assert_eq!(z("2 - sqrt(2)"), Some(false));
    assert_eq!(z("pi - 3"), Some(false));
    assert_eq!(z("e - 2"), Some(false));
    assert_eq!(z("sin(pi/6)"), Some(false)); // = 1/2
    assert_eq!(z("5"), Some(false));
    assert_eq!(z("sqrt(2)"), Some(false));
}

// ---------- adversarial almost-zeros: never certified zero ----------

#[test]
fn adversarial_almost_zero_never_yes() {
    // Ramanujan's constant: e^{π√163} ≈ 640320³ + 744, off by ~7.5e-13.
    // Must NOT be reported as zero. Unknown is the honest S1 verdict
    // (variable-free, exact evaluator declines on exp of an irrational).
    let r = z("exp(pi*sqrt(163)) - (640320^3 + 744)");
    assert_ne!(r, Some(true), "almost-zero must never certify as zero");
    assert_eq!(r, None, "S1 has no method to decide this constant");

    // A transcendental identity S1 cannot fold either way stays Unknown,
    // never a wrong No.
    assert_eq!(z("ln(4) - 2*ln(2)"), None);
    assert_eq!(z("sin(1)^2 + cos(1)^2 - 1"), None);
}

// ---------- variable expressions ----------

#[test]
fn rootof_algebraic_identities() {
    // √2 as a RootOf, squared, is 2 (either root order).
    assert_eq!(z("rootof(z^2 - 2, 1)^2 - 2"), Some(true));
    assert_eq!(z("rootof(z^2 - 2, 0)^2 - 2"), Some(true));
    // ∛2 cubed is 2.
    assert_eq!(z("rootof(z^3 - 2, 0)^3 - 2"), Some(true));
    // The defining relation itself: α³ = α + 1 for the plastic-number root.
    assert_eq!(z("rootof(z^3 - z - 1, 0)^3 - rootof(z^3 - z - 1, 0) - 1"), Some(true));
    // Honest limitation: a nonzero algebraic is Unknown, never a wrong No.
    assert_eq!(z("rootof(z^2 - 2, 1) - 1"), None);
}

#[test]
fn polynomial_identities_via_expand() {
    assert_eq!(z("(x+1)^2 - x^2 - 2*x - 1"), Some(true));
    assert_eq!(z("x*(x-1) - x^2 + x"), Some(true));
    assert_eq!(z("x - x"), Some(true));
    assert_eq!(z("(a-b)*(a+b) - a^2 + b^2"), Some(true));
}

#[test]
fn variable_nonzero_by_sampling() {
    assert_eq!(z("x^2 - 1"), Some(false));
    assert_eq!(z("x + y"), Some(false));
    assert_eq!(z("sin(x)"), Some(false));
}

#[test]
fn trig_identity_with_free_var_is_unknown() {
    // Identically zero, but S1 has no trig-contraction; sampling can only
    // refute (never confirm), so the honest answer is Unknown.
    assert_eq!(z("sin(x)^2 + cos(x)^2 - 1"), None);
}

#[test]
fn integer_powers_of_e() {
    // e^k (k ≥ 0 integer) is an exact basis monomial, so certified-zero can now
    // decide these instead of returning Unknown. `e*e` already worked (Mul folds
    // to the e^2 monomial); the Pow form did not until eval_exp handled e^k.
    assert_eq!(z("e^2 - e*e"), Some(true));
    assert_eq!(z("e^3 - e*e*e"), Some(true));
    assert_eq!(z("e^2 - exp(2)"), Some(true)); // Pow form ≡ exp form (was None)
    assert_eq!(z("exp(3) - e^3"), Some(true)); // (was None)
    assert_eq!(z("e^2 - 7"), Some(false)); // certified nonzero (was None)
    assert_eq!(z("e^2"), Some(false)); // certified nonzero (was None)
    // A negative power of e is outside the representable ring, so with no
    // structural partner to cancel against it stays undecided (never wrong).
    // (`e^(-2) - 1/(e*e)` would instead be *structurally* zero — both sides
    // canonicalize to the same reciprocal tree — so it is not a ring test.)
    assert_eq!(z("e^(-2) - 7"), None);
}

#[test]
fn oversized_surd_radicand_is_undecided_not_abort() {
    // ARCHITECTURE_REVIEW §8 surd guard: a radicand that overflows u128 must make
    // the exact `sqrt` evaluator decline (→ Unknown), never panic. `10^40`
    // exceeds `u128::MAX`, so `to_u128()` returns None and is_zero is undecided
    // rather than aborting.
    assert_eq!(z("sqrt(10^40) - 1"), None);
    assert_eq!(z("sqrt(10^40)"), None);
    // A tractable surd still certifies (guard is not over-broad).
    assert_eq!(z("sqrt(4) - 2"), Some(true));
}
