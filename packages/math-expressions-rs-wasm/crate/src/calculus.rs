//! Integration (INTEGRATION_PLAN / DIVERGENCE_PLAN) and arbitrary-precision
//! evaluation of constant expressions.

use super::Expression;
use math_expressions::to_text;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Expression {
    /// Evaluate a constant expression to `digits` significant decimal digits
    /// (arbitrary precision). Returns the digits in normalized scientific
    /// form (`"1.4142…e0"`), `"re + im i"` for complex values, or
    /// `undefined` when not decidable within budget.
    pub fn evaluate_to_precision(&self, digits: usize) -> Option<String> {
        let p = math_expressions::precise::evaluate_to_precision(&self.0, digits);
        p.to_decimal_string(digits)
    }

    /// Indefinite integral in `var` (INTEGRATION_PLAN I1+I2), gate-verified
    /// by differentiation; `undefined` = no elementary form found.
    pub fn integrate(&self, var: &str) -> Option<Expression> {
        math_expressions::integrate::integrate(&self.0, var, &math_expressions::Assumptions::new()).map(Expression)
    }

    /// Certified definite integral over [a, b] to `digits` significant
    /// digits (guaranteed accuracy or `undefined` — never an estimate).
    pub fn integrate_to_precision(
        &self,
        var: &str,
        a: &Expression,
        b: &Expression,
        digits: usize,
    ) -> Option<String> {
        math_expressions::precise::integrate_to_precision(&self.0, var, &a.0, &b.0, digits)
            .to_decimal_string(digits)
    }

    /// Three-way definite-integral analysis (DIVERGENCE_PLAN): JSON
    /// `{"status":"value","value":…}` |
    /// `{"status":"divergent","singularities":[{"location":…,"exact":…?}]}` |
    /// `{"status":"unknown","reason":…}`.
    pub fn integrate_analyzed(
        &self,
        var: &str,
        a: &Expression,
        b: &Expression,
        digits: usize,
    ) -> String {
        use math_expressions::precise::IntegralVerdict;
        let v = math_expressions::precise::integrate_analyzed(&self.0, var, &a.0, &b.0, digits);
        match v {
            IntegralVerdict::Value(p) => serde_json::json!({
                "status": "value",
                "value": p.to_f64(),
                "digits": p.to_decimal_string(digits),
            })
            .to_string(),
            IntegralVerdict::Divergent { at } => {
                let sing: Vec<serde_json::Value> = at
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "location": s.location,
                            "exact": s.exact.as_ref().map(|e| to_text(e, &Default::default())),
                        })
                    })
                    .collect();
                serde_json::json!({"status": "divergent", "singularities": sing}).to_string()
            }
            IntegralVerdict::Unknown(reason) => {
                serde_json::json!({"status": "unknown", "reason": reason}).to_string()
            }
        }
    }
}
