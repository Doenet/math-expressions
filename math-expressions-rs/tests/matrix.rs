//! MATRIX_PLAN.md Layer 1 (M1): matrix algebra in the canonical layer.
//! Written test-first: these specify the M1 contract from §0/§1a of the plan.

use math_expressions::{
    canonicalize, equals, matmul, to_text, trace, transpose, EqOptions, Expr, TextOpts, TextToAst,
    TextToAstOptions,
};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

/// Build a literal matrix from entry strings (row-major).
fn mat(rows: u32, cols: u32, entries: &[&str]) -> Expr {
    assert_eq!(entries.len() as u32, rows * cols);
    Expr::Matrix {
        rows,
        cols,
        entries: entries.iter().map(|s| parse(s)).collect(),
    }
}

/// Canonicalize both and require identical trees.
fn assert_canon_eq(a: &Expr, b: &Expr, ctx: &str) {
    let (ca, cb) = (canonicalize(a), canonicalize(b));
    assert_eq!(ca, cb, "{ctx}:\n  left  {ca:?}\n  right {cb:?}");
}

fn mul2(a: Expr, b: Expr) -> Expr {
    Expr::Mul(vec![a, b])
}
fn add2(a: Expr, b: Expr) -> Expr {
    Expr::Add(vec![a, b])
}
fn eq(a: &Expr, b: &Expr) -> bool {
    equals(a, b, &EqOptions::default())
}

// ---- §1a sums ----

#[test]
fn matrix_addition_is_entrywise() {
    let a = mat(2, 2, &["1", "2", "3", "4"]);
    let b = mat(2, 2, &["5", "6", "7", "8"]);
    let want = mat(2, 2, &["6", "8", "10", "12"]);
    assert_canon_eq(&add2(a, b), &want, "A+B");
}

#[test]
fn symbolic_entries_combine_like_terms() {
    let a = mat(2, 1, &["x", "y"]);
    let b = mat(2, 1, &["2x", "1"]);
    let want = mat(2, 1, &["3x", "y + 1"]);
    assert_canon_eq(&add2(a, b), &want, "symbolic A+B");
}

#[test]
fn dimension_mismatch_addition_stays_unevaluated() {
    let a = mat(2, 2, &["1", "2", "3", "4"]);
    let b = mat(2, 1, &["1", "2"]);
    let c = canonicalize(&add2(a, b));
    assert!(
        matches!(&c, Expr::Add(ts) if ts.len() == 2),
        "2x2 + 2x1 must stay an unevaluated Add, got {c:?}"
    );
}

#[test]
fn matrix_plus_scalar_stays_unevaluated() {
    let a = mat(2, 2, &["1", "2", "3", "4"]);
    let c = canonicalize(&add2(a, Expr::int(5)));
    assert!(
        matches!(&c, Expr::Add(ts) if ts.len() == 2),
        "matrix + scalar must stay unevaluated, got {c:?}"
    );
}

// ---- §0.3 scalar action distributes into entries ----

#[test]
fn scalar_multiple_distributes_into_entries() {
    let a = mat(2, 2, &["1", "x", "0", "y"]);
    assert_canon_eq(
        &mul2(Expr::int(2), a.clone()),
        &mat(2, 2, &["2", "2x", "0", "2y"]),
        "2·A",
    );
    assert_canon_eq(
        &mul2(parse("z"), a.clone()),
        &mat(2, 2, &["z", "z x", "0", "z y"]),
        "z·A (symbols are scalars)",
    );
    assert_canon_eq(
        &Expr::Neg(Box::new(a.clone())),
        &mat(2, 2, &["-1", "-x", "0", "-y"]),
        "−A",
    );
    assert_canon_eq(
        &Expr::Div(Box::new(a), Box::new(Expr::int(2))),
        &mat(2, 2, &["1/2", "x/2", "0", "y/2"]),
        "A/2",
    );
}

#[test]
fn scalar_matrix_like_terms_combine() {
    let a = mat(2, 2, &["1", "0", "0", "1"]);
    // 2A + 3A = 5A
    let sum = add2(
        mul2(Expr::int(2), a.clone()),
        mul2(Expr::int(3), a.clone()),
    );
    assert_canon_eq(&sum, &mat(2, 2, &["5", "0", "0", "5"]), "2A+3A");
    // xA + yA = (x+y)A entrywise
    let sum = add2(mul2(parse("x"), a.clone()), mul2(parse("y"), a));
    assert_canon_eq(
        &sum,
        &mat(2, 2, &["x + y", "0", "0", "x + y"]),
        "xA+yA",
    );
}

#[test]
fn zero_scalar_gives_zero_matrix() {
    let a = mat(2, 2, &["1", "x", "3", "y"]);
    assert_canon_eq(
        &mul2(Expr::int(0), a),
        &mat(2, 2, &["0", "0", "0", "0"]),
        "0·A keeps its dimensions",
    );
}

// ---- §1a products ----

#[test]
fn literal_matrix_product_folds() {
    let a = mat(2, 2, &["1", "2", "3", "4"]);
    let b = mat(2, 2, &["5", "6", "7", "8"]);
    let want = mat(2, 2, &["19", "22", "43", "50"]);
    assert_canon_eq(&mul2(a, b), &want, "A·B");
}

#[test]
fn rectangular_product_dimensions() {
    // (2×3)·(3×1) → 2×1
    let a = mat(2, 3, &["1", "2", "3", "4", "5", "6"]);
    let b = mat(3, 1, &["x", "y", "z"]);
    let want = mat(2, 1, &["x + 2y + 3z", "4x + 5y + 6z"]);
    assert_canon_eq(&mul2(a, b), &want, "(2×3)·(3×1)");
}

#[test]
fn matrix_product_is_not_commutative() {
    let a = mat(2, 2, &["0", "1", "0", "0"]);
    let b = mat(2, 2, &["0", "0", "1", "0"]);
    let ab = canonicalize(&mul2(a.clone(), b.clone()));
    let ba = canonicalize(&mul2(b, a));
    assert_ne!(ab, ba, "AB must differ from BA for this witness");
    assert!(!eq(&ab, &ba), "equals must also distinguish them");
}

#[test]
fn incompatible_product_stays_unevaluated_in_order() {
    // (2×2)·(3×1): no fold, factors keep their order.
    let a = mat(2, 2, &["1", "2", "3", "4"]);
    let b = mat(3, 1, &["1", "2", "3"]);
    let c = canonicalize(&mul2(a.clone(), b.clone()));
    let Expr::Mul(fs) = &c else {
        panic!("expected unevaluated Mul, got {c:?}");
    };
    assert_eq!(fs.len(), 2);
    assert_eq!(fs[0], canonicalize(&a), "left factor first");
    assert_eq!(fs[1], canonicalize(&b), "right factor second");
}

#[test]
fn scalars_commute_with_matrices() {
    let a = mat(2, 2, &["1", "2", "3", "4"]);
    let b = mat(2, 2, &["0", "1", "1", "0"]);
    // 2·A·B == A·2·B == A·B·2 — scalars slide out and fold.
    let m1 = canonicalize(&Expr::Mul(vec![Expr::int(2), a.clone(), b.clone()]));
    let m2 = canonicalize(&Expr::Mul(vec![a.clone(), Expr::int(2), b.clone()]));
    let m3 = canonicalize(&Expr::Mul(vec![a, b, Expr::int(2)]));
    assert_eq!(m1, m2);
    assert_eq!(m2, m3);
}

#[test]
fn product_distributes_after_sum_folds() {
    // (A + B)·C folds bottom-up and equals AC + BC.
    let a = mat(2, 2, &["1", "0", "0", "1"]);
    let b = mat(2, 2, &["0", "1", "1", "0"]);
    let c = mat(2, 2, &["x", "0", "0", "y"]);
    let lhs = mul2(add2(a.clone(), b.clone()), c.clone());
    let rhs = add2(mul2(a, c.clone()), mul2(b, c));
    assert!(eq(&lhs, &rhs), "(A+B)C = AC + BC");
}

#[test]
fn associativity_of_fold() {
    let a = mat(2, 3, &["1", "2", "3", "4", "5", "6"]);
    let b = mat(3, 2, &["1", "0", "0", "1", "1", "1"]);
    let c = mat(2, 2, &["2", "0", "0", "2"]);
    let lhs = mul2(mul2(a.clone(), b.clone()), c.clone());
    let rhs = mul2(a, mul2(b, c));
    assert_canon_eq(&lhs, &rhs, "(AB)C = A(BC)");
}

// ---- §1a powers ----

#[test]
fn matrix_powers() {
    let a = mat(2, 2, &["1", "1", "0", "1"]);
    // A² = A·A
    assert_canon_eq(
        &Expr::Pow(Box::new(a.clone()), Box::new(Expr::int(2))),
        &mat(2, 2, &["1", "2", "0", "1"]),
        "A^2",
    );
    // A⁵ (binary powering)
    assert_canon_eq(
        &Expr::Pow(Box::new(a.clone()), Box::new(Expr::int(5))),
        &mat(2, 2, &["1", "5", "0", "1"]),
        "A^5",
    );
    // A¹ = A
    assert_canon_eq(
        &Expr::Pow(Box::new(a.clone()), Box::new(Expr::int(1))),
        &a,
        "A^1",
    );
    // A⁰ = I for square matrices
    assert_canon_eq(
        &Expr::Pow(Box::new(a.clone()), Box::new(Expr::int(0))),
        &mat(2, 2, &["1", "0", "0", "1"]),
        "A^0 = I",
    );
    // A⁻¹ with symbolic entries stays unevaluated in canonicalize (the
    // assumption-gated inverse is the §1b `matrix_inverse`; rational
    // matrices fold — see negative_matrix_powers_fold_for_rational_matrices).
    let sym = mat(2, 2, &["x", "1", "0", "1"]);
    let inv = canonicalize(&Expr::Pow(Box::new(sym), Box::new(Expr::int(-1))));
    assert!(
        matches!(&inv, Expr::Pow(b, _) if matches!(**b, Expr::Matrix { .. })),
        "symbolic A^-1 must stay an unevaluated Pow, got {inv:?}"
    );
    // Non-square powers stay unevaluated.
    let r = mat(2, 3, &["1", "2", "3", "4", "5", "6"]);
    let p = canonicalize(&Expr::Pow(Box::new(r), Box::new(Expr::int(2))));
    assert!(
        matches!(&p, Expr::Pow(..)),
        "non-square A^2 must stay unevaluated, got {p:?}"
    );
    // Symbolic exponent stays unevaluated.
    let p = canonicalize(&Expr::Pow(Box::new(a), Box::new(parse("n"))));
    assert!(matches!(&p, Expr::Pow(..)), "A^n must stay unevaluated");
}

// ---- transpose / trace / matmul API ----

#[test]
fn transpose_of_literal() {
    let a = mat(2, 3, &["1", "2", "3", "4", "5", "6"]);
    let want = mat(3, 2, &["1", "4", "2", "5", "3", "6"]);
    assert_canon_eq(&transpose(&a), &want, "transpose");
    // Non-matrix argument: opaque, not a wrong answer.
    let t = transpose(&parse("x"));
    assert!(
        matches!(&t, Expr::OtherOp(name, _) if name.name() == "transpose"),
        "transpose(x) must stay opaque, got {t:?}"
    );
}

#[test]
fn trace_of_literal() {
    let a = mat(2, 2, &["1", "x", "y", "4"]);
    assert_canon_eq(&trace(&a), &parse("5"), "tr");
    let s = mat(2, 2, &["a", "0", "0", "b"]);
    assert!(eq(&trace(&s), &parse("a + b")), "symbolic trace");
    // Non-square: opaque.
    let r = trace(&mat(2, 3, &["1", "2", "3", "4", "5", "6"]));
    assert!(
        matches!(&r, Expr::OtherOp(name, _) if name.name() == "trace"),
        "trace of non-square must stay opaque, got {r:?}"
    );
}

#[test]
fn transpose_reverses_products() {
    let a = mat(2, 2, &["1", "2", "3", "4"]);
    let b = mat(2, 2, &["0", "1", "x", "0"]);
    let lhs = transpose(&matmul(&a, &b));
    let rhs = matmul(&transpose(&b), &transpose(&a));
    assert!(eq(&lhs, &rhs), "(AB)ᵀ = BᵀAᵀ");
}

// ---- integration: equals, simplify idempotence, display safety ----

#[test]
fn equals_uses_folded_form() {
    let a = mat(2, 2, &["1", "2", "3", "4"]);
    let b = mat(2, 2, &["5", "6", "7", "8"]);
    let folded = mat(2, 2, &["19", "22", "43", "50"]);
    assert!(eq(&mul2(a.clone(), b.clone()), &folded));
    assert!(!eq(&mul2(a.clone(), b.clone()), &mat(2, 2, &["1", "0", "0", "1"])));
    // simplify is idempotent on matrix expressions.
    let s = math_expressions::simplify(&mul2(a, b));
    assert_eq!(math_expressions::simplify(&s), s);
}

#[test]
fn presentation_never_puts_matrices_under_a_fraction_bar() {
    // Pow(A, -1) is matrix-valued; display must not become 1/A. (Symbolic
    // entries so the inverse stays an unevaluated Pow.)
    let a = mat(2, 2, &["x", "1", "0", "1"]);
    let inv = math_expressions::simplify(&Expr::Pow(Box::new(a), Box::new(Expr::int(-1))));
    let text = to_text(&inv, &TextOpts::default());
    assert!(
        !text.starts_with("1/"),
        "matrix inverse must not display as a scalar fraction: {text}"
    );
}

// ================= §1b: det / inverse / rref / rank / nullspace =================

use math_expressions::{det, matrix_inverse, nullspace, rank, rref, Assumptions};

fn assume(s: &str) -> Assumptions {
    let mut a = Assumptions::new();
    a.add(&parse(s));
    a
}

#[test]
fn det_rational() {
    assert_canon_eq(&det(&mat(2, 2, &["1", "2", "3", "4"])), &parse("-2"), "2x2");
    assert_canon_eq(
        &det(&mat(3, 3, &["2", "0", "1", "1", "3", "0", "0", "1", "4"])),
        &parse("25"),
        "3x3",
    );
    assert_canon_eq(&det(&mat(2, 2, &["1", "2", "2", "4"])), &parse("0"), "singular");
    assert_canon_eq(&det(&mat(3, 3, &["1", "0", "0", "0", "1", "0", "0", "0", "1"])), &parse("1"), "I");
    assert_canon_eq(&det(&mat(2, 2, &["1/2", "1/3", "1/4", "1/5"])), &parse("1/60"), "rational entries");
}

#[test]
fn det_symbolic() {
    let d = det(&mat(2, 2, &["a", "b", "c", "d"]));
    assert!(eq(&d, &parse("a d - b c")), "ad - bc, got {d:?}");
    let d = det(&mat(2, 2, &["x", "1", "1", "x"]));
    assert!(eq(&d, &parse("x^2 - 1")), "x^2 - 1, got {d:?}");
}

#[test]
fn det_is_multiplicative() {
    let a = mat(2, 2, &["1", "2", "0", "3"]);
    let b = mat(2, 2, &["x", "1", "1", "y"]);
    let lhs = det(&matmul(&a, &b));
    let rhs = canonicalize(&mul2(det(&a), det(&b)));
    assert!(eq(&lhs, &rhs), "det(AB) = det A · det B");
}

#[test]
fn det_polynomial_tier_beyond_symbolic_cap() {
    // 8×8 diagonal in x: n > max_symbolic_det_dim, entries polynomial →
    // the polynomial tier must still produce x^8.
    let n = 8u32;
    let entries: Vec<String> = (0..n * n)
        .map(|i| if i % (n as u64 as u32 + 1) == 0 { "x".to_string() } else { "0".to_string() })
        .collect();
    let refs: Vec<&str> = entries.iter().map(String::as_str).collect();
    let d = det(&mat(n, n, &refs));
    assert!(eq(&d, &parse("x^8")), "diagonal x det, got {d:?}");
}

#[test]
fn det_opacity() {
    // Non-matrix and non-square arguments stay opaque.
    for e in [det(&parse("x")), det(&mat(2, 3, &["1", "2", "3", "4", "5", "6"]))] {
        assert!(
            matches!(&e, Expr::OtherOp(name, _) if name.name() == "det"),
            "expected opaque det node, got {e:?}"
        );
    }
}

#[test]
fn inverse_rational() {
    let a = mat(2, 2, &["1", "2", "3", "4"]);
    let inv = matrix_inverse(&a, &Assumptions::new());
    assert_canon_eq(&inv, &mat(2, 2, &["-2", "1", "3/2", "-1/2"]), "known inverse");
    // A · A⁻¹ = I
    assert_canon_eq(&matmul(&a, &inv), &mat(2, 2, &["1", "0", "0", "1"]), "A A^-1 = I");
    // Singular → opaque.
    let s = matrix_inverse(&mat(2, 2, &["1", "2", "2", "4"]), &Assumptions::new());
    assert!(
        matches!(&s, Expr::OtherOp(name, _) if name.name() == "inverse"),
        "singular inverse must stay opaque, got {s:?}"
    );
}

#[test]
fn inverse_symbolic_gated_on_assumptions() {
    let a = mat(2, 2, &["a", "0", "0", "b"]);
    // Without assumptions the determinant ab is not provably nonzero → opaque.
    let no = matrix_inverse(&a, &Assumptions::new());
    assert!(
        matches!(&no, Expr::OtherOp(name, _) if name.name() == "inverse"),
        "unproven det must stay opaque, got {no:?}"
    );
    // With a ≠ 0 and b ≠ 0 it inverts entrywise.
    let mut asm = Assumptions::new();
    asm.add(&parse("a != 0"));
    asm.add(&parse("b != 0"));
    let inv = matrix_inverse(&a, &asm);
    assert!(eq(&inv, &mat(2, 2, &["1/a", "0", "0", "1/b"])), "diag inverse, got {inv:?}");
}

#[test]
fn negative_matrix_powers_fold_for_rational_matrices() {
    let a = mat(2, 2, &["1", "2", "3", "4"]);
    // Canonicalize folds A⁻¹ for an invertible rational matrix…
    assert_canon_eq(
        &Expr::Pow(Box::new(a.clone()), Box::new(Expr::int(-1))),
        &mat(2, 2, &["-2", "1", "3/2", "-1/2"]),
        "A^-1 folds",
    );
    // …so A⁻¹·A = I and A⁻² = (A⁻¹)².
    assert_canon_eq(
        &mul2(Expr::Pow(Box::new(a.clone()), Box::new(Expr::int(-1))), a.clone()),
        &mat(2, 2, &["1", "0", "0", "1"]),
        "A^-1 A = I",
    );
    let want = canonicalize(&Expr::Pow(
        Box::new(canonicalize(&Expr::Pow(Box::new(a.clone()), Box::new(Expr::int(-1))))),
        Box::new(Expr::int(2)),
    ));
    assert_canon_eq(
        &Expr::Pow(Box::new(a), Box::new(Expr::int(-2))),
        &want,
        "A^-2 = (A^-1)^2",
    );
    // Singular matrices keep the unevaluated Pow.
    let s = mat(2, 2, &["1", "2", "2", "4"]);
    let p = canonicalize(&Expr::Pow(Box::new(s), Box::new(Expr::int(-1))));
    assert!(matches!(&p, Expr::Pow(..)), "singular A^-1 stays Pow, got {p:?}");
}

#[test]
fn rref_and_rank_rational() {
    let asm = Assumptions::new();
    assert_canon_eq(
        &rref(&mat(2, 2, &["1", "2", "2", "4"]), &asm),
        &mat(2, 2, &["1", "2", "0", "0"]),
        "rank-1 rref",
    );
    assert_canon_eq(
        &rref(&mat(2, 2, &["0", "1", "1", "0"]), &asm),
        &mat(2, 2, &["1", "0", "0", "1"]),
        "permutation rref",
    );
    assert_eq!(rank(&mat(2, 2, &["1", "2", "2", "4"]), &asm), Some(1));
    assert_eq!(rank(&mat(2, 2, &["0", "1", "1", "0"]), &asm), Some(2));
    assert_eq!(rank(&mat(2, 3, &["1", "2", "3", "2", "4", "6"]), &asm), Some(1));
}

#[test]
fn nullspace_rational() {
    let asm = Assumptions::new();
    let a = mat(2, 2, &["1", "2", "2", "4"]);
    let basis = nullspace(&a, &asm).unwrap();
    assert_eq!(basis.len(), 1, "n - rank = 1");
    // Normalized: first nonzero component 1 → (1, -1/2).
    assert_canon_eq(&basis[0], &mat(2, 1, &["1", "-1/2"]), "normalized null vector");
    // A·v = 0 (the zero 2×1 matrix).
    assert_canon_eq(&matmul(&a, &basis[0]), &mat(2, 1, &["0", "0"]), "A v = 0");
    // Full-rank matrix → empty basis.
    assert!(nullspace(&mat(2, 2, &["0", "1", "1", "0"]), &asm).unwrap().is_empty());
}

#[test]
fn symbolic_pivots_are_assumption_gated() {
    let a = mat(2, 2, &["x", "1", "0", "1"]);
    // Unknown pivot sign-status → opaque, never a guessed elimination.
    let no = rref(&a, &Assumptions::new());
    assert!(
        matches!(&no, Expr::OtherOp(name, _) if name.name() == "rref"),
        "undecidable pivot must stay opaque, got {no:?}"
    );
    assert_eq!(rank(&a, &Assumptions::new()), None);
    // With x ≠ 0 the elimination completes: rref = I, rank 2.
    let asm = assume("x != 0");
    assert_canon_eq(&rref(&a, &asm), &mat(2, 2, &["1", "0", "0", "1"]), "gated rref");
    assert_eq!(rank(&a, &asm), Some(2));
}
