// Helpers for working with the `pm` (plus-minus) operator.
//
// AST shape: `pm` is unary, analogous to unary `-`. A pm-bearing expression
// like `5 \pm 3` is represented as `["+", 5, ["pm", 3]]`. Each `["pm", x]`
// denotes the set `{x, -x}` with an independent sign choice.

// Maximum number of `pm` operators allowed in a single expression for
// sign-expansion. `expand_pm_signs` produces 2^MAX_PM_COUNT variants, so
// raising this trades exponential work for the ability to handle more
// independent ± choices.
const MAX_PM_COUNT = 10;
const MAX_PM_EXPANSIONS = 1 << MAX_PM_COUNT; // 1024

// Returns true if `tree` contains any `pm` operator anywhere in its subtree.
export function contains_pm(tree) {
  if (!Array.isArray(tree)) return false;
  if (tree[0] === "pm") return true;
  for (let i = 1; i < tree.length; i++) {
    if (contains_pm(tree[i])) return true;
  }
  return false;
}

// Returns the number of `pm` operators anywhere in `tree`.
export function count_pm(tree) {
  if (!Array.isArray(tree)) return 0;
  let n = tree[0] === "pm" ? 1 : 0;
  for (let i = 1; i < tree.length; i++) {
    n += count_pm(tree[i]);
  }
  return n;
}

// Enumerate all 2^n sign assignments for the `pm` operators in `tree`.
// Each `["pm", x]` is replaced either by `x` (sign = +) or by `["-", x]`
// (sign = -). Throws if the count would exceed MAX_PM_COUNT.
export function expand_pm_signs(tree) {
  const n = count_pm(tree);
  if (n === 0) return [tree];
  if (n > MAX_PM_COUNT) {
    throw new Error(
      `pm: cannot expand ${n} plus-minus operators (limit is ${MAX_PM_COUNT} → ${MAX_PM_EXPANSIONS} combinations)`,
    );
  }
  const total = 1 << n;
  const results = [];
  for (let mask = 0; mask < total; mask++) {
    const counter = { idx: 0 };
    results.push(replace_pm(tree, mask, counter));
  }
  return results;
}

function replace_pm(tree, mask, counter) {
  if (!Array.isArray(tree)) return tree;
  if (tree[0] === "pm") {
    const bit = (mask >> counter.idx) & 1;
    counter.idx += 1;
    const inner = replace_pm(tree[1], mask, counter);
    return bit === 0 ? inner : ["-", inner];
  }
  const out = [tree[0]];
  for (let i = 1; i < tree.length; i++) {
    out.push(replace_pm(tree[i], mask, counter));
  }
  return out;
}
