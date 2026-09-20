//! The compat polynomial engine's boundary: one entry point, JSON in, JSON out.
//!
//! Every operation here works on the `["polynomial", v, [[d, c], …]]` AST
//! rather than on an [`Expression`](crate::Expression), so none of them fits the
//! handle-method shape the rest of these bindings use — and the argument types
//! differ per operation (a polynomial, a list of them, a monomial, an index).
//! Rather than twenty near-identical `#[wasm_bindgen]` functions that would each
//! re-do the same decode, the operation is named by a string and the arguments
//! arrive as a JSON array. The JS side is a table of one-line wrappers.
//!
//! `undefined` means either "this operation has no answer" (`mono_div` on a
//! non-divisor, `polynomial_pow` on a non-integer exponent) or "the arguments
//! could not be read". The compat API's callers have no separate error channel
//! — the legacy engine returned `undefined` for both — so neither does this.

use math_expressions::polynomials::compat as poly;
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

/// Run one compat polynomial operation. See the module docs for the encoding.
#[wasm_bindgen]
pub fn poly_op(op: &str, args_json: &str) -> Option<String> {
    let args: Vec<Value> = serde_json::from_str(args_json).ok()?;
    Some(dispatch(op, &args).ok()??.to_string())
}

/// `Err` for an unreadable argument, `Ok(None)` for an operation with no
/// answer. Both reach JS as `undefined`; they are kept apart here only so the
/// `?` operator can be used on the decoders.
fn dispatch(op: &str, args: &[Value]) -> Result<Option<Value>, String> {
    let arg = |i: usize| args.get(i).ok_or_else(|| format!("{op}: missing argument"));
    let p = |i: usize| poly::poly_from_json(arg(i)?);
    let m = |i: usize| poly::mono_from_json(arg(i)?);
    let ps = |i: usize| poly::polys_from_json(arg(i)?);

    let out = match op {
        "expression_to_polynomial" => {
            let e = math_expressions::expr::serde::try_from_js(arg(0)?)?;
            match poly::expression_to_polynomial(&e) {
                Some(p) => poly::poly_to_json(&p),
                None => Value::Bool(false),
            }
        }
        "polynomial_to_expression" => {
            math_expressions::expr::serde::to_js(&poly::polynomial_to_expression(&p(0)?))
        }

        "polynomial_add" => poly::poly_to_json(&poly::polynomial_add(&p(0)?, &p(1)?)),
        "polynomial_sub" => poly::poly_to_json(&poly::polynomial_sub(&p(0)?, &p(1)?)),
        "polynomial_mul" => poly::poly_to_json(&poly::polynomial_mul(&p(0)?, &p(1)?)),
        "polynomial_neg" => poly::poly_to_json(&poly::polynomial_neg(&p(0)?)),
        "polynomial_pow" => match poly::polynomial_pow(&p(0)?, &p(1)?) {
            Some(r) => poly::poly_to_json(&r),
            None => return Ok(None),
        },

        "initial_term" => poly::mono_to_json(&poly::initial_term(&p(0)?)),
        "mono_less_than" => Value::Bool(poly::mono_less_than(&m(0)?, &m(1)?)),
        "mono_is_div" => Value::Bool(poly::mono_is_div(&m(0)?, &m(1)?)),
        "mono_gcd" => poly::mono_to_json(&poly::mono_gcd(&m(0)?, &m(1)?)),
        "mono_to_poly" => poly::poly_to_json(&poly::mono_to_poly(&m(0)?)),
        "mono_div" => match poly::mono_div(&m(0)?, &m(1)?) {
            Some(r) => poly::mono_to_json(&r),
            None => return Ok(None),
        },

        // `0` rather than `undefined` for "no term is divisible": the legacy
        // spelling, and the callers test it with `!== 0`.
        "max_div_init" => {
            let monos = poly::monos_from_json(arg(1)?)?;
            match poly::max_div_init(&p(0)?, &monos) {
                Some((mono, i)) => json!([poly::mono_to_json(&mono), i]),
                None => json!(0),
            }
        }
        "poly_div" => {
            let (quotient, remainder) = poly::poly_div(&p(0)?, &ps(1)?);
            json!([
                quotient
                    .iter()
                    .map(|(i, mono)| json!([i, poly::mono_to_json(mono)]))
                    .collect::<Vec<_>>(),
                poly::poly_to_json(&remainder),
            ])
        }
        "reduce" => poly::polys_to_json(&poly::reduce(&ps(0)?)),
        "reduce_ith" => {
            let i = arg(0)?.as_u64().ok_or("reduce_ith: index")? as usize;
            let polys = ps(1)?;
            // `reduce_ith` indexes the list directly, and this crate is built
            // `panic = "abort"`, so an out-of-range index from JS would trap
            // the module for the whole page rather than raise an error.
            if i >= polys.len() {
                return Ok(None);
            }
            poly::poly_to_json(&poly::reduce_ith(i, &polys))
        }
        "reduced_grobner" => poly::polys_to_json(&poly::reduced_grobner(&ps(0)?)),

        "poly_gcd" => match poly::poly_gcd(&p(0)?, &p(1)?) {
            Some(r) => poly::poly_to_json(&r),
            None => return Ok(None),
        },
        "poly_lcm" => match poly::poly_lcm(&p(0)?, &p(1)?) {
            Some(r) => poly::poly_to_json(&r),
            None => return Ok(None),
        },
        "reduce_rational_expression" => match poly::reduce_rational_expression(&p(0)?, &p(1)?) {
            Some((top, bottom)) => json!([poly::poly_to_json(&top), poly::poly_to_json(&bottom)]),
            None => return Ok(None),
        },

        _ => return Err(format!("unknown polynomial operation: {op}")),
    };
    Ok(Some(out))
}
