//! Integration (INTEGRATION_PLAN / DIVERGENCE_PLAN) and arbitrary-precision
//! evaluation of constant expressions.

use super::Expression;
use math_expressions::eval_numeric::certified_digits as cd;
use math_expressions::{Expr, Number};
use wasm_bindgen::prelude::*;

/// How `evaluate_to_precision` renders its digits (mirrors the core
/// [`cd::DecimalFormat`] across the JS boundary).
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum DecimalFormat {
    /// Plain decimal expansion (`"1.41421356…"`).
    Plain,
    /// Normalized scientific form (`"1.4142…e0"`).
    Scientific,
}

impl From<DecimalFormat> for cd::DecimalFormat {
    fn from(f: DecimalFormat) -> Self {
        match f {
            DecimalFormat::Plain => cd::DecimalFormat::Plain,
            DecimalFormat::Scientific => cd::DecimalFormat::Scientific,
        }
    }
}

#[wasm_bindgen]
impl Expression {
    /// Evaluate a constant expression to `digits` significant decimal digits
    /// (arbitrary precision). Renders per `format` — [`DecimalFormat::Plain`]
    /// (the default) or [`DecimalFormat::Scientific`]; `"re + im i"` for complex
    /// values, or `undefined` when not decidable within budget.
    pub fn evaluate_to_precision(
        &self,
        digits: usize,
        format: Option<DecimalFormat>,
    ) -> Option<String> {
        let fmt = format.unwrap_or(DecimalFormat::Plain).into();
        let p = cd::evaluate_to_precision(&self.0, digits);
        p.to_decimal_string_fmt(digits, fmt)
    }

    /// Indefinite integral in `var` (INTEGRATION_PLAN I1+I2), gate-verified
    /// by differentiation; `undefined` = no elementary form found.
    pub fn integrate(&self, var: &str) -> Option<Expression> {
        math_expressions::integrate(&self.0, var, &math_expressions::Assumptions::new())
            .map(|e| self.derive(e))
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
        cd::integrate_to_precision(&self.0, var, &a.0, &b.0, digits).to_decimal_string(digits)
    }

    /// Best-effort numeric definite integral over `[lower, upper]` — the port of
    /// the JS `integrateNumerically(var, lower, upper)`. Backed by the CERTIFIED
    /// `integrate_to_precision` reduced to an `f64`: returns the value when it
    /// can be certified, `undefined` when it cannot. This is the honest
    /// divergence from JS, which always returns a (possibly inaccurate) estimate
    /// — here a hard/divergent integrand yields `undefined` rather than a
    /// silently-wrong number.
    ///
    /// 10 significant digits (not the ≤13 certified max): ample for an f64
    /// estimate, with margin so near-cancellation cases — e.g. `∫₀^π sin`, which
    /// fails to certify at 13 — still return a value.
    pub fn integrate_numerically(&self, var: &str, lower: f64, upper: f64) -> Option<f64> {
        let a = Expr::Num(Number::from_f64(lower));
        let b = Expr::Num(Number::from_f64(upper));
        cd::integrate_to_precision(&self.0, var, &a, &b, 10).to_f64()
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
        use cd::IntegralVerdict;
        let v = cd::integrate_analyzed(&self.0, var, &a.0, &b.0, digits);
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
                            "exact": s.exact.as_ref().map(|e| self.text_of(e)),
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

#[cfg(test)]
mod integrate_numerically_tests {
    use crate::parse::parse_text;

    fn quad(f: &str, lo: f64, hi: f64) -> Option<f64> {
        parse_text(f)
            .unwrap_or_else(|_| panic!("parse {f:?}"))
            .integrate_numerically("x", lo, hi)
    }

    fn assert_close(got: Option<f64>, want: f64, what: &str) {
        let v = got.unwrap_or_else(|| panic!("{what}: expected a certified value, got undefined"));
        assert!(
            (v - want).abs() <= 1e-9 * want.abs().max(1.0),
            "{what}: got {v}, want {want}"
        );
    }

    /// The values a caller of the JS `integrateNumerically` shim would get.
    #[test]
    fn returns_the_certified_value() {
        assert_close(quad("x^3", 0.0, 1.0), 0.25, "∫₀¹ x³");
        assert_close(quad("1/x", 1.0, 2.0), std::f64::consts::LN_2, "∫₁² 1/x");
        assert_close(
            quad("exp(-x^2)", -1.0, 1.0),
            1.493648265624854,
            "∫₋₁¹ e^-x²",
        );
        // Reversed limits are the negated integral, not a failure.
        assert_close(quad("x^3", 1.0, 0.0), -0.25, "∫₁⁰ x³");
        // Degenerate interval.
        assert_close(quad("sin(x)", 1.0, 1.0), 0.0, "∫₁¹ sin");
    }

    /// The reason the binding asks for 10 digits rather than the certified
    /// maximum: `∫₀^π sin x = 2` is a near-cancellation case that fails to
    /// certify at 13 digits. It must still deliver a value.
    #[test]
    fn near_cancellation_still_certifies_at_ten_digits() {
        assert_close(
            quad("sin(x)", 0.0, std::f64::consts::PI),
            2.0,
            "∫₀^π sin (via the f64 π endpoint)",
        );
    }

    /// The honest divergence from JS: rather than the silently-wrong estimate
    /// the JS midpoint rule returns, an integrand it cannot certify yields
    /// `undefined` (which the js-compat shim maps to `NaN`).
    #[test]
    fn uncertifiable_integrands_are_undefined_not_wrong() {
        // Non-integrable singularity inside the interval: ∫₋₁¹ 1/x diverges.
        assert_eq!(quad("1/x", -1.0, 1.0), None);
        // A free variable other than the integration variable is not a number.
        assert_eq!(quad("x*y", 0.0, 1.0), None);
    }
}
