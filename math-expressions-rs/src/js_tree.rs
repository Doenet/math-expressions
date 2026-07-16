//! Converter to the JS `Tree` JSON shape (PORTING_PLAN.md §5, "JS tree
//! interop"). ALL the ad-hoc JS encodings live here: parallel bool-tuples
//! for chained inequalities, boolean interval-closure leaves, the "＿" blank
//! symbol, single-arg apply with tuple wrapping.
//!
//! Infinity/NaN cannot be represented in JSON; they are encoded as
//! {"$": "Inf"} / {"$": "-Inf"} / {"$": "NaN"}, matching the fixture
//! extraction script (a JS Tree never contains plain objects, so this is
//! unambiguous).

use crate::expr::{Expr, RelOp};
use crate::num::Number;
use serde_json::{json, Value};

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
