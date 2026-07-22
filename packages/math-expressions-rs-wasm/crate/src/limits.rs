//! §7f resource limits at the wasm boundary: JS embedders (long-lived grading
//! workers) configure the deterministic operation/size bounds once per worker.

use math_expressions::resource_limits::{self, ResourceLimits};
use wasm_bindgen::prelude::*;

macro_rules! limit_fields {
    ($($f:ident),+ $(,)?) => {
        /// Host-testable core of [`set_resource_limits`] (`JsError` cannot be
        /// constructed off-wasm, so errors are `String` here).
        fn set_resource_limits_impl(options_json: &str) -> Result<(), String> {
            let v: serde_json::Value =
                serde_json::from_str(options_json).map_err(|e| e.to_string())?;
            let obj = v
                .as_object()
                .ok_or_else(|| "expected a JSON object of limit fields".to_string())?;
            const KNOWN: &[&str] = &[$(stringify!($f)),+];
            if let Some(bad) = obj.keys().find(|k| !KNOWN.contains(&k.as_str())) {
                return Err(format!("unknown resource limit {bad:?}"));
            }
            let mut l = resource_limits::current();
            $(
                if let Some(x) = obj.get(stringify!($f)).and_then(|x| x.as_i64()) {
                    if x < 0 {
                        return Err(format!(
                            "resource limit {} must be nonnegative",
                            stringify!($f)
                        ));
                    }
                    l.$f = x as _;
                }
            )+
            resource_limits::set_current(l);
            Ok(())
        }

        /// Set resource limits from a JSON object; keys are the snake_case
        /// field names of `ResourceLimits` (e.g. `{"max_expand_terms": 8000}`).
        /// Absent keys keep their current values. Unknown keys are an error —
        /// a typo'd limit must not silently keep the default.
        #[wasm_bindgen]
        pub fn set_resource_limits(options_json: &str) -> Result<(), JsError> {
            set_resource_limits_impl(options_json).map_err(|e| JsError::new(&e))
        }

        /// The limits currently in effect, as a JSON object (same keys as
        /// [`set_resource_limits`] accepts).
        #[wasm_bindgen]
        pub fn get_resource_limits() -> String {
            let l: ResourceLimits = resource_limits::current();
            let mut m = serde_json::Map::new();
            $(
                m.insert(
                    stringify!($f).to_string(),
                    serde_json::Value::from(l.$f as i64),
                );
            )+
            serde_json::Value::Object(m).to_string()
        }
    };
}

limit_fields!(
    max_expand_power,
    max_expand_terms,
    max_simplify_rounds,
    max_trial_divisor,
    max_factorial,
    max_residues,
    max_round_decimals,
    max_pow_bits,
    max_matrix_dim,
    max_symbolic_det_dim,
    max_eval_precision_bits,
    max_ziv_rounds,
    max_series_terms,
    max_tape_ops,
    max_trig_arg_bits,
    max_rootof_degree,
    max_isolation_bits,
    max_quadrature_segments,
    max_integration_steps,
    max_integration_candidates,
    max_lrt_degree,
    max_factor_degree,
    max_exact_eval_ops,
    max_squarefree_trial_divisor,
    max_ode_steps,
    max_singularity_candidates,
    max_certificate_bisections,
    max_improper_refinements,
);

#[cfg(test)]
mod tests {
    #[test]
    fn set_then_get_round_trips_and_rejects_unknown_keys() {
        assert!(super::set_resource_limits_impl(r#"{"max_expand_terms": 8000}"#).is_ok());
        let got = super::get_resource_limits();
        let v: serde_json::Value = serde_json::from_str(&got).unwrap();
        assert_eq!(v["max_expand_terms"], 8000);
        assert!(super::set_resource_limits_impl(r#"{"max_expand_termz": 1}"#).is_err());
        assert!(super::set_resource_limits_impl(r#"{"max_expand_terms": -5}"#).is_err());
        // Restore the default for other tests on this thread.
        math_expressions::resource_limits::set_current(Default::default());
    }
}
