//! The compat polynomial engine: the AST spelling is the contract.
//!
//! Callers of this API hand `["polynomial", v, [[d, c], …]]` in as a literal
//! and compare what comes back with deep equality, so a result that is
//! *value*-correct but differently arranged is a broken result. These tests are
//! written against the JSON codec for that reason: they check the spelling, not
//! just the mathematics.

use math_expressions::polynomials::compat as poly;
use math_expressions::{Expr, TextToAst, TextToAstOptions};
use serde_json::{json, Value};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

/// `text` read as a polynomial, in its AST spelling.
fn read(text: &str) -> Value {
    match poly::expression_to_polynomial(&parse(text)) {
        Some(p) => poly::poly_to_json(&p),
        None => Value::Bool(false),
    }
}

fn of(v: Value) -> poly::Poly {
    poly::poly_from_json(&v).expect("well-formed polynomial")
}

#[test]
fn a_polynomial_is_recursive_in_default_order() {
    assert_eq!(read("1+x^3"), json!(["polynomial", "x", [[0, 1], [3, 1]]]));
    // The *later* variable becomes the coefficient, so which variable a
    // polynomial is written in is decided by the legacy default order.
    assert_eq!(
        read("(x+y)^2"),
        json!([
            "polynomial",
            "x",
            [
                [0, ["polynomial", "y", [[2, 1]]]],
                [1, ["polynomial", "y", [[1, 2]]]],
                [2, 1]
            ]
        ])
    );
}

#[test]
fn pi_is_a_coefficient_and_sin_is_a_variable() {
    // The distinction that decides every gcd downstream: `π` is a number, so it
    // multiplies terms; `sin(x)` is an indeterminate, so it divides them.
    assert_eq!(
        read("9x^(2/3)-pi*x"),
        json!([
            "polynomial",
            "x",
            [
                [0, ["polynomial", ["^", "x", ["/", 1, 3]], [[2, 9]]]],
                [1, ["-", "pi"]]
            ]
        ])
    );
    assert_eq!(
        read("x sin(x)-x"),
        json!([
            "polynomial",
            "x",
            [[1, ["polynomial", ["apply", "sin", "x"], [[0, -1], [1, 1]]]]]
        ])
    );
}

// `3.1415` is a decimal with an awkward denominator, not an attempt at π.
#[allow(clippy::approx_constant)]
#[test]
fn a_non_integer_power_becomes_a_root_variable_only_when_the_root_is_small() {
    // `t^3.1` is a polynomial in `t^(1/10)`; `t^3.1415` would need `t^(1/2000)`,
    // which is not worth naming, so the whole power stays opaque.
    assert_eq!(
        read("5t^(3.1)"),
        json!(["polynomial", ["^", "t", ["/", 1, 10]], [[31, 5]]])
    );
    assert_eq!(
        read("5t^(3.1415)"),
        json!(["polynomial", ["^", "t", 3.1415], [[1, 5]]])
    );
}

#[test]
fn a_tuple_is_not_a_polynomial() {
    assert_eq!(read("(3,4)"), json!(false));
}

#[test]
fn the_representation_stays_sparse() {
    // A dense representation cannot hold this at all; the API is asked about it
    // directly, so sparsity is a correctness property here, not a speed one.
    assert_eq!(
        read("t-t^1000000000"),
        json!(["polynomial", "t", [[1, 1], [1000000000, -1]]])
    );
}

#[test]
fn a_round_trip_through_an_expression_is_a_fixpoint() {
    let round = |e: &Expr| {
        poly::polynomial_to_expression(&poly::expression_to_polynomial(e).expect("a polynomial"))
    };
    let once = round(&parse("(x+y)^3"));
    assert_eq!(round(&once), once);
}

#[test]
fn gcd_cancels_an_opaque_variable() {
    // `sin(x)` divides both sides exactly as a named variable would.
    let (top, bottom) = poly::reduce_rational_expression(
        &of(read("x sin(x)-y sin(x)")),
        &of(read("x^2 sin(x)-z^2 sin(x)")),
    )
    .expect("reducible");
    assert_eq!(poly::poly_to_json(&top), read("x-y"));
    assert_eq!(poly::poly_to_json(&bottom), read("x^2-z^2"));
}

#[test]
fn a_reduced_denominator_is_monic() {
    // The denominator's leading coefficient divides out of both sides, which is
    // what makes the answer canonical rather than merely reduced.
    let (top, bottom) =
        poly::reduce_rational_expression(&of(read("(x^2+x+y)(z+t)")), &of(read("(u^4+x)(2z+2t)")))
            .expect("reducible");
    assert_eq!(poly::poly_to_json(&top), read("1/2x^2+1/2x+1/2y"));
    assert_eq!(poly::poly_to_json(&bottom), read("u^4+x"));
}

#[test]
fn gcd_and_lcm_agree_on_a_two_variable_ideal() {
    let f = of(json!([
        "polynomial",
        "x",
        [[1, ["polynomial", "y", [[2, 7]]]]]
    ]));
    let g = of(json!([
        "polynomial",
        "x",
        [[2, ["polynomial", "y", [[1, ["/", 1, 2]]]]]]
    ]));
    assert_eq!(
        poly::poly_to_json(&poly::poly_gcd(&f, &g).expect("a gcd")),
        json!(["polynomial", "x", [[1, ["polynomial", "y", [[1, 1]]]]]])
    );
    assert_eq!(
        poly::poly_to_json(&poly::poly_lcm(&f, &g).expect("an lcm")),
        json!(["polynomial", "x", [[2, ["polynomial", "y", [[2, 1]]]]]])
    );
}

#[test]
fn an_ideal_containing_a_unit_has_the_trivial_basis() {
    let basis = poly::reduced_grobner(&[
        of(json!(["polynomial", "x", [[1, 1]]])),
        of(json!(["polynomial", "x", [[0, 1], [2, 1]]])),
    ]);
    assert_eq!(poly::polys_to_json(&basis), json!([1]));
}

#[test]
fn division_records_the_quotient_it_built() {
    // `x^3 + 5x + 2 = (x^2 + 5)·x + 2`, reported as the monomials that were
    // cancelled — reading them back out is how exact division recovers a
    // quotient.
    let (quotient, remainder) = poly::poly_div(
        &of(json!(["polynomial", "x", [[0, 2], [1, 5], [3, 1]]])),
        &[of(json!(["polynomial", "x", [[1, 1]]]))],
    );
    let quotient: Vec<Value> = quotient
        .iter()
        .map(|(i, m)| json!([i, poly::mono_to_json(m)]))
        .collect();
    assert_eq!(
        Value::Array(quotient),
        json!([[0, ["monomial", 1, [["x", 2]]]], [0, 5]])
    );
    assert_eq!(poly::poly_to_json(&remainder), json!(2));
}

#[test]
fn the_elimination_variable_does_not_depend_on_how_the_input_is_spelled() {
    // `poly_lcm` eliminates by rejecting the basis elements led by its
    // auxiliary variable, and a `Poly::Rec` is led by its *least* variable
    // under the default order. The JS engine's `_t` sorts after every
    // uppercase name (`"A" < "_t"` byte-wise), so a gcd in `A` used to come
    // back still carrying the auxiliary variable and reduce to nothing.
    for v in ["x", "A", "Z", "alpha"] {
        assert_eq!(
            read_pair_reduced(&format!("{v}^2-1"), &format!("{v}^2+2{v}+1")),
            (
                json!(["polynomial", v, [[0, -1], [1, 1]]]),
                json!(["polynomial", v, [[0, 1], [1, 1]]])
            ),
            "reducing a rational expression in {v:?}"
        );
    }
}

/// `top/bottom` in lowest terms, both sides in their AST spelling.
fn read_pair_reduced(top: &str, bottom: &str) -> (Value, Value) {
    let top = poly::expression_to_polynomial(&parse(top)).expect("a polynomial");
    let bottom = poly::expression_to_polynomial(&parse(bottom)).expect("a polynomial");
    let (t, b) = poly::reduce_rational_expression(&top, &bottom).expect("a reduction");
    (poly::poly_to_json(&t), poly::poly_to_json(&b))
}
