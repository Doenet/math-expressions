//! WHATS_LEFT A.2/A.3: the autograder-facing surface — equality variants,
//! structural comparison, certified zero-equivalence, and analyticity.

use super::parse::{read_opt_bool, read_opt_f64};
use super::Expression;
use crate::{
    check_structural_comparison as rust_check_structural, Assumptions, EqOptions,
    StructuralComparison, StructuralComparisonResult,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Expression {
    // ---- equality variants (items 10, 11) ----

    /// Numerical equality by sampling real points only, gated on both sides
    /// being analytic (JS `equalsViaReal`).
    pub fn equals_via_real(&self, other: &Expression) -> bool {
        crate::equals_via_real(&self.0, &other.0, &EqOptions::default())
    }

    /// Equality with grading options as JSON. Keys (all optional): numbers —
    /// `relativeTolerance`, `absoluteTolerance`, `toleranceForZero`,
    /// `allowedErrorInNumbers`; bools — `includeErrorInNumberExponents`,
    /// `allowedErrorIsAbsolute`, `allowBlanks`. So `3.14 == pi` becomes true
    /// with `{"allowedErrorInNumbers": 0.01}`.
    pub fn equals_with_options(&self, other: &Expression, options_json: &str) -> bool {
        let v: serde_json::Value = match serde_json::from_str(options_json) {
            Ok(v) => v,
            Err(_) => return self.equals(other),
        };
        let mut o = EqOptions::default();
        read_opt_f64(&v, "relativeTolerance", &mut o.relative_tolerance);
        read_opt_f64(&v, "absoluteTolerance", &mut o.absolute_tolerance);
        read_opt_f64(&v, "toleranceForZero", &mut o.tolerance_for_zero);
        read_opt_f64(&v, "allowedErrorInNumbers", &mut o.allowed_error_in_numbers);
        read_opt_bool(
            &v,
            "includeErrorInNumberExponents",
            &mut o.include_error_in_number_exponents,
        );
        read_opt_bool(&v, "allowedErrorIsAbsolute", &mut o.allowed_error_is_absolute);
        read_opt_bool(&v, "allowBlanks", &mut o.allow_blanks);
        crate::equals(&self.0, &other.0, &o)
    }

    // ---- structural comparison (STRUCTURAL_COMPARISON F3) ----

    /// Check whether this answer has a required *structure* (distinct from value).
    /// `comparison` is either a bare name (`"factoredCompletely"`) or an object
    /// (`{"type":"decimal","places":3}`,
    /// `{"type":"hasIntegrationConstant","exclude":"x"}`). Returns a JSON
    /// `{"ok":bool,"why":string|null}`; an unrecognized comparison yields
    /// `{"ok":false,"why":"unknown structural comparison"}`. Value-equality
    /// against a key is separate — see [`Self::structural_equality`].
    pub fn check_structural_comparison(&self, comparison: &str) -> String {
        let v: serde_json::Value =
            serde_json::from_str(comparison).unwrap_or(serde_json::Value::Null);
        match structural_comparison_from_json(&v) {
            Some(c) => comparison_result_to_json(&rust_check_structural(&self.0, &c)).to_string(),
            None => comparison_result_to_json(&StructuralComparisonResult {
                ok: false,
                why: Some("unknown structural comparison".into()),
            })
            .to_string(),
        }
    }

    /// Structural equality — the autograder primitive, a sibling to
    /// [`Self::equals`]: `self` (the student answer) is in the form `comparison`
    /// **and** value-equal to `key`. `comparison` uses the same JSON encoding as
    /// [`Self::check_structural_comparison`]; an unknown comparison is `false`.
    pub fn structural_equality(&self, key: &Expression, comparison: &str) -> bool {
        let v: serde_json::Value =
            serde_json::from_str(comparison).unwrap_or(serde_json::Value::Null);
        match structural_comparison_from_json(&v) {
            Some(c) => crate::structural_equality(&self.0, &key.0, &c, &EqOptions::default()),
            None => false,
        }
    }

    // ---- certified zero-equivalence (FULL_SIMPLIFY S1) ----

    /// Certified test for `self ≡ 0`: `true` = provably zero, `false` =
    /// provably nonzero, `undefined` = undecided. Never certifies a wrong
    /// answer (adversarial almost-zeros return `undefined`, not `true`).
    pub fn is_zero(&self) -> Option<bool> {
        crate::exact::is_zero(&self.0, &Assumptions::new())
    }

    // ---- analyticity (item 7) ----

    /// Is this an analytic expression (only `+ - * / ^`, sequences, and
    /// analytic functions)? `allow_abs`/`allow_arg` permit those functions;
    /// `allow_relation` permits the order relations (JS `isAnalytic`).
    pub fn is_analytic(&self, allow_abs: bool, allow_arg: bool, allow_relation: bool) -> bool {
        crate::is_analytic(
            &self.0,
            &crate::AnalyticOpts {
                allow_abs,
                allow_arg,
                allow_relation,
            },
        )
    }
}

/// Decode a `StructuralComparison` from either a bare name string or a
/// `{"type": …}` object (STRUCTURAL_COMPARISON F3). Returns `None` for an unknown name.
fn structural_comparison_from_json(v: &serde_json::Value) -> Option<StructuralComparison> {
    let name = v.as_str().or_else(|| v.get("type").and_then(|t| t.as_str()))?;
    Some(match name {
        "reducedFraction" => StructuralComparison::ReducedFraction,
        "mixedNumber" => StructuralComparison::MixedNumber,
        "improperFraction" => StructuralComparison::ImproperFraction,
        "decimal" => StructuralComparison::Decimal {
            places: v.get("places").and_then(|p| p.as_u64()).map(|n| n as u32),
        },
        "exactValue" => StructuralComparison::ExactValue,
        "combinedLikeTerms" => StructuralComparison::CombinedLikeTerms,
        "expanded" => StructuralComparison::Expanded,
        "factoredCompletely" => StructuralComparison::FactoredCompletely,
        "singleFraction" => StructuralComparison::SingleFraction,
        "noNegativeExponents" => StructuralComparison::NoNegativeExponents,
        "radicalSimplified" => StructuralComparison::RadicalSimplified,
        "completedSquare" => StructuralComparison::CompletedSquare,
        "sameStructure" => StructuralComparison::SameStructure,
        "hasIntegrationConstant" => StructuralComparison::HasIntegrationConstant {
            exclude: v
                .get("exclude")
                .and_then(|e| e.as_str())
                .map(str::to_string),
        },
        _ => return None,
    })
}

fn comparison_result_to_json(r: &StructuralComparisonResult) -> serde_json::Value {
    serde_json::json!({
        "ok": r.ok,
        "why": r.why,
    })
}
