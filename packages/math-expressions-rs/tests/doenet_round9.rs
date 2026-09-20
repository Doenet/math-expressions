//! The round-9 DoenetML items: matrix–vector products, powers of `i`, and the
//! sign of a zero that rounding produces. (The fourth, an explicit bound on
//! `unflatten`, is pinned by `unflatten_refuses_a_node_too_wide_to_fold` in
//! `math-expressions-rs-wasm/src-rust/js_match.rs`, beside the function it
//! bounds — the bound is not reachable from this crate.)
//!
//! Each `assert` here is a row of an expected-behaviour table DoenetML filed,
//! including the neighbouring rows that must *not* change — those are the
//! point of the test, not padding.

use math_expressions::{expand, expr, num::Number, simplify, Expr, LatexToAst, TextToAst};

fn t(s: &str) -> Expr {
    TextToAst::new(Default::default()).convert(s).unwrap()
}

fn l(s: &str) -> Expr {
    LatexToAst::new(Default::default()).convert(s).unwrap()
}

fn js(e: &Expr) -> String {
    expr::serde::to_js(e).to_string()
}

const M: &str = r"\begin{bmatrix}a&b\\c&d\end{bmatrix}";

// ---- item 20: matrix products ------------------------------------------

/// `M·v` contracts under `expand` — the pass whose whole job is multiplying
/// things out — and the vector keeps its own container: a tuple stays a tuple,
/// `⟨p,q⟩` stays an altvector.
#[test]
fn a_matrix_times_a_vector_contracts_under_expand() {
    assert_eq!(
        js(&expand(&l(&format!("{M}(e,f)")))),
        r#"["tuple",["+",["*","a","e"],["*","b","f"]],["+",["*","c","e"],["*","d","f"]]]"#
    );
    assert_eq!(
        js(&expand(&l(&format!(r"{M}\langle p,q\rangle")))),
        r#"["altvector",["+",["*","a","p"],["*","b","q"]],["+",["*","c","p"],["*","d","q"]]]"#
    );
    // Numeric entries fold all the way.
    assert_eq!(
        js(&expand(&l(r"\begin{bmatrix}3&-1\\1&2\end{bmatrix}(1,2)"))),
        r#"["tuple",1,5]"#
    );
}

/// The contraction goes back through the expansion, so the factors around it
/// get their turn on the vector it leaves behind: a scalar lands *inside* the
/// components rather than stranded outside them, and a second matrix contracts
/// in turn instead of stopping after one.
#[test]
fn the_factors_around_a_contraction_are_expanded_too() {
    assert_eq!(
        js(&expand(&l(&format!("2{M}(e,f)")))),
        r#"["tuple",["+",["*",2,"a","e"],["*",2,"b","f"]],["+",["*",2,"c","e"],["*",2,"d","f"]]]"#
    );
    // `M·M·v`: both matrices contract, left to right. The four terms of each
    // component come out graded-lexicographically (`a²e, abf, bce, bdf`) rather
    // than grouped by the vector component (`a²e, bce, abf, bdf`): `e` and `f`
    // are the coordinate names here, and nothing gives `e` precedence over `b`.
    assert_eq!(
        js(&expand(&l(&format!("{M}{M}(e,f)")))),
        r#"["tuple",["+",["*",["^","a",2],"e"],["*","a","b","f"],["*","b","c","e"],["*","b","d","f"]],["+",["*","a","c","e"],["*","b","c","f"],["*","c","d","e"],["*",["^","d",2],"f"]]]"#
    );
}

/// Vectors combine under `expand` whichever sign they carry. Subtraction is the
/// case that matters: the negated term reaches the sum as `Neg(Seq)`, and until
/// negation distributed over a vector too, the whole sum declined to combine
/// and `expand` fell short of `simplify` on the most ordinary input there is.
#[test]
fn vector_sums_and_differences_both_combine_under_expand() {
    assert_eq!(
        js(&expand(&t("-(a,b)"))),
        r#"["tuple",["-","a"],["-","b"]]"#
    );
    assert_eq!(
        js(&expand(&t("(a,b)-(c,d)"))),
        js(&simplify(&t("(a,b)-(c,d)")))
    );
    assert_eq!(
        js(&expand(&t("(a,b)-(c,d)"))),
        r#"["tuple",["+","a",["-","c"]],["+","b",["-","d"]]]"#
    );
    // A mixed-class sum still declines to *merge*: `[c,d]` is read as an
    // interval, so it stays its own term. The sign distributes into it either
    // way — that part is scalar multiplication, not addition — and `simplify`
    // does exactly the same, which is the property worth pinning.
    assert_eq!(
        js(&expand(&t("(a,b)-[c,d]"))),
        r#"["+",["tuple","a","b"],["array",["-","c"],["-","d"]]]"#
    );
    assert_eq!(
        js(&expand(&t("(a,b)-[c,d]"))),
        js(&simplify(&t("(a,b)-[c,d]")))
    );
}

/// Nothing contracts without being asked. A `<math>` that requests no
/// simplification renders what the author typed, so the product stays written
/// as a product — and, in particular, the vector is *not* distributed into the
/// matrix entries, which is what it used to do (`[[a·(e,f), b·(e,f)], …]`).
#[test]
fn the_product_is_left_alone_until_expand() {
    let written = r#"["*",["matrix",["tuple",2,2],["tuple",["tuple","a","b"],["tuple","c","d"]]],["tuple","e","f"]]"#;
    assert_eq!(js(&l(&format!("{M}(e,f)"))), written);
    assert_eq!(js(&simplify(&l(&format!("{M}(e,f)")))), written);
}

/// A vector on the *left* is a row (1×N) and contracts with the matrix on the
/// right: `(e,f)·M` → the row-vector product `(ea+fc, eb+fd)`, in the vector's
/// own notation. (The positional rule: left operand of a `·` is a row, right is
/// a column — so `M·v` is a column and `v·M` a row.)
#[test]
fn a_vector_on_the_left_contracts_as_a_row() {
    assert_eq!(
        js(&expand(&l(&format!("(e,f){M}")))),
        r#"["tuple",["+",["*","a","e"],["*","c","f"]],["+",["*","b","e"],["*","d","f"]]]"#
    );
    // A length mismatch still does not conform, so it stays a product.
    assert_eq!(
        js(&expand(&l(&format!("{M}(e,f,g)")))),
        r#"["*",["matrix",["tuple",2,2],["tuple",["tuple","a","b"],["tuple","c","d"]]],["tuple","e","f","g"]]"#
    );
}

/// Matrix × matrix already folded, and still does; a scalar still rides into
/// the entries.
#[test]
fn matrix_times_matrix_and_scalar_scaling_are_unchanged() {
    assert_eq!(
        js(&simplify(&l(&format!(
            r"{M}\begin{{bmatrix}}1&0\\0&1\end{{bmatrix}}"
        )))),
        r#"["matrix",["tuple",2,2],["tuple",["tuple","a","b"],["tuple","c","d"]]]"#
    );
    assert_eq!(
        js(&simplify(&l(&format!("2{M}")))),
        r#"["matrix",["tuple",2,2],["tuple",["tuple",["*",2,"a"],["*",2,"b"]],["tuple",["*",2,"c"],["*",2,"d"]]]]"#
    );
}

// ---- item 21: powers of `i` --------------------------------------------

/// The four-cycle. Without it `i` was the one number in the engine that did no
/// arithmetic.
#[test]
fn integer_powers_of_i_close_the_cycle() {
    assert_eq!(js(&simplify(&t("i^2"))), "-1");
    assert_eq!(js(&simplify(&t("i^3"))), r#"["-","i"]"#);
    assert_eq!(js(&simplify(&t("i^4"))), "1");
    assert_eq!(js(&simplify(&t("i^5"))), "\"i\"");
    // Negative exponents come out of the same cycle: `1/i` is `−i`.
    assert_eq!(js(&simplify(&t("1/i"))), r#"["-","i"]"#);
    // Collected inside a product: `a·i·b·i·c·i` is `i³` times the rest.
    assert_eq!(
        js(&simplify(&t("aibici"))),
        r#"["-",["*","a","b","c","i"]]"#
    );
}

/// A variable-free complex expression evaluates exactly, in ℚ(i). The product
/// of two sums is the case the smart constructor cannot see, because `simplify`
/// does not expand.
#[test]
fn variable_free_complex_arithmetic_is_exact() {
    assert_eq!(js(&simplify(&t("(1+i)(1-i)"))), "2");
    assert_eq!(js(&simplify(&t("(2+3i)(2-3i)"))), "13");
    assert_eq!(js(&simplify(&t("(1+i)/(1-i)"))), "\"i\"");
    assert_eq!(js(&simplify(&t("(1+i)^8"))), "16");
    assert_eq!(
        js(&simplify(&t("(1+2i)+(3-5i)"))),
        r#"["+",["*",-3,"i"],4]"#
    );
    // Exact stays exact: no floats appear.
    assert_eq!(js(&simplify(&t("(1/2+i)(1/2-i)"))), r#"["/",5,4]"#);
}

/// What the ℚ(i) evaluation must *not* touch: anything with a free variable,
/// anything outside the field, and a value that is already what it computes.
#[test]
fn the_complex_fold_declines_outside_its_field() {
    assert_eq!(js(&simplify(&t("2i"))), r#"["*",2,"i"]"#);
    assert_eq!(js(&simplify(&t("x+i"))), r#"["+","i","x"]"#);
    assert_eq!(
        js(&simplify(&t("sqrt(2)i"))),
        r#"["*","i",["apply","sqrt",2]]"#
    );
    assert_eq!(js(&simplify(&t("pi i"))), r#"["*","i","pi"]"#);
}

/// Expansion of a symbolic complex product now finishes, because the `i²` it
/// produces folds.
#[test]
fn expanding_a_symbolic_complex_product_finishes() {
    assert_eq!(
        js(&simplify(&expand(&t("(a+bi)(c+di)")))),
        r#"["+",["*","a","d","i"],["*","b","c","i"],["*","a","c"],["-",["*","b","d"]]]"#
    );
    assert_eq!(
        js(&simplify(&expand(&t("(a+bi)(a-bi)")))),
        r#"["+",["^","a",2],["^","b",2]]"#
    );
}

// ---- item 23: the sign of a rounded zero --------------------------------

/// Rounding a small negative float to zero keeps the sign, as legacy's
/// `parseFloat((-0.001).toFixed(2))` did. The sign is not decoration: it is
/// what makes the reciprocal `−∞`.
#[test]
fn rounding_to_zero_keeps_the_sign() {
    let rounded =
        math_expressions::round_numbers_to_decimals(&Expr::Num(Number::from_f64(-0.001)), 2);
    let Expr::Num(n) = &rounded else {
        panic!("expected a number, got {}", js(&rounded))
    };
    assert!(n.is_neg_zero(), "−0.001 rounded to 2 places should be −0");
    // A positive value rounding to zero is still `+0`, and a value that does
    // not round to zero is untouched.
    let pos = math_expressions::round_numbers_to_decimals(&Expr::Num(Number::from_f64(0.001)), 2);
    assert_eq!(js(&pos), "0");
    let keeps =
        math_expressions::round_numbers_to_decimals(&Expr::Num(Number::from_f64(-1.006)), 2);
    assert_eq!(js(&keeps), "-1.01");
}

/// The sign is only observable inside the engine: `to_js` spells every zero
/// `0`, which `tests/signed_zero.rs` pins deliberately. Recorded here so the
/// half-open state is visible from the item it belongs to rather than
/// surprising the next reader.
#[test]
fn the_sign_does_not_cross_the_js_boundary() {
    let rounded =
        math_expressions::round_numbers_to_decimals(&Expr::Num(Number::from_f64(-0.001)), 2);
    assert_eq!(js(&rounded), "0");
}
