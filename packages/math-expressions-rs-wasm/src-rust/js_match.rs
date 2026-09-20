//! Template matching on JS trees — the port of `me.utils.match`
//! (`lib/trees/basic.js` `match`). [`match_template`] is the **default mode**
//! (`match(tree, template)` with no params); [`match_template_with_options`]
//! adds the params Doenet passes.
//!
//! Default mode:
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
//! [`MatchOptions`] adds the three params Doenet does use: `variables` (which
//! names are placeholders, and what each may bind), `allow_permutations`, and
//! `allow_implicit_identities`.
//!
//! # Deprecated and wontfix: arbitrary per-parameter conditions
//!
//! Legacy let a caller give any parameter a **predicate function** or a
//! **`RegExp`** as its condition. [`VarKind`] replaces both with a closed
//! vocabulary, and the open forms will not be supported.
//!
//! Not merely because a function cannot cross the wasm boundary — a bridge is
//! buildable. It is that the matcher backtracks, so a predicate would be called
//! back into JS once per *candidate* binding, on a search whose cost is not
//! visible to the caller; the matcher would stop being a pure Rust search and
//! would become uncacheable and untestable in Rust. And the conditions callers
//! write in practice are two. DoenetML's `<matchesPattern>` — the only real
//! consumer — passes exactly `(m) => !isNaN(evaluate_to_constant(m))` under
//! `requireNumericMatches` and `(m) => typeof m === "string"` under
//! `requireVariableMatches`; those are [`VarKind::Number`] and
//! [`VarKind::Variable`], which this module already implements. The declarative
//! form is also the sharper one: `Number` means "evaluates to a real numeric
//! constant", where a hand-written `typeof s === "number"` quietly rejected `π`.
//!
//! The legacy specs covering the open forms are skipped as wontfix; the two
//! options Doenet does use keep their own coverage in `quick_trees`.
//!
//! Still not ported (a gap, not a decision): `allow_extended_match`.
//! Binding consistency uses structural JSON equality where the JS uses its
//! syntactic `equal` — stricter in corner cases (e.g. `1` vs `1.0` differ only
//! in JS number spelling, which JSON round-tripping already collapses).
//!
//! Operates on `serde_json::Value` JS trees (not `Expr`): Doenet passes raw
//! ASTs and consumes raw subtree bindings, and converting through the
//! canonical layer would change the trees being matched.

use serde_json::{Map, Value};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};

/// Move a unary minus onto a factor, folding it into a numeric literal.
///
/// `-(9·y)` matched against `b·y` binds `b`. Wrapping the literal as
/// `["-", 9]` is the same *value* as `-9`, but it is not a *number* to anything
/// that inspects the binding, and these bindings are read as coefficients:
/// DoenetML's `<matchesPattern>` hands them straight to components that expect
/// `-9`. A non-numeric factor keeps the wrapper, since there is nothing to fold
/// into.
fn negate_factor(v: &Value) -> Value {
    if let Some(i) = v.as_i64() {
        if let Some(neg) = i.checked_neg() {
            return Value::Number(neg.into());
        }
    } else if let Some(f) = v.as_f64() {
        if let Some(n) = serde_json::Number::from_f64(-f) {
            return Value::Number(n);
        }
    }
    Value::Array(vec![Value::String("-".to_string()), v.clone()])
}

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

/// What a declared parameter is allowed to bind.
///
/// The closed replacement for legacy's predicate-function and `RegExp`
/// conditions, which are deprecated and wontfix — see the module docs. These
/// three are what DoenetML actually asks for, declared instead of computed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum VarKind {
    /// Any subtree (JS `true`).
    #[default]
    Any,
    /// Must evaluate to a real numeric constant — DoenetML's
    /// `requireNumericMatches`. `π` and `√3` qualify; `a` and `x+1` do not.
    Number,
    /// Must be a bare variable, i.e. a string leaf — DoenetML's
    /// `requireVariableMatches`. `x` qualifies; `x+x` does not.
    Variable,
    /// Admits nothing: a parameter declared with an unusable condition (JS
    /// `false`, an unknown kind string, or a deprecated `RegExp`/predicate). A
    /// pattern that must bind it therefore cannot match, so `match` fails
    /// gracefully (returns no match) rather than throwing — the legacy contract
    /// tested by `invalid matching conditions fail gracefully`.
    Nothing,
}

/// Options for [`match_template_with_options`].
#[derive(Clone, Debug, Default)]
pub struct MatchOptions {
    /// Declared parameters and their kinds. `None` keeps the legacy default,
    /// where *every* string leaf in the pattern is a wildcard. `Some(map)`
    /// means only these names bind and every other leaf is a literal — so an
    /// empty map declares no placeholders and only an exact match succeeds.
    pub variables: Option<HashMap<String, VarKind>>,
    /// Match operands of `+` and `*` in any order.
    pub allow_permutations: bool,
    /// Parameters that may bind the operator's identity (`0` for `+`, `1` for
    /// `*`) when the tree has no operand for them, so `a x + b` matches `x`
    /// with `a = 1`, `b = 0`.
    pub implicit_identities: HashSet<String>,
    /// Every parameter may take an identity — the legacy `true` spelling.
    ///
    /// Separate from stuffing all the names into `implicit_identities` because
    /// in default mode the parameters are the *pattern's* string leaves, which
    /// a caller cannot enumerate; only the matcher knows them.
    pub implicit_identities_all: bool,
}

/// Everything the recursion needs: the declared wildcards, the two flags, and
/// the step budget that keeps the search from running away.
struct Ctx {
    wildcards: HashMap<String, VarKind>,
    allow_permutations: bool,
    implicit_identities: HashSet<String>,
    implicit_identities_all: bool,
    /// Steps left in this search, and whether one was ever refused.
    ///
    /// `Cell` rather than `&mut` because the recursion threads `&Ctx` through
    /// `match_inner` → `match_operands` → `go` → `match_inner` and holds
    /// overlapping borrows across those frames. wasm is single-threaded, so
    /// the shared mutability costs nothing.
    budget: Cell<u64>,
    exhausted: Cell<bool>,
}

impl Ctx {
    fn kind_of(&self, name: &str) -> Option<VarKind> {
        self.wildcards.get(name).copied()
    }
    /// Charge one step, or refuse once the budget is gone. A refusal unwinds
    /// the whole search as "no match", and the `exhausted` flag is what turns
    /// that into [`MatchBudgetExceeded`] at the entry point rather than a
    /// silent — and wrong — "the tree did not match".
    fn spend(&self) -> bool {
        match self.budget.get() {
            0 => {
                self.exhausted.set(true);
                false
            }
            n => {
                self.budget.set(n - 1);
                true
            }
        }
    }
    fn is_implicit(&self, pattern: &Value) -> bool {
        matches!(pattern, Value::String(s)
            if (self.implicit_identities_all || self.implicit_identities.contains(s))
                && self.wildcards.contains_key(s))
    }
}

/// The search ran past [`MAX_MATCH_STEPS`], so no answer was reached. Distinct
/// from `Ok(None)` — "the tree does not match" — because on a grading path the
/// two must never be confused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchBudgetExceeded;

/// Ceiling on the steps (one per [`match_inner`] entry) that a single
/// `match_template*` call may take.
///
/// [`MAX_PERMUTED_OPERANDS`] bounds the orderings tried at *one* node, but
/// `match_operands` recurses back into `match_inner`, so with
/// `allow_permutations` the costs of nested `+`/`*` levels **multiply**. A
/// 341-character tree of 7 summands of 7 factors each ran past 90 s against a
/// three-operand pattern; wasm cannot be interrupted and the grading call is
/// synchronous, so that is a hung worker. A per-node cap cannot fix a product
/// across nodes — only a budget for the search as a whole can.
///
/// Sized against measurement rather than taste: a real three-operand pattern
/// against a quadratic settles in single-digit milliseconds, while the
/// pathological trees above exhaust this in well under half a second. That is
/// two orders of magnitude of headroom over legitimate use and still a bound a
/// synchronous grading call can absorb — reaching it means the input is
/// pathological, not merely large.
pub const MAX_MATCH_STEPS: u64 = 250_000;

/// Attempt to match `tree` against `pattern` (default mode — see module
/// docs). `Ok(Some(bindings))` maps each pattern wildcard to the subtree it
/// bound; `Ok(None)` means no match. An exact variable-free match yields an
/// empty map.
pub fn match_template(
    tree: &Value,
    pattern: &Value,
) -> Result<Option<Map<String, Value>>, MatchBudgetExceeded> {
    match_template_with_options(tree, pattern, &MatchOptions::default())
}

/// Rewrite every ordered relation to its left-pointing form, reversing the
/// operands: `a > b` → `b < a`, `a ≥ b` → `b ≤ a`, and the containment pair
/// `∋`/`⊃` → `∈`/`⊂`. Recursive, and a no-op on everything else.
fn orient_relations(v: &Value) -> Value {
    let Value::Array(items) = v else {
        return v.clone();
    };
    let mut out: Vec<Value> = items.iter().map(orient_relations).collect();
    let Some(Value::String(op)) = out.first().cloned() else {
        return Value::Array(out);
    };
    let flipped = match op.as_str() {
        ">" => "<",
        "ge" => "le",
        "ni" => "in",
        "notni" => "notin",
        "superset" => "subset",
        "notsuperset" => "notsubset",
        // `superseteq`, not the LaTeX command `supseteq`: these are JS AST
        // operator names, spelled by `RelOp`'s `Display` in `expr/tree.rs`.
        "superseteq" => "subseteq",
        "notsuperseteq" => "notsubseteq",
        _ => return Value::Array(out),
    };
    // Binary form only: a chained relation ("x < y < z") carries its operators
    // differently and is left alone.
    if out.len() != 3 {
        return Value::Array(out);
    }
    out.swap(1, 2);
    out[0] = Value::String(flipped.to_string());
    Value::Array(out)
}

/// [`match_template`] with the JS `match` options honored rather than dropped.
pub fn match_template_with_options(
    tree: &Value,
    pattern: &Value,
    opts: &MatchOptions,
) -> Result<Option<Map<String, Value>>, MatchBudgetExceeded> {
    let wildcards = match &opts.variables {
        Some(declared) => declared.clone(),
        None => {
            let mut names = HashSet::new();
            pattern_variables(pattern, &mut names);
            names.into_iter().map(|n| (n, VarKind::Any)).collect()
        }
    };
    let ctx = Ctx {
        wildcards,
        allow_permutations: opts.allow_permutations,
        implicit_identities: opts.implicit_identities.clone(),
        implicit_identities_all: opts.implicit_identities_all,
        budget: Cell::new(MAX_MATCH_STEPS),
        exhausted: Cell::new(false),
    };
    // Under permutations, point every ordered relation the same way first.
    // `x > y` and `y < x` are one statement, and which one an author typed
    // must not decide whether a pattern matches — JS did this by running
    // `default_order` over both trees here, "as it orients operators such as
    // inequalities and containments to a direction that won't be affected by
    // permutations". Only the orienting part is needed: the sorting half of
    // that pass is what this matcher's permutation search already does.
    let (oriented_tree, oriented_pattern);
    let (tree, pattern) = if opts.allow_permutations {
        oriented_tree = orient_relations(tree);
        oriented_pattern = orient_relations(pattern);
        (&oriented_tree, &oriented_pattern)
    } else {
        (tree, pattern)
    };
    let found = match_inner(tree, pattern, &ctx);
    // A refused step unwinds as `None`, so an exhausted search is
    // indistinguishable from a genuine non-match here — report it as neither.
    if ctx.exhausted.get() {
        return Err(MatchBudgetExceeded);
    }
    Ok(found)
}

/// Does `tree` satisfy the declared kind for a parameter?
fn kind_admits(kind: VarKind, tree: &Value) -> bool {
    match kind {
        VarKind::Any => true,
        VarKind::Nothing => false,
        VarKind::Variable => tree.is_string(),
        // Mirrors DoenetML's `isNumericConstant(fromAst(m).evaluate_to_constant())`,
        // which is a *finiteness* test. `evaluate_to_constant` deliberately
        // reports a proven `NaN` or `±∞` as a value rather than declining, so
        // both have to be excluded here rather than falling out of the `None`
        // case — otherwise a `"number"` coefficient slot binds `1/0`.
        VarKind::Number => math_expressions::expr::serde::try_from_js(tree)
            .ok()
            .and_then(|e| math_expressions::evaluate_to_constant(&e))
            .is_some_and(|c| c.re.is_finite() && c.im == 0.0),
    }
}

/// The identity element of an associative operator, for implicit-identity
/// binding: `0` for `+`, `1` for `*`.
fn identity_of(op: &str) -> Option<Value> {
    match op {
        "+" => Some(Value::Number(0.into())),
        "*" => Some(Value::Number(1.into())),
        _ => None,
    }
}

fn match_inner(tree: &Value, pattern: &Value, ctx: &Ctx) -> Option<Map<String, Value>> {
    // Every branch of the search reaches here, so charging one step per entry
    // bounds the whole thing — including the permutation loop below, whose
    // cost multiplies across nested levels.
    if !ctx.spend() {
        return None;
    }

    // A wildcard binds the whole tree, provided its declared kind admits it.
    if let Value::String(name) = pattern {
        if let Some(kind) = ctx.kind_of(name) {
            if !kind_admits(kind, tree) {
                return None;
            }
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
                    neg_first = Some(negate_factor(tree_operands[0]));
                }
            }
        }
    }
    // With implicit identities on, a pattern operand may take the operator's
    // identity instead of a tree operand. That makes a tree which is not a
    // `+`/`*` at all still matchable — `x` against `a x + b` is the whole tree
    // as the single summand, with `b` taking the `0` — so the arity checks
    // below have to let those through rather than bail early.
    let can_implicit =
        identity_of(op).is_some() && pattern_operands.iter().any(|p| ctx.is_implicit(p));
    if neg_first.is_none() {
        if !matches_shape {
            if !can_implicit {
                return None;
            }
            tree_operands.push(tree);
        }
        if tree_operands.len() < pattern_operands.len() && !can_implicit {
            return None;
        }
    }

    // Materialize the operands once so a permutation can reorder them.
    let owned_first = neg_first;
    let operands: Vec<&Value> = (0..tree_operands.len())
        .map(|i| match (&owned_first, i) {
            (Some(v), 0) => v,
            _ => tree_operands[i],
        })
        .collect();

    // Permutations are for the commutative operators only. The count is the
    // *tree's* operand count, so it is bounded before it is enumerated;
    // above the cap the in-order match stands rather than the match silently
    // becoming a very slow one.
    if ctx.allow_permutations
        && matches!(op, "+" | "*")
        && (2..=MAX_PERMUTED_OPERANDS).contains(&operands.len())
    {
        for order in permutations(operands.len()) {
            let permuted: Vec<&Value> = order.iter().map(|&i| operands[i]).collect();
            if let Some(m) = match_operands(op, &permuted, pattern_operands, ctx) {
                return Some(m);
            }
        }
        return None;
    }

    match_operands(op, &operands, pattern_operands, ctx)
}

/// Cap on how many tree operands will be permuted **at one node**. `8! =
/// 40_320` orderings is the most we will enumerate there.
///
/// This is not on its own a bound on the search: `match_operands` recurses
/// back into `match_inner`, so a nested tree multiplies this cost level by
/// level. [`MAX_MATCH_STEPS`] is what actually bounds the total.
const MAX_PERMUTED_OPERANDS: usize = 8;

/// All orderings of `0..n`, in lexicographic order.
fn permutations(n: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let mut cur: Vec<usize> = (0..n).collect();
    let mut used = vec![false; n];
    fn go(
        n: usize,
        used: &mut Vec<bool>,
        cur: &mut Vec<usize>,
        depth: usize,
        out: &mut Vec<Vec<usize>>,
    ) {
        if depth == n {
            out.push(cur.clone());
            return;
        }
        for i in 0..n {
            if used[i] {
                continue;
            }
            used[i] = true;
            cur[depth] = i;
            go(n, used, cur, depth + 1, out);
            used[i] = false;
        }
    }
    go(n, &mut used, &mut cur, 0, &mut out);
    out
}

/// Sequential operand matching with grouping (the JS default path of
/// `matchOperands`): pattern operand `i` tries absorbing 1..=max_group
/// consecutive tree operands (max_group > 1 only for group-allowing
/// operators); the last pattern operand must absorb the remainder exactly.
fn match_operands(
    op: &str,
    tree_operands: &[&Value],
    pattern_operands: &[Value],
    ctx: &Ctx,
) -> Option<Map<String, Value>> {
    fn chunk(op: &str, operands: &[&Value], start: usize, len: usize) -> Value {
        if len == 1 {
            operands[start].clone()
        } else {
            let mut arr = vec![Value::String(op.to_string())];
            arr.extend(operands[start..start + len].iter().map(|v| (*v).clone()));
            Value::Array(arr)
        }
    }

    fn consistent(a: &Map<String, Value>, b: &Map<String, Value>) -> bool {
        a.iter().all(|(k, v)| b.get(k).is_none_or(|w| v == w))
    }

    fn go(
        op: &str,
        operands: &[&Value],
        pattern_operands: &[Value],
        ctx: &Ctx,
        start: usize,
        pat_ind: usize,
        acc: &Map<String, Value>,
    ) -> Option<Map<String, Value>> {
        let n_pats = pattern_operands.len();
        let remaining = operands.len() - start;
        if pat_ind == n_pats {
            return (remaining == 0).then(|| acc.clone());
        }
        let last = pat_ind == n_pats - 1;
        // How many operands the *later* pattern operands still need. Normally
        // one each, but an implicit-identity parameter can take none, so it
        // must not reserve an operand this group could have absorbed —
        // otherwise `a x + b` against `x` leaves nothing for `a x`.
        let later_required = pattern_operands[pat_ind + 1..]
            .iter()
            .filter(|p| !ctx.is_implicit(p))
            .count();
        let max_group = if allows_groups(op) {
            remaining.saturating_sub(later_required)
        } else {
            1
        };
        // The last pattern operand must absorb everything left (JS: no
        // extended match). For non-group operators that means exactly one.
        let mut sizes: Vec<usize> = if last {
            (remaining == max_group.max(1) && remaining >= 1)
                .then_some(remaining)
                .into_iter()
                .collect()
        } else {
            (1..=max_group).collect()
        };
        // A declared implicit-identity parameter may absorb *nothing* and take
        // the operator's identity instead. Tried last, so a real operand always
        // wins over an invented one: `a x + b` against `2x+y` binds `b = y`
        // rather than `b = 0` with `y` left over.
        if ctx.is_implicit(&pattern_operands[pat_ind]) && !sizes.contains(&0) {
            sizes.push(0);
        }
        for size in sizes {
            let m = if size == 0 {
                let identity = identity_of(op)?;
                match_inner(&identity, &pattern_operands[pat_ind], ctx)
            } else {
                match_inner(
                    &chunk(op, operands, start, size),
                    &pattern_operands[pat_ind],
                    ctx,
                )
            };
            let Some(m) = m else { continue };
            if !consistent(&m, acc) {
                continue;
            }
            let mut combined = acc.clone();
            combined.extend(m);
            if let Some(result) = go(
                op,
                operands,
                pattern_operands,
                ctx,
                start + size,
                pat_ind + 1,
                &combined,
            ) {
                return Some(result);
            }
        }
        None
    }

    go(op, tree_operands, pattern_operands, ctx, 0, 0, &Map::new())
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
pub fn unflatten_left(tree: &Value) -> Option<Value> {
    (widest_associative_run(tree) <= MAX_UNFLATTEN_OPERANDS).then(|| unflatten(tree, true))
}

/// Right-associate: `["+", a, b, c] → ["+", a, ["+", b, c]]`.
pub fn unflatten_right(tree: &Value) -> Option<Value> {
    (widest_associative_run(tree) <= MAX_UNFLATTEN_OPERANDS).then(|| unflatten(tree, false))
}

/// How many operands an associative operator may carry into an unflatten.
///
/// The fold turns *width* into *depth*: `["+", a₁, …, aₙ]` is depth 2 as JSON
/// and comes back `n − 1` levels deep. Nothing downstream survives that
/// unbounded — serializing the result recurses in serde_json, and on the JS
/// side both `JSON.parse` and the `jsonToAst` walk recurse again, which is
/// where it actually broke: width 1,000 round-trips, width 3,000 raises
/// `RangeError: Maximum call stack size exceeded`. The bound sits below the
/// first failure with room to spare, and well above any authored expression —
/// a sum of a thousand terms is not something a person writes.
pub const MAX_UNFLATTEN_OPERANDS: usize = 1000;

/// The largest number of operands any single associative node in `tree` holds.
///
/// Checked before folding rather than during, so the refusal costs nothing and
/// no partial result is built.
fn widest_associative_run(tree: &Value) -> usize {
    let Some(arr) = tree.as_array() else { return 0 };
    let here = match arr.first().and_then(Value::as_str) {
        Some(op) if is_associative(op) => arr.len().saturating_sub(1),
        _ => 0,
    };
    arr.iter()
        .map(widest_associative_run)
        .fold(here, usize::max)
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

#[cfg(test)]
mod tests {
    //! The JS-tree utility surface Doenet uses via `me.utils`: default-mode
    //! template `match`, `flatten`/`unflatten{Left,Right}` (all `js_match`),
    //! plus `expr::serde::to_js` structural equality and the crate `substitute`
    //! (core-crate items, exercised here through the same JS-tree surface).
    //! Ported from `spec/quick_trees.spec.js`, plus the option surface
    //! (`variables` kinds, `allow_permutations`, `allow_implicit_identities`)
    //! that `match_template_with_options` adds — see this file's module docs
    //! and JS_TEST_COVERAGE_AUDIT.md.
    use super::{flatten_tree, unflatten_left, unflatten_right, MAX_UNFLATTEN_OPERANDS};
    use serde_json::Map;

    /// [`super::match_template`] with the step budget asserted away. These
    /// fixtures are a handful of operands wide, so reaching the budget would
    /// itself be the bug.
    fn match_template(tree: &Value, pattern: &Value) -> Option<Map<String, Value>> {
        super::match_template(tree, pattern).expect("fixture stays within the step budget")
    }
    use math_expressions::expr::serde::to_js;
    use math_expressions::{equals, substitute, EqOptions, Expr, TextToAst, TextToAstOptions};
    use serde_json::{json, Value};
    use std::collections::HashMap;

    fn parse(s: &str) -> Expr {
        TextToAst::new(TextToAstOptions::default())
            .convert(s)
            .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
    }

    /// The JS `TREE(s)` helper: parse text and take the raw JS tree.
    fn tree(s: &str) -> Value {
        to_js(&parse(s))
    }

    /// Structural tree equality (JS `trees.equal`) is JSON identity of the encoding.
    fn equal(a: &Value, b: &Value) -> bool {
        a == b
    }

    fn eq_expr(a: &Expr, b: &Expr) -> bool {
        equals(a, b, &EqOptions::default())
    }

    // ---- tree basics ----

    #[test]
    fn structural_equality_is_exact_and_order_sensitive() {
        assert!(equal(&tree("cos x"), &tree("cos x")));
        assert!(!equal(&tree("cos x"), &tree("cos y")));
        // Structural equality does NOT allow order changes (that is `equals`).
        assert!(!equal(&tree("x+y"), &tree("y+x")));
    }

    #[test]
    fn flatten_and_unflatten() {
        // unflattenRight: ["+",1,2,3] -> ["+",1,["+",2,3]]
        assert_eq!(
            unflatten_right(&json!(["+", 1, 2, 3])),
            Some(json!(["+", 1, ["+", 2, 3]]))
        );
        // unflattenLeft: ["+",1,2,3] -> ["+",["+",1,2],3]
        assert_eq!(
            unflatten_left(&json!(["+", 1, 2, 3])),
            Some(json!(["+", ["+", 1, 2], 3]))
        );
        // flatten both nestings back to the n-ary form.
        assert_eq!(
            flatten_tree(&json!(["+", 1, ["+", 2, 3]])),
            json!(["+", 1, 2, 3])
        );
        assert_eq!(
            flatten_tree(&json!(["+", ["+", 1, 2], 3])),
            json!(["+", 1, 2, 3])
        );
    }

    /// The unflatten bound ([`super::MAX_UNFLATTEN_OPERANDS`]). The fold turns
    /// width into depth, and past the bound nothing downstream survives it —
    /// serializing recurses here, and `JSON.parse` plus the `jsonToAst` walk
    /// recurse again on the JS side. Both directions are pinned: the bound is a
    /// contract the JS boundary now throws on, so widening it silently is as
    /// much a regression as losing it.
    #[test]
    fn unflatten_refuses_a_node_too_wide_to_fold() {
        let sum = |n: usize| {
            let mut v = vec![json!("+")];
            v.extend((0..n).map(|i| json!(i)));
            Value::Array(v)
        };

        // At the bound, both directions still fold.
        assert!(unflatten_left(&sum(MAX_UNFLATTEN_OPERANDS)).is_some());
        assert!(unflatten_right(&sum(MAX_UNFLATTEN_OPERANDS)).is_some());

        // One operand past it, both refuse.
        assert_eq!(unflatten_left(&sum(MAX_UNFLATTEN_OPERANDS + 1)), None);
        assert_eq!(unflatten_right(&sum(MAX_UNFLATTEN_OPERANDS + 1)), None);

        // The width that matters is the *widest node anywhere*, not the root's:
        // a narrow root carrying a wide child is exactly as unfoldable.
        let buried = json!(["*", 2, sum(MAX_UNFLATTEN_OPERANDS + 1)]);
        assert_eq!(unflatten_left(&buried), None);
        assert_eq!(unflatten_right(&buried), None);

        // A wide *non*-associative node is not folded at all, so it is not
        // bounded either — a 3000-entry tuple passes through untouched.
        let mut wide_tuple = vec![json!("tuple")];
        wide_tuple.extend((0..3000).map(|i| json!(i)));
        let wide_tuple = Value::Array(wide_tuple);
        assert_eq!(unflatten_left(&wide_tuple), Some(wide_tuple.clone()));
        assert_eq!(unflatten_right(&wide_tuple), Some(wide_tuple));
    }

    #[test]
    fn substitute_symbols() {
        let sub = |e: &str, pairs: &[(&str, Expr)]| {
            let map: HashMap<String, Expr> = pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect();
            substitute(&parse(e), &map)
        };

        // x+y becomes 1+2 when x:=1 and y:=2
        assert!(eq_expr(
            &sub("x+y", &[("x", parse("1")), ("y", parse("2"))]),
            &parse("1+2")
        ));
        // simultaneous swap: x := y^2 and y := x^2
        assert!(eq_expr(
            &sub("x+y", &[("x", parse("y^2")), ("y", parse("x^2"))]),
            &parse("y^2 + x^2")
        ));
        // recurses through apply / div
        assert!(eq_expr(
            &sub("cos(x+y)/sin(x*y)", &[("x", parse("1")), ("y", parse("2"))]),
            &parse("cos(1+2)/sin(1*2)")
        ));
        // recurses through relations (chained inequality)
        assert!(eq_expr(
            &sub(
                "x < y < z",
                &[("x", parse("a")), ("y", parse("b")), ("z", parse("c"))]
            ),
            &parse("a < b < c")
        ));
        assert!(eq_expr(
            &sub(
                "x < y <= z",
                &[("x", parse("a")), ("y", parse("b")), ("z", parse("c"))]
            ),
            &parse("a < b <= c")
        ));
    }

    // ---- default-mode template matching ----

    #[test]
    fn match_binds_wildcards() {
        let m = match_template(&tree("x+y"), &tree("a+b")).expect("x+y matches a+b");
        assert_eq!(m.get("a"), Some(&json!("x")));
        assert_eq!(m.get("b"), Some(&json!("y")));
    }

    #[test]
    fn match_requires_same_operator_and_whole_tree() {
        // x+y does not match a*b
        assert!(match_template(&tree("x+y"), &tree("a*b")).is_none());
        // a wildcard match must cover the entire tree
        assert!(match_template(&tree("x+y/z"), &tree("a/b")).is_none());
    }

    #[test]
    fn match_must_be_consistent() {
        // x+y/z matches a+b/c (all distinct) ...
        assert!(match_template(&tree("x+y/z"), &tree("a+b/c")).is_some());
        // ... but not a+b/a (would need y/z's numerator == denominator)
        assert!(match_template(&tree("x+y/z"), &tree("a+b/a")).is_none());
        // x+y/x DOES match a+b/a (x bound consistently)
        assert!(match_template(&tree("x+y/x"), &tree("a+b/a")).is_some());
    }

    #[test]
    fn match_multichar_placeholders_and_exact_numbers() {
        // multi-character pattern leaves are still wildcards by default
        assert!(match_template(&json!(["+", "x", "y"]), &json!(["+", "a", "bc"])).is_some());
        assert!(match_template(&json!(["+", "x", "bc"]), &json!(["+", "a", "bc"])).is_some());
        // numbers must match exactly
        assert!(match_template(&tree("3x+5"), &tree("ab+5")).is_some());
        assert!(match_template(&tree("3x+5"), &tree("ab+6")).is_none());
    }

    #[test]
    fn match_addition_matches_subtraction_not_vice_versa() {
        // x-y is ["+","x",["-","y"]]; a wildcard b absorbs the negated term.
        assert!(match_template(&tree("x-y"), &tree("a+b")).is_some());
        // but x+y cannot match a-b (the second operand must be a negation)
        assert!(match_template(&tree("x+y"), &tree("a-b")).is_none());
    }

    #[test]
    fn match_template_default_mode() {
        // ["+", ["*", 2, "x"], 3] against ["+", ["*", "a", "x"], "b"]:
        // wildcards a, x, b (all pattern variables).
        let tree = json!(["+", ["*", 2, "x"], 3]);
        let pat = json!(["+", ["*", "a", "y"], "b"]);
        let m = match_template(&tree, &pat).unwrap();
        assert_eq!(m.get("a").unwrap(), &json!(2));
        assert_eq!(m.get("y").unwrap(), &json!("x"));
        assert_eq!(m.get("b").unwrap(), &json!(3));

        // Grouping: last wildcard absorbs the rest of an associative operator.
        let tree = json!(["+", 1, 2, 3]);
        let m = match_template(&tree, &json!(["+", "u", "v"])).unwrap();
        assert_eq!(m.get("u").unwrap(), &json!(1));
        assert_eq!(m.get("v").unwrap(), &json!(["+", 2, 3]));

        // Repeated wildcard must bind equal subtrees.
        assert!(match_template(&json!(["+", "x", "x"]), &json!(["+", "u", "u"])).is_some());
        assert!(match_template(&json!(["+", "x", "y"]), &json!(["+", "u", "u"])).is_none());

        // Unary minus of product matches a * pattern.
        let tree = json!(["-", ["*", "x", "y"]]);
        let m = match_template(&tree, &json!(["*", "a", "b"])).unwrap();
        assert_eq!(m.get("a").unwrap(), &json!(["-", "x"]));
        assert_eq!(m.get("b").unwrap(), &json!("y"));

        // A *numeric* first factor absorbs the minus, so the binding is a
        // number: `-(9y)` against `b·y` gives `b = -9`, not `["-", 9]`.
        let tree = json!(["-", ["*", 9, "y"]]);
        let m = match_template(&tree, &json!(["*", "b", "y"])).unwrap();
        assert_eq!(m.get("b").unwrap(), &json!(-9));
        let tree = json!(["-", ["*", 1.5, "y"]]);
        let m = match_template(&tree, &json!(["*", "b", "y"])).unwrap();
        assert_eq!(m.get("b").unwrap(), &json!(-1.5));

        // Operators must match exactly; no match across operators.
        assert!(match_template(&json!(["*", 1, 2]), &json!(["+", "u", "v"])).is_none());
        // Exact variable-free match -> empty bindings.
        assert_eq!(
            match_template(&json!(["+", 1, 2]), &json!(["+", 1, 2]))
                .unwrap()
                .len(),
            0
        );
    }

    /// `match_template` is `pub` and runs on raw caller-supplied JS trees. A
    /// degenerate `["-", ["*"]]` (unary minus of a nullary product) against any
    /// `["*", …]` pattern used to index an empty operand vec → abort under
    /// `panic = "abort"`. It must now cleanly return `None`.
    #[test]
    fn match_template_nullary_product_minus_does_not_abort() {
        assert_eq!(
            match_template(&json!(["-", ["*"]]), &json!(["*", "a"])),
            None
        );
    }

    /// Differential corpus generated from the JS oracle (`me.utils.match`) by
    /// `scripts/generate-numeric-corpus.mjs`. The fixture is shared with the
    /// core crate's numeric corpus and lives there; read it across the crate
    /// boundary (the `match` slice is the only part `js_match` owns).
    fn match_corpus() -> Value {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../math-expressions-rs/tests/fixtures/numeric-corpus.json"
        ))
        .expect("run scripts/generate-numeric-corpus.mjs first");
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn match_agrees_with_js_default_mode() {
        for case in match_corpus()["match"].as_array().unwrap() {
            let got = match_template(&case["tree"], &case["pattern"]);
            match (&case["bindings"], got) {
                (Value::Null, None) => {}
                (Value::Null, Some(m)) => panic!(
                    "JS found no match but we bound {:?} in {case}",
                    Value::Object(m)
                ),
                (expected, None) => panic!("JS bound {expected} but we found no match in {case}"),
                (expected, Some(m)) => {
                    let exp = expected.as_object().unwrap();
                    assert_eq!(
                        exp.len(),
                        m.len(),
                        "binding sets differ in {case}: JS {expected}, ours {:?}",
                        Value::Object(m.clone())
                    );
                    for (k, v) in exp {
                        assert_eq!(
                            m.get(k),
                            Some(v),
                            "binding {k} differs in {case}: ours {:?}",
                            Value::Object(m.clone())
                        );
                    }
                }
            }
        }
    }

    // ---- the option surface -------------------------------------------

    use super::{MatchOptions, VarKind};

    /// As with `match_template` above: the budget is far above what these
    /// fixtures reach, so an exhausted search is a failure, not a case to
    /// handle.
    fn match_template_with_options(
        tree: &Value,
        pattern: &Value,
        opts: &MatchOptions,
    ) -> Option<Map<String, Value>> {
        super::match_template_with_options(tree, pattern, opts)
            .expect("fixture stays within the step budget")
    }

    fn pattern_of(s: &str) -> Value {
        to_js(
            &TextToAst::new(TextToAstOptions::default())
                .convert(s)
                .unwrap(),
        )
    }

    fn matched(tree: &str, pattern: &str, opts: &MatchOptions) -> String {
        match match_template_with_options(&pattern_of(tree), &pattern_of(pattern), opts) {
            Some(m) => Value::Object(m).to_string(),
            None => "false".to_string(),
        }
    }

    fn declared(names: &[(&str, VarKind)]) -> MatchOptions {
        MatchOptions {
            variables: Some(names.iter().map(|(n, k)| (n.to_string(), *k)).collect()),
            ..MatchOptions::default()
        }
    }

    /// Declaring the parameter list is the whole point: an empty list declares
    /// no placeholders, and a symbol that was never a parameter (`x`) is a
    /// literal rather than a binding.
    #[test]
    fn only_declared_names_bind() {
        assert_eq!(matched("3x+5", "a x + b", &declared(&[])), "false");
        assert_eq!(
            matched(
                "3x+5",
                "a x + b",
                &declared(&[("a", VarKind::Any), ("b", VarKind::Any)])
            ),
            r#"{"a":3,"b":5}"#
        );
        // No options at all keeps the legacy default: every string leaf binds.
        assert_eq!(
            Value::Object(match_template(&pattern_of("3x+5"), &pattern_of("a x + b")).unwrap())
                .to_string(),
            r#"{"a":3,"b":5,"x":"x"}"#
        );
    }

    #[test]
    fn kinds_constrain_what_a_parameter_may_bind() {
        let num = declared(&[("a", VarKind::Number), ("b", VarKind::Number)]);
        assert_eq!(matched("3x+5", "a x + b", &num), r#"{"a":3,"b":5}"#);
        // `y` is not a number, so there is no match at all.
        assert_eq!(matched("yx+5", "a x + b", &num), "false");

        let var = declared(&[("a", VarKind::Variable), ("b", VarKind::Variable)]);
        assert_eq!(matched("ax+b", "a x + b", &var), r#"{"a":"a","b":"b"}"#);
        // `b` would have to bind `x+x`, which is not a bare variable.
        assert_eq!(matched("ax+x+x", "a x + b", &var), "false");
    }

    /// The permutation search multiplies across nesting levels, so a tree that
    /// is unremarkable in size can run effectively forever. Before the budget,
    /// this input did not finish in 90 s; wasm cannot be interrupted and the
    /// grading call is synchronous, so that is a hung worker.
    ///
    /// The assertion is `Err`, not `Ok(None)`: giving up and not matching lead
    /// to different grades, so they must not share a spelling.
    #[test]
    fn a_runaway_permutation_search_is_an_error_not_a_hang() {
        let pattern = tree("a x^2 + b x + c");
        let mut opts = declared(&[
            ("a", VarKind::Number),
            ("b", VarKind::Number),
            ("c", VarKind::Number),
            ("x", VarKind::Variable),
        ]);
        opts.allow_permutations = true;
        opts.implicit_identities = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();

        let wide: Vec<String> = (0..7)
            .map(|i| {
                (0..7)
                    .map(|j| format!("z{i}{j}"))
                    .collect::<Vec<_>>()
                    .join("*")
            })
            .collect();
        let hostile = tree(&wide.join("+"));
        assert_eq!(
            super::match_template_with_options(&hostile, &pattern, &opts),
            Err(super::MatchBudgetExceeded)
        );

        // The budget must not fire on the shapes DoenetML actually sends.
        assert_eq!(
            super::match_template_with_options(&tree("3x^2+4x+5"), &pattern, &opts)
                .expect("a real pattern stays far inside the budget"),
            Some(
                [
                    ("a", json!(3)),
                    ("b", json!(4)),
                    ("c", json!(5)),
                    ("x", json!("x"))
                ]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            )
        );
    }

    #[test]
    fn permutations_are_opt_in() {
        let mut opts = declared(&[("a", VarKind::Any), ("b", VarKind::Any)]);
        assert_eq!(matched("5+3x", "a x + b", &opts), "false");
        opts.allow_permutations = true;
        assert_eq!(matched("5+3x", "a x + b", &opts), r#"{"a":3,"b":5}"#);
        // Also inside the product: `x*2` against `a x`.
        assert_eq!(matched("x*2+y", "a x + b", &opts), r#"{"a":2,"b":"y"}"#);
    }

    /// An implicit identity lets a parameter take `1` under `*` or `0` under
    /// `+` when the tree has no operand for it, so `a x + b` matches a bare `x`.
    #[test]
    fn implicit_identities_are_opt_in() {
        let mut opts = declared(&[("a", VarKind::Any), ("b", VarKind::Any)]);
        opts.allow_permutations = true;
        assert_eq!(matched("x", "a x + b", &opts), "false");
        opts.implicit_identities = ["a", "b"].iter().map(|s| s.to_string()).collect();
        assert_eq!(matched("x", "a x + b", &opts), r#"{"a":1,"b":0}"#);
        assert_eq!(matched("x+y", "a x + b", &opts), r#"{"a":1,"b":"y"}"#);
        // A real operand still wins over an invented one.
        assert_eq!(matched("2x+y", "a x + b", &opts), r#"{"a":2,"b":"y"}"#);
    }
}
