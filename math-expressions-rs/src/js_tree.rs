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
pub fn from_js(value: &Value) -> Expr {
    match value {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Expr::Num(Number::Int(i))
            } else {
                Expr::Num(Number::from_f64(n.as_f64().unwrap()))
            }
        }
        Value::String(s) => Expr::sym(s),
        Value::Object(_) => match value.get("$").and_then(Value::as_str) {
            Some("Inf") => Expr::Const(MathConst::Inf),
            Some("-Inf") => Expr::Const(MathConst::NegInf),
            Some("NaN") => Expr::Const(MathConst::NaN),
            other => panic!("from_js: unknown special {:?}", other),
        },
        Value::Array(arr) => from_js_array(arr),
        _ => panic!("from_js: unexpected value {value}"),
    }
}

fn from_js_array(arr: &[Value]) -> Expr {
    let head = arr[0].as_str().unwrap_or("");
    let operands = &arr[1..];
    let each = || operands.iter().map(from_js).collect::<Vec<_>>();
    let boxed = |i: usize| Box::new(from_js(&operands[i]));

    if let Some(kind) = seq_kind(head) {
        return Expr::Seq(kind, each());
    }
    if let Some(op) = rel_op(head) {
        // binary or chained-equality relation
        let operands = each();
        let ops = vec![op; operands.len() - 1];
        return Expr::Relation { operands, ops };
    }

    match head {
        "+" => Expr::Add(each()),
        "*" => Expr::Mul(each()),
        "/" => Expr::Div(boxed(0), boxed(1)),
        "^" => Expr::Pow(boxed(0), boxed(1)),
        "-" => Expr::Neg(boxed(0)),
        "and" => Expr::And(each()),
        "or" => Expr::Or(each()),
        "not" => Expr::Not(boxed(0)),
        "union" => Expr::Union(each()),
        "intersect" => Expr::Intersect(each()),
        "prime" => Expr::Prime(boxed(0)),
        "_" => Expr::Index(boxed(0), boxed(1)),
        "ldots" => Expr::Ldots,
        "apply" => {
            let head = boxed(0);
            let arg = &operands[1];
            let args = match arg.as_array() {
                Some(a) if a.first().and_then(Value::as_str) == Some("tuple") => {
                    a[1..].iter().map(from_js).collect()
                }
                _ => vec![from_js(arg)],
            };
            Expr::Apply(head, args)
        }
        "interval" => {
            let ep = operands[0].as_array().expect("interval endpoints");
            let cl = operands[1].as_array().expect("interval closed");
            Expr::Interval {
                endpoints: Box::new((from_js(&ep[1]), from_js(&ep[2]))),
                closed: (
                    cl[1].as_bool().unwrap_or(false),
                    cl[2].as_bool().unwrap_or(false),
                ),
            }
        }
        "lts" | "gts" => {
            let args = operands[0].as_array().expect("lts/gts args");
            let strict = operands[1].as_array().expect("lts/gts strict");
            let operands: Vec<Expr> = args[1..].iter().map(from_js).collect();
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
            let size = operands[0].as_array().expect("matrix size");
            let body = operands[1].as_array().expect("matrix body");
            let rows = size[1].as_u64().unwrap() as u32;
            let cols = size[2].as_u64().unwrap() as u32;
            let mut entries = Vec::with_capacity((rows * cols) as usize);
            for r in 0..rows as usize {
                let row = body[r + 1].as_array().expect("matrix row");
                for c in 0..cols as usize {
                    entries.push(from_js(&row[c + 1]));
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
        other => Expr::OtherOp(crate::sym::Sym::new(other), each()),
    }
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
        Number::Float(f) => {
            let v = f.get();
            // JSON.stringify(3.0) === "3": integral floats serialise as ints.
            if v.fract() == 0.0 && v.is_finite() && v.abs() < 9e15 {
                json!(v as i64)
            } else {
                json!(v)
            }
        }
        Number::Rat(num, den) => json!(["/", num, den]),
        Number::Big(_) => unimplemented!("Big numbers are not produced by the parser"),
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
