// Discrete-infinite-set equality (`{offset + k·period}`, e.g. `π/4 + nπ`).
//
// The interesting part of the legacy API is not the boolean — `expr.equals(…)`
// already routes through this stage inside the Rust `equals` chain — but the
// `match_partial` grading signal, which returns the *fraction* of residue
// classes the two sets have in common. Only this entry point can express it,
// so it is the reason the module exists separately from `Expression.equals`.
import wasm from "../../_wasm";
import Context from "../../math-expressions";
import { get_tree } from "../../trees/util";
import { astToJson } from "../../converters/ast-json";

/** Expression-or-tree → a throwaway wasm handle the caller must `free()`. */
function handle(value) {
  return wasm.from_ast(astToJson(get_tree(value)));
}

/**
 * Is `expr` (a discrete infinite set) equal to `other` (another set, or a list
 * ending in `...`)?
 *
 * With `match_partial`, returns a number in `[0, 1]` — the fraction of `expr`
 * matched, for partial credit — instead of a boolean; a pair with nothing in
 * common scores `0` rather than a fraction, as in the legacy grader.
 *
 * `min_elements_match` is not accepted: the Rust stage fixes it at the legacy
 * default of 3, and silently ignoring a caller's other value would grade a
 * listed sequence against a rule they did not ask for.
 *
 * The context's assumptions are consulted (a symbolic period has to be known
 * nonzero before its ratios mean anything), matching the legacy version, which
 * pulled them off each argument's `.context`. This port has the one context, so
 * it reads it directly rather than requiring the arguments to be Expressions.
 */
export function equals(expr, other, { match_partial = false } = {}) {
  const a = handle(expr);
  try {
    const b = handle(other);
    try {
      const score = Context._assumptionsHandle.match_discrete_infinite_set(
        a,
        b,
        match_partial,
      );
      return match_partial ? score : score >= 1;
    } finally {
      b.free();
    }
  } finally {
    a.free();
  }
}

export default { equals };
