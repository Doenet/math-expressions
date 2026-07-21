//! Structural comparison (STRUCTURAL_COMPARISON_PLAN.md): predicates over
//! the **faithful** (pre-`canonicalize`) `Expr` that decide whether a student
//! wrote an answer with a required *structure* — factored, expanded, reduced,
//! radical-simplified, a decimal vs an exact value, etc. — as opposed to
//! whether it has the right *value* (that is `equals`). Each requirement is a
//! [`StructuralComparison`].
//!
//! Two rules make these checks meaningful:
//!
//! 1. **Never canonicalize the input first.** `canonicalize` folds `2/4 → 1/2`,
//!    rewrites `Div`→`Mul·Pow⁻¹`, sorts, and combines like terms — i.e. it
//!    erases the very structure under test. Predicates inspect the faithful tree
//!    (`TextToAst::convert` output) directly, using `canonicalize`/`expand`/
//!    `factor`/`reduce_rational` only as *oracles* applied to controlled
//!    sub-comparisons.
//! 2. **Structure ⊥ value.** [`check_structural_comparison`] answers only "is it in
//!    this structure?". The autograder primitive "…and equal to the key" is
//!    [`structural_equality`], a sibling to [`equals`](crate::equals) that
//!    follows the JS `equalsVia*` family (no batch "grade" step).
//!
//! Prior art: STACK answer tests (`FacForm`, `Expanded`, `LowestTerms`,
//! `SingleFrac`, `CompletedSquare`) and WeBWorK strict contexts. Standards
//! diverge on what to enforce, so every comparison is opt-in and independent.
//!
//! ## Vocabulary (value vs structural)
//!
//! - [`equals`](crate::equals) — **value** equality (do they mean the same
//!   number/expression?).
//! - [`structural_equality`] — **structural** comparison against a key, with a
//!   [`StructuralComparison`] method. Its base method
//!   [`SameStructure`](StructuralComparison::SameStructure) is order-sensitive whole-tree
//!   equality — the JS `equalsViaSyntax`, also exposed under its JS-parity name
//!   [`equals_syntactic`](crate::equals_syntactic). The other methods are
//!   specific-structure criteria (factored, reduced, …), each requiring value
//!   equality too.
//! - [`check_structural_comparison`] — a **unary** structural predicate (is *this*
//!   expression factored / reduced / …?), no key.
//!
//! So "syntactic equality" is not a separate concept: it is the `SameStructure`
//! structural comparison; `equals_syntactic` is its convenience name.

use crate::eq::EqOptions;
use crate::expr::{Expr, SeqKind};
use crate::norm::canonicalize;
use crate::num::Number;

/// A single structural criterion to test against a student answer. Each variant is
/// independent and opt-in (§2). Value-equality is *not* implied — use
/// [`structural_equality`] when both structure and value are wanted.
#[derive(Debug, Clone)]
pub enum StructuralComparison {
    /// Every rational subterm is in lowest terms; no surd in a denominator.
    ReducedFraction,
    /// `a + b/c` with `a` a nonzero integer and `b/c` a proper fraction.
    MixedNumber,
    /// A single fraction `a/b` with `|a| >= |b|` (an improper, not mixed, fraction).
    ImproperFraction,
    /// A decimal numeral. `places` (F2/provenance) is not yet enforced; the
    /// boolean "is it a bare decimal number" is checked today.
    Decimal { places: Option<u32> },
    /// Exact — no decimal approximation anywhere (radicals/fractions/π are ok).
    ExactValue,
    /// No two combinable like terms remain and no un-reduced constant
    /// arithmetic (`2 + 3`) is left.
    CombinedLikeTerms,
    /// Fully distributed: no product has a sum factor and no sum is raised to
    /// an integer power ≥ 2.
    Expanded,
    /// A product of factors, each irreducible over ℚ (univariate).
    FactoredCompletely,
    /// The whole expression is one fraction (root is `Div`), not a sum of them.
    SingleFraction,
    /// No explicit negative exponent (`x^(-2)`); `1/x^2` is allowed.
    NoNegativeExponents,
    /// No radical in a denominator and every integer radicand is square-free.
    RadicalSimplified,
    /// `a(x - h)^2 + k` shape (single variable).
    CompletedSquare,
    /// A `+ C` term is present (a bare additive symbol other than `exclude`).
    HasIntegrationConstant { exclude: Option<String> },
    /// **Binary** — the student is written in the *same structure* as the key:
    /// order-sensitive whole-tree equality up to light normalization (function
    /// spelling, exponents/primes outside applications, negative placement,
    /// geometry arg order). This is the JS `equalsViaSyntax` port — also exposed
    /// as [`equals_syntactic`](crate::equals_syntactic) — folded in here as the
    /// base structural comparison. Only meaningful in [`structural_equality`]
    /// (it needs the key); [`check_structural_comparison`] rejects it.
    SameStructure,
}

/// The verdict for one [`StructuralComparison`]. `why` carries short feedback when `!ok`.
#[derive(Debug, Clone)]
pub struct StructuralComparisonResult {
    pub ok: bool,
    pub why: Option<String>,
}

impl StructuralComparisonResult {
    fn pass() -> Self {
        StructuralComparisonResult { ok: true, why: None }
    }
    fn fail(why: &str) -> Self {
        StructuralComparisonResult {
            ok: false,
            why: Some(why.to_string()),
        }
    }
    fn of(ok: bool, why: &str) -> Self {
        if ok {
            Self::pass()
        } else {
            Self::fail(why)
        }
    }
}

/// Test a single structural criterion against a faithful expression tree
/// (`TextToAst::convert` output — do **not** pass a canonicalized tree).
/// Flattens first (merging raw associative grouping) but never canonicalizes,
/// so `Div`/`Neg`/operand-order/number-spelling — the structure under test — survive.
pub fn check_structural_comparison(e: &Expr, check: &StructuralComparison) -> StructuralComparisonResult {
    let e = &crate::expr::flatten(e.clone());
    match check {
        StructuralComparison::ReducedFraction => StructuralComparisonResult::of(
            is_reduced_fraction(e),
            "fraction is not in lowest terms (or has a surd denominator)",
        ),
        StructuralComparison::MixedNumber => {
            StructuralComparisonResult::of(is_mixed_number(e), "not written as a mixed number a + b/c")
        }
        StructuralComparison::ImproperFraction => StructuralComparisonResult::of(
            is_improper_fraction(e),
            "not written as a single improper fraction a/b",
        ),
        StructuralComparison::Decimal { .. } => {
            StructuralComparisonResult::of(is_decimal_number(e), "not written as a decimal")
        }
        StructuralComparison::ExactValue => StructuralComparisonResult::of(
            !contains_decimal(e),
            "contains a decimal approximation; give the exact value",
        ),
        StructuralComparison::CombinedLikeTerms => StructuralComparisonResult::of(
            !like_terms_remain(e),
            "like terms (or constant arithmetic) can still be combined",
        ),
        StructuralComparison::Expanded => StructuralComparisonResult::of(is_expanded(e), "not fully expanded"),
        StructuralComparison::FactoredCompletely => {
            StructuralComparisonResult::of(is_factored_completely(e), "not completely factored")
        }
        StructuralComparison::SingleFraction => {
            StructuralComparisonResult::of(is_single_fraction(e), "not written as a single fraction")
        }
        StructuralComparison::NoNegativeExponents => {
            StructuralComparisonResult::of(!has_negative_exponent(e), "contains a negative exponent")
        }
        StructuralComparison::RadicalSimplified => StructuralComparisonResult::of(
            is_radical_simplified(e),
            "radical not simplified (surd in denominator or non-square-free radicand)",
        ),
        StructuralComparison::CompletedSquare => {
            StructuralComparisonResult::of(is_completed_square(e), "not in completed-square shape a(x-h)^2 + k")
        }
        StructuralComparison::HasIntegrationConstant { exclude } => StructuralComparisonResult::of(
            has_integration_constant(e, exclude.as_deref()),
            "missing the constant of integration (+ C)",
        ),
        // `SameStructure` is binary — it compares against a key — so it has no
        // unary verdict. `structural_equality` handles it.
        StructuralComparison::SameStructure => StructuralComparisonResult::fail(
            "`SameStructure` compares two expressions; use `structural_equality`",
        ),
    }
}

/// Structural equality — the single structural comparison entry, a sibling to
/// [`equals`] in the JS `equalsVia*` family. `comparison` selects the method:
///
/// - [`SameStructure`](StructuralComparison::SameStructure): `student` and `key` are
///   in the same structure — order-sensitive whole-tree equality (the JS
///   `equalsViaSyntax`; value equality is implied by tree identity). This is
///   the same computation as [`equals_syntactic`](crate::equals_syntactic).
/// - any *specific-structure* criterion (`FactoredCompletely`, `ReducedFraction`, …):
///   `student` is in that structure **and** is value-equal to `key`, matching how
///   STACK/WeBWorK answer tests pair a structural check with equivalence.
///
/// For a *pure* structural check with no key, call [`check_structural_comparison`];
/// for a *pure* value check, call [`equals`]. There is deliberately no batch
/// "grade" step — callers compose these per problem.
pub fn structural_equality(
    student: &Expr,
    key: &Expr,
    comparison: &StructuralComparison,
    opts: &EqOptions,
) -> bool {
    match comparison {
        StructuralComparison::SameStructure => crate::equals_syntactic(student, key, opts),
        _ => check_structural_comparison(student, comparison).ok && crate::equals(student, key, opts),
    }
}

// ---- number helpers ----

fn as_int(e: &Expr) -> Option<i64> {
    match e {
        Expr::Num(Number::Int(i)) => Some(*i),
        _ => None,
    }
}

/// The (numerator, denominator) of a **written** fraction — a `Div` of two
/// integers. A decimal literal (`Num(Rat)`, e.g. `2.5`) is *not* a written
/// fraction `a/b`, so it returns `None` (keeping the decimal-vs-fraction
/// distinction the whole module rests on).
fn int_div(e: &Expr) -> Option<(i64, i64)> {
    match e {
        Expr::Div(a, b) => Some((as_int(a)?, as_int(b)?)),
        _ => None,
    }
}

/// A signed integer term, folding a leading `Neg` (`-2` and `Neg(2)` → `-2`).
fn signed_int(e: &Expr) -> Option<i64> {
    let (neg, rest) = split_sign(e);
    let v = as_int(rest)?;
    Some(if neg { -v } else { v })
}

/// A signed written fraction, folding a leading `Neg` into the numerator
/// (`-(1/3)` → `(-1, 3)`). Decimals still return `None` (see [`int_div`]).
fn signed_div(e: &Expr) -> Option<(i64, i64)> {
    let (neg, rest) = split_sign(e);
    let (n, d) = int_div(rest)?;
    Some(if neg { (-n, d) } else { (n, d) })
}

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.unsigned_abs(), b.unsigned_abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a as i64
}

/// Is `n` free of any perfect `k`-th-power factor > 1? (`k = 2` is square-free,
/// for `sqrt`; `k = 3` cube-free, for `cbrt`; etc.) A `k`-th root of such an
/// integer cannot be simplified by pulling a factor out.
fn is_power_free_int(n: i64, k: u32) -> bool {
    if k < 2 {
        return true;
    }
    let mut n = n.unsigned_abs();
    if n == 0 {
        return false;
    }
    let mut d: u64 = 2;
    while let Some(dk) = d.checked_pow(k) {
        if dk > n {
            break;
        }
        if n.is_multiple_of(dk) {
            return false;
        }
        while n.is_multiple_of(d) {
            n /= d;
        }
        d += 1;
    }
    true
}

/// The integer numeric content of a written expression — the gcd of its
/// coefficients — used to spot a common numeric factor between a fraction's
/// numerator and denominator (`2x/2`, `4x/6`). Symbols/powers/functions are 1.
fn content(e: &Expr) -> i64 {
    match split_sign(e).1 {
        Expr::Num(Number::Int(n)) => *n,
        Expr::Mul(fs) => fs.iter().map(content).fold(1i64, |a, b| a.saturating_mul(b)),
        Expr::Add(ts) => ts.iter().map(content).fold(0i64, gcd),
        _ => 1,
    }
}

// ---- decimal / exact ----

fn is_decimal_literal(n: &Number) -> bool {
    // In the faithful tree a decimal parses to Rat (fractional part) or Float
    // (huge). Integers are `Int`; typed fractions are `Div`, not `Rat`.
    matches!(n, Number::Rat(..) | Number::Float(_))
        || matches!(n, Number::Big(b) if matches!(**b, crate::num::BigNumber::Rat(_)))
}

fn contains_decimal(e: &Expr) -> bool {
    e.any_subexpr(&|n| matches!(n, Expr::Num(m) if is_decimal_literal(m)))
}

/// Root is a bare number (optionally negated). Integers count — `3` is a valid
/// decimal answer; the F2 provenance tag is what distinguishes `3` from `3.0`.
fn is_decimal_number(e: &Expr) -> bool {
    match e {
        Expr::Num(_) => true,
        Expr::Neg(x) => is_decimal_number(x),
        _ => false,
    }
}

// ---- fractions ----

fn denom_has_radical(e: &Expr) -> bool {
    // A root written with a negative exponent (`x^(-1/2)` = 1/√x) already sits
    // in a denominator — checked here so every exponent spelling is caught.
    if root_of(e).is_some_and(|r| r.in_denominator) {
        return true;
    }
    match e {
        Expr::Div(_, d) => contains_radical(d) || denom_has_radical(d),
        // `base^(negative)` = 1/base^|·|: a surd there if the base has one.
        Expr::Pow(b, exp) if is_negative_sign(exp) => {
            contains_radical(b) || denom_has_radical(b)
        }
        _ => e.children().iter().any(|c| denom_has_radical(c)),
    }
}

fn is_reduced_fraction(e: &Expr) -> bool {
    // No un-reduced rational and no surd denominator anywhere.
    fn every_fraction_reduced(e: &Expr) -> bool {
        let here = match e {
            Expr::Div(num, den) => match (as_int(num), as_int(den)) {
                // Pure integer fraction: lowest terms ⇔ coprime.
                (Some(n), Some(d)) => d != 0 && gcd(n, d) == 1,
                // Otherwise reduced ⇔ (a) numerator and denominator share no
                // common numeric factor — `canonicalize` folds `2x/2 → x`, so
                // this must be checked on the *written* form, not the canonical
                // one — and (b) no polynomial factor cancels.
                _ => {
                    gcd(content(num), content(den)) == 1
                        && canonicalize(e) == canonicalize(&crate::reduce_rational(e))
                }
            },
            // A decimal-origin rational is always stored in lowest terms.
            Expr::Num(Number::Rat(..)) => true,
            _ => true,
        };
        here && e.children().iter().all(|c| every_fraction_reduced(c))
    }
    !denom_has_radical(e) && every_fraction_reduced(e)
}

fn is_mixed_number(e: &Expr) -> bool {
    // `±(a + b/c)`: an integer and a proper written fraction, both of the same
    // sign — so `2+1/3` and `-2-1/3` qualify, but `2-1/3` (= 5/3) does not.
    let Expr::Add(terms) = e else { return false };
    if terms.len() != 2 {
        return false;
    }
    let (int_part, frac) = match (signed_int(&terms[0]), signed_div(&terms[1])) {
        (Some(i), Some(f)) => (i, f),
        _ => match (signed_int(&terms[1]), signed_div(&terms[0])) {
            (Some(i), Some(f)) => (i, f),
            _ => return false,
        },
    };
    let (n, d) = frac;
    if int_part == 0 || n == 0 || d == 0 {
        return false;
    }
    // Same sign, proper (|n| < |d|), reduced.
    let int_positive = int_part > 0;
    let frac_positive = (n > 0) == (d > 0);
    int_positive == frac_positive
        && n.unsigned_abs() < d.unsigned_abs()
        && gcd(n, d) == 1
}

fn is_improper_fraction(e: &Expr) -> bool {
    let strip = strip_neg(e);
    match int_div(strip) {
        Some((n, d)) => d != 0 && n.unsigned_abs() >= d.unsigned_abs(),
        None => false,
    }
}

fn is_single_fraction(e: &Expr) -> bool {
    matches!(strip_neg(e), Expr::Div(..))
}

// ---- radicals ----
//
// Every radical check goes through one decomposition, [`root_of`], so the many
// syntactic spellings of a root — `sqrt(x)`, `cbrt(x)`, `nthroot(x, m)`,
// `x^(1/m)` (a `Div` exponent), `x^0.5` (a `Rat` exponent), and their negative
// (in-denominator) variants — are recognized in exactly one place.

/// A root node decomposed into its radicand, index (`m ≥ 2`), and whether the
/// exponent is negative (so it sits under a fraction bar). `None` if `e` is not
/// a root. Only *unit-fraction* powers `x^(±1/m)` count — `x^(2/3)` and
/// integer-valued exponents like `x^(4/2)` are deliberately excluded.
struct Root<'a> {
    radicand: &'a Expr,
    index: u32,
    in_denominator: bool,
}

fn root_of(e: &Expr) -> Option<Root<'_>> {
    match e {
        Expr::Apply(h, args) => {
            let Expr::Sym(s) = &**h else { return None };
            let (index, radicand) = match s.name().as_str() {
                "sqrt" => (2, args.first()?),
                "cbrt" => (3, args.first()?),
                // `nthroot(x, m)` is the m-th root; single-arg is a square root.
                "nthroot" => match args.get(1).and_then(as_int) {
                    Some(m) => (valid_index(m)?, args.first()?),
                    None if args.len() == 1 => (2, args.first()?),
                    _ => return None,
                },
                _ => return None,
            };
            Some(Root { radicand, index, in_denominator: false })
        }
        Expr::Pow(b, exp) => {
            let (index, negative) = unit_root_exponent(exp)?;
            Some(Root { radicand: b, index, in_denominator: negative })
        }
        _ => None,
    }
}

/// A unit-fraction exponent `±1/m` → `(m, negative)`, across the `Div` (written
/// `1/m`), `Rat` (decimal `0.5`), and `Neg`-wrapped spellings.
fn unit_root_exponent(exp: &Expr) -> Option<(u32, bool)> {
    match exp {
        Expr::Neg(x) => unit_root_exponent(x).map(|(m, neg)| (m, !neg)),
        Expr::Num(Number::Rat(n, d)) => unit_from(*n, *d),
        Expr::Div(a, b) => unit_from(as_int(a)?, as_int(b)?),
        _ => None,
    }
}

/// `n/d` as a root: numerator ±1, denominator the index `m ≥ 2`.
fn unit_from(n: i64, d: i64) -> Option<(u32, bool)> {
    if n.unsigned_abs() != 1 || d == 0 {
        return None;
    }
    let index = u32::try_from(d.unsigned_abs()).ok().filter(|&k| k >= 2)?;
    Some((index, (n < 0) ^ (d < 0)))
}

/// A valid root index `m ≥ 2` (guards the i64→u32 conversion against absurd
/// indices like `nthroot(x, 10^12)`).
fn valid_index(m: i64) -> Option<u32> {
    u32::try_from(m).ok().filter(|&k| k >= 2)
}

fn contains_radical(e: &Expr) -> bool {
    e.any_subexpr(&|n| root_of(n).is_some())
}

fn is_radical_simplified(e: &Expr) -> bool {
    if denom_has_radical(e) {
        return false;
    }
    // Every integer radicand must be free of a perfect power matching its index
    // (`sqrt` → square-free, `cbrt` → cube-free, `x^(1/m)` → m-th-power free);
    // otherwise a factor could be pulled out.
    fn radicands_ok(e: &Expr) -> bool {
        let here = match root_of(e) {
            Some(r) => as_int(r.radicand).is_none_or(|n| is_power_free_int(n, r.index)),
            None => true,
        };
        here && e.children().iter().all(|c| radicands_ok(c))
    }
    radicands_ok(e)
}

// ---- exponents ----

fn has_negative_exponent(e: &Expr) -> bool {
    e.any_subexpr(&|n| matches!(n, Expr::Pow(_, exp) if is_negative_sign(exp)))
}

// ---- expanded / factored ----

fn is_sum(e: &Expr) -> bool {
    matches!(e, Expr::Add(_))
}

/// Fully distributed: no `Mul` has a sum factor, and no sum is raised to an
/// integer power ≥ 2. (Matches STACK's `Expanded`: `x²-(a+b)x+ab` is expanded,
/// `(x-a)(x-b)` and `(x+1)^2` are not.)
fn is_expanded(e: &Expr) -> bool {
    let here = match e {
        Expr::Mul(fs) => !fs.iter().any(is_sum),
        Expr::Pow(b, exp) => !(is_sum(b) && as_int(exp).map(|k| k >= 2).unwrap_or(false)),
        _ => true,
    };
    here && e.children().iter().all(|c| is_expanded(c))
}

/// A product of irreducible factors. Oracle: `factor` produces the fully
/// factored form and `canonicalize` keeps products factored, so an already
/// completely-factored `e` satisfies `canonicalize(e) == canonicalize(factor(e))`.
/// (Univariate over ℚ — `factor` returns multivariate/non-poly unchanged, so
/// those are reported factored; documented limitation.)
fn is_factored_completely(e: &Expr) -> bool {
    canonicalize(e) == canonicalize(&crate::factor(e))
}

// ---- combine like terms ----

/// The "monomial key" of an additive term: its canonical form with any leading
/// numeric coefficient stripped. `None` marks a pure constant.
fn term_key(t: &Expr) -> Option<Expr> {
    match canonicalize(t) {
        Expr::Num(_) => None,
        Expr::Mul(mut fs) if matches!(fs.first(), Some(Expr::Num(_))) => {
            fs.remove(0);
            Some(canonicalize(&Expr::Mul(fs)))
        }
        other => Some(other),
    }
}

/// Two summands share a monomial key (combinable), or two summands are pure
/// constants (un-reduced arithmetic), in some `Add` node.
fn like_terms_remain(e: &Expr) -> bool {
    if let Expr::Add(terms) = e {
        let mut keys: Vec<Expr> = Vec::new();
        let mut consts = 0;
        for t in terms {
            match term_key(t) {
                None => consts += 1,
                Some(k) => {
                    if keys.contains(&k) {
                        return true;
                    }
                    keys.push(k);
                }
            }
        }
        if consts >= 2 {
            return true;
        }
    }
    e.children().iter().any(|c| like_terms_remain(c))
}

// ---- completed square ----

/// `a(x - h)^2 + k`: a sum whose only non-constant term is a (coefficient times
/// a) square of a degree-1 expression in a single variable.
fn is_completed_square(e: &Expr) -> bool {
    let Expr::Add(terms) = e else { return false };
    let vars = crate::variables(e);
    if vars.len() != 1 {
        return false;
    }
    let var = &vars[0];
    let mentions = |t: &Expr| t.any_subexpr(&|c| matches!(c, Expr::Sym(s) if &s.name()==var));
    let mut squares = 0;
    for t in terms {
        if !mentions(t) {
            continue; // the constant k
        }
        if is_square_of_linear(t, var) {
            squares += 1;
        } else {
            return false; // a variable term that is not the square
        }
    }
    squares == 1
}

/// `(linear)^2` or `c*(linear)^2`, linear = degree-1 in `var`.
fn is_square_of_linear(t: &Expr, var: &str) -> bool {
    match t {
        Expr::Mul(fs) => {
            // exactly one factor is the square; the rest are var-free.
            let mut sq = 0;
            for f in fs {
                let has_var = f.any_subexpr(&|c| matches!(c, Expr::Sym(s) if s.name()==var));
                if has_var {
                    if is_square_of_linear(f, var) {
                        sq += 1;
                    } else {
                        return false;
                    }
                }
            }
            sq == 1
        }
        Expr::Pow(b, exp) => as_int(exp) == Some(2) && is_linear_in(b, var),
        _ => false,
    }
}

/// Is `e` a degree-1 polynomial in `var` (`a·var + b`, with `a`/`b` free of
/// `var` and at least one `a·var` term)? Every canonical additive term must be
/// var-free or `coeff·var` with `var` a *bare* factor — so `var` inside a power
/// (`x^2`), a function (`sin(x)`), or a denominator (`1/x`) is rejected.
fn is_linear_in(e: &Expr, var: &str) -> bool {
    let terms = match canonicalize(e) {
        Expr::Add(ts) => ts,
        other => vec![other],
    };
    let mut has_var_term = false;
    for t in &terms {
        if symbol_occurrences(t, var) == 0 {
            continue; // a var-free (constant) term
        }
        let linear_term = match t {
            Expr::Sym(s) => s.name() == var,
            Expr::Mul(fs) => {
                let var_factors = fs
                    .iter()
                    .filter(|f| matches!(f, Expr::Sym(s) if s.name() == var))
                    .count();
                // exactly one bare `var` factor, and no `var` hiding elsewhere.
                var_factors == 1
                    && fs
                        .iter()
                        .filter(|f| !matches!(f, Expr::Sym(s) if s.name() == var))
                        .all(|f| symbol_occurrences(f, var) == 0)
            }
            _ => false, // var inside a Pow / Apply / Div / …
        };
        if !linear_term {
            return false;
        }
        has_var_term = true;
    }
    has_var_term
}

// ---- integration constant ----

/// Number of times the symbol `name` appears anywhere in `e`.
fn symbol_occurrences(e: &Expr, name: &str) -> usize {
    let here = matches!(e, Expr::Sym(s) if s.name() == name) as usize;
    here + e
        .children()
        .iter()
        .map(|c| symbol_occurrences(c, name))
        .sum::<usize>()
}

fn has_integration_constant(e: &Expr, exclude: Option<&str>) -> bool {
    let Expr::Add(terms) = e else { return false };
    // A `+ C` is an *isolated* additive symbol: a bare symbol (not the excluded
    // integration variable) that appears nowhere else. Requiring it appear
    // exactly once rejects a variable that merely happens to be a lone term
    // (`x + x^2`). It cannot distinguish `x + C` from `x + y` — that genuinely
    // needs `exclude` — so pass the integration variable there when known.
    terms.iter().any(|t| match strip_neg(t) {
        Expr::Sym(s) => {
            let name = s.name();
            exclude.map(|v| name != v).unwrap_or(true) && symbol_occurrences(e, &name) == 1
        }
        _ => false,
    })
}

// ---- sign / negation ----
//
// Every check that reasons about a leading sign goes through [`split_sign`], so
// the spellings of "negative" — a `Neg` wrapper, a negative numeric literal, a
// fraction with a negative part (`Div(-1, 2)`) — are handled in one place.

/// Peel leading `Neg` wrappers: returns whether the overall sign is flipped (an
/// odd number of `Neg`s) and the un-wrapped expression.
fn split_sign(e: &Expr) -> (bool, &Expr) {
    match e {
        Expr::Neg(x) => {
            let (neg, inner) = split_sign(x);
            (!neg, inner)
        }
        other => (false, other),
    }
}

/// The expression with any leading `Neg` wrappers removed (magnitude only).
fn strip_neg(e: &Expr) -> &Expr {
    split_sign(e).1
}

/// Is `e` written with an overall negative sign? Folds a `Neg` wrapper, a
/// negative numeric literal, and a fraction with an odd number of negative
/// parts, so every spelling agrees (used on exponents and fraction parts).
fn is_negative_sign(e: &Expr) -> bool {
    let (neg, rest) = split_sign(e);
    neg ^ match rest {
        Expr::Num(n) => n.is_negative(),
        Expr::Div(a, b) => is_negative_sign(a) ^ is_negative_sign(b),
        _ => false,
    }
}

// Kept for a future `MatchesTemplate`/sequence-aware check; silence dead-code
// until then without dropping the intent.
#[allow(dead_code)]
fn is_seq(e: &Expr, kind: SeqKind) -> bool {
    matches!(e, Expr::Seq(k, _) if *k == kind)
}
