//! Template matching on JS trees — the port of `me.utils.match`
//! (`lib/trees/basic.js` `match`) in its **default mode**, which is the only
//! mode Doenet uses (`match(tree, template)` with no params):
//!
//! - operators and numbers must match exactly;
//! - every variable (string leaf) appearing in the pattern is a wildcard
//!   bound to a subtree;
//! - repeated wildcards must bind syntactically equal subtrees;
//! - for associative operators (`+ * and or union intersect`) and
//!   tuple/vector shapes, tree operands are flattened and a pattern wildcard
//!   may absorb a *group* of consecutive operands (rewrapped in the
//!   operator), with the last pattern operand absorbing the remainder;
//! - a unary minus of a product matches a `*` pattern with the minus moved
//!   onto the first factor (the JS special case).
//!
//! Not ported (never used by Doenet, all opt-in params in JS):
//! `allow_permutations`, `allow_extended_match`, `allow_implicit_identities`,
//! regex/function wildcard conditions. Binding consistency uses structural
//! JSON equality where the JS uses its syntactic `equal` — stricter in
//! corner cases (e.g. `1` vs `1.0` differ only in JS number spelling, which
//! JSON round-tripping already collapses).
//!
//! Operates on `serde_json::Value` JS trees (not `Expr`): Doenet passes raw
//! ASTs and consumes raw subtree bindings, and converting through the
//! canonical layer would change the trees being matched.

use serde_json::{Map, Value};
use std::collections::HashSet;

/// Is this operator associative in the JS tree sense (`flatten.is_associative`)?
fn is_associative(op: &str) -> bool {
    matches!(op, "+" | "*" | "and" | "or" | "union" | "intersect")
}

/// May a wildcard absorb a group of operands under this operator?
/// (JS: associative operators plus tuple/vector shapes.)
fn allows_groups(op: &str) -> bool {
    is_associative(op) || matches!(op, "tuple" | "vector" | "altvector")
}

fn head(tree: &Value) -> Option<&str> {
    tree.as_array()?.first()?.as_str()
}

/// All operands of `tree` as though nested same-operator applications had
/// been flattened (JS `flatten.allChildren`).
fn all_children<'a>(tree: &'a Value, out: &mut Vec<&'a Value>) {
    let Some(arr) = tree.as_array() else { return };
    let Some(op) = arr.first().and_then(Value::as_str) else {
        return;
    };
    for operand in &arr[1..] {
        if is_associative(op) && head(operand) == Some(op) {
            all_children(operand, out);
        } else {
            out.push(operand);
        }
    }
}

/// Collect the wildcard names of a pattern: every distinct string leaf in
/// operand position (mirrors JS `variables_in(pattern)`, which drops
/// operators and `apply` heads).
fn pattern_variables(pattern: &Value, out: &mut HashSet<String>) {
    match pattern {
        Value::String(s) => {
            out.insert(s.clone());
        }
        Value::Array(arr) => {
            let is_apply = arr.first().and_then(Value::as_str) == Some("apply");
            for (i, operand) in arr.iter().enumerate().skip(1) {
                // The function name of an `apply` is not a variable.
                if is_apply && i == 1 && operand.is_string() {
                    continue;
                }
                pattern_variables(operand, out);
            }
        }
        _ => {}
    }
}

/// Attempt to match `tree` against `pattern` (default mode — see module
/// docs). `Some(bindings)` maps each pattern wildcard to the subtree it
/// bound; `None` means no match. An exact variable-free match yields an
/// empty map.
pub fn match_template(tree: &Value, pattern: &Value) -> Option<Map<String, Value>> {
    let mut wildcards = HashSet::new();
    pattern_variables(pattern, &mut wildcards);
    match_inner(tree, pattern, &wildcards)
}

fn match_inner(
    tree: &Value,
    pattern: &Value,
    wildcards: &HashSet<String>,
) -> Option<Map<String, Value>> {
    // A wildcard binds the whole tree.
    if let Value::String(name) = pattern {
        if wildcards.contains(name) {
            let mut m = Map::new();
            m.insert(name.clone(), tree.clone());
            return Some(m);
        }
    }

    // Non-array pattern with no binding: leaves must be identical.
    // (Numbers compare as JSON values; `1` vs `1.0` both parse to the same
    // f64 and serde_json preserves the distinction only in spelling.)
    let Value::Array(parr) = pattern else {
        return leaf_eq(tree, pattern).then(Map::new);
    };
    let op = parr.first()?.as_str()?;
    let pattern_operands = &parr[1..];

    let mut tree_operands: Vec<&Value> = Vec::new();
    let matches_shape = head(tree) == Some(op);
    if matches_shape {
        all_children(tree, &mut tree_operands);
    }

    // JS special case: a `*` pattern also matches `-(a·b·…)`, with the
    // minus moved onto the first factor.
    let mut neg_first: Option<Value> = None;
    if (!matches_shape || tree_operands.len() < pattern_operands.len()) && op == "*" {
        if let Some(arr) = tree.as_array() {
            if arr.len() == 2 && arr[0].as_str() == Some("-") && head(&arr[1]) == Some("*") {
                tree_operands.clear();
                all_children(&arr[1], &mut tree_operands);
                // A degenerate nullary product `["*"]` leaves no factors to
                // carry the minus; leave `neg_first` unset so the `None` path
                // below is taken instead of indexing an empty vec (an abort
                // under `panic = "abort"` — `match_template` is `pub` and runs
                // on raw caller-supplied JS trees).
                if !tree_operands.is_empty() {
                    neg_first = Some(Value::Array(vec![
                        Value::String("-".to_string()),
                        tree_operands[0].clone(),
                    ]));
                }
            }
        }
    }
    if neg_first.is_none() && (!matches_shape || tree_operands.len() < pattern_operands.len()) {
        return None;
    }
    let owned_first = neg_first;
    let operand_at = |i: usize| -> &Value {
        match (&owned_first, i) {
            (Some(v), 0) => v,
            _ => tree_operands[i],
        }
    };

    match_operands(op, &tree_operands, operand_at, pattern_operands, wildcards)
}

/// Sequential operand matching with grouping (the JS default path of
/// `matchOperands`): pattern operand `i` tries absorbing 1..=max_group
/// consecutive tree operands (max_group > 1 only for group-allowing
/// operators); the last pattern operand must absorb the remainder exactly.
fn match_operands<'a>(
    op: &str,
    tree_operands: &[&'a Value],
    operand_at: impl Fn(usize) -> &'a Value + Copy,
    pattern_operands: &[Value],
    wildcards: &HashSet<String>,
) -> Option<Map<String, Value>> {
    fn chunk<'a>(
        op: &str,
        operand_at: impl Fn(usize) -> &'a Value,
        start: usize,
        len: usize,
    ) -> Value {
        if len == 1 {
            operand_at(start).clone()
        } else {
            let mut arr = vec![Value::String(op.to_string())];
            arr.extend((start..start + len).map(|i| operand_at(i).clone()));
            Value::Array(arr)
        }
    }

    fn consistent(a: &Map<String, Value>, b: &Map<String, Value>) -> bool {
        a.iter().all(|(k, v)| b.get(k).is_none_or(|w| v == w))
    }

    #[allow(clippy::too_many_arguments)]
    fn go<'a>(
        op: &str,
        n_tree: usize,
        operand_at: impl Fn(usize) -> &'a Value + Copy,
        pattern_operands: &[Value],
        wildcards: &HashSet<String>,
        start: usize,
        pat_ind: usize,
        acc: &Map<String, Value>,
    ) -> Option<Map<String, Value>> {
        let n_pats = pattern_operands.len();
        let remaining = n_tree - start;
        if pat_ind == n_pats {
            return (remaining == 0).then(|| acc.clone());
        }
        let last = pat_ind == n_pats - 1;
        let max_group = if allows_groups(op) {
            remaining.saturating_sub(n_pats - pat_ind - 1)
        } else {
            1
        };
        // The last pattern operand must absorb everything left (JS: no
        // extended match). For non-group operators that means exactly one.
        let sizes: Vec<usize> = if last {
            (remaining == max_group.max(1) && remaining >= 1)
                .then_some(remaining)
                .into_iter()
                .collect()
        } else {
            (1..=max_group).collect()
        };
        for size in sizes {
            let piece = chunk(op, operand_at, start, size);
            let Some(m) = match_inner(&piece, &pattern_operands[pat_ind], wildcards) else {
                continue;
            };
            if !consistent(&m, acc) {
                continue;
            }
            let mut combined = acc.clone();
            combined.extend(m);
            if let Some(result) = go(
                op,
                n_tree,
                operand_at,
                pattern_operands,
                wildcards,
                start + size,
                pat_ind + 1,
                &combined,
            ) {
                return Some(result);
            }
        }
        None
    }

    go(
        op,
        tree_operands.len(),
        operand_at,
        pattern_operands,
        wildcards,
        0,
        0,
        &Map::new(),
    )
}

/// Leaf equality: strings by identity, numbers by numeric value, booleans by
/// value (JS `tree === pattern`).
fn leaf_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        _ => a == b,
    }
}

// ---- JS-tree shape utilities (ports of `me.utils.flatten`/`unflatten*`) ----

/// Flatten nested associative operators: `["+", ["+", a, b], c] → ["+", a, b, c]`.
pub fn flatten_tree(tree: &Value) -> Value {
    let Some(arr) = tree.as_array() else {
        return tree.clone();
    };
    let Some(op) = arr.first().and_then(Value::as_str) else {
        return tree.clone();
    };
    if is_associative(op) {
        let mut operands = Vec::new();
        all_children(tree, &mut operands);
        let mut out = vec![Value::String(op.to_string())];
        out.extend(operands.iter().map(|o| flatten_tree(o)));
        Value::Array(out)
    } else {
        let mut out = vec![arr[0].clone()];
        out.extend(arr[1..].iter().map(flatten_tree));
        Value::Array(out)
    }
}

/// Left-associate an n-ary associative operator:
/// `["+", a, b, c] → ["+", ["+", a, b], c]`.
pub fn unflatten_left(tree: &Value) -> Value {
    unflatten(tree, true)
}

/// Right-associate: `["+", a, b, c] → ["+", a, ["+", b, c]]`.
pub fn unflatten_right(tree: &Value) -> Value {
    unflatten(tree, false)
}

fn unflatten(tree: &Value, left: bool) -> Value {
    let Some(arr) = tree.as_array() else {
        return tree.clone();
    };
    let Some(op) = arr.first().and_then(Value::as_str) else {
        return tree.clone();
    };
    let operands: Vec<Value> = arr[1..].iter().map(|o| unflatten(o, left)).collect();
    if !is_associative(op) || operands.len() <= 2 {
        let mut out = vec![arr[0].clone()];
        out.extend(operands);
        return Value::Array(out);
    }
    let wrap = |a: Value, b: Value| Value::Array(vec![Value::String(op.to_string()), a, b]);
    let mut iter = operands.into_iter();
    if left {
        let first = iter.next().unwrap();
        iter.fold(first, wrap)
    } else {
        let all: Vec<Value> = iter.collect();
        let mut rev = all.into_iter().rev();
        let last = rev.next().unwrap();
        rev.fold(last, |acc, x| wrap(x, acc))
    }
}
