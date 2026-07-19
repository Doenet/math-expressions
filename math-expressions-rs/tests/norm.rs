//! Canonicalization tests: exact folding, like-term/like-power combination,
//! identity elimination, and idempotence over the whole parser fixture corpus.

use math_expressions::norm::canonicalize;
use math_expressions::{Expr, Number, TextToAst, TextToAstOptions};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn canon(s: &str) -> Expr {
    canonicalize(&parse(s))
}

/// Canonical form of `a` equals canonical form of `b` (structurally).
fn same(a: &str, b: &str) {
    assert_eq!(canon(a), canon(b), "canon({a:?}) vs canon({b:?})");
}

#[test]
fn exact_constant_folding() {
    assert_eq!(canon("1/3 + 1/6"), Expr::Num(Number::rat(1, 2)));
    assert_eq!(canon("0.1 + 0.2"), Expr::Num(Number::rat(3, 10)));
    assert_eq!(canon("2 * 3 + 4"), Expr::Num(Number::Int(10)));
    assert_eq!(canon("2^10"), Expr::Num(Number::Int(1024)));
    assert_eq!(canon("1/2"), Expr::Num(Number::rat(1, 2)));
}

#[test]
fn like_terms_and_powers() {
    same("x + x", "2x");
    same("2x + 3x", "5x");
    same("x - x", "0");
    same("x * x", "x^2");
    same("x^2 * x^3", "x^5");
    same("x * x * x", "x^3");
    same("3*x*2", "6x");
}

#[test]
fn identity_elimination() {
    same("x + 0", "x");
    same("x * 1", "x");
    same("x * 0", "0");
    same("x^1", "x");
    same("x^0", "1");
    same("1^x", "1");
}

#[test]
fn commutativity_canonicalizes() {
    same("x + y", "y + x");
    same("a*b*c", "c*b*a");
    same("x*y + y*x", "2*x*y");
}

#[test]
fn div_and_neg_rewrite_away() {
    // Canonical form has no Div/Neg variants.
    fn has_div_or_neg(e: &Expr) -> bool {
        match e {
            Expr::Div(..) | Expr::Neg(_) => true,
            Expr::Add(xs) | Expr::Mul(xs) => xs.iter().any(has_div_or_neg),
            Expr::Pow(a, b) => has_div_or_neg(a) || has_div_or_neg(b),
            _ => false,
        }
    }
    for s in ["a/b", "-x", "a - b", "-(x+1)", "x/y/z", "1 - 2/3"] {
        assert!(!has_div_or_neg(&canon(s)), "{s:?} still has Div/Neg");
    }
}

#[test]
fn function_names_normalized() {
    same("arcsin(x)", "asin(x)");
    same("ln(x)", "log(x)");
    same("arctan(y)", "atan(y)");
}

#[test]
fn distributed_forms_are_not_equal_without_expansion() {
    // canonicalize does not expand; `(x+1)^2` stays a power. (Expansion is a
    // separate simplify concern; equality of these relies on the numerical
    // stage, tested elsewhere.)
    assert_ne!(canon("(x+1)^2"), canon("x^2 + 2x + 1"));
}

/// Canonicalization is idempotent on every expression the parser produces.
#[test]
fn idempotent_over_corpus() {
    #[derive(serde::Deserialize)]
    struct Case {
        input: String,
    }
    let files = [
        include_str!("fixtures/text-to-ast.json"),
        include_str!("fixtures/text-to-ast-edge.json"),
    ];
    let mut n = 0;
    for f in files {
        for case in serde_json::from_str::<Vec<Case>>(f).unwrap() {
            let Ok(expr) = TextToAst::new(TextToAstOptions::default()).convert(&case.input) else {
                continue;
            };
            let once = canonicalize(&expr);
            let twice = canonicalize(&once);
            assert_eq!(once, twice, "not idempotent: {:?}", case.input);
            n += 1;
        }
    }
    assert!(n > 100, "expected a substantial corpus, got {n}");
}

/// Canonicalization stays fast on adversarial inputs: folds that would
/// materialize astronomically large numbers are refused, not attempted.
#[test]
fn adversarial_folds_are_bounded() {
    // Would hang before the caps (10^12 multiplications / a 10^12-bit BigInt).
    assert!(matches!(canon("99999999999999!"), Expr::Apply(..)));
    assert!(matches!(canon("2^99999999999999"), Expr::Pow(..)));
    // Ordinary folds still work.
    assert_eq!(canon("10!"), Expr::Num(Number::Int(3628800)));
    assert_eq!(canon("2^62"), Expr::Num(Number::Int(1 << 62)));
}

#[test]
fn symmetric_relations_canonicalize() {
    same("x = y", "y = x");
    same("a + b = c", "c = b + a");
    same("x != y", "y != x");
    same("a = b = c", "c = b = a");
    // Directional relations are NOT symmetric.
    assert_ne!(canon("x < y"), canon("y < x"));
}

#[test]
fn pow_distribution_keeps_mul_flat() {
    // Regression (2026-07-18 review): when like-power combining inside mul()
    // produces an integer power of a product, the power-of-product rule
    // returns a Mul — its factors must merge with the surrounding product,
    // not nest (Mul inside Mul breaks the flat canonical invariant and
    // silently degrades stage-1 structural equality).
    let a = canonicalize(&parse("z (x y)^(1/2) (x y)^(3/2)"));
    let b = canonicalize(&parse("z x^2 y^2"));
    assert_eq!(a, b, "canonical forms must be identical (flat)");
    // Distributed factors must also combine with existing ones.
    let a = canonicalize(&parse("x^(-2) (x y)^(1/2) (x y)^(3/2)"));
    let b = canonicalize(&parse("y^2"));
    assert_eq!(a, b, "x's must cancel after distribution");
}

#[test]
fn limits_are_scoped_and_effective() {
    use math_expressions::limits::{self, Limits};
    use math_expressions::norm::expand; // via re-export? use crate path below if needed
    // Tight expand cap: a modest power-of-sum bails to the unexpanded form.
    let e = parse("(a+b)^6");
    let strict = Limits {
        max_expand_terms: 5,
        ..Limits::default()
    };
    let under = limits::with(strict, || expand(&e));
    assert!(
        matches!(under, Expr::Pow(..)),
        "expected unexpanded under tight cap, got {under:?}"
    );
    // Restored afterwards: same input expands normally.
    let after = expand(&e);
    assert!(matches!(after, Expr::Add(_)), "limits not restored");
}
