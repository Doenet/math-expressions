//! Converter to and from the JS `Tree` JSON shape. ALL the ad-hoc JS
//! encodings live here: parallel bool-tuples for chained inequalities, boolean
//! interval-closure leaves, the "＿" blank symbol, single-arg apply with tuple
//! wrapping.
//!
//! Infinity/NaN cannot be represented in JSON; they are encoded as
//! {"$": "Inf"} / {"$": "-Inf"} / {"$": "NaN"}, matching the fixture
//! extraction script (a JS Tree never contains plain objects, so this is
//! unambiguous).

use crate::expr::{Expr, MathConst, RelOp, SeqKind};
use crate::num::Number;
use serde_json::{json, Value};

// Recursion is deliberately not depth-capped here: the realistic input path is
// a JSON string deserialized by `serde_json`, whose own recursion limit (128)
// rejects deeply-nested input before a `Value` is built, so this never sees a
// tree deep enough to overflow. A hand-constructed `Value` could, but that is
// not a user-input vector.
/// Parse a JS `Tree` JSON value into an `Expr`. Inverse of [`to_js`] for the
/// tree shapes the parsers produce (Rat is not reconstructed — a `["/", a, b]`
/// node becomes `Div`, matching the parser). Malformed shapes return an `Err`
/// description and never panic (wasm builds abort on panic, so a bad tree from
/// JS must not unwind; trusted callers such as test fixtures just `.expect()`).
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
        Value::Bool(b) => Ok(Expr::Bool(*b)),
        Value::String(s) => Ok(Expr::sym(s)),
        Value::Object(_) => match value.get("$").and_then(Value::as_str) {
            Some("Inf") => Ok(Expr::Const(MathConst::Inf)),
            Some("-Inf") => Ok(Expr::Const(MathConst::NegInf)),
            Some("NaN") => Ok(Expr::Const(MathConst::NaN)),
            Some("None") => Ok(Expr::Const(MathConst::None)),
            // Report the two failures apart. `{"$":"None"}` is a *valid* tree
            // (matched above), so an object with no usable `$` must not be
            // described with the word `None` — that is the `Option::None` of the
            // lookup leaking into the message, and it read as though a legal
            // input had been rejected. It cost the DoenetML team a debugging
            // cycle; the fix is naming what was actually wrong.
            Some(tag) => Err(format!(
                "unknown special {tag:?} (expected \"Inf\", \"-Inf\", \"NaN\" or \"None\")"
            )),
            None => Err(
                "object is not a tree node: expected a `$` string tag such as {\"$\":\"NaN\"}"
                    .into(),
            ),
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
            if args.len() < 2 || strict.len() != args.len() - 1 {
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
            // `Mat::new` re-checks the entry count that the loop above just
            // built, so the shape is validated by the type rather than by this
            // function getting the loop right.
            Expr::Matrix(crate::expr::Mat::new(rows, cols, entries).ok_or("matrix shape mismatch")?)
        }
        // everything else (unit, pm, angle, binom, vec, linesegment,
        // derivative_leibniz, forall, arrows, implies, iff, perp, ":", "|", d)
        other => Expr::OtherOp(crate::expr::sym::Sym::new(other), each()?),
    })
}

/// A `["tuple", a, b]`-shaped 3-element array (head + two entries).
fn tuple3<'a>(v: Option<&'a Value>, what: &str) -> Result<&'a Vec<Value>, String> {
    let arr = v
        .and_then(Value::as_array)
        .ok_or_else(|| what.to_string())?;
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

/// Serialize an `Expr` to the JS `Tree` JSON shape. Flattens first: parsing is
/// now faithful (keeps raw associative grouping), but the JS reference AST is
/// flat (`["+", a, b, c]`, not `["+", ["+", a, b], c]`), so consumers such as
/// the wasm `tree_json` stay JS-compatible. Idempotent on already-flat trees.
pub fn to_js(expr: &Expr) -> Value {
    to_js_rec(&crate::expr::flatten(expr.clone()))
}

fn to_js_rec(expr: &Expr) -> Value {
    match expr {
        Expr::Num(n) => number_to_js(n),
        // Serialized as its `rootof(p(t), k)` application; deserialization
        // re-canonicalizes that back into the leaf.
        Expr::RootOf { poly, index } => {
            to_js_rec(&crate::polynomials::rootof::as_apply(poly, *index))
        }
        Expr::Sym(s) => Value::String(s.name()),
        Expr::Bool(b) => Value::Bool(*b),
        Expr::Blank => Value::String("\u{ff3f}".to_string()),
        Expr::Ldots => json!(["ldots"]),
        Expr::Const(c) => match c {
            crate::expr::MathConst::Inf => json!({"$": "Inf"}),
            crate::expr::MathConst::NegInf => json!({"$": "-Inf"}),
            crate::expr::MathConst::NaN => json!({"$": "NaN"}),
            crate::expr::MathConst::None => json!({"$": "None"}),
            crate::expr::MathConst::Pi => Value::String("pi".to_string()),
            crate::expr::MathConst::E => Value::String("e".to_string()),
            crate::expr::MathConst::I => Value::String("i".to_string()),
        },

        Expr::Add(args) => op("+", args),
        Expr::Mul(args) => op("*", args),
        Expr::Div(a, b) => json!(["/", to_js_rec(a), to_js_rec(b)]),
        Expr::Pow(a, b) => json!(["^", to_js_rec(a), to_js_rec(b)]),
        Expr::Neg(a) => json!(["-", to_js_rec(a)]),

        Expr::And(args) => op("and", args),
        Expr::Or(args) => op("or", args),
        Expr::Not(a) => json!(["not", to_js_rec(a)]),
        Expr::Union(args) => op("union", args),
        Expr::Intersect(args) => op("intersect", args),

        Expr::Apply(head, args) => {
            // JS applies take exactly one argument; multiple args are a tuple.
            let arg = if args.len() == 1 {
                to_js_rec(&args[0])
            } else {
                op("tuple", args)
            };
            json!(["apply", to_js_rec(head), arg])
        }

        Expr::Prime(a) => json!(["prime", to_js_rec(a)]),
        Expr::Index(a, b) => json!(["_", to_js_rec(a), to_js_rec(b)]),

        Expr::Seq(kind, args) => op(kind.js_name(), args),

        Expr::Interval { endpoints, closed } => json!([
            "interval",
            ["tuple", to_js_rec(&endpoints.0), to_js_rec(&endpoints.1)],
            ["tuple", closed.0, closed.1]
        ]),

        Expr::Relation { operands, ops } => relation_to_js(operands, ops),

        Expr::Matrix(m) => {
            // ["matrix", ["tuple", rows, cols], ["tuple", <row-tuples>]]
            // Indexing `entries` is in bounds for every `r < rows`, `c < cols`
            // by `Mat`'s invariant.
            let ncols = m.cols() as usize;
            let entries = m.entries();
            let mut body = vec![Value::String("tuple".to_string())];
            for r in 0..m.rows() as usize {
                let mut row = vec![Value::String("tuple".to_string())];
                for c in 0..ncols {
                    row.push(to_js_rec(&entries[r * ncols + c]));
                }
                body.push(Value::Array(row));
            }
            json!(["matrix", ["tuple", m.rows(), m.cols()], Value::Array(body)])
        }

        Expr::OtherOp(name, args) => {
            let mut v = vec![Value::String(name.name())];
            v.extend(args.iter().map(to_js_rec));
            Value::Array(v)
        }
    }
}

fn op(name: &str, args: &[Expr]) -> Value {
    let mut v = vec![Value::String(name.to_string())];
    v.extend(args.iter().map(to_js_rec));
    Value::Array(v)
}

fn number_to_js(n: &Number) -> Value {
    match n {
        Number::Int(i) => json!(i),
        // −0 serializes as plain `0` (JSON has no exact negative zero, and it is
        // value-equal to `0` anyway).
        //
        // This is what keeps the sign *inside* the engine: `round_to_decimals`
        // returns `−0` for a small negative value, and `1/(−0)` is `−∞`, but a
        // caller reading `.tree` sees `0` and loses it on the way back in.
        // Emitting `-0.0` would carry it (`JSON.parse("-0.0")` is JS `−0`) and
        // is pinned *against* by `tests/signed_zero.rs`, so it is a decision to
        // revisit deliberately rather than a line to flip.
        Number::NegZero => json!(0),
        Number::Float(_) => f64_to_js(n.to_f64()),
        // Exact rationals split on their recorded `Spelling`.
        //
        // A *decimal*-spelled one keeps its positional spelling when the
        // expansion terminates: user-typed decimals parse to exact rationals,
        // so `19.9` is `Rat(199, 10)` and emitting `["/", 199, 10]` for it
        // would be a wrong answer, not a stylistic one.
        //
        // A *fraction*-spelled one (`3/6`, `cos(pi/3)`) emits `["/", n, d]`.
        // Before the spelling was tracked this branch could only ask whether
        // the expansion terminated, which meant `3/6` crossed as `0.5` and
        // DoenetML's `ReducedFraction`/`ExactValue` criteria could not see a
        // fraction that was no longer there.
        //
        // Either way, a value the JS side cannot hold exactly falls back to the
        // f64 projection.
        Number::Rat(..) | Number::Big(_) => match exact_ratio(n) {
            Some((num, den)) => json!(["/", num, den]),
            None => f64_to_js(n.to_f64()),
        },
    }
}

/// The largest integer a JS number holds exactly (2^53 − 1). Past it a
/// `["/", num, den]` pair is no more recoverable on the JS side than the f64
/// projection is, so there is nothing to gain by emitting it.
const JS_MAX_SAFE_INT: u64 = 9_007_199_254_740_991;

/// Numerator/denominator for a rational that must *not* be decimalized.
/// `None` when the value displays as a decimal (see
/// [`Number::decimal_spelling`]) or when the parts exceed JS's exact-integer
/// range.
///
/// The `Rat` normal form puts the sign on the numerator with `den > 0`, so
/// negatives come out as `["/", -2, 3]` — the spelling the JS fixtures use.
fn exact_ratio(n: &Number) -> Option<(i64, i64)> {
    if n.decimal_spelling().is_some() {
        return None;
    }
    let (num, den) = n.rational_parts()?;
    let num: i64 = num.parse().ok()?;
    let den: i64 = den.parse().ok()?;
    (num.unsigned_abs() <= JS_MAX_SAFE_INT && den.unsigned_abs() <= JS_MAX_SAFE_INT)
        .then_some((num, den))
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
        return json!([
            ops[0].js_name(),
            to_js_rec(&operands[0]),
            to_js_rec(&operands[1])
        ]);
    }
    // Chained </<= and >/>= use the ["lts"/"gts", ["tuple", ...operands],
    // ["tuple", ...strict-flags]] encoding, because a run of them can mix
    // strictness (`a < b <= c`). Checked first, so a uniform `<=` chain still
    // goes out as `lts` rather than as a flat `["le", …]`.
    if ops.iter().all(|o| matches!(o, RelOp::Lt | RelOp::Le)) {
        return chained_inequality_to_js("lts", RelOp::Lt, operands, ops);
    }
    if ops.iter().all(|o| matches!(o, RelOp::Gt | RelOp::Ge)) {
        return chained_inequality_to_js("gts", RelOp::Gt, operands, ops);
    }
    // Any other *uniform* chain — `a = b = c`, `a ≠ b ≠ c`, `x ∈ A ∈ B`,
    // `a ⊆ b ⊆ c` — serializes flat as `[op, ...operands]`, the exact inverse
    // of `rel_op`'s flat `vec![op; n-1]` reconstruction, so it round-trips to
    // the identical tree. `try_from_js` builds precisely these uniform chains,
    // so this branch handles every relation an injected tree can carry.
    if let [first, rest @ ..] = ops {
        if rest.iter().all(|o| o == first) {
            let mut v = vec![Value::String(first.js_name().to_string())];
            v.extend(operands.iter().map(to_js_rec));
            return Value::Array(v);
        }
    }
    // A genuinely mixed-direction chain reaches here. The parser nests such
    // chains and `try_from_js` only ever builds uniform ones, so no input
    // produces this shape — but serialization must never panic (wasm builds are
    // `panic = "abort"`, so a would-be `unreachable!` is an uncatchable worker
    // abort on a `to_serialized` round-trip). Lower it to a conjunction of the
    // adjacent binary relations, which `try_from_js` reads straight back.
    let mut conj = vec![Value::String("and".to_string())];
    for (i, op) in ops.iter().enumerate() {
        conj.push(json!([
            op.js_name(),
            to_js_rec(&operands[i]),
            to_js_rec(&operands[i + 1])
        ]));
    }
    Value::Array(conj)
}

/// A chained inequality in the JS `["lts"/"gts", ["tuple", ...operands],
/// ["tuple", ...strict-flags]]` shape, where a `strict` flag marks `<`/`>`
/// (as opposed to `<=`/`>=`).
fn chained_inequality_to_js(
    head: &str,
    strict_op: RelOp,
    operands: &[Expr],
    ops: &[RelOp],
) -> Value {
    let mut args = vec![Value::String("tuple".to_string())];
    args.extend(operands.iter().map(to_js_rec));
    let mut strict = vec![Value::String("tuple".to_string())];
    strict.extend(ops.iter().map(|o| Value::Bool(*o == strict_op)));
    json!([head, args, strict])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::num::Spelling;

    // A chained inequality `["gts"/"lts", ["tuple", ...operands],
    // ["tuple", ...strict-flags]]` has one more operand than strict-flag, so
    // the tuple-with-head arrays satisfy `strict.len() == args.len() - 1`.
    // Regression: an off-by-one in that check used to reject every chained
    // inequality, panicking `from_js` (the ast-to-{latex,text} formatter path).
    #[test]
    fn chained_inequality_from_js_round_trips() {
        for head in ["gts", "lts"] {
            let tree = json!([head, ["tuple", "x", "y", "z"], ["tuple", true, false]]);
            let expr = try_from_js(&tree).expect("chained inequality should parse");
            let Expr::Relation { operands, ops } = &expr else {
                panic!("expected Relation, got {expr:?}");
            };
            assert_eq!(operands.len(), 3);
            assert_eq!(ops.len(), 2);
            // to_js is the inverse for this shape.
            assert_eq!(to_js_rec(&expr), tree);
        }
    }

    /// A matrix whose declared size outruns its body is rejected with an `Err`
    /// rather than producing a tree that later readers index out of bounds.
    /// `Mat`'s private fields make that structural: there is no way to build
    /// the mis-shaped value in the first place, so the check cannot be skipped
    /// by a future caller that forgets it.
    #[test]
    fn a_matrix_body_smaller_than_its_declared_size_is_an_error() {
        // Declares 2×2, supplies one row of two.
        let short_body = json!(["matrix", ["tuple", 2, 2], ["tuple", ["tuple", 1, 2]]]);
        assert!(try_from_js(&short_body).is_err());
        // Declares 2×2, supplies rows of one.
        let short_rows = json!([
            "matrix",
            ["tuple", 2, 2],
            ["tuple", ["tuple", 1], ["tuple", 3]]
        ]);
        assert!(try_from_js(&short_rows).is_err());
        // The well-formed one still round-trips, and arrives with the shape
        // invariant intact.
        let ok = json!([
            "matrix",
            ["tuple", 2, 2],
            ["tuple", ["tuple", 1, 2], ["tuple", 3, 4]]
        ]);
        let expr = try_from_js(&ok).expect("2x2 is well formed");
        let Expr::Matrix(m) = &expr else {
            panic!("expected a matrix, got {expr:?}")
        };
        assert_eq!(m.entries().len(), (m.rows() * m.cols()) as usize);
        assert_eq!(to_js_rec(&expr), ok);
    }

    /// A flat, uniform chain of any *non-order* relation operator must
    /// serialize without panicking and round-trip to the identical tree. These
    /// are exactly the shapes `try_from_js` builds from `["ne", a, b, c]`,
    /// `["in", x, A, B]`, `["subset", …]`, etc. via `rel_op`'s `vec![op; n-1]`.
    /// Regression: `relation_to_js` used to `unreachable!()` on every one of
    /// them (only `=`, `lts`, `gts` were handled), so a plain
    /// `from_ast(["ne", a, b, c]).to_serialized()` — a routine DoenetML state
    /// save — aborted the whole wasm worker (`panic = "abort"`).
    #[test]
    fn uniform_relation_chains_round_trip_flat() {
        for tree in [
            json!(["ne", "a", "b", "c"]),
            json!(["in", "x", "A", "B"]),
            json!(["ni", "A", "x", "y"]),
            json!(["subset", "a", "b", "c"]),
            json!(["superset", "a", "b", "c"]),
            json!(["subseteq", "a", "b", "c", "d"]),
            json!(["=", "a", "b", "c"]),
        ] {
            let expr = try_from_js(&tree).unwrap_or_else(|e| panic!("{tree}: {e}"));
            assert_eq!(to_js_rec(&expr), tree, "round trip of {tree}");
        }
    }

    /// The order chains keep their dedicated `lts`/`gts` tuple encoding — the
    /// generalization above must not divert a uniform `<=`/`>=` run into the
    /// flat form.
    #[test]
    fn order_relation_chains_keep_the_tuple_encoding() {
        for tree in [
            json!(["lts", ["tuple", "a", "b", "c"], ["tuple", true, false]]),
            json!(["gts", ["tuple", "a", "b", "c"], ["tuple", false, true]]),
        ] {
            let expr = try_from_js(&tree).unwrap_or_else(|e| panic!("{tree}: {e}"));
            assert_eq!(to_js_rec(&expr), tree, "round trip of {tree}");
        }
    }

    /// A rational whose decimal expansion does not terminate crosses to JS as
    /// `["/", num, den]`, not as a truncated f64. `1/3` used to go out as
    /// `0.3333333333333333`, which nothing on the JS side can turn back into a
    /// third — an irreversible loss on every state save/load, not merely a
    /// display defect.
    #[test]
    fn non_terminating_rationals_cross_as_exact_fractions() {
        for (num, den) in [(1, 3), (5, 6), (-2, 3), (-1, 7), (22, 7)] {
            let n = Number::rat(num, den);
            assert_eq!(
                number_to_js(&n),
                json!(["/", num, den]),
                "{num}/{den} must not decimalize"
            );
        }
    }

    /// The other half of the same rule, and the reason the naive "emit every
    /// `Rat` as a fraction" version is wrong: user-typed decimals parse to
    /// exact rationals, so `19.9` *is* `Rat(199, 10)`. A rational carrying the
    /// `Decimal` spelling keeps the positional form the JS trees use, or `19.9`
    /// would go out as `["/", 199, 10]`.
    #[test]
    fn decimal_spelled_rationals_keep_their_positional_form() {
        for (num, den, expected) in [(1, 2, 0.5), (199, 10, 19.9), (-3, 4, -0.75)] {
            assert_eq!(
                number_to_js(&Number::rat_spelled(num, den, Spelling::Decimal)),
                json!(expected),
                "{num}/{den} must stay positional"
            );
        }
    }

    /// The same values with the other spelling. This is the pair that could not
    /// be told apart before `Spelling` existed, and the reason DoenetML's
    /// structural criteria could not see a fraction in `3/6`.
    #[test]
    fn fraction_spelled_rationals_cross_as_fractions_even_when_they_terminate() {
        for (num, den) in [(1, 2), (199, 10), (-3, 4)] {
            assert_eq!(
                number_to_js(&Number::rat(num, den)),
                json!(["/", num, den]),
                "{num}/{den} must stay a fraction"
            );
        }
    }

    /// Past JS's exact-integer range a fraction is no more recoverable than the
    /// f64 projection, so there is nothing to gain by emitting one — and the
    /// pair must not be silently truncated into a *wrong* fraction.
    #[test]
    fn out_of_range_rationals_fall_back_to_the_float_projection() {
        use num_bigint::BigInt;
        use num_rational::BigRational;
        let huge = BigRational::new(BigInt::from(1), BigInt::from(3u8).pow(60));
        let n = Number::from_bigrational(huge);
        assert!(
            number_to_js(&n).is_f64(),
            "an out-of-range denominator should project to a float"
        );
    }

    /// `Tree = number | string | boolean | Tree[]`, so a boolean leaf is a
    /// legal tree. It used to fall through to `Err("unexpected value …")`,
    /// making `["and", true, false]` unconstructible — the whole boolean
    /// algebra existed (`And`/`Or`/`Not`) with no values to put in it.
    #[test]
    fn boolean_leaves_round_trip_through_the_js_tree() {
        for tree in [
            json!(true),
            json!(false),
            json!(["and", true, false]),
            json!(["not", true]),
            json!(["or", ["and", true, "x"], false]),
        ] {
            let expr = try_from_js(&tree).expect("a boolean leaf is a legal tree");
            assert_eq!(to_js_rec(&expr), tree, "round trip of {tree}");
        }
    }

    /// All four `{"$":…}` specials must round-trip. `None` was the odd one out:
    /// `try_from_js` accepted `Inf`/`-Inf`/`NaN` and rejected `{"$":"None"}` with
    /// `unknown special "None"`, so a DoenetML tree carrying a "no value here"
    /// leaf — an undefined polygon vertex, an empty piecewise branch — could not
    /// be revived at all, taking the whole expression down with it.
    #[test]
    fn the_none_special_round_trips_like_the_other_three() {
        for tag in ["Inf", "-Inf", "NaN", "None"] {
            let tree = json!({ "$": tag });
            let expr = try_from_js(&tree).unwrap_or_else(|e| panic!("{tag}: {e}"));
            assert_eq!(to_js_rec(&expr), tree, "round trip of {tag}");
        }
        assert_eq!(
            try_from_js(&json!({"$": "None"})).unwrap(),
            Expr::Const(MathConst::None)
        );
        // A `None` inside a container revives with the container intact, which is
        // the shape DoenetML actually feeds in.
        let nested = json!(["tuple", {"$": "None"}, 2]);
        assert_eq!(to_js_rec(&try_from_js(&nested).unwrap()), nested);
    }

    /// The distinction the whole variant exists for: a boolean must come back
    /// as a JSON boolean, not as the *string* `"true"`. Mapping booleans onto
    /// symbols (or onto `MathConst`, whose members all serialize to strings)
    /// would type-check and still lose the type on every round trip.
    #[test]
    fn a_boolean_is_not_the_symbol_of_the_same_name() {
        assert_eq!(try_from_js(&json!(true)).unwrap(), Expr::Bool(true));
        assert_ne!(try_from_js(&json!(true)).unwrap(), Expr::sym("true"));
        assert_eq!(to_js_rec(&Expr::Bool(true)), json!(true));
        assert_eq!(to_js_rec(&Expr::sym("true")), json!("true"));
    }

    /// Interval closures and chained-inequality strictness are metadata on
    /// `Expr::Interval`/`Expr::Relation`, not `Expr::Bool` children. Adding the
    /// boolean leaf must not divert those flag tuples into it — the flags carry
    /// more than a bool (`("lts", false)` is `Le`, `("gts", false)` is `Ge`),
    /// and the metadata is what makes `operands.len() == ops.len() + 1`
    /// structural rather than a runtime check.
    #[test]
    fn flag_tuples_stay_metadata_and_do_not_become_boolean_children() {
        let interval = try_from_js(&json!([
            "interval",
            ["tuple", 0, 1],
            ["tuple", true, false]
        ]))
        .expect("interval should parse");
        assert!(
            matches!(&interval, Expr::Interval { closed, .. } if *closed == (true, false)),
            "closure belongs in the `closed` field, got {interval:?}"
        );
        assert!(
            !interval.any_subexpr(&|e| matches!(e, Expr::Bool(_))),
            "no boolean child should appear anywhere in {interval:?}"
        );
    }
}
