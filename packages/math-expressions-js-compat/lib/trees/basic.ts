// `me.utils` tree basics: structural `equal`, `match`, `substitute`, and the
// pattern-rewriting layer built on them (`transform` / `replaceSubtree` /
// `applyAllTransformations` / …).
//
// Everything here is plain JS over the raw AST arrays, as in the legacy
// library. That is deliberate and not a shortcut: these operate on trees the
// caller assembled by hand (`["+", "a", "b"]`), the rewriting is driven by
// caller-supplied patterns, and `replaceSubtree` keys on *reference* identity —
// none of which survives a round trip through the canonical `Expr` layer.
export { match } from "./flatten";
import { match } from "./flatten";
import me from "../math-expressions";

/** Structural tree equality (numbers compared by value, so 1 === 1.0). */
export function equal(a, b) {
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    return a.every((x, i) => equal(x, b[i]));
  }
  if (typeof a === "number" && typeof b === "number") return a === b;
  return a === b;
}

/** Substitute string-leaf variables with their bound subtrees. */
export function substitute(tree, bindings) {
  if (typeof tree === "string") {
    return Object.prototype.hasOwnProperty.call(bindings, tree)
      ? bindings[tree]
      : tree;
  }
  if (Array.isArray(tree)) {
    // index 0 is the operator/head; never a substitutable variable
    return tree.map((t, i) => (i === 0 ? t : substitute(t, bindings)));
  }
  return tree;
}

/**
 * Structural copy.
 *
 * Recursive rather than the legacy's `JSON.parse(JSON.stringify(...))` with its
 * two reviver hooks: `.tree` hands out real `Infinity`/`NaN`, which
 * `JSON.stringify` turns into `null`. Cloning by walking the array never sees
 * the value as JSON and so cannot lose it.
 */
function deepClone(tree) {
  return Array.isArray(tree) ? tree.map(deepClone) : tree;
}

/**
 * Call `callback(subtree, root)` bottom-up — children before their parent.
 *
 * `root` is threaded through unchanged so a callback can rebuild the whole tree
 * around the subtree it was handed; that is what
 * [`applyTransformationEachSubtree`] needs.
 */
export function traverse(tree, callback, root?) {
  if (root === undefined) root = tree;
  if (Array.isArray(tree)) {
    for (let i = 1; i < tree.length; i++) traverse(tree[i], callback, root);
  }
  callback(tree, root);
}

/** Rewrite every node bottom-up through `F`, which returns the new subtree. */
export function transform(tree, F) {
  if (Array.isArray(tree)) {
    const rebuilt = [tree[0]];
    for (let i = 1; i < tree.length; i++) rebuilt.push(transform(tree[i], F));
    return F(rebuilt);
  }
  return F(tree);
}

/**
 * Replace the subtree `tree` of `root` with `replacement`.
 *
 * Matching is by `===`, so for an *array* subtree it is reference identity:
 * `tree` has to be a node actually inside `root`, and an equal-looking tree
 * built separately is left alone. That does not extend to leaves, where `===`
 * is value equality — replacing `"x"` in `["+","x","x"]` rewrites both
 * occurrences, not one.
 */
export function replaceSubtree(root, tree, replacement) {
  if (root === tree) return deepClone(replacement);
  if (Array.isArray(root))
    return root.map((c) => replaceSubtree(c, tree, replacement));
  return root;
}

/**
 * Fold numbers in a rewritten subtree, for `params.evaluate_numbers`.
 *
 * The only thing in this file that touches the core. `math-expressions` does
 * not import this module, so the edge is one-way and there is no cycle.
 *
 * Legacy also forwards `params.assumptions` (substituted through the match
 * bindings). The port's `evaluate_numbers` reads assumptions from the
 * expression's context rather than taking them per call, so a transformation's
 * own assumptions are not honored here.
 */
function evaluateNumbers(tree, params) {
  return me.fromAst(tree).evaluate_numbers({
    max_digits: params.max_digits,
    evaluate_functions: params.evaluate_functions,
  }).tree;
}

/**
 * Rewrite `tree` by every `[pattern, replacement, params]` in `transformations`,
 * repeating until nothing changes or `depth` rounds have passed.
 *
 * The depth bound is the termination guarantee, not an optimization: a rule set
 * containing both `a+b → b+a` and its mirror never reaches a fixpoint, and the
 * legacy callers lean on the cap (`simplify` passes 20 or 40).
 */
export function applyAllTransformations(tree, transformations, depth = 5) {
  let newTree = tree;
  for (; depth > 0; depth--) {
    const oldTree = newTree;
    for (const [pattern, replacement, rawParams] of transformations) {
      const params = rawParams === undefined ? {} : rawParams;
      newTree = transform(newTree, (subtree) => {
        const m = match(subtree, pattern, params);
        if (!m) return subtree;
        let result = substitute(replacement, m);

        // An extended match consumed only part of an n-ary operand list; the
        // operands it stepped over have to be spliced back around the rewritten
        // part, or the transformation silently deletes them.
        //
        // `_skipped` is live: `allow_extended_match` is handled on the JS side
        // (`trees/flatten.ts`, which sets it in `extendedMatch`) rather than in
        // `js_match.rs`. `_skipped_before` is not — `extendedMatch` never
        // reports operands to the *left* separately — so the `addLeft` half is
        // dead for now. Kept because the alternative to writing it when
        // extended match starts reporting both is a silent operand-dropping
        // bug.
        const skipped = m as {
          _skipped?: unknown[];
          _skipped_before?: unknown[];
        };
        const addLeft = skipped._skipped_before ?? [];
        const addRight = skipped._skipped ?? [];
        if (addLeft.length > 0 || addRight.length > 0) {
          if (Array.isArray(result)) {
            result = result[0] === pattern[0] ? result.slice(1) : [result];
          } else {
            result = [result];
          }
          result = [pattern[0]].concat(addLeft, result, addRight);
        }

        // Legacy folds both before *and* after the splice; this folds only
        // after. The pre-fold's only observable effect is on the `result[0]
        // === pattern[0]` test above — a replacement that folds to a bare
        // number is wrapped rather than spliced flat — and the fold below then
        // combines the same operands either way. Left as one pass because a
        // second `fromAst` round-trip per rewrite, per round, is the hot loop
        // of `simplify`.
        if (params.evaluate_numbers) result = evaluateNumbers(result, params);
        return result;
      });
    }
    if (equal(oldTree, newTree)) return newTree;
  }
  return newTree;
}

/**
 * Every one-step rewrite of `tree`: one result per subtree that matches
 * `pattern`, each with only that subtree replaced.
 */
export function applyTransformationEachSubtree(tree, pattern, replacement) {
  const results = [];
  traverse(tree, (subtree, root) => {
    const m = match(subtree, pattern);
    if (m)
      results.push(replaceSubtree(root, subtree, substitute(replacement, m)));
  });
  return results;
}

/** Curry a `[pattern, replacement]` rule into a one-step rewriter. */
export function patternTransformer(pattern, replacement) {
  return (tree) => applyTransformationEachSubtree(tree, pattern, replacement);
}

/**
 * Whether `left` and `right` meet under repeated rewriting by `transformers`.
 *
 * Grows both sides breadth-first and looks for a common form, rather than
 * normalizing either one — the transformers are not confluent (commutativity
 * alone is not), so there is no normal form to compare.
 *
 * Returns `true` on a meeting, `false` once both frontiers stop growing without
 * one, and **`undefined`** when `depth` runs out first: the search was cut off,
 * so "not equal" was never established.
 */
export function equalAfterTransformations(
  left,
  right,
  transformers,
  depth = 5,
  comparer = equal,
) {
  const leftQueue = [left];
  const rightQueue = [right];

  // One breadth-first round: append every rewrite not already in the queue.
  // Returns whether anything was added, i.e. whether this side is still moving.
  const evolve = (queue) => {
    const toAppend = [];
    for (const item of queue) {
      for (const transformer of transformers) {
        for (const result of transformer(item)) {
          if (queue.every((other) => !comparer(result, other)))
            toAppend.push(result);
        }
      }
    }
    queue.push(...toAppend);
    return toAppend.length > 0;
  };

  for (; depth > 0; depth--) {
    const noMoreLeft = !evolve(leftQueue);
    const noMoreRight = !evolve(rightQueue);
    for (const a of leftQueue) {
      for (const b of rightQueue) if (comparer(a, b)) return true;
    }
    if (noMoreLeft && noMoreRight) return false;
  }
  return undefined;
}
