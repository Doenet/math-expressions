//! The `pi`/`e`/`i` declaration at the wasm boundary — the port of the JS
//! library's `createInstance({define_e, define_pi, define_i})`.
//!
//! Set once per document (or per worker): whether those three names denote the
//! mathematical constants or ordinary variables, and whether the declared ones
//! sort ahead of variables when an expression is displayed. See
//! `math_expressions::constant_policy` for what the declaration reaches and,
//! importantly, what it deliberately does not (no comparator changes, so
//! canonical trees stay comparable across settings).

use math_expressions::constant_policy::{self, ConstantPolicy};
use wasm_bindgen::prelude::*;

macro_rules! policy_fields {
    ($($f:ident),+ $(,)?) => {
        /// Host-testable core of [`set_constant_policy`] (`JsError` cannot be
        /// constructed off-wasm, so errors are `String` here).
        fn set_constant_policy_impl(options_json: &str) -> Result<(), String> {
            let v: serde_json::Value =
                serde_json::from_str(options_json).map_err(|e| e.to_string())?;
            let obj = v
                .as_object()
                .ok_or_else(|| "expected a JSON object of constant-policy fields".to_string())?;
            const KNOWN: &[&str] = &[$(stringify!($f)),+];
            if let Some(bad) = obj.keys().find(|k| !KNOWN.contains(&k.as_str())) {
                return Err(format!("unknown constant policy field {bad:?}"));
            }
            let mut p = constant_policy::current();
            $(
                if let Some(x) = obj.get(stringify!($f)) {
                    p.$f = x.as_bool().ok_or_else(|| {
                        format!("constant policy {} must be a boolean", stringify!($f))
                    })?;
                }
            )+
            constant_policy::set(p);
            Ok(())
        }

        /// Declare which of `pi`, `e`, `i` are constants here, from a JSON
        /// object keyed by the `ConstantPolicy` field names — e.g.
        /// `{"define_e": false}` for a document whose points are `(e, f)`.
        /// Absent keys keep their current values; unknown keys are an error, so
        /// a typo'd declaration cannot silently leave the default in place.
        #[wasm_bindgen]
        pub fn set_constant_policy(options_json: &str) -> Result<(), JsError> {
            set_constant_policy_impl(options_json).map_err(|e| JsError::new(&e))
        }

        /// The policy currently in effect, as a JSON object (same keys as
        /// [`set_constant_policy`] accepts).
        #[wasm_bindgen]
        pub fn get_constant_policy() -> String {
            let p: ConstantPolicy = constant_policy::current();
            let mut m = serde_json::Map::new();
            $(
                m.insert(stringify!($f).to_string(), serde_json::Value::from(p.$f));
            )+
            serde_json::Value::Object(m).to_string()
        }
    };
}

policy_fields!(
    define_pi,
    define_e,
    define_i,
    sort_constants_first,
    pow_strict
);

#[cfg(test)]
mod tests {
    #[test]
    fn set_then_get_round_trips_and_rejects_bad_input() {
        assert!(super::set_constant_policy_impl(r#"{"define_e": false}"#).is_ok());
        let v: serde_json::Value = serde_json::from_str(&super::get_constant_policy()).unwrap();
        assert_eq!(v["define_e"], false);
        // Untouched keys keep their values.
        assert_eq!(v["define_pi"], true);
        assert!(super::set_constant_policy_impl(r#"{"define_ee": true}"#).is_err());
        assert!(super::set_constant_policy_impl(r#"{"define_e": 1}"#).is_err());
        // Restore the default for other tests on this thread.
        math_expressions::constant_policy::set(Default::default());
    }
}
