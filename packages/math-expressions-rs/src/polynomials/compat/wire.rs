//! The AST spelling of a polynomial, and the codec for it.
//!
//! `["polynomial", v, [[d, c], …]]` and `["monomial", c, [[v, d], …]]` are not
//! expressions — no parser produces them and `expr::serde` cannot read them —
//! but they *contain* expressions at every leaf. So the codec is written here:
//! the polynomial skeleton by hand, the leaves through
//! [`expr::serde`](crate::expr::serde), which is the only part that has to
//! agree with the rest of the library.
//!
//! This spelling is the tested contract of the compat API: callers hand these
//! arrays in as literals and compare what comes back with deep equality.

use super::rep::{Mono, Poly};
use crate::expr::serde::{to_js, try_from_js};
use serde_json::{json, Value};

const POLY: &str = "polynomial";
const MONO: &str = "monomial";

/// Is this array headed by `head`?
fn headed(v: &Value, head: &str) -> bool {
    v.as_array()
        .and_then(|a| a.first())
        .and_then(Value::as_str)
        .is_some_and(|s| s == head)
}

pub fn poly_to_json(p: &Poly) -> Value {
    match p {
        Poly::Coeff(e) => to_js(e),
        Poly::Rec { var, terms } => json!([
            POLY,
            to_js(var),
            terms
                .iter()
                .map(|(d, c)| json!([d, poly_to_json(c)]))
                .collect::<Vec<_>>(),
        ]),
    }
}

pub fn poly_from_json(v: &Value) -> Result<Poly, String> {
    if !headed(v, POLY) {
        return Ok(Poly::Coeff(try_from_js(v)?));
    }
    let a = v.as_array().expect("checked by `headed`");
    let var = try_from_js(a.get(1).ok_or("polynomial: no variable")?)?;
    let terms = a
        .get(2)
        .and_then(Value::as_array)
        .ok_or("polynomial: no term list")?;
    Ok(Poly::Rec {
        var,
        terms: terms
            .iter()
            .map(|t| {
                let t = t.as_array().ok_or("polynomial: malformed term")?;
                let deg = degree(t.first().ok_or("polynomial: term without a degree")?)?;
                Ok((deg, poly_from_json(t.get(1).unwrap_or(&Value::Null))?))
            })
            .collect::<Result<Vec<_>, String>>()?,
    })
}

pub fn mono_to_json(m: &Mono) -> Value {
    match m {
        Mono::Coeff(e) => to_js(e),
        Mono::Term { coeff, vars } => json!([
            MONO,
            to_js(coeff),
            vars.iter()
                .map(|(v, d)| json!([to_js(v), d]))
                .collect::<Vec<_>>(),
        ]),
    }
}

pub fn mono_from_json(v: &Value) -> Result<Mono, String> {
    if !headed(v, MONO) {
        return Ok(Mono::Coeff(try_from_js(v)?));
    }
    let a = v.as_array().expect("checked by `headed`");
    let coeff = try_from_js(a.get(1).ok_or("monomial: no coefficient")?)?;
    let vars = a
        .get(2)
        .and_then(Value::as_array)
        .ok_or("monomial: no variable list")?;
    Ok(Mono::Term {
        coeff,
        vars: vars
            .iter()
            .map(|p| {
                let p = p.as_array().ok_or("monomial: malformed variable power")?;
                let var = try_from_js(p.first().ok_or("monomial: power without a variable")?)?;
                Ok((var, degree(p.get(1).unwrap_or(&Value::Null))?))
            })
            .collect::<Result<Vec<_>, String>>()?,
    })
}

/// An exponent, which JSON carries as a plain number.
fn degree(v: &Value) -> Result<i64, String> {
    v.as_i64()
        .or_else(|| v.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64))
        .ok_or_else(|| format!("not a degree: {v}"))
}

/// A list of polynomials.
pub fn polys_from_json(v: &Value) -> Result<Vec<Poly>, String> {
    v.as_array()
        .ok_or("expected a list of polynomials")?
        .iter()
        .map(poly_from_json)
        .collect()
}

pub fn polys_to_json(polys: &[Poly]) -> Value {
    Value::Array(polys.iter().map(poly_to_json).collect())
}

/// A list of monomials.
pub fn monos_from_json(v: &Value) -> Result<Vec<Mono>, String> {
    v.as_array()
        .ok_or("expected a list of monomials")?
        .iter()
        .map(mono_from_json)
        .collect()
}
