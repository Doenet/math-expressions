//! The [`StructuralComparison`] criterion enum and its verdict type.

/// A single structural criterion to test against a student answer. Each variant is
/// independent and opt-in (§2). Value-equality is *not* implied — use
/// [`structural_equality`](super::structural_equality) when both structure and
/// value are wanted.
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
    /// base structural comparison. Only meaningful in
    /// [`structural_equality`](super::structural_equality) (it needs the key);
    /// [`check_structural_comparison`](super::check_structural_comparison) rejects it.
    SameStructure,
}

/// The verdict for one [`StructuralComparison`]. `why` carries short feedback when `!ok`.
#[derive(Debug, Clone)]
pub struct StructuralComparisonResult {
    pub ok: bool,
    pub why: Option<String>,
}

impl StructuralComparisonResult {
    pub(super) fn pass() -> Self {
        StructuralComparisonResult { ok: true, why: None }
    }
    pub(super) fn fail(why: &str) -> Self {
        StructuralComparisonResult {
            ok: false,
            why: Some(why.to_string()),
        }
    }
    pub(super) fn of(ok: bool, why: &str) -> Self {
        if ok {
            Self::pass()
        } else {
            Self::fail(why)
        }
    }
}
