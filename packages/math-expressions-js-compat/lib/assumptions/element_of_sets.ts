// The `is_integer` / `is_real` / … predicates. Each takes an Expression (and an
// optional assumptions source) and returns true / false / undefined, mapping to
// the wasm `Assumptions` three-valued predicates.
import wasm from "../_wasm";

const EMPTY = new wasm.Assumptions();

function handleFor(assumptions) {
  if (!assumptions) return EMPTY;
  // Our Context exposes its live handle as `.assumptions`.
  if (assumptions._assumptionsHandle) return assumptions._assumptionsHandle;
  if (typeof assumptions.is_integer === "function") return assumptions; // a raw handle
  return EMPTY;
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
