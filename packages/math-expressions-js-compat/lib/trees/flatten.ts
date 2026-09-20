// Raw JS-tree utilities (`me.utils.flatten` / `unflatten*` / `match`), backed by
// the wasm ports which take/return the JSON tree encoding.
import wasm from "../_wasm";
import { astToJson, jsonToAst } from "../converters/ast-json";
import { default_order } from "./default_order";

// `astToJson`/`jsonToAst` rather than bare `JSON.stringify`/`JSON.parse`: these
// take trees straight from `.tree`, which hands out real `Infinity`/`NaN`, and
// `JSON.stringify(Infinity)` is `null`. That turned `me.utils.flatten(e.tree)`
// into a silently wrong tree — no throw, just a `null` where a value was.
function viaWasm(fn, tree) {
  if (!Array.isArray(tree)) return tree;
  const out = fn(astToJson(tree));
  return out === undefined ? tree : jsonToAst(out);
}

export function flatten(tree) {
  return viaWasm(wasm.flatten_ast, tree);
}
export function unflattenLeft(tree) {
  return viaWasm(wasm.unflatten_left, tree);
}
export function unflattenRight(tree) {
  return viaWasm(wasm.unflatten_right, tree);
}

/** JS reimplementation of `flatten.allChildren` (flatten same-operator nests). */
export function allChildren(tree) {
  if (!Array.isArray(tree)) return tree;
  const op = tree[0];
  const associative = ["+", "*", "and", "or", "union", "intersect"].includes(
    op,
  );
  const out = [];
  for (const operand of tree.slice(1)) {
    if (associative && Array.isArray(operand) && operand[0] === op) {
      out.push(...allChildren(operand));
    } else {
      out.push(operand);
    }
  }
  return out;
}

/**
 * Normalize the JS `match` params into the JSON the wasm option decoder takes.
 *
 * Lives here rather than in `math-expressions.ts` because that module imports
 * *this* one; both entry points share it so they cannot drift, which is what
 * let `me.utils.match` keep dropping its params after `expr.match` learned to
 * honor them.
 *
 * **Deprecated, wontfix:** legacy also accepted a predicate function or a
 * `RegExp` as a parameter's condition. Neither is supported and neither will
 * be. A function would have to be called back across the wasm boundary once per
 * candidate binding, inside a matcher that backtracks — the cost is not the
 * bridge, it is that the matcher stops being a pure Rust search. And the
 * conditions callers actually write are two: "is a number" and "is a bare
 * variable". DoenetML's `<matchesPattern>`, the only real consumer, passes
 * exactly those two closures (`MatchesPattern.js`, under `requireNumericMatches`
 * / `requireVariableMatches`), and they are already spelled `"number"` and
 * `"variable"`. Declaring a kind is the supported replacement, and it is
 * strictly better defined: `"number"` means "evaluates to a real numeric
 * constant", where a caller's `typeof s === "number"` silently missed `π`.
 */
export function normalizeMatchOptions(options) {
  const opts: Record<string, unknown> = {};
  if (options.variables !== undefined) {
    const vars: Record<string, unknown> = {};
    for (const [name, kind] of Object.entries(options.variables)) {
      const arbitrary =
        typeof kind === "function"
          ? "a predicate function"
          : kind instanceof RegExp
            ? "a regular expression"
            : null;
      if (arbitrary !== null) {
        throw new Error(
          `match: 'variables.${name}' is ${arbitrary}. Arbitrary per-parameter ` +
            "conditions are deprecated and will not be supported — declare a " +
            'kind instead: "number", "variable", "any" (or true).',
        );
      }
      vars[name] = kind;
    }
    opts.variables = vars;
  }
  if (options.allow_permutations !== undefined) {
    opts.allow_permutations = !!options.allow_permutations;
  }
  if (options.allow_implicit_identities !== undefined) {
    const ii = options.allow_implicit_identities;
    opts.allow_implicit_identities = Array.isArray(ii) ? ii : !!ii;
  }
  return opts;
}

/**
 * Template match; `false` when it does not match.
 *
 * `params` is honored rather than dropped. Ignoring it silently was the exact
 * "confidently wrong bindings" failure the option decoder exists to prevent:
 * with no params every string leaf in the pattern is a wildcard, so a caller
 * who declared two parameters got a match on three.
 */
export function match(tree, pattern, params?) {
  const hasParams =
    params !== null && typeof params === "object" && !Array.isArray(params);
  // `allow_extended_match` lets a sum/product pattern match a *subset* of a
  // larger sum/product, leaving the rest. It is handled here rather than in the
  // Rust matcher: enumerate operand subsets, match the (unextended) pattern
  // against each with the core matcher, and report the untouched operands as
  // `_skipped` for `applyAllTransformations` to splice back. See that function.
  if (
    hasParams &&
    params.allow_extended_match &&
    Array.isArray(pattern) &&
    (pattern[0] === "+" || pattern[0] === "*")
  ) {
    return extendedMatch(tree, pattern, params);
  }
  const res = hasParams
    ? wasm.match_template_with_options(
        astToJson(tree),
        astToJson(pattern),
        JSON.stringify(normalizeMatchOptions(params)),
      )
    : wasm.match_template(astToJson(tree), astToJson(pattern));
  return res === undefined ? false : jsonToAst(res);
}

/** All size-`k` index subsets of `[0, n)`, in lexicographic order. */
function combinations(n: number, k: number): number[][] {
  const out: number[][] = [];
  const pick = (start: number, chosen: number[]) => {
    if (chosen.length === k) {
      out.push(chosen.slice());
      return;
    }
    for (let i = start; i <= n - (k - chosen.length); i++) {
      chosen.push(i);
      pick(i + 1, chosen);
      chosen.pop();
    }
  };
  pick(0, []);
  return out;
}

/**
 * Match a `+`/`*` pattern against a subset of a larger `+`/`*` tree.
 *
 * The tree's operands are put in canonical order first (`default_order`) so the
 * skipped remainder comes out in the order the callers' expected results assume
 * — legacy sorts before matching, and the splice appends `_skipped` verbatim.
 * The pattern's own operands are matched against each candidate subset by the
 * core matcher (honoring `allow_permutations`), so a coefficient like the `x`
 * in `x·cos(b)² + x·sin(b)²` still binds.
 */
function extendedMatch(tree, pattern, params) {
  const op = pattern[0];
  if (!Array.isArray(tree) || tree[0] !== op) return false;
  const patOperands = pattern.slice(1);
  const k = patOperands.length;

  const treeOperands = default_order(tree).slice(1);
  const n = treeOperands.length;
  // Guard the combinatorial search; a graded response never has this many terms.
  if (n < k || n > 16) return false;

  // The subset is matched unextended; drop the flag so this does not recurse.
  const subParams = { ...params };
  delete subParams.allow_extended_match;

  for (const idx of combinations(n, k)) {
    const chosen = new Set(idx);
    const candidate = [op, ...idx.map((i) => treeOperands[i])];
    const m = match(candidate, pattern, subParams);
    if (m) {
      const skipped = treeOperands.filter((_, i) => !chosen.has(i));
      if (skipped.length > 0) (m as { _skipped?: unknown[] })._skipped = skipped;
      return m;
    }
  }
  return false;
}
