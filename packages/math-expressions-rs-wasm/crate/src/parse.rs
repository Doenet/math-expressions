//! Text/LaTeX parsing entry points (plain and with JS-style option objects),
//! plus the shared JSON option readers used here and in [`super::grading`].

use super::Expression;
use math_expressions::{
    Digits, Grouping, LatexToAst, LatexToAstOptions, NumberNotation, TextToAst, TextToAstOptions,
};
use wasm_bindgen::prelude::*;

/// Parse text syntax (e.g. `"sin^2 x + cos^2 x"`).
#[wasm_bindgen]
pub fn parse_text(s: &str) -> Result<Expression, JsError> {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .map(Expression::with_default_notation)
        .map_err(|e| JsError::new(&e.to_string()))
}

/// Parse LaTeX syntax (e.g. `"\\frac{1}{2}"`).
#[wasm_bindgen]
pub fn parse_latex(s: &str) -> Result<Expression, JsError> {
    LatexToAst::new(LatexToAstOptions::default())
        .convert(s)
        .map(Expression::with_default_notation)
        .map_err(|e| JsError::new(&e.to_string()))
}

/// Parse text with JS-style parser parameters (the port of Doenet's
/// `new me.converters.textToAstObj({...})` pattern). `options_json` keys —
/// all optional, JS spellings: `splitSymbols`, `unsplitSymbols`,
/// `appliedFunctionSymbols`, `functionSymbols`, `operatorSymbols`,
/// `allowSimplifiedFunctionApplication`, `parseLeibnizNotation`,
/// `parseScientificNotation`.
#[wasm_bindgen]
pub fn parse_text_with_options(s: &str, options_json: &str) -> Result<Expression, JsError> {
    let v: serde_json::Value =
        serde_json::from_str(options_json).map_err(|e| JsError::new(&e.to_string()))?;
    let mut o = TextToAstOptions::default();
    read_opt_bool(&v, "splitSymbols", &mut o.split_symbols);
    read_opt_bool(
        &v,
        "allowSimplifiedFunctionApplication",
        &mut o.allow_simplified_function_application,
    );
    read_opt_bool(&v, "parseLeibnizNotation", &mut o.parse_leibniz_notation);
    read_opt_bool(&v, "parseScientificNotation", &mut o.parse_scientific_notation);
    read_opt_strings(&v, "unsplitSymbols", &mut o.unsplit_symbols);
    read_opt_strings(&v, "appliedFunctionSymbols", &mut o.applied_function_symbols);
    read_opt_strings(&v, "functionSymbols", &mut o.function_symbols);
    read_opt_strings(&v, "operatorSymbols", &mut o.operator_symbols);
    read_notation(&v, &mut o.notation).map_err(|e| JsError::new(&e))?;
    let notation = o.notation.clone();
    TextToAst::new(o)
        .convert(s)
        .map(|e| Expression(e, notation))
        .map_err(|e| JsError::new(&e.to_string()))
}

/// Parse LaTeX with JS-style parser parameters (keys: `allowedLatexSymbols`,
/// `appliedFunctionSymbols`, `functionSymbols`,
/// `allowSimplifiedFunctionApplication`, `parseLeibnizNotation`,
/// `parseScientificNotation`).
#[wasm_bindgen]
pub fn parse_latex_with_options(s: &str, options_json: &str) -> Result<Expression, JsError> {
    let v: serde_json::Value =
        serde_json::from_str(options_json).map_err(|e| JsError::new(&e.to_string()))?;
    let mut o = LatexToAstOptions::default();
    read_opt_bool(
        &v,
        "allowSimplifiedFunctionApplication",
        &mut o.allow_simplified_function_application,
    );
    read_opt_bool(&v, "parseLeibnizNotation", &mut o.parse_leibniz_notation);
    read_opt_bool(&v, "parseScientificNotation", &mut o.parse_scientific_notation);
    read_opt_strings(&v, "allowedLatexSymbols", &mut o.allowed_latex_symbols);
    read_opt_strings(&v, "appliedFunctionSymbols", &mut o.applied_function_symbols);
    read_opt_strings(&v, "functionSymbols", &mut o.function_symbols);
    read_notation(&v, &mut o.notation).map_err(|e| JsError::new(&e))?;
    let notation = o.notation.clone();
    LatexToAst::new(o)
        .convert(s)
        .map(|e| Expression(e, notation))
        .map_err(|e| JsError::new(&e.to_string()))
}

pub(super) fn read_opt_f64(v: &serde_json::Value, key: &str, target: &mut f64) {
    if let Some(x) = v.get(key).and_then(serde_json::Value::as_f64) {
        *target = x;
    }
}

pub(super) fn read_opt_bool(v: &serde_json::Value, key: &str, target: &mut bool) {
    if let Some(b) = v.get(key).and_then(serde_json::Value::as_bool) {
        *target = b;
    }
}

pub(super) fn read_opt_strings(v: &serde_json::Value, key: &str, target: &mut Vec<String>) {
    if let Some(arr) = v.get(key).and_then(serde_json::Value::as_array) {
        *target = arr
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect();
    }
}

/// First char of a string option, if present.
fn opt_char(v: &serde_json::Value, key: &str) -> Option<char> {
    v.get(key).and_then(|x| x.as_str()).and_then(|s| s.chars().next())
}

/// Read the `notation` sub-object (I18N_MATH_NOTATION_PLAN) into `n`. Keys:
/// `decimalSeparator`, `argumentSeparator` (single chars); `alsoAcceptDecimal`
/// (array of single-char strings); `groupSeparator` (single char);
/// `grouping` (`"western"` | `"indian"` | `"none"`); `digits` (`"latin"` |
/// `"arabic"` | `"devanagari"`). Absent keys keep the current value.
///
/// Convenience for the two special-cased pairs: if the decimal separator is
/// given but the argument separator is **not**, the argument separator is
/// auto-filled from [`NumberNotation::paired_argument_separator`] (`.`→`,`,
/// `,`→`;`).
///
/// Validation is built in (an explicit ambiguous pair, e.g. decimal and
/// argument both `,`, is `Err`, not a silent misparse) so that a future
/// notation-accepting entry point cannot forget it.
pub(super) fn read_notation(
    v: &serde_json::Value,
    n: &mut NumberNotation,
) -> Result<(), String> {
    let Some(o) = v.get("notation") else {
        return Ok(());
    };
    let (dec, arg) = (opt_char(o, "decimalSeparator"), opt_char(o, "argumentSeparator"));
    if let Some(d) = dec {
        n.decimal_separator = d;
    }
    match arg {
        Some(a) => n.argument_separator = a,
        // Only the decimal was given → fill in its conventional partner.
        None if dec.is_some() => {
            if let Some(p) = NumberNotation::paired_argument_separator(n.decimal_separator) {
                n.argument_separator = p;
            }
        }
        None => {}
    }
    if let Some(arr) = o.get("alsoAcceptDecimal").and_then(|x| x.as_array()) {
        let chars: Vec<char> = arr
            .iter()
            .filter_map(|x| x.as_str())
            .filter_map(|s| s.chars().next())
            .collect();
        n.also_accept_decimal = (!chars.is_empty()).then_some(chars);
    }
    if let Some(s) = o.get("groupSeparator").and_then(|x| x.as_str()) {
        n.group_separator = s.chars().next();
    }
    if let Some(s) = o.get("grouping").and_then(|x| x.as_str()) {
        n.grouping = match s.to_ascii_lowercase().as_str() {
            "western" => Grouping::Western,
            "indian" => Grouping::Indian,
            _ => Grouping::None,
        };
    }
    if let Some(s) = o.get("digits").and_then(|x| x.as_str()) {
        n.digits = match s.to_ascii_lowercase().as_str() {
            "arabic" => Digits::Arabic,
            "devanagari" => Digits::Devanagari,
            _ => Digits::Latin,
        };
    }
    // The Phase-2 keys are part of the stable JSON schema (I18N plan decision
    // C1) but their behavior is unimplemented. Declaring one and silently not
    // honoring it would be a wrong parse presented as success — refuse
    // explicitly until Phase 2 lands.
    if n.group_separator.is_some() || n.grouping != Grouping::None || n.digits != Digits::Latin {
        return Err(
            "notation keys groupSeparator/grouping/digits are not yet supported (i18n Phase 2)"
                .to_string(),
        );
    }
    n.validate()
}

#[cfg(test)]
mod tests {
    use super::read_notation;
    use math_expressions::NumberNotation;

    fn resolve(json: &str) -> Result<NumberNotation, String> {
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let mut n = NumberNotation::default();
        read_notation(&v, &mut n)?; // validation is built into read_notation
        Ok(n)
    }

    #[test]
    fn decimal_only_autofills_paired_argument() {
        // ',' decimal alone → ';' argument (the special-cased pair).
        let n = resolve(r#"{"notation":{"decimalSeparator":","}}"#).unwrap();
        assert_eq!((n.decimal_separator, n.argument_separator), (',', ';'));
        // '.' decimal alone → ',' argument.
        let n = resolve(r#"{"notation":{"decimalSeparator":"."}}"#).unwrap();
        assert_eq!((n.decimal_separator, n.argument_separator), ('.', ','));
    }

    #[test]
    fn argument_only_leaves_decimal_default() {
        // Only the argument given: decimal stays '.', no auto-pairing kicks in.
        let n = resolve(r#"{"notation":{"argumentSeparator":";"}}"#).unwrap();
        assert_eq!((n.decimal_separator, n.argument_separator), ('.', ';'));
    }

    #[test]
    fn explicit_pair_is_respected_and_overrides_convention() {
        // Both given explicitly (even the unconventional '.'/';' combo).
        let n = resolve(r#"{"notation":{"decimalSeparator":".","argumentSeparator":";"}}"#).unwrap();
        assert_eq!((n.decimal_separator, n.argument_separator), ('.', ';'));
    }

    #[test]
    fn explicit_ambiguous_pair_is_an_error_not_a_panic() {
        // Decimal ',' with an explicit argument ',' is ambiguous → Err (no panic).
        let r = resolve(r#"{"notation":{"decimalSeparator":",","argumentSeparator":","}}"#);
        assert!(r.is_err(), "ambiguous explicit pair must error");
    }

    #[test]
    fn absent_notation_keeps_defaults() {
        let n = resolve(r#"{}"#).unwrap();
        assert_eq!(n, NumberNotation::default());
    }

    #[test]
    fn phase2_stub_keys_error_instead_of_silently_not_applying() {
        for json in [
            r#"{"notation":{"groupSeparator":" "}}"#,
            r#"{"notation":{"grouping":"western"}}"#,
            r#"{"notation":{"digits":"arabic"}}"#,
        ] {
            assert!(resolve(json).is_err(), "{json} must be rejected until Phase 2");
        }
        // Explicit no-op values of the stub keys are fine.
        assert!(resolve(r#"{"notation":{"grouping":"none","digits":"latin"}}"#).is_ok());
    }
}
