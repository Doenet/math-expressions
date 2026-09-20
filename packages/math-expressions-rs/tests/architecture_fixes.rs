//! Regressions for the 2026-07-22 architecture-review point fixes
//! (active-plans/ARCHITECTURE_REVIEW.md §9 item 1).

use math_expressions::{canonicalize, Expr, Mat, MathConst, TextToAst};

fn parse(s: &str) -> Expr {
    TextToAst::new(Default::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e:?}"))
}

/// Const(Pi/E/I) and Sym("pi"/"e"/"i") are two spellings of the same
/// constant; canonicalize must unify them (Sym is canonical), so `==` on
/// canonical trees is semantic equality for constants.
#[test]
fn canonicalize_unifies_const_and_sym_constants() {
    for (c, name) in [
        (MathConst::Pi, "pi"),
        (MathConst::E, "e"),
        (MathConst::I, "i"),
    ] {
        assert_eq!(
            canonicalize(&Expr::Const(c)),
            canonicalize(&parse(name)),
            "Const({name}) must canonicalize to the Sym spelling"
        );
    }
    // Inf/NegInf/NaN stay Const — they have no Sym spelling to unify with.
    assert!(matches!(
        canonicalize(&Expr::Const(MathConst::Inf)),
        Expr::Const(MathConst::Inf)
    ));
}

/// The deg-unit desugaring inside `equals` used to mint `Const(Pi)`, which the
/// error-tolerance machinery didn't parameterize like `Sym("pi")`. End-to-end:
/// `90 deg` must equal `pi/2`.
#[test]
fn deg_units_equal_pi_over_two() {
    assert!(math_expressions::equals(
        &parse("90 deg"),
        &parse("pi/2"),
        &Default::default()
    ));
}

/// `OdeSolution::at(NaN)` must propagate NaN, not panic (a panic is an abort
/// under wasm's panic=abort).
#[test]
fn ode_dense_output_at_nan_does_not_panic() {
    // y' = y, y(0) = 1 over [0, 1].
    let sol = math_expressions::solve_ode_with(
        |_t, y, out: &mut [f64]| {
            out[0] = y[0];
            true
        },
        0.0,
        1.0,
        &[1.0],
        1e-8,
        10_000,
    );
    let v = sol.at(f64::NAN);
    assert_eq!(v.len(), 1);
    assert!(v[0].is_nan());
    // Ordinary queries still work.
    assert!((sol.at(1.0)[0] - std::f64::consts::E).abs() < 1e-6);
}

/// `equals` must recognize exact constant identities the numeric sampler and
/// finite-field stages false-reject (`cos(π/3) = 1/2`, surds), via the
/// accept-only certified-exact stage — without breaking true inequalities.
#[test]
fn equals_certifies_constant_identities() {
    let eq = |a: &str, b: &str| math_expressions::equals(&parse(a), &parse(b), &Default::default());
    // Previously false negatives (numeric ill-conditioning / finite-field):
    assert!(eq("1/2", "cos(pi/3)"));
    assert!(eq("cos(pi/3)", "1/2"));
    assert!(eq("sqrt(8)", "2*sqrt(2)"));
    assert!(eq("tan(pi/4)", "1"));
    assert!(eq("sin(pi/6) + cos(pi/3)", "1"));
    assert!(eq("exp(ln(3))", "3"));
    // Must NOT introduce false positives (accept-only, sound):
    assert!(!eq("cos(pi/3)", "1/3"));
    assert!(!eq("pi", "3.14"));
    assert!(!eq("sqrt(2)", "1.41"));
    assert!(!eq("sqrt(2) + sqrt(3)", "sqrt(5)"));
}

/// Pivot/rank decisions must use *certified* zero, not syntactic zero: an
/// entry that is symbolically zero (`sqrt(8) - 2 sqrt(2)`) must not count as
/// a nonzero pivot.
#[test]
fn symbolic_zero_entry_is_not_a_pivot() {
    use math_expressions::{rank, Assumptions};
    let zero_entry = parse("sqrt(8) - 2*sqrt(2)");
    let m = Expr::Matrix(Mat::new(1, 1, vec![zero_entry]).expect("test matrix shape"));
    assert_eq!(rank(&m, &Assumptions::new()), Some(0));
    // Sanity: a genuinely nonzero surd entry still has rank 1.
    let m1 =
        Expr::Matrix(Mat::new(1, 1, vec![parse("sqrt(8) - sqrt(2)")]).expect("test matrix shape"));
    assert_eq!(rank(&m1, &Assumptions::new()), Some(1));
}

/// Sign propagation must accept both the canonical `log` and the `ln` alias
/// (registry builders emit `ln`; canonicalization normalizes, but the
/// assumptions engine can see unnormalized trees).
#[test]
fn assumptions_sign_propagation_handles_ln_alias() {
    use math_expressions::Assumptions;
    let mut a = Assumptions::new();
    a.add(&parse("x > 1"));
    for spelling in ["log(x)", "ln(x)"] {
        assert_eq!(
            math_expressions::is_real(&parse(spelling), &a),
            math_expressions::is_real(&parse("log(x)"), &a),
            "{spelling} must propagate like the canonical spelling"
        );
    }
}
