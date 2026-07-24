//! The two public entry points: the unary criterion dispatch
//! ([`check_structural_comparison`]) and the against-a-key
//! [`structural_equality`].

use super::forms::{has_integration_constant, is_completed_square, is_expanded, is_factored_completely, like_terms_remain};
use super::fractions::{
    contains_decimal, is_decimal_number, is_improper_fraction, is_mixed_number, is_reduced_fraction,
    is_single_fraction,
};
use super::radicals::{has_negative_exponent, is_radical_simplified};
use super::types::{StructuralComparison, StructuralComparisonResult};
use crate::equality::EqOptions;
use crate::expr::Expr;

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
/// [`equals`](crate::equals) in the JS `equalsVia*` family. `comparison` selects the method:
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
/// for a *pure* value check, call [`equals`](crate::equals). There is
/// deliberately no batch "grade" step — callers compose these per problem.
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
