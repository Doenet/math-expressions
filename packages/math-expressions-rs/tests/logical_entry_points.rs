//! The standalone entry points the JS compat layer's assumption store binds to:
//! [`push_not`] / [`flatten_logical`] (logical normal form *without* relation
//! canonicalization), [`cmp_default_order`] (the legacy sort key as a
//! comparator) and [`linear_decomposition`].

use math_expressions::{
    cmp_default_order, flatten_logical, linear_decomposition, push_not, Expr, RelOp,
};
use std::cmp::Ordering;

fn rel(op: RelOp, a: &str, b: &str) -> Expr {
    Expr::Relation {
        operands: vec![Expr::sym(a), Expr::sym(b)],
        ops: vec![op],
    }
}

/// The reason `push_not` is exposed at all. `simplify_logical` canonicalizes
/// first, which restates `x > a` as `a < x`; the store must file the fact with
/// the operands the caller wrote, so this path may not touch their order.
#[test]
fn push_not_negates_a_relation_without_reorienting_it() {
    for (op, negated) in [
        (RelOp::Gt, RelOp::Le),
        (RelOp::Lt, RelOp::Ge),
        (RelOp::Ge, RelOp::Lt),
        (RelOp::Le, RelOp::Gt),
        (RelOp::Eq, RelOp::Ne),
        (RelOp::In, RelOp::NotIn),
        (RelOp::Superset, RelOp::NotSuperset),
    ] {
        let got = push_not(&Expr::Not(Box::new(rel(op, "x", "a"))));
        assert_eq!(got, rel(negated, "x", "a"), "not({op:?}) reoriented");
    }
}

/// A relation `push_not` merely walks past is left exactly as it came, too.
#[test]
fn push_not_leaves_un_negated_relations_alone() {
    let tree = Expr::And(vec![rel(RelOp::Gt, "x", "a"), rel(RelOp::Ni, "S", "y")]);
    assert_eq!(push_not(&tree), tree);
}

#[test]
fn push_not_is_de_morgan_plus_double_negation() {
    let and = Expr::And(vec![rel(RelOp::Lt, "x", "a"), rel(RelOp::Lt, "y", "b")]);
    assert_eq!(
        push_not(&Expr::Not(Box::new(and.clone()))),
        Expr::Or(vec![rel(RelOp::Ge, "x", "a"), rel(RelOp::Ge, "y", "b")])
    );
    assert_eq!(
        push_not(&Expr::Not(Box::new(Expr::Not(Box::new(and.clone()))))),
        and
    );
    // Nothing to push through: the `not` stays put rather than being dropped.
    let opaque = Expr::Not(Box::new(Expr::Add(vec![Expr::sym("x"), Expr::sym("y")])));
    assert_eq!(push_not(&opaque), opaque);
}

#[test]
fn flatten_logical_merges_and_collapses() {
    let nested = Expr::And(vec![
        Expr::And(vec![rel(RelOp::Lt, "x", "a"), rel(RelOp::Lt, "y", "b")]),
        rel(RelOp::Lt, "z", "c"),
    ]);
    assert_eq!(
        flatten_logical(&nested),
        Expr::And(vec![
            rel(RelOp::Lt, "x", "a"),
            rel(RelOp::Lt, "y", "b"),
            rel(RelOp::Lt, "z", "c"),
        ])
    );
    // A one-operand connective *is* its operand.
    let single = rel(RelOp::Lt, "x", "a");
    assert_eq!(flatten_logical(&Expr::And(vec![single.clone()])), single);
}

#[test]
fn cmp_default_order_uses_the_legacy_key() {
    // Numbers (key tag 0) before symbols (tag 1) before products (tag 4).
    assert_eq!(
        cmp_default_order(&Expr::int(3), &Expr::sym("x")),
        Ordering::Less
    );
    assert_eq!(
        cmp_default_order(
            &Expr::sym("x"),
            &Expr::Mul(vec![Expr::int(2), Expr::sym("x")])
        ),
        Ordering::Less
    );
    assert_eq!(
        cmp_default_order(&Expr::sym("y"), &Expr::sym("x")),
        Ordering::Greater
    );
    assert_eq!(
        cmp_default_order(&Expr::sym("x"), &Expr::sym("x")),
        Ordering::Equal
    );
}

#[test]
fn linear_decomposition_reads_affine_combinations() {
    let vars = |names: &[&str]| names.iter().map(|s| s.to_string()).collect::<Vec<_>>();

    // 3a + 4b + 5
    let e = Expr::Add(vec![
        Expr::Mul(vec![Expr::int(3), Expr::sym("a")]),
        Expr::Mul(vec![Expr::int(4), Expr::sym("b")]),
        Expr::int(5),
    ]);
    let (coeffs, b) = linear_decomposition(&e, &vars(&["a", "b"])).expect("affine");
    assert_eq!(coeffs, vec![Expr::int(3), Expr::int(4)]);
    assert_eq!(b, Expr::int(5));

    // q − x, the shape `get_assumptions` restates facts through.
    let e = Expr::Add(vec![Expr::sym("q"), Expr::Neg(Box::new(Expr::sym("x")))]);
    let (coeffs, b) = linear_decomposition(&e, &vars(&["q", "x"])).expect("affine");
    assert_eq!(coeffs, vec![Expr::int(1), Expr::int(-1)]);
    assert_eq!(b, Expr::int(0));

    // A variable that does not occur still gets an answer: zero.
    let (coeffs, _) = linear_decomposition(&Expr::sym("a"), &vars(&["a", "z"])).expect("affine");
    assert_eq!(coeffs, vec![Expr::int(1), Expr::int(0)]);

    // Not affine: a product of two of the named variables, a square, a
    // non-numeric coefficient, and a leftover variable outside the list.
    let ab = Expr::Mul(vec![Expr::sym("a"), Expr::sym("b")]);
    assert!(linear_decomposition(&ab, &vars(&["a", "b"])).is_none());
    let a2 = Expr::Pow(Box::new(Expr::sym("a")), Box::new(Expr::int(2)));
    assert!(linear_decomposition(&a2, &vars(&["a"])).is_none());
    let ca = Expr::Mul(vec![Expr::sym("c"), Expr::sym("a")]);
    assert!(linear_decomposition(&ca, &vars(&["a"])).is_none());
    let a_plus_z = Expr::Add(vec![Expr::sym("a"), Expr::sym("z")]);
    assert!(linear_decomposition(&a_plus_z, &vars(&["a"])).is_none());

    // A fraction is not a "number" here — JS asks `typeof tree === "number"`
    // and `−4/3` spells as `["/", -4, 3]`. `get_assumptions` depends on the
    // rejection: it falls back to the per-variable facts, which is where the
    // transitive consequences come from.
    let four_thirds_a = Expr::Div(
        Box::new(Expr::Mul(vec![Expr::int(-4), Expr::sym("a")])),
        Box::new(Expr::int(3)),
    );
    assert!(linear_decomposition(&four_thirds_a, &vars(&["a"])).is_none());
    // A decimal, which does spell as a bare number, is accepted.
    let half_a = Expr::Mul(vec![
        Expr::Num(math_expressions::Number::from_f64(0.5)),
        Expr::sym("a"),
    ]);
    let (coeffs, _) = linear_decomposition(&half_a, &vars(&["a"])).expect("affine");
    assert_eq!(
        coeffs,
        vec![Expr::Num(math_expressions::Number::from_f64(0.5))]
    );
}
