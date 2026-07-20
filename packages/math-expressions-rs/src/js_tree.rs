//! Converter to the JS `Tree` JSON shape (PORTING_PLAN.md §5, "JS tree
//! interop"). ALL the ad-hoc JS encodings live here: parallel bool-tuples
//! for chained inequalities, boolean interval-closure leaves, the "＿" blank
//! symbol, single-arg apply with tuple wrapping.
//!
//! Infinity/NaN cannot be represented in JSON; they are encoded as
//! {"$": "Inf"} / {"$": "-Inf"} / {"$": "NaN"}, matching the fixture
//! extraction script (a JS Tree never contains plain objects, so this is
//! unambiguous).

use crate::expr::{Expr, MathConst, RelOp, SeqKind};
use crate::num::Number;
use serde_json::{json, Value};

/// Parse a JS `Tree` JSON value into an `Expr`. Inverse of [`to_js`] for the
/// tree shapes the parsers produce (Rat is not reconstructed — a `["/", a, b]`
/// node becomes `Div`, matching the parser). Panics on malformed input; the
/// caller (WASM boundary, tests) supplies well-formed trees.
///
/// Recursion is not depth-capped here (§6e): the realistic input path is a
/// JSON string deserialized by `serde_json`, whose own recursion limit (128)
/// rejects deeply-nested input before a `Value` is built, so `from_js` never
/// sees a tree deep enough to overflow. A hand-constructed `Value` could, but
/// that is not a user-input vector.
pub fn from_js(value: &Value) -> Expr {
    try_from_js(value).unwrap_or_else(|e| panic!("from_js: {e}"))
}

/// Non-panicking [`from_js`] for untrusted input (the wasm `from_ast`
/// boundary): malformed shapes become `Err` descriptions instead of panics
/// (wasm builds abort on panic, so a bad tree from JS must not unwind).
pub fn try_from_js(value: &Value) -> Result<Expr, String> {
    match value {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Expr::Num(Number::Int(i)))
            } else {
                Ok(Expr::Num(Number::from_f64(
                    n.as_f64().ok_or("non-finite JSON number")?,
                )))
            }
        }
        Value::String(s) => Ok(Expr::sym(s)),
        Value::Object(_) => match value.get("$").and_then(Value::as_str) {
            Some("Inf") => Ok(Expr::Const(MathConst::Inf)),
            Some("-Inf") => Ok(Expr::Const(MathConst::NegInf)),
            Some("NaN") => Ok(Expr::Const(MathConst::NaN)),
            other => Err(format!("unknown special {other:?}")),
        },
        Value::Array(arr) => from_js_array(arr),
        other => Err(format!("unexpected value {other}")),
    }
}

fn from_js_array(arr: &[Value]) -> Result<Expr, String> {
    let head = arr
        .first()
        .ok_or("empty array is not a tree")?
        .as_str()
        .ok_or("array head must be an operator string")?;
    let operands = &arr[1..];
    let each = || -> Result<Vec<Expr>, String> { operands.iter().map(try_from_js).collect() };
    let boxed = |i: usize| -> Result<Box<Expr>, String> {
        Ok(Box::new(try_from_js(operands.get(i).ok_or_else(
            || format!("operator {head:?} is missing operand {i}"),
        )?)?))
    };

    if let Some(kind) = seq_kind(head) {
        return Ok(Expr::Seq(kind, each()?));
    }
    if let Some(op) = rel_op(head) {
        // binary or chained-equality relation
        let operands = each()?;
        if operands.is_empty() {
            return Err(format!("relation {head:?} has no operands"));
        }
        let ops = vec![op; operands.len() - 1];
        return Ok(Expr::Relation { operands, ops });
    }

    Ok(match head {
        "+" => Expr::Add(each()?),
        "*" => Expr::Mul(each()?),
        "/" => Expr::Div(boxed(0)?, boxed(1)?),
        "^" => Expr::Pow(boxed(0)?, boxed(1)?),
        "-" => Expr::Neg(boxed(0)?),
        "and" => Expr::And(each()?),
        "or" => Expr::Or(each()?),
        "not" => Expr::Not(boxed(0)?),
        "union" => Expr::Union(each()?),
        "intersect" => Expr::Intersect(each()?),
        "prime" => Expr::Prime(boxed(0)?),
        "_" => Expr::Index(boxed(0)?, boxed(1)?),
        "ldots" => Expr::Ldots,
        "apply" => {
            let f = boxed(0)?;
            let arg = operands.get(1).ok_or("apply is missing its argument")?;
            let args = match arg.as_array() {
                Some(a) if a.first().and_then(Value::as_str) == Some("tuple") => {
                    a[1..].iter().map(try_from_js).collect::<Result<_, _>>()?
                }
                _ => vec![try_from_js(arg)?],
            };
            Expr::Apply(f, args)
        }
        "interval" => {
            let ep = tuple3(operands.first(), "interval endpoints")?;
            let cl = tuple3(operands.get(1), "interval closed")?;
            Expr::Interval {
                endpoints: Box::new((try_from_js(&ep[1])?, try_from_js(&ep[2])?)),
                closed: (
                    cl[1].as_bool().unwrap_or(false),
                    cl[2].as_bool().unwrap_or(false),
                ),
            }
        }
        "lts" | "gts" => {
            let args = operands
                .first()
                .and_then(Value::as_array)
                .ok_or("lts/gts args")?;
            let strict = operands
                .get(1)
                .and_then(Value::as_array)
                .ok_or("lts/gts strict")?;
            if args.len() < 2 || strict.len() != args.len() - 1 + 1 {
                return Err("lts/gts args/strict length mismatch".to_string());
            }
            let operands: Vec<Expr> = args[1..]
                .iter()
                .map(try_from_js)
                .collect::<Result<_, _>>()?;
            let ops = strict[1..]
                .iter()
                .map(|b| {
                    let s = b.as_bool().unwrap_or(false);
                    match (head, s) {
                        ("lts", true) => RelOp::Lt,
                        ("lts", false) => RelOp::Le,
                        (_, true) => RelOp::Gt,
                        (_, false) => RelOp::Ge,
                    }
                })
                .collect();
            Expr::Relation { operands, ops }
        }
        "matrix" => {
            let size = tuple3(operands.first(), "matrix size")?;
            let body = operands
                .get(1)
                .and_then(Value::as_array)
                .ok_or("matrix body")?;
            let rows = size[1].as_u64().ok_or("matrix rows")? as u32;
            let cols = size[2].as_u64().ok_or("matrix cols")? as u32;
            if rows.saturating_mul(cols) > 1_000_000 {
                return Err("matrix too large".to_string());
            }
            let mut entries = Vec::with_capacity((rows * cols) as usize);
            for r in 0..rows as usize {
                let row = body
                    .get(r + 1)
                    .and_then(Value::as_array)
                    .ok_or("matrix row")?;
                for c in 0..cols as usize {
                    entries.push(try_from_js(row.get(c + 1).ok_or("matrix entry")?)?);
                }
            }
            Expr::Matrix {
                rows,
                cols,
                entries,
            }
        }
        // everything else (unit, pm, angle, binom, vec, linesegment,
        // derivative_leibniz, forall, arrows, implies, iff, perp, ":", "|", d)
        other => Expr::OtherOp(crate::sym::Sym::new(other), each()?),
    })
}

/// A `["tuple", a, b]`-shaped 3-element array (head + two entries).
fn tuple3<'a>(v: Option<&'a Value>, what: &str) -> Result<&'a Vec<Value>, String> {
    let arr = v.and_then(Value::as_array).ok_or_else(|| what.to_string())?;
    if arr.len() < 3 {
        return Err(format!("{what}: expected 3 elements"));
    }
    Ok(arr)
}

fn seq_kind(name: &str) -> Option<SeqKind> {
    Some(match name {
        "tuple" => SeqKind::Tuple,
        "array" => SeqKind::Array,
        "list" => SeqKind::List,
        "set" => SeqKind::Set,
        "vector" => SeqKind::Vector,
        "altvector" => SeqKind::AltVector,
        _ => return None,
    })
}

fn rel_op(name: &str) -> Option<RelOp> {
    Some(match name {
        "=" => RelOp::Eq,
        "ne" => RelOp::Ne,
        "<" => RelOp::Lt,
        ">" => RelOp::Gt,
        "le" => RelOp::Le,
        "ge" => RelOp::Ge,
        "in" => RelOp::In,
        "notin" => RelOp::NotIn,
        "ni" => RelOp::Ni,
        "notni" => RelOp::NotNi,
        "subset" => RelOp::Subset,
        "notsubset" => RelOp::NotSubset,
        "subseteq" => RelOp::SubsetEq,
        "notsubseteq" => RelOp::NotSubsetEq,
        "superset" => RelOp::Superset,
        "notsuperset" => RelOp::NotSuperset,
        "superseteq" => RelOp::SupersetEq,
        "notsuperseteq" => RelOp::NotSupersetEq,
        _ => return None,
    })
}

pub fn to_js(expr: &Expr) -> Value {
    match expr {
        Expr::Num(n) => number_to_js(n),
        // Serialized as its `rootof(p(t), k)` application; deserialization
        // re-canonicalizes that back into the leaf.
        Expr::RootOf { poly, index } => to_js(&crate::rootof::as_apply(poly, *index)),
        Expr::Sym(s) => Value::String(s.name()),
        Expr::Blank => Value::String("\u{ff3f}".to_string()),
        Expr::Ldots => json!(["ldots"]),
        Expr::Const(c) => match c {
            crate::expr::MathConst::Inf => json!({"$": "Inf"}),
            crate::expr::MathConst::NegInf => json!({"$": "-Inf"}),
            crate::expr::MathConst::NaN => json!({"$": "NaN"}),
            crate::expr::MathConst::Pi => Value::String("pi".to_string()),
            crate::expr::MathConst::E => Value::String("e".to_string()),
            crate::expr::MathConst::I => Value::String("i".to_string()),
        },

        Expr::Add(args) => op("+", args),
        Expr::Mul(args) => op("*", args),
        Expr::Div(a, b) => json!(["/", to_js(a), to_js(b)]),
        Expr::Pow(a, b) => json!(["^", to_js(a), to_js(b)]),
        Expr::Neg(a) => json!(["-", to_js(a)]),

        Expr::And(args) => op("and", args),
        Expr::Or(args) => op("or", args),
        Expr::Not(a) => json!(["not", to_js(a)]),
        Expr::Union(args) => op("union", args),
        Expr::Intersect(args) => op("intersect", args),

        Expr::Apply(head, args) => {
            // JS applies take exactly one argument; multiple args are a tuple.
            let arg = if args.len() == 1 {
                to_js(&args[0])
            } else {
                op("tuple", args)
            };
            json!(["apply", to_js(head), arg])
        }

        Expr::Prime(a) => json!(["prime", to_js(a)]),
        Expr::Index(a, b) => json!(["_", to_js(a), to_js(b)]),

        Expr::Seq(kind, args) => op(kind.js_name(), args),

        Expr::Interval { endpoints, closed } => json!([
            "interval",
            ["tuple", to_js(&endpoints.0), to_js(&endpoints.1)],
            ["tuple", closed.0, closed.1]
        ]),

        Expr::Relation { operands, ops } => relation_to_js(operands, ops),

        Expr::Matrix {
            rows,
            cols,
            entries,
        } => {
            // ["matrix", ["tuple", rows, cols], ["tuple", <row-tuples>]]
            let ncols = *cols as usize;
            let mut body = vec![Value::String("tuple".to_string())];
            for r in 0..*rows as usize {
                let mut row = vec![Value::String("tuple".to_string())];
                for c in 0..ncols {
                    row.push(to_js(&entries[r * ncols + c]));
                }
                body.push(Value::Array(row));
            }
            json!(["matrix", ["tuple", rows, cols], Value::Array(body)])
        }

        Expr::OtherOp(name, args) => {
            let mut v = vec![Value::String(name.name())];
            v.extend(args.iter().map(to_js));
            Value::Array(v)
        }
    }
}

fn op(name: &str, args: &[Expr]) -> Value {
    let mut v = vec![Value::String(name.to_string())];
    v.extend(args.iter().map(to_js));
    Value::Array(v)
}

fn number_to_js(n: &Number) -> Value {
    match n {
        Number::Int(i) => json!(i),
        // Exact rationals (§3a) and Big numbers project to the nearest f64 —
        // what the JS trees actually hold — so the tree fixtures and the
        // differential harness stay meaningful.
        Number::Float(_) | Number::Rat(..) | Number::Big(_) => f64_to_js(n.to_f64()),
    }
}

/// Serialise an f64 the way a JS `Tree` holds a number: integral values as
/// ints (`JSON.stringify(3.0) === "3"`), non-finite as the `{"$": ...}`
/// specials the fixture extraction uses (JSON has no infinity/NaN).
fn f64_to_js(v: f64) -> Value {
    if v.is_nan() {
        return json!({ "$": "NaN" });
    }
    if v.is_infinite() {
        return json!({ "$": if v > 0.0 { "Inf" } else { "-Inf" } });
    }
    if v.fract() == 0.0 && v.abs() < 9e15 {
        json!(v as i64)
    } else {
        json!(v)
    }
}

fn relation_to_js(operands: &[Expr], ops: &[RelOp]) -> Value {
    if ops.len() == 1 {
        return json!([ops[0].js_name(), to_js(&operands[0]), to_js(&operands[1])]);
    }
    if ops.iter().all(|o| *o == RelOp::Eq) {
        // Chained equality: ["=", a, b, c, ...]
        let mut v = vec![Value::String("=".to_string())];
        v.extend(operands.iter().map(to_js));
        return Value::Array(v);
    }
    // Chained inequalities: ["lts"/"gts", ["tuple", ...operands],
    // ["tuple", ...strict-flags]] where strict means < or > (not <=/>=).
    let (head, strict_op) = if ops.iter().all(|o| matches!(o, RelOp::Lt | RelOp::Le)) {
        ("lts", RelOp::Lt)
    } else if ops.iter().all(|o| matches!(o, RelOp::Gt | RelOp::Ge)) {
        ("gts", RelOp::Gt)
    } else {
        unreachable!("parser nests mixed-direction relation chains");
    };
    let mut args = vec![Value::String("tuple".to_string())];
    args.extend(operands.iter().map(to_js));
    let mut strict = vec![Value::String("tuple".to_string())];
    strict.extend(ops.iter().map(|o| Value::Bool(*o == strict_op)));
    json!([head, args, strict])
}
