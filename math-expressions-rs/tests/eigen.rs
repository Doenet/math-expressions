//! MATRIX_PLAN.md M3+M4: `RootOf`, characteristic polynomial, eigenvalues,
//! eigenvectors. Written test-first.
//!
//! No SymPy in the container, so the oracle is the plan's §6.2 self-verifying
//! property — strictly stronger than corpus comparison: for every reported
//! pair (λ, v), `A·v − λ·v` must canonicalize to the zero vector and
//! `char_poly(A)(λ)` must equal 0, both through the library's own `equals`
//! (which exercises `RootOf` reduction and numeric evaluation end-to-end).

use math_expressions::eval::{eval_complex, Env};
use math_expressions::matrix::{char_poly, eigenvalues, eigenvectors};
use math_expressions::{
    canonicalize, equals, expand, simplify, Assumptions, EqOptions, Expr, TextToAst,
    TextToAstOptions,
};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn eq(a: &Expr, b: &Expr) -> bool {
    equals(a, b, &EqOptions::default())
}

fn subst_t(p: &Expr, v: &Expr) -> Expr {
    let subs = std::collections::HashMap::from([("t".to_string(), v.clone())]);
    math_expressions::substitute(p, &subs)
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

fn column(entries: &[Expr]) -> Expr {
    Expr::Matrix {
        rows: entries.len() as u32,
        cols: 1,
        entries: entries.to_vec(),
    }
}

fn zero_column(n: usize) -> Expr {
    column(&vec![parse("0"); n])
}

/// §6.2 self-verification: A·v − λ·v expands/simplifies to the zero vector.
/// (`expand` distributes the −λ·v product so like terms in the abstract root
/// fold structurally; the canonical layer alone never auto-distributes.)
fn assert_eigen_pair(a: &Expr, value: &Expr, v: &[Expr], ctx: &str) {
    let vcol = column(v);
    let av = Expr::Mul(vec![a.clone(), vcol.clone()]);
    let lv = Expr::Mul(vec![parse("-1"), value.clone(), vcol]);
    // Canonicalize first so the products fold into a single literal matrix,
    // THEN expand to a fixpoint — RootOf power reduction can introduce fresh
    // `c·(…)` products mid-expansion (r⁴ → 2r²−3), which the next pass
    // distributes.
    let diff = simplify(&expand_fix(&Expr::Add(vec![av, lv])));
    assert!(
        eq(&diff, &zero_column(v.len())),
        "{ctx}: A·v − λ·v = {diff:?}, want zero vector (λ = {value:?}, v = {v:?})"
    );
}

/// `p(λ) = 0`, compared as `p(λ) + 1 = 1`: a structural zero passes
/// directly, and a radical form that only closes numerically avoids the
/// relative-tolerance-at-zero trap of comparing against literal 0.
fn expand_fix(e: &Expr) -> Expr {
    let mut cur = canonicalize(e);
    for _ in 0..4 {
        let next = canonicalize(&expand(&cur));
        if next == cur {
            break;
        }
        cur = next;
    }
    cur
}

fn assert_annihilates(p: &Expr, v: &Expr, ctx: &str) {
    let at = simplify(&expand_fix(&subst_t(p, v)));
    let shifted = Expr::Add(vec![at.clone(), parse("1")]);
    assert!(
        eq(&shifted, &parse("1")),
        "{ctx}: char poly at {v:?} = {at:?}, want 0"
    );
}

/// Companion matrix (last-column convention) of monic t^n − c_{n−1}t^{n−1} … − c_0:
/// entries below the diagonal are 1, last column is c_0..c_{n−1}.
fn companion(cs: &[&str]) -> Expr {
    let n = cs.len();
    let mut entries = Vec::new();
    for (r, coeff) in cs.iter().enumerate() {
        for c in 0..n {
            if c + 1 == n {
                entries.push(parse(coeff));
            } else if r == c + 1 {
                entries.push(parse("1"));
            } else {
                entries.push(parse("0"));
            }
        }
    }
    Expr::Matrix {
        rows: n as u32,
        cols: n as u32,
        entries,
    }
}

// ================= M3: characteristic polynomial =================

#[test]
fn char_poly_rational_2x2() {
    let a = mat(2, 2, &["2", "1", "1", "2"]);
    let p = char_poly(&a, "x").expect("char poly");
    assert!(eq(&p, &parse("x^2 - 4x + 3")), "got {p:?}");
}

#[test]
fn char_poly_symbolic_2x2() {
    let a = mat(2, 2, &["a", "b", "c", "d"]);
    let p = char_poly(&a, "x").expect("char poly");
    assert!(
        eq(&p, &parse("x^2 - (a+d)x + (a d - b c)")),
        "got {p:?}"
    );
}

#[test]
fn char_poly_rational_3x3_with_fractions() {
    let a = mat(3, 3, &["1/2", "0", "0", "0", "1/3", "0", "0", "0", "2"]);
    let p = char_poly(&a, "t").expect("char poly");
    assert!(eq(&p, &parse("(t - 1/2)(t - 1/3)(t - 2)")), "got {p:?}");
}

#[test]
fn char_poly_rejects_non_square_and_non_matrix() {
    assert!(char_poly(&mat(2, 3, &["1", "2", "3", "4", "5", "6"]), "x").is_none());
    assert!(char_poly(&parse("x + 1"), "x").is_none());
}

#[test]
fn companion_matrix_has_its_polynomial() {
    // companion(&["1", "1", "0"]) realizes t³ − 0t² − 1t − 1 = t³ − t − 1.
    let a = companion(&["1", "1", "0"]);
    let p = char_poly(&a, "t").expect("char poly");
    assert!(eq(&p, &parse("t^3 - t - 1")), "got {p:?}");
}

// ================= M3: eigenvalues, closed forms =================

#[test]
fn eigenvalues_symmetric_integer() {
    let a = mat(2, 2, &["2", "1", "1", "2"]);
    let vals = eigenvalues(&a, &Assumptions::new()).expect("eigenvalues");
    assert_eq!(vals.len(), 2);
    // Real eigenvalues in ascending order.
    assert!(eq(&vals[0].0, &parse("1")), "got {:?}", vals[0].0);
    assert!(eq(&vals[1].0, &parse("3")), "got {:?}", vals[1].0);
    assert_eq!((vals[0].1, vals[1].1), (1, 1));
}

#[test]
fn eigenvalues_report_algebraic_multiplicity() {
    let a = mat(2, 2, &["2", "0", "0", "2"]);
    let vals = eigenvalues(&a, &Assumptions::new()).expect("eigenvalues");
    assert_eq!(vals.len(), 1);
    assert!(eq(&vals[0].0, &parse("2")));
    assert_eq!(vals[0].1, 2);
}

#[test]
fn eigenvalues_rational_entries() {
    let a = mat(2, 2, &["1/2", "0", "0", "1/3"]);
    let vals = eigenvalues(&a, &Assumptions::new()).expect("eigenvalues");
    assert!(eq(&vals[0].0, &parse("1/3")));
    assert!(eq(&vals[1].0, &parse("1/2")));
}

#[test]
fn eigenvalues_quadratic_closed_form_real() {
    // [[0,2],[1,0]]: λ² = 2 → ±√2, ascending.
    let a = mat(2, 2, &["0", "2", "1", "0"]);
    let vals = eigenvalues(&a, &Assumptions::new()).expect("eigenvalues");
    assert!(eq(&vals[0].0, &parse("-sqrt(2)")), "got {:?}", vals[0].0);
    assert!(eq(&vals[1].0, &parse("sqrt(2)")), "got {:?}", vals[1].0);
}

#[test]
fn eigenvalues_rotation_matrix_complex_pair() {
    // Rotation by π/2: λ = ∓i; complex pair ordered negative imaginary first.
    let a = mat(2, 2, &["0", "-1", "1", "0"]);
    let vals = eigenvalues(&a, &Assumptions::new()).expect("eigenvalues");
    assert_eq!(vals.len(), 2);
    assert!(eq(&vals[0].0, &parse("-i")), "got {:?}", vals[0].0);
    assert!(eq(&vals[1].0, &parse("i")), "got {:?}", vals[1].0);
}

#[test]
fn eigenvalues_singular_matrix_include_zero() {
    let a = mat(3, 3, &["1", "2", "3", "4", "5", "6", "7", "8", "9"]);
    let vals = eigenvalues(&a, &Assumptions::new()).expect("eigenvalues");
    assert!(
        vals.iter().any(|(v, _)| eq(v, &parse("0"))),
        "0 must be an eigenvalue of a singular matrix: {vals:?}"
    );
    let n: u32 = vals.iter().map(|(_, m)| m).sum();
    assert_eq!(n, 3, "algebraic multiplicities sum to the dimension");
}

// ================= M3: RootOf =================

#[test]
fn eigenvalues_cubic_are_rootof() {
    let a = companion(&["1", "1", "0"]); // t³ − t − 1, irreducible over ℚ
    let vals = eigenvalues(&a, &Assumptions::new()).expect("eigenvalues");
    assert_eq!(vals.len(), 3);
    for (k, (v, m)) in vals.iter().enumerate() {
        assert_eq!(*m, 1);
        let Expr::RootOf { index, .. } = v else {
            panic!("expected RootOf, got {v:?}")
        };
        assert_eq!(*index as usize, k, "canonical index order");
    }
    // Index 0 is the lone real root (the plastic number ≈ 1.3247…).
    let z = eval_complex(&vals[0].0, &Env::new()).expect("numeric");
    assert!((z.re - 1.324_717_957_244_746).abs() < 1e-9 && z.im.abs() < 1e-12);
    // Indices 1,2 are the conjugate pair, negative imaginary part first.
    let z1 = eval_complex(&vals[1].0, &Env::new()).expect("numeric");
    let z2 = eval_complex(&vals[2].0, &Env::new()).expect("numeric");
    assert!(z1.im < 0.0 && z2.im > 0.0 && (z1.re - z2.re).abs() < 1e-9);
    // And each satisfies its polynomial, through the library's own equality.
    for (v, _) in &vals {
        assert_annihilates(&parse("t^3 - t - 1"), v, "cubic");
    }
}

#[test]
fn eigenvalues_degree4_index_ordering() {
    // t⁴ − t − 1: two real roots (indices 0,1 ascending), one conjugate pair
    // (indices 2,3, negative imaginary part first).
    let a = companion(&["1", "1", "0", "0"]);
    let p = char_poly(&a, "t").expect("char poly");
    assert!(eq(&p, &parse("t^4 - t - 1")), "got {p:?}");
    let vals = eigenvalues(&a, &Assumptions::new()).expect("eigenvalues");
    assert_eq!(vals.len(), 4);
    let zs: Vec<_> = vals
        .iter()
        .map(|(v, _)| eval_complex(v, &Env::new()).expect("numeric"))
        .collect();
    assert!(zs[0].im.abs() < 1e-12 && zs[1].im.abs() < 1e-12);
    assert!(zs[0].re < zs[1].re, "real roots ascending");
    assert!(zs[2].im < 0.0 && zs[3].im > 0.0, "conjugates, negative first");
    assert!((zs[2].re - zs[3].re).abs() < 1e-9);
}

#[test]
fn rootof_parses_and_prints_round_trip() {
    let r = parse("rootof(t^3 - t - 1, 2)");
    let c = canonicalize(&r);
    assert!(
        matches!(c, Expr::RootOf { index: 2, .. }),
        "canonical form is the RootOf atom: {c:?}"
    );
    let text = math_expressions::to_text(&c, &Default::default());
    let back = canonicalize(&parse(&text));
    assert_eq!(c, back, "text round trip through {text:?}");
}

#[test]
fn rootof_power_reduction() {
    // In ℚ[t]/(t³−t−1): t³ = t + 1 and t⁴ = t² + t.
    let cubed = parse("rootof(t^3 - t - 1, 0)^3");
    assert!(eq(&cubed, &parse("rootof(t^3 - t - 1, 0) + 1")));
    let fourth = parse("rootof(t^3 - t - 1, 0)^4");
    assert!(eq(
        &fourth,
        &parse("rootof(t^3 - t - 1, 0)^2 + rootof(t^3 - t - 1, 0)")
    ));
    // p(RootOf(p, k)) → 0 falls out of the same reduction.
    let z = simplify(&parse(
        "rootof(t^3 - t - 1, 0)^3 - rootof(t^3 - t - 1, 0) - 1",
    ));
    assert!(eq(&z, &parse("0")), "got {z:?}");
}

#[test]
fn rootof_normalizes_its_polynomial() {
    // Scaling and sign don't change the root set: canonical form is the
    // primitive integer polynomial with positive leading coefficient.
    let a = canonicalize(&parse("rootof(t^3 - t - 1, 1)"));
    let b = canonicalize(&parse("rootof(2t^3 - 2t - 2, 1)"));
    let c = canonicalize(&parse("rootof(-t^3 + t + 1, 1)"));
    assert_eq!(a, b);
    assert_eq!(a, c);
}

#[test]
fn rootof_equality_is_numeric_where_closed_forms_exist() {
    // t² − 2, index 1 = the positive root = √2.
    assert!(eq(&parse("rootof(t^2 - 2, 1)"), &parse("sqrt(2)")));
    assert!(!eq(&parse("rootof(t^2 - 2, 1)"), &parse("-sqrt(2)")));
    assert!(eq(&parse("rootof(t^2 - 2, 0)"), &parse("-sqrt(2)")));
}

// ================= M4: eigenvectors =================

#[test]
fn eigenvectors_symmetric_integer() {
    let a = mat(2, 2, &["2", "1", "1", "2"]);
    let pairs = eigenvectors(&a, &Assumptions::new()).expect("eigenvectors");
    assert_eq!(pairs.len(), 2);
    for pair in &pairs {
        assert_eq!(pair.alg_mult, 1);
        assert_eq!(pair.basis.len(), 1, "geometric multiplicity 1");
        // First nonzero component normalized to 1.
        assert!(eq(&pair.basis[0][0], &parse("1")));
        assert_eigen_pair(&a, &pair.value, &pair.basis[0], "symmetric 2×2");
    }
    // λ = 1 → (1, −1); λ = 3 → (1, 1).
    assert!(eq(&pairs[0].value, &parse("1")) && eq(&pairs[0].basis[0][1], &parse("-1")));
    assert!(eq(&pairs[1].value, &parse("3")) && eq(&pairs[1].basis[0][1], &parse("1")));
}

#[test]
fn eigenvectors_defective_matrix_shows_multiplicity_gap() {
    let a = mat(2, 2, &["0", "1", "0", "0"]);
    let pairs = eigenvectors(&a, &Assumptions::new()).expect("eigenvectors");
    assert_eq!(pairs.len(), 1);
    assert!(eq(&pairs[0].value, &parse("0")));
    assert_eq!(pairs[0].alg_mult, 2);
    assert_eq!(pairs[0].basis.len(), 1, "defective: geometric 1 < algebraic 2");
    assert_eigen_pair(&a, &pairs[0].value, &pairs[0].basis[0], "defective");
}

#[test]
fn eigenvectors_identity_full_eigenspace() {
    let a = mat(2, 2, &["1", "0", "0", "1"]);
    let pairs = eigenvectors(&a, &Assumptions::new()).expect("eigenvectors");
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].alg_mult, 2);
    assert_eq!(pairs[0].basis.len(), 2, "identity: full eigenspace");
}

#[test]
fn eigenvectors_abstract_rootof_self_verify() {
    // Companion of t³ − t − 1: eigenvectors are polynomials in the abstract
    // eigenvalue; A·v = λ·v must hold exactly through RootOf reduction.
    let a = companion(&["1", "1", "0"]);
    let pairs = eigenvectors(&a, &Assumptions::new()).expect("eigenvectors");
    assert_eq!(pairs.len(), 3);
    for pair in &pairs {
        assert!(matches!(pair.value, Expr::RootOf { .. }));
        assert_eq!(pair.basis.len(), 1);
        assert!(eq(&pair.basis[0][0], &parse("1")), "normalized leading 1");
        assert_eigen_pair(&a, &pair.value, &pair.basis[0], "companion cubic");
    }
}

#[test]
fn eigenvectors_quadratic_closed_form_self_verify() {
    let a = mat(2, 2, &["0", "2", "1", "0"]); // λ = ±√2
    let pairs = eigenvectors(&a, &Assumptions::new()).expect("eigenvectors");
    assert_eq!(pairs.len(), 2);
    for pair in &pairs {
        assert_eq!(pair.basis.len(), 1);
        assert_eigen_pair(&a, &pair.value, &pair.basis[0], "±√2");
    }
}

#[test]
fn eigenvectors_block_diagonal_discovers_factors() {
    // diag(companion(t³−t−1), companion(t³−2t−5)): the char poly is the
    // product of two irreducible cubics — squarefree with no rational or
    // quadratic factors, so the RootOf carries the reducible degree-6 poly.
    // Quotient-ring elimination must survive the zero divisors (discovered
    // factor → split → restart) and still produce verified eigenpairs.
    let mut entries = Vec::new();
    let b1 = ["1", "1", "0"]; // t³ − t − 1
    let b2 = ["5", "2", "0"]; // t³ − 2t − 5
    let block = |cs: &[&str; 3], r: usize, c: usize| -> String {
        if c == 2 {
            cs[r].to_string()
        } else if r == c + 1 {
            "1".into()
        } else {
            "0".into()
        }
    };
    for r in 0..6 {
        for c in 0..6 {
            let s = match (r < 3, c < 3) {
                (true, true) => block(&b1, r, c),
                (false, false) => block(&b2, r - 3, c - 3),
                _ => "0".into(),
            };
            entries.push(parse(&s));
        }
    }
    let a = Expr::Matrix {
        rows: 6,
        cols: 6,
        entries,
    };
    let pairs = eigenvectors(&a, &Assumptions::new()).expect("eigenvectors");
    let total: u32 = pairs.iter().map(|p| p.alg_mult).sum();
    assert_eq!(total, 6);
    for pair in &pairs {
        assert_eq!(pair.basis.len(), 1);
        assert_eigen_pair(&a, &pair.value, &pair.basis[0], "block diagonal");
    }
}

// ================= §6.2 self-verification sweep =================

#[test]
fn eigen_self_verification_sweep() {
    let cases: Vec<(&str, Expr)> = vec![
        ("triangular", mat(3, 3, &["1", "2", "3", "0", "4", "5", "0", "0", "6"])),
        ("singular", mat(3, 3, &["1", "2", "3", "4", "5", "6", "7", "8", "9"])),
        ("symmetric", mat(3, 3, &["2", "1", "0", "1", "2", "1", "0", "1", "2"])),
        (
            "integer 4×4",
            mat(
                4,
                4,
                &[
                    "1", "0", "2", "0", "0", "3", "0", "0", "1", "0", "1", "0", "0", "0", "0", "2",
                ],
            ),
        ),
        ("companion quartic", companion(&["-3", "0", "2", "0"])),
    ];
    for (name, a) in &cases {
        let p = char_poly(a, "t").unwrap_or_else(|| panic!("{name}: char poly"));
        let vals = eigenvalues(a, &Assumptions::new()).unwrap_or_else(|| panic!("{name}: values"));
        let n: u32 = vals.iter().map(|(_, m)| m).sum();
        let Expr::Matrix { rows, .. } = a else { unreachable!() };
        assert_eq!(n, *rows, "{name}: multiplicities sum to n");
        for (v, _) in &vals {
            assert_annihilates(&p, v, name);
        }
        let pairs = eigenvectors(a, &Assumptions::new()).unwrap_or_else(|| panic!("{name}: vectors"));
        for pair in &pairs {
            assert!(!pair.basis.is_empty(), "{name}: at least one eigenvector");
            for v in &pair.basis {
                assert_eigen_pair(a, &pair.value, v, name);
            }
        }
    }
}

// ================= symbolic entries: quadratic closed forms =================

#[test]
fn symbolic_2x2_eigenvalues_satisfy_trace_and_det() {
    let a = mat(2, 2, &["a", "b", "c", "d"]);
    let vals = eigenvalues(&a, &Assumptions::new()).expect("symbolic 2×2 closed form");
    assert_eq!(vals.len(), 2);
    let sum = Expr::Add(vec![vals[0].0.clone(), vals[1].0.clone()]);
    let prod = Expr::Mul(vec![vals[0].0.clone(), vals[1].0.clone()]);
    assert!(eq(&sum, &parse("a + d")), "trace identity: {sum:?}");
    assert!(eq(&prod, &parse("a d - b c")), "det identity: {prod:?}");
}

#[test]
fn symbolic_larger_matrices_return_none() {
    // Beyond degree-2 closed forms, symbolic entries have no honest answer
    // (RootOf is ℚ-coefficients only — plan §8 Q1).
    let a = mat(3, 3, &["a", "0", "0", "0", "b", "0", "0", "0", "c"]);
    assert!(eigenvalues(&a, &Assumptions::new()).is_none());
    assert!(eigenvectors(&a, &Assumptions::new()).is_none());
}

// ================= caps and refusals =================

#[test]
fn eigen_caps_are_honest_refusals() {
    use math_expressions::resource_limits::{self, ResourceLimits};
    assert!(eigenvalues(&mat(2, 3, &["1", "2", "3", "4", "5", "6"]), &Assumptions::new()).is_none());
    assert!(eigenvalues(&parse("x"), &Assumptions::new()).is_none());
    // Degree cap: a cubic RootOf is refused under max_rootof_degree = 2.
    let a = companion(&["1", "1", "0"]);
    let strict = ResourceLimits {
        max_rootof_degree: 2,
        ..ResourceLimits::default()
    };
    resource_limits::with(strict, || {
        assert!(eigenvalues(&a, &Assumptions::new()).is_none());
    });
}
