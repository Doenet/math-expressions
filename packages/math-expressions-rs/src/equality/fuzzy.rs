//! Number-error-tolerant comparison: fuzzy structural equality (`trees/basic.js
//! equal`) and the first-order sensitivity tolerance (`tolerance_function`) that
//! the sampling stages add to their per-point comparisons.

use super::EqOptions;
use crate::eval_numeric::complex::{eval_complex, Env};
use crate::expr::Expr;
use num_complex::Complex64;

/// Structural equality with number leaves compared within the allowed error
/// (canonical trees; port of `trees/basic.js equal`). Exponents of `Pow` are
/// compared exactly unless `include_error_in_number_exponents`.
pub(super) fn fuzzy_tree_eq(a: &Expr, b: &Expr, opts: &EqOptions) -> bool {
    match (a, b) {
        (Expr::Num(x), Expr::Num(y)) => fuzzy_number_eq(x, y, opts),
        (Expr::Pow(b1, e1), Expr::Pow(b2, e2)) => {
            let base_ok = fuzzy_tree_eq(b1, b2, opts);
            let exp_ok = if opts.include_error_in_number_exponents || !is_literal_exponent(e1) {
                fuzzy_tree_eq(e1, e2, opts)
            } else {
                // The allowance does not reach the exponent, but "no allowance"
                // is not "bit-identical": JS re-enters its comparison with
                // `allowed_error_in_numbers` left at its default of 0, which
                // still admits a 1e-14 *relative* difference. A structural `==`
                // here made `x^2` and `x^2.0` — the same exponent, one arriving
                // as an integer and one as a float from a different code path —
                // grade as different expressions.
                fuzzy_tree_eq(
                    e1,
                    e2,
                    &EqOptions {
                        allowed_error_in_numbers: 0.0,
                        ..opts.clone()
                    },
                )
            };
            base_ok && exp_ok
        }
        _ => {
            if std::mem::discriminant(a) != std::mem::discriminant(b) {
                return false;
            }
            // Same variant: compare non-Expr structure via a cheap projection,
            // then children pairwise. Kind/name/op mismatches show up either
            // in the discriminant or in the skeleton compare below.
            if !same_skeleton(a, b) {
                return false;
            }
            let (ca, cb) = (a.children(), b.children());
            if ca.len() != cb.len() {
                return false;
            }
            if ca
                .iter()
                .zip(cb.iter())
                .all(|(x, y)| fuzzy_tree_eq(x, y, opts))
            {
                return true;
            }
            if matches!(a, Expr::Add(_)) && opts.allowed_error_in_numbers > 0.0 {
                return unordered_eq(&ca, &cb, opts);
            }
            false
        }
    }
}

/// Re-match the terms of a sum without regard to order, as a fallback when the
/// pairwise compare fails under a nonzero number allowance.
///
/// **Why this exists.** Term order is decided by the *values* of the numbers in
/// the tree, and this comparison was then asked to forgive those same numbers up
/// to `allowed_error_in_numbers`. The two are in direct conflict: a perturbation
/// small enough to forgive at a leaf can still be large enough to move the term
/// that contains it. `exp(0.01xy + 1000q^2)` against
/// `exp(0.01xy + 1000q^(2−0.00009))` is the whole failure — `xy` has total
/// degree 2 and `q^1.99991` has 1.99991, so the sum comes back reordered, and a
/// pairwise walk then compares `0.01xy` against `1000q^…` after having already
/// accepted every number it looked at. Perturbing the exponent *upward* passes,
/// which is the tell: nothing about the arithmetic is direction-dependent, only
/// the sort.
///
/// So the rule is: a comparison that forgives ε in a number must not depend on
/// an ordering derived from that number.
///
/// **Scope.** Only `Add`, and only as a fallback after the ordered compare has
/// already failed. Not `Mul`: multiplication is not commutative here (matrices),
/// so re-matching factors would grade `AB` equal to `BA`. And only under a
/// tolerance — with none set, the order is a function of numbers that are being
/// compared exactly, so it cannot drift, and `equals_syntactic`'s documented
/// order-sensitivity (`(x+y)+z` ≠ `z+x+y`) is preserved untouched on that path.
///
/// **Matching, not greedy pairing.** Fuzzy number equality is not transitive, so
/// a greedy first-fit can fail on operands a perfect matching would pair up.
/// This is Kuhn's augmenting-path algorithm over the "these two are fuzzy-equal"
/// bipartite graph, which answers the actual question — is there *any* pairing
/// under which every term matches?
///
/// **Only when the allowance was actually spent.** The justification above is
/// that forgiving ε in a number may have moved the term holding it, so the
/// fallback declines whenever the terms match up *exactly* — a permutation that
/// needs no allowance is not sort drift, it is the two expressions being written
/// in different orders, and `equals_syntactic`'s order sensitivity has to stand
/// for it. DoenetML's `<answer symbolicEquality allowedErrorInNumbers="...">`
/// is the case that made this concrete: it is documented and tested as refusing
/// a reordered response, and `e·25.6 + 2.15π` against `2.15π + e·25.6` — every
/// number identical — was being graded correct purely because a tolerance had
/// been requested somewhere else in the expression.
fn unordered_eq(a: &[&Expr], b: &[&Expr], opts: &EqOptions) -> bool {
    // Guard the O(n²) edge build and O(n³) matching. A sum this wide is not a
    // graded response, and the ordered compare has already had its say.
    const MAX_TERMS: usize = 32;
    let n = a.len();
    if n > MAX_TERMS {
        return false;
    }
    // A pure permutation: matched with the allowance switched off, so nothing
    // about the ordering can be blamed on a forgiven number.
    let exact = EqOptions {
        allowed_error_in_numbers: 0.0,
        ..opts.clone()
    };
    if matching_exists(a, b, &exact) {
        return false;
    }
    matching_exists(a, b, opts)
}

/// Is there a pairing of `a` with `b` under which every term is
/// [`fuzzy_tree_eq`] at `opts`?
fn matching_exists(a: &[&Expr], b: &[&Expr], opts: &EqOptions) -> bool {
    let n = a.len();
    let edges: Vec<Vec<usize>> = a
        .iter()
        .map(|x| {
            (0..n)
                .filter(|&j| fuzzy_tree_eq(x, b[j], opts))
                .collect::<Vec<_>>()
        })
        .collect();
    let mut paired: Vec<Option<usize>> = vec![None; n];
    (0..n).all(|i| augment(i, &edges, &mut vec![false; n], &mut paired))
}

/// Find an augmenting path for left node `i`, repairing earlier pairings if
/// that is what it takes to fit everyone.
pub(super) fn augment(
    i: usize,
    edges: &[Vec<usize>],
    seen: &mut [bool],
    paired: &mut [Option<usize>],
) -> bool {
    for &j in &edges[i] {
        if seen[j] {
            continue;
        }
        seen[j] = true;
        if match paired[j] {
            None => true,
            Some(other) => augment(other, edges, seen, paired),
        } {
            paired[j] = Some(i);
            return true;
        }
    }
    false
}

/// Is this exponent a bare number the author typed, like the `2` in `x^2`?
///
/// That is the case the exempt-exponents rule is about: a student must not
/// collect slack on an exponent, so `x^2.0002` does not pass for `x^2` unless
/// the author asks for it. Anything else in the exponent position is an
/// ordinary expression whose numbers are ordinary numbers —
/// `e^(7x²/(0.00003−√y))` is an exponential, and its "exponent" is a function
/// argument. The JS library never had to draw this line: its
/// `normalize_function_names` spells that as `exp(…)`, so the argument was
/// never in an exponent to begin with. This engine folds the pair the other
/// way, into powers, so the line is drawn here instead.
fn is_literal_exponent(e: &Expr) -> bool {
    matches!(e, Expr::Num(_))
}

/// Non-child structure equal (symbol names, seq kinds, relation ops, matrix
/// shape, interval closure)?
fn same_skeleton(a: &Expr, b: &Expr) -> bool {
    match (a, b) {
        (Expr::Sym(x), Expr::Sym(y)) => x == y,
        (Expr::Const(x), Expr::Const(y)) => x == y,
        (Expr::Seq(k1, _), Expr::Seq(k2, _)) => k1 == k2,
        (Expr::OtherOp(n1, _), Expr::OtherOp(n2, _)) => n1 == n2,
        (Expr::Relation { ops: o1, .. }, Expr::Relation { ops: o2, .. }) => o1 == o2,
        (Expr::Matrix(m1), Expr::Matrix(m2)) => m1.rows() == m2.rows() && m1.cols() == m2.cols(),
        (Expr::Interval { closed: cl1, .. }, Expr::Interval { closed: cl2, .. }) => cl1 == cl2,
        // Leaves whose entire content is the payload. They have no children,
        // so leaving them to `_ => true` made them compare equal to each other
        // unconditionally: with any tolerance set, `["and", true, false]`
        // matched `["and", false, true]`, and `rootof(p, 0)` matched
        // `rootof(p, 1)` — √2 grading equal to −√2.
        (Expr::Bool(x), Expr::Bool(y)) => x == y,
        (
            Expr::RootOf {
                poly: p1,
                index: i1,
            },
            Expr::RootOf {
                poly: p2,
                index: i2,
            },
        ) => p1 == p2 && i1 == i2,
        _ => true,
    }
}

/// JS `trees/basic.js` number comparison: relative mode uses
/// `max(1e-14, allowed)·min(|l|,|r|)`; absolute mode `max(1e-14·min, allowed)`.
fn fuzzy_number_eq(x: &crate::num::Number, y: &crate::num::Number, opts: &EqOptions) -> bool {
    let (l, r) = (x.to_f64(), y.to_f64());
    if !l.is_finite() || !r.is_finite() {
        return x == y;
    }
    let min_abs = l.abs().min(r.abs());
    let tol = if opts.allowed_error_is_absolute {
        (1e-14 * min_abs).max(opts.allowed_error_in_numbers)
    } else {
        1e-14f64.max(opts.allowed_error_in_numbers) * min_abs
    };
    (l - r).abs() <= tol
}

/// The per-sample-point extra tolerance from the allowed number error: a
/// first-order sensitivity bound. Numbers in `expr` are replaced by
/// parameters; the tolerance expression is
/// `allowed_error · Σᵢ ∂f/∂pᵢ · (valᵢ if relative)` and is evaluated at each
/// sample point (port of the JS `tolerance_function`).
pub(super) struct FuzzyTol {
    tolerance_expr: Expr,
    /// Parameter name → its numeric value, added to every evaluation env.
    params: Vec<(String, f64)>,
}

pub(super) fn build_fuzzy_tol(expr: &Expr, vars: &[String], opts: &EqOptions) -> Option<FuzzyTol> {
    let mut params: Vec<(String, f64)> = Vec::new();
    let with_params = replace_numbers(
        expr,
        vars,
        opts.include_error_in_number_exponents,
        &mut params,
    );
    if params.is_empty() {
        return None;
    }
    let mut terms = Vec::new();
    for (name, val) in &params {
        let d = crate::calculus::diff::derivative(&with_params, name);
        let term = if opts.allowed_error_is_absolute {
            d
        } else {
            Expr::Mul(vec![d, Expr::Num(crate::num::Number::from_f64(*val))])
        };
        terms.push(term);
    }
    let tolerance_expr = Expr::Mul(vec![
        Expr::Num(crate::num::Number::from_f64(opts.allowed_error_in_numbers)),
        Expr::Add(terms),
    ]);
    Some(FuzzyTol {
        tolerance_expr,
        params,
    })
}

/// Replace each nonzero number literal (and the constants pi/e) with a fresh
/// parameter symbol, recording its value. `Pow` exponents are left untouched
/// unless `include_exponents`.
fn replace_numbers(
    e: &Expr,
    vars: &[String],
    include_exponents: bool,
    params: &mut Vec<(String, f64)>,
) -> Expr {
    let fresh = |val: f64, params: &mut Vec<(String, f64)>| -> Expr {
        let mut n = params.len() + 1;
        let mut name = format!("par{n}");
        while vars.contains(&name) {
            n += 1;
            name = format!("par{n}");
        }
        params.push((name.clone(), val));
        Expr::sym(&name)
    };
    match e {
        // A `-1` factor in a product is a *sign*, not a magnitude the author
        // typed. Canonicalization spells `a - b` as `a + (-1)·b`, so every
        // subtraction would otherwise contribute a parameter, and the tolerance
        // would include a `∂f/∂(-1)` term — "what if the minus sign were 0.01%
        // more negative", which is not a thing a response can get wrong.
        //
        // It is not a small effect. For `10 exp(7x²/(3-sqrt(y)))` the spurious
        // parameter *dominated*: the tolerance came out 4.3× the JS value, and
        // answers perturbed by twice the allowed error graded as correct. The JS
        // never had this to deal with — its `-` is a unary node with no number
        // in it — so this restores parity rather than diverging from it.
        // (`-2x` still parameterizes its `-2`: only exactly `-1` is structural.)
        Expr::Mul(fs) => Expr::Mul(
            fs.iter()
                .map(|f| match f {
                    Expr::Num(n) if n.to_f64() == -1.0 => f.clone(),
                    other => replace_numbers(other, vars, include_exponents, params),
                })
                .collect(),
        ),
        Expr::Num(n) => {
            let v = n.to_f64();
            if v == 0.0 || !v.is_finite() {
                e.clone()
            } else {
                fresh(v, params)
            }
        }
        // Both spellings must be parameterized alike (canonicalize unifies
        // `Const(Pi/E)` → `Sym`, but this pass can see pre-canonical trees), and
        // only while the name is *declared* a constant: an undeclared `e` is a
        // free variable and gets sampled as one by the caller instead.
        _ if crate::constant_policy::is_pi(e) => fresh(std::f64::consts::PI, params),
        _ if crate::constant_policy::is_e(e) => fresh(std::f64::consts::E, params),
        Expr::Pow(b, x) if !include_exponents && is_literal_exponent(x) => Expr::Pow(
            Box::new(replace_numbers(b, vars, include_exponents, params)),
            x.clone(),
        ),
        _ => crate::expr::map_children(e, |c| replace_numbers(c, vars, include_exponents, params)),
    }
}

impl FuzzyTol {
    /// |tolerance| at a sample point; `None` when it cannot be evaluated
    /// (treated as a disagreeing point, like the JS).
    pub(super) fn at(&self, bindings: &Env) -> Option<f64> {
        let mut env = bindings.clone();
        for (name, val) in &self.params {
            env.insert(name.clone(), Complex64::new(*val, 0.0));
        }
        let v = eval_complex(&self.tolerance_expr, &env)?;
        let t = v.norm();
        t.is_finite().then_some(t)
    }
}
