//! Declaring `pi`/`e`/`i` constants or variables, and the one display option
//! that depends on the declaration.
//!
//! The two halves of this file are the point of the design: the *semantic*
//! declaration reaches folding, sampling, assumptions and the polynomial
//! reader, while the *order* is alphabetical either way unless a document opts
//! into the conventional reading. Nothing here may change what
//! `normalize::order` does — that comparator is what makes `==` on canonical
//! trees mean equality, and canonical trees outlive the session that made them.

use math_expressions::constant_policy::{self, ConstantPolicy};
use math_expressions::{
    equals, expand, simplify, to_text, variables, EqOptions, Expr, TextOpts, TextToAst,
    TextToAstOptions,
};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn txt(e: &Expr) -> String {
    to_text(e, &TextOpts::default())
}

/// `simplify`, printed, under `policy`.
fn simp_under(policy: ConstantPolicy, s: &str) -> String {
    constant_policy::with(policy, || txt(&simplify(&parse(s))))
}

fn simp(s: &str) -> String {
    simp_under(ConstantPolicy::default(), s)
}

// ---------------------------------------------------------------------------
// Order is alphabetical, and stays that way
// ---------------------------------------------------------------------------

/// The default: `π`, `e` and `i` sort by name, exactly where a reader scanning
/// alphabetically would look for them, in sums and products alike.
///
/// This is what the JS oracle does under *every* setting of its own
/// `define_pi`/`define_e`/`define_i` — its `default_order` never consulted
/// them. A promotion rule was tried and reverted; the cases below are the ones
/// it got wrong.
#[test]
fn constants_sort_alphabetically_by_default() {
    // A product and a sum agree with each other about `e` vs `π`.
    assert_eq!(simp("2 pi e x"), "2 e π x");
    assert_eq!(simp("x + e + pi + a"), "a + e + π + x");
    // Consecutive letters stay consecutive: `e` and `i` are not lifted out of
    // an obvious run of variable names.
    assert_eq!(simp("d + e + f"), "d + e + f");
    assert_eq!(simp("h + i + j"), "h + i + j");
    assert_eq!(simp("f + pi"), "f + π");
    // Products place a constant by name, not by class.
    assert_eq!(simp("n pi"), "n π");
    assert_eq!(simp("pi r^2"), "π r^2");
}

/// Declaring a name has no effect on where it sorts. Only
/// [`ConstantPolicy::sort_constants_first`] moves anything, and that is a
/// display option a document opts into.
#[test]
fn the_declaration_alone_never_reorders() {
    for policy in [ConstantPolicy::default(), ConstantPolicy::ALL_VARIABLES] {
        assert_eq!(simp_under(policy, "x + e + pi + a"), "a + e + π + x");
        assert_eq!(simp_under(policy, "n pi"), "n π");
    }
}

/// The opt-in reading: declared constants lead the term, so the notation an
/// author expects to see comes out that way.
#[test]
fn sort_constants_first_gives_the_conventional_reading() {
    let conventional = ConstantPolicy {
        sort_constants_first: true,
        ..ConstantPolicy::default()
    };
    assert_eq!(simp_under(conventional, "r 2 pi"), "2 π r");
    assert_eq!(simp_under(conventional, "x + e + pi + a"), "π + e + a + x");
    assert_eq!(simp_under(conventional, "n pi"), "π n");

    // It promotes only *declared* names: with `e` a variable, `π` still leads
    // but `e` sorts among the variables.
    let pi_only = ConstantPolicy {
        define_e: false,
        define_i: false,
        sort_constants_first: true,
        ..ConstantPolicy::default()
    };
    assert_eq!(simp_under(pi_only, "x + e + pi + a"), "π + a + e + x");
}

/// The canonical comparator is policy-blind, so a tree canonicalized under one
/// declaration still compares equal to the same tree canonicalized under
/// another. This is the property that lets a stored expression outlive a change
/// of declaration.
#[test]
fn canonical_order_does_not_depend_on_the_policy() {
    for src in ["x + e + pi + a", "2 pi e x", "a b + pi x"] {
        let declared = constant_policy::with(ConstantPolicy::default(), || simplify(&parse(src)));
        let undeclared =
            constant_policy::with(ConstantPolicy::ALL_VARIABLES, || simplify(&parse(src)));
        assert_eq!(
            declared, undeclared,
            "canonical form of {src:?} moved with the policy"
        );
    }
}

// ---------------------------------------------------------------------------
// The declaration governs meaning
// ---------------------------------------------------------------------------

/// With `e` undeclared, `e^x` is a coordinate raised to a power — folding it to
/// `exp(x)` would silently rewrite the expression.
#[test]
fn undeclared_e_stops_being_the_exponential() {
    let vars = ConstantPolicy::ALL_VARIABLES;
    assert_eq!(simp("e^x"), "e^x");
    assert_eq!(simp_under(vars, "e^x"), "e^x");
    // `log(e)` is 1 only while `e` is Euler's number.
    assert_eq!(simp("log(e)"), "1");
    assert_eq!(simp_under(vars, "log(e)"), "log(e)");
}

/// With `i` undeclared, the complex folds stand down: a document whose
/// coordinates run `g, h, i` gets `i·i = i²`, not `−1`.
#[test]
fn undeclared_i_stops_the_complex_folds() {
    let vars = ConstantPolicy::ALL_VARIABLES;
    assert_eq!(simp("i i"), "-1");
    assert_eq!(simp_under(vars, "i i"), "i^2");
    assert_eq!(simp("(1+i)(1-i)"), "2");
    // Undeclared, there is no complex arithmetic to do, so the product does not
    // collapse — and expanding it leaves the `i²` standing.
    assert_eq!(simp_under(vars, "(1+i)(1-i)"), "(i + 1) (-i + 1)");
    assert_eq!(
        constant_policy::with(vars, || txt(&expand(&parse("(1+i)(1-i)")))),
        "-i^2 + 1"
    );
}

/// An undeclared name is a free variable to the equality sampler, so two
/// expressions that differ in it are no longer equal — the sampler must not
/// keep quietly substituting 3.14159… for a coordinate called `pi`.
#[test]
fn undeclared_constants_are_sampled_as_free_variables() {
    let vars = ConstantPolicy::ALL_VARIABLES;
    let opts = EqOptions::default();

    // Declared: `sin(π)` is zero, so the two sides agree.
    assert!(equals(&parse("sin(pi)"), &parse("0"), &opts));
    // Undeclared: `sin(pi)` is a function of a free variable and is not zero.
    assert!(constant_policy::with(vars, || !equals(
        &parse("sin(pi)"),
        &parse("0"),
        &opts
    )));
}

/// An undeclared name carries no facts of its own. `π > 0` is a thing to know
/// about the constant, not about a coordinate that happens to be spelled `pi`.
#[test]
fn undeclared_constants_carry_no_assumptions() {
    use math_expressions::{is_positive, Assumptions};
    let a = Assumptions::new();
    assert_eq!(is_positive(&parse("pi"), &a), Some(true));
    assert_eq!(
        constant_policy::with(ConstantPolicy::ALL_VARIABLES, || is_positive(
            &parse("pi"),
            &a
        )),
        None
    );
}

/// To the polynomial reader a declared constant is a coefficient and an
/// undeclared one an indeterminate, which is what decides whether `pi x` is
/// linear in one variable or quadratic in two.
#[test]
fn the_polynomial_reader_follows_the_declaration() {
    // Declared: `π` is a coefficient, so this expands as a single monomial.
    assert_eq!(txt(&expand(&parse("pi x (pi + 1)"))), "π^2 x + π x");
    // Undeclared: `pi` is just another variable, and the same expansion holds —
    // what changes is `variables`, which the reader keys on.
    let vars = ConstantPolicy::ALL_VARIABLES;
    assert!(constant_policy::with(vars, || {
        variables(&parse("pi x")).contains(&"pi".to_string())
    }));
}

/// `variables` lists the names a tree mentions and does not filter by policy —
/// alpha94's filter keeps `e` precisely when `define_e` is on, and the
/// constant/variable distinction the passes act on is `is_constant_symbol`, not
/// this listing.
#[test]
fn variables_lists_the_constant_names_under_either_policy() {
    for policy in [ConstantPolicy::default(), ConstantPolicy::ALL_VARIABLES] {
        let got = constant_policy::with(policy, || variables(&parse("pi + e + i + x")));
        assert_eq!(got, vec!["pi", "e", "i", "x"]);
    }
}

/// The explicit `MathConst` spelling means the constant whatever the naming
/// declaration says — that is how a document with `(e, f)` points can still
/// write Euler's number where it means it. Declared, the two spellings unify;
/// undeclared, they are different values and must not compare equal.
#[test]
fn the_explicit_constant_spelling_survives_an_undeclared_name() {
    use math_expressions::MathConst;
    let konst = Expr::Const(MathConst::E);
    let name = Expr::sym("e");

    assert_eq!(simplify(&konst), simplify(&name));
    constant_policy::with(ConstantPolicy::ALL_VARIABLES, || {
        assert_ne!(
            simplify(&konst),
            simplify(&name),
            "an undeclared `e` must not collapse onto Euler's number"
        );
    });
}

/// Undeclared, the two spellings can stand in one tree — and then the
/// comparator has to separate them. It reports the same *name* for both so they
/// sort together, but a tie there would leave their order to the (stable) sort's
/// input order, and two sums of the same terms would canonicalize to trees that
/// `==` calls unequal.
#[test]
fn the_two_spellings_order_deterministically_when_both_appear() {
    use math_expressions::MathConst;
    let konst = Expr::Const(MathConst::Pi);
    let name = Expr::sym("pi");

    constant_policy::with(ConstantPolicy::ALL_VARIABLES, || {
        let one = simplify(&Expr::Add(vec![konst.clone(), name.clone()]));
        let other = simplify(&Expr::Add(vec![name.clone(), konst.clone()]));
        assert_eq!(
            one, other,
            "canonical form of `Const(Pi) + Sym(\"pi\")` depends on which was written first"
        );
    });
}

/// The infinity fold in the `add` constructor absorbs a *constant* term, and
/// a declared `π` is one — `canonicalize` and `simplify` must not disagree about
/// which sums reduce. Undeclared, the same `pi` is a free variable of unknown
/// magnitude and blocks the fold, exactly as `x` does.
#[test]
fn an_infinity_absorbs_a_declared_constant_but_not_a_variable() {
    assert_eq!(simp("pi + infinity"), "∞");
    assert_eq!(simp("x + infinity"), "x + ∞");
    assert_eq!(
        simp_under(ConstantPolicy::ALL_VARIABLES, "pi + infinity"),
        "π + ∞"
    );
}

/// A policy scope restores the previous one, including when the body panics —
/// ambient state that leaked would silently mis-declare every later call on the
/// thread.
#[test]
fn the_scope_restores_the_previous_policy_on_unwind() {
    assert!(constant_policy::current().define_e);
    let caught = std::panic::catch_unwind(|| {
        constant_policy::with(ConstantPolicy::ALL_VARIABLES, || panic!("boom"));
    });
    assert!(caught.is_err());
    assert!(constant_policy::current().define_e);
}
