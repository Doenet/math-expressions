//! Read-only inspection of an expression: the applied function names, operator
//! heads, and free variables it contains. Component access lives next door in
//! [`components`](super::components).

use crate::expr::Expr;
use std::collections::HashSet;

/// The applied function names in `e`, first-appearance order, de-duplicated —
/// the port of `me.functions` (`sin(x)+f(y)` → `["sin","f"]`). Only bare-Sym
/// application heads count (a `Pow` head like `sin^2` contributes its inner
/// name via the canonical faithful tree's head structure being Sym-rooted).
pub fn functions(e: &Expr) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    fn walk(e: &Expr, out: &mut Vec<String>, seen: &mut HashSet<String>) {
        if let Expr::Apply(head, _) = e {
            // Dig a bare name out of the head (`sin`, and `sin` inside `sin^2`).
            fn head_name(h: &Expr) -> Option<String> {
                match h {
                    Expr::Sym(s) => Some(s.name()),
                    Expr::Pow(b, _) => head_name(b),
                    Expr::Prime(x) => head_name(x),
                    _ => None,
                }
            }
            if let Some(name) = head_name(head) {
                if seen.insert(name.clone()) {
                    out.push(name);
                }
            }
        }
        for c in e.children() {
            walk(c, out, seen);
        }
    }
    walk(e, &mut out, &mut seen);
    out
}

/// The operator heads used in `e`, first-appearance order, de-duplicated, in
/// the JS tree's spelling (`+`, `*`, `^`, `tuple`, `interval`, `<`, …) — the
/// port of `me.operators`.
///
/// Every node that is an *array* in the JS tree contributes its head, so the
/// answer is a whitelist test's input, not a summary: a caller asking "is this
/// built only from `+ - * / ^`" needs `tuple` and `interval` to come back, or
/// it accepts trees it means to reject. `apply` is the one head JS drops, and
/// it drops the application's *head* subtree with it (`sin²(x)` contributes no
/// `^`), so both are reproduced here.
pub fn operators(e: &Expr) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    walk_operators(e, &mut out, &mut seen);
    out
}

fn walk_operators(e: &Expr, out: &mut Vec<String>, seen: &mut HashSet<String>) {
    let Some(head) = js_head(e) else {
        return; // a leaf: `Array.isArray(tree)` is false, so JS reports nothing
    };
    if head != "apply" && seen.insert(head.clone()) {
        out.push(head);
    }
    // `js_operands` reconstructs the operand list of the JS tree — which is why
    // an interval's implicit `["tuple", …]` pair and a multi-argument call's
    // argument tuple show up, as they do in JS. An application's head is the one
    // operand JS skips (`tree.slice(2)`).
    let operands = crate::normalize::js_operands(e);
    let operands = match e {
        Expr::Apply(..) => &operands[1..],
        _ => &operands[..],
    };
    for c in operands {
        walk_operators(c, out, seen);
    }
}

/// The head this node carries in the JS tree, or `None` for a leaf.
fn js_head(e: &Expr) -> Option<String> {
    match e {
        Expr::Num(_) | Expr::Sym(_) | Expr::Bool(_) | Expr::Blank => None,
        // `pi`/`e`/`i` are bare strings; `Inf`/`NaN` are tagged *objects*.
        // Neither is an array, so neither carries an operator.
        Expr::Const(_) => None,
        // No JS spelling of its own — it serializes as a `rootof(…)` call.
        Expr::RootOf { .. } => Some("apply".to_string()),
        other => Some(crate::normalize::legacy_operator(other)),
    }
}

/// The free variable names of `e`, in first-appearance order, de-duplicated.
/// Matches `me.variables`: the constant symbols `pi`/`e`/`i` ARE included (they
/// are ordinary symbols here), but a function-application head (`sin` in
/// `sin(x)`, `f` in `f(x)`) is NOT.
///
/// That the constants are listed looks wrong and is not: alpha94's filter reads
/// `(math.define_e || v !== "e")`, which *keeps* `e` precisely when `define_e`
/// is on. The listing is not the crate's constant/variable distinction — that
/// is [`crate::expr::sym::is_constant_symbol`], which every pass that reduces
/// or sample-evaluates an expression consults, and which
/// [`crate::constant_policy`] governs. `variables` reports the names a tree
/// mentions, and it reports them whatever the policy says.
///
/// Only the `Const` *spelling* is excluded, since `∞`/`NaN`/`None` have no name
/// to report and the three that do are unified into symbols by `canonicalize`
/// long before a caller asks.
pub fn variables(e: &Expr) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    collect(e, &mut out, &mut seen);
    out
}

fn collect(e: &Expr, out: &mut Vec<String>, seen: &mut HashSet<String>) {
    match e {
        Expr::Sym(s) => {
            let name = s.name();
            if seen.insert(name.clone()) {
                out.push(name);
            }
        }
        Expr::Num(_)
        | Expr::Const(_)
        | Expr::Bool(_)
        | Expr::RootOf { .. }
        | Expr::Blank
        | Expr::Ldots => {}

        // An application head is never a variable source — JS drops the head
        // wholesale (`tree.slice(2)` in lib/expression/variables.js), even a
        // compound one like `f'` or `sin^2` — so `f'(x)` has variables `[x]`,
        // not `[f, x]`.
        Expr::Apply(_, args) => {
            for a in args {
                collect(a, out, seen);
            }
        }

        Expr::Add(xs)
        | Expr::Mul(xs)
        | Expr::And(xs)
        | Expr::Or(xs)
        | Expr::Union(xs)
        | Expr::Intersect(xs)
        | Expr::Seq(_, xs)
        | Expr::OtherOp(_, xs) => {
            for c in xs {
                collect(c, out, seen);
            }
        }
        Expr::Div(a, b) | Expr::Pow(a, b) | Expr::Index(a, b) => {
            collect(a, out, seen);
            collect(b, out, seen);
        }
        Expr::Neg(x) | Expr::Not(x) | Expr::Prime(x) => collect(x, out, seen),
        Expr::Interval { endpoints, .. } => {
            collect(&endpoints.0, out, seen);
            collect(&endpoints.1, out, seen);
        }
        Expr::Relation { operands, .. } => {
            for c in operands {
                collect(c, out, seen);
            }
        }
        Expr::Matrix(m) => {
            for c in m.entries() {
                collect(c, out, seen);
            }
        }
    }
}
