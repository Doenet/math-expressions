//! STRUCTURAL_COMPARISON F1: structural comparisons over the faithful tree.

use math_expressions::{
    check_structural_comparison, structural_equality, EqOptions, Expr, StructuralComparison,
    TextToAst, TextToAstOptions,
};

/// Parse to the faithful (un-canonicalized) tree.
fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn ok(s: &str, c: &StructuralComparison) -> bool {
    check_structural_comparison(&parse(s), c).ok
}

#[test]
fn reduced_fraction() {
    let c = StructuralComparison::ReducedFraction;
    assert!(ok("1/2", &c));
    assert!(ok("3/2", &c));
    assert!(ok("0.5", &c)); // decimal -> Rat(1,2), reduced by construction
    assert!(ok("x/2", &c));
    assert!(ok("(x+1)/(x-1)", &c));
    assert!(!ok("2/4", &c));
    assert!(!ok("(x^2-1)/(x-1)", &c)); // cancels to x+1
    assert!(!ok("1/sqrt(2)", &c)); // surd in denominator
    // Regression: a numeric factor common to a symbolic numerator/denominator
    // is not lowest terms (`canonicalize` folds these, so check the written form).
    assert!(!ok("2x/2", &c)); // -> x
    assert!(!ok("4x/6", &c)); // -> 2x/3
    assert!(ok("3x/2", &c)); // genuinely reduced
}

#[test]
fn mixed_and_improper() {
    let m = StructuralComparison::MixedNumber;
    assert!(ok("2+1/3", &m));
    assert!(ok("-2-1/3", &m)); // negative mixed number: -(2 + 1/3)
    assert!(!ok("2-1/3", &m)); // = 5/3, opposite signs — not a mixed number
    assert!(!ok("1/3", &m));
    assert!(!ok("2+3/2", &m)); // 3/2 is not a proper fraction
    assert!(!ok("5", &m));

    let i = StructuralComparison::ImproperFraction;
    assert!(ok("7/2", &i));
    assert!(ok("3/2", &i));
    assert!(!ok("1/2", &i));
    assert!(!ok("x/2", &i));
    // Regression: a decimal is not a written fraction a/b.
    assert!(!ok("2.5", &i));
    assert!(!ok("2+0.5", &m)); // nor a mixed number
}

#[test]
fn decimal_and_exact() {
    let d = StructuralComparison::Decimal { places: None };
    assert!(ok("0.5", &d));
    assert!(ok("3", &d));
    assert!(!ok("1/2", &d));
    assert!(!ok("sqrt(2)", &d));

    let e = StructuralComparison::ExactValue;
    assert!(ok("sqrt(2)/2", &e));
    assert!(ok("1/2", &e));
    assert!(!ok("0.5", &e));
    assert!(!ok("0.5+x", &e));
}

#[test]
fn combined_like_terms() {
    let c = StructuralComparison::CombinedLikeTerms;
    assert!(ok("6x+2y", &c));
    assert!(ok("x+1", &c));
    assert!(ok("x^2+2x+1", &c));
    assert!(!ok("2x+3x", &c));
    assert!(!ok("2+3", &c));
    assert!(!ok("x+2x", &c));
}

#[test]
fn expanded() {
    let c = StructuralComparison::Expanded;
    assert!(ok("x^2+2x+1", &c));
    assert!(ok("x^2-1", &c));
    assert!(!ok("(x+1)^2", &c));
    assert!(!ok("x*(x+1)", &c));
    assert!(!ok("(x+1)(x-1)", &c));
}

#[test]
fn factored_completely() {
    let c = StructuralComparison::FactoredCompletely;
    assert!(ok("(x-1)(x+1)", &c));
    assert!(ok("x^2+1", &c)); // irreducible over Q -> already factored
    assert!(ok("2(x-1)(x+1)", &c));
    assert!(!ok("x^2-1", &c));
    assert!(!ok("(x^2-1)(x+2)", &c)); // first factor still reducible
}

#[test]
fn single_fraction() {
    let c = StructuralComparison::SingleFraction;
    assert!(ok("(x+1)/(x-1)", &c));
    assert!(!ok("1/x+1/y", &c));
    assert!(!ok("x+1", &c));
}

#[test]
fn no_negative_exponents() {
    let c = StructuralComparison::NoNegativeExponents;
    assert!(ok("x^2", &c));
    assert!(ok("1/x^2", &c)); // division is allowed
    assert!(!ok("x^(-2)", &c));
    assert!(!ok("2x^(-1)", &c));
}

#[test]
fn radical_simplified() {
    let c = StructuralComparison::RadicalSimplified;
    assert!(ok("2*sqrt(3)", &c));
    assert!(ok("sqrt(2)/2", &c));
    assert!(!ok("sqrt(8)", &c)); // 8 = 4*2 not square-free
    assert!(!ok("1/sqrt(2)", &c)); // surd in denominator
    // Regression: cube roots must be cube-free, not just square-free.
    assert!(!ok("cbrt(16)", &c)); // 16 = 8*2 -> 2 cbrt(2)
    assert!(ok("cbrt(2)", &c)); // cube-free
    // Regression: nthroot(x, m) radicands are checked by index m.
    assert!(!ok("nthroot(16,4)", &c)); // 16 = 2^4
    assert!(ok("nthroot(2,3)", &c)); // 3rd-power-free
    // Regression: fraction-exponent radicals `^(1/m)` (the common written form,
    // a Div exponent) are checked, and a negative fractional power is a surd in
    // the denominator.
    assert!(!ok("8^(1/2)", &c)); // = 2 sqrt(2)
    assert!(!ok("16^(1/4)", &c)); // = 2
    assert!(ok("2^(1/3)", &c)); // cube-free radicand
    assert!(!ok("x^(-1/2)", &c)); // 1/sqrt(x): surd in denominator
    assert!(ok("x^(-2)", &c)); // 1/x^2: no radical
}

#[test]
fn completed_square() {
    let c = StructuralComparison::CompletedSquare;
    assert!(ok("(x-3)^2+4", &c));
    assert!(ok("2(x-1)^2+5", &c));
    assert!(ok("(2x-3)^2+1", &c)); // coefficient inside the linear part
    assert!(!ok("x^2-6x+13", &c));
    assert!(!ok("x^2", &c));
    // Regression: the squared base must be a *linear polynomial* in the variable —
    // var inside a denominator or a function is not linear.
    assert!(!ok("(1/x)^2+5", &c));
    assert!(!ok("sin(x)^2+5", &c));
}

#[test]
fn integration_constant() {
    let c = StructuralComparison::HasIntegrationConstant { exclude: None };
    assert!(ok("x^2+C", &c));
    assert!(ok("x+C", &c)); // antiderivative of 1 — still recognized
    assert!(!ok("x^2", &c));
    // Regression: a variable that merely appears as a lone term is not "+C"
    // (it is not isolated — `x` also appears in `x^2`).
    assert!(!ok("x+x^2", &c));

    let cx = StructuralComparison::HasIntegrationConstant {
        exclude: Some("x".into()),
    };
    assert!(check_structural_comparison(&parse("x^2+C"), &cx).ok);
}

#[test]
fn parse_is_faithful_and_checks_flatten_internally() {
    use math_expressions::expr::flatten;
    // Parsing is now faithful: explicit right-nesting `a+(b+c)` survives, so the
    // raw tree differs from its flattened form...
    let raw = parse("a+(b+c)");
    assert_ne!(raw, flatten(raw.clone()), "grouping should survive parsing");
    // ...but the checks flatten internally, so they are unaffected by grouping.
    assert!(ok("a+(b+c)", &StructuralComparison::CombinedLikeTerms));
}

#[test]
fn structural_equality_requires_structure_and_value() {
    let opts = EqOptions::default();
    let se = |student: &str, key: &str, c: &StructuralComparison| {
        structural_equality(&parse(student), &parse(key), c, &opts)
    };
    let c = StructuralComparison::FactoredCompletely;
    // factored AND value-equal -> pass
    assert!(se("(x-1)(x+1)", "x^2-1", &c));
    // value-equal but NOT factored -> fail on structure
    assert!(!se("x^2-1", "x^2-1", &c));
    // factored but NOT value-equal -> fail on value
    assert!(!se("(x-1)(x+1)", "x^2+5", &c));
}

#[test]
fn same_structure_is_equals_syntactic() {
    use math_expressions::equals_syntactic;
    let opts = EqOptions::default();
    let same = StructuralComparison::SameStructure;
    // `SameStructure` is exactly `equals_syntactic` (JS equalsViaSyntax).
    for (a, b) in [("ln(x)", "log(x)"), ("x+y", "x+y"), ("x+y", "y+x")] {
        assert_eq!(
            structural_equality(&parse(a), &parse(b), &same, &opts),
            equals_syntactic(&parse(a), &parse(b), &opts),
            "SameStructure must match equals_syntactic for {a:?} vs {b:?}",
        );
    }
    assert!(structural_equality(&parse("ln(x)"), &parse("log(x)"), &same, &opts));
    assert!(!structural_equality(&parse("x+y"), &parse("y+x"), &same, &opts));
    // As a unary check it is rejected (needs a key).
    assert!(!check_structural_comparison(&parse("x+y"), &same).ok);
}
