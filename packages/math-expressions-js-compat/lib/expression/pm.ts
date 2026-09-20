// Helpers for working with the `pm` (plus-minus) operator.
//
// AST shape: `pm` is unary, analogous to unary `-`. A pm-bearing expression
// like `5 \pm 3` is represented as `["+", 5, ["pm", 3]]`. Each `["pm", x]`
// denotes the set `{x, -x}` with an independent sign choice.
//
// These are tree-level operations with no math in them, so they stay in JS
// rather than crossing the wasm boundary: `expand_pm_signs` returns up to 1024
// trees, and shipping each one through wasm would cost more than building it.
import type { Tree } from "../math-expressions";

/**
 * Maximum number of `pm` operators allowed in a single expression for
 * sign-expansion. `expand_pm_signs` produces 2^MAX_PM_COUNT variants, so
 * raising this trades exponential work for the ability to handle more
 * independent ± choices.
 */
const MAX_PM_COUNT = 10;
const MAX_PM_EXPANSIONS = 1 << MAX_PM_COUNT; // 1024

/** Whether `tree` contains any `pm` operator anywhere in its subtree. */
export function contains_pm(tree: Tree): boolean {
  if (!Array.isArray(tree)) return false;
  if (tree[0] === "pm") return true;
  for (let i = 1; i < tree.length; i++) {
    if (contains_pm(tree[i])) return true;
  }
  return false;
}

/** The number of `pm` operators anywhere in `tree`. */
export function count_pm(tree: Tree): number {
  if (!Array.isArray(tree)) return 0;
  let n = tree[0] === "pm" ? 1 : 0;
  for (let i = 1; i < tree.length; i++) {
    n += count_pm(tree[i]);
  }
  return n;
}

/**
 * Enumerate all 2^n sign assignments for the `pm` operators in `tree`. Each
 * `["pm", x]` is replaced either by `x` (sign = +) or by `["-", x]` (sign = −).
 * Throws if the count would exceed `MAX_PM_COUNT`.
 *
 * The n-th `pm` in left-to-right order reads bit n of the mask, so every
 * operator gets an *independent* sign — which is the whole point of the
 * operator, and why `(±x)(±x)` is not `(±x)²`.
 */
export function expand_pm_signs(tree: Tree): Tree[] {
  const n = count_pm(tree);
  if (n === 0) return [tree];
  if (n > MAX_PM_COUNT) {
    throw new Error(
      `pm: cannot expand ${n} plus-minus operators (limit is ${MAX_PM_COUNT} → ${MAX_PM_EXPANSIONS} combinations)`,
    );
  }
  const total = 1 << n;
  const results: Tree[] = [];
  for (let mask = 0; mask < total; mask++) {
    results.push(replace_pm(tree, mask, { idx: 0 }));
  }
  return results;
}

function replace_pm(tree: Tree, mask: number, counter: { idx: number }): Tree {
  if (!Array.isArray(tree)) return tree;
  if (tree[0] === "pm") {
    const bit = (mask >> counter.idx) & 1;
    counter.idx += 1;
    const inner = replace_pm(tree[1], mask, counter);
    return bit === 0 ? inner : ["-", inner];
  }
  const out: Tree[] = [tree[0]];
  for (let i = 1; i < tree.length; i++) {
    out.push(replace_pm(tree[i], mask, counter));
  }
  return out;
}

export default { contains_pm, count_pm, expand_pm_signs };
