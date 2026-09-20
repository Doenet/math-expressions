// The `is_integer` / `is_real` / … predicates. Each takes an Expression (and an
// optional assumptions source) and returns true / false / undefined, mapping to
// the wasm `Assumptions` three-valued predicates.
import wasm, { onWasmModuleChange } from "../_wasm";
import Context from "../math-expressions";

// Constructed lazily, like `Context._assumptionsHandle` and for the same
// reason: a `new wasm.Assumptions()` evaluated in this module's body would
// force the wasm load before a host had any chance to `setWasmModule`. And
// dropped on a swap for the other reason a cached handle must be: it belongs to
// the module that minted it.
let emptyCache;
const empty = () => (emptyCache ??= new wasm.Assumptions());
onWasmModuleChange(() => {
  emptyCache = undefined;
});

function handleFor(assumptions) {
  // No explicit source: consult the context's live global assumptions, so
  // `is_real(me.fromText("x+y"))` sees `me.add_assumption(...)` state. The
  // original JS predicates defaulted to the global store this way; falling
  // back to an empty one made every no-argument query answer "unknown".
  // `Context.assumptions` is the *facade*, not the handle — it mirrors the
  // predicate methods, so calling one on it works, and it is a lazily built
  // object that is never nullish (hence no fallback here). Reaching through it
  // to `_assumptionsHandle` would be the tidier symmetry with the branch below,
  // but the facade is what the rest of the API hands out, so this keeps one
  // answer to "what are the current assumptions".
  if (!assumptions) return Context.assumptions;
  // A facade passed in explicitly, whose live handle is what the caller means.
  if (assumptions._assumptionsHandle) return assumptions._assumptionsHandle;
  if (typeof assumptions.is_integer === "function") return assumptions; // a raw handle
  return empty();
}

function rawExpr(expression) {
  if (expression && expression._w) return expression._w;
  return expression; // already a raw wasm handle
}

function predicate(name) {
  return function (expression, assumptions) {
    return handleFor(assumptions)[name](rawExpr(expression));
  };
}

export const is_integer = predicate("is_integer");
export const is_real = predicate("is_real");
export const is_complex = predicate("is_complex");
export const is_nonzero = predicate("is_nonzero");
export const is_nonnegative = predicate("is_nonnegative");
export const is_nonpositive = predicate("is_nonpositive");
export const is_positive = predicate("is_positive");
export const is_negative = predicate("is_negative");
