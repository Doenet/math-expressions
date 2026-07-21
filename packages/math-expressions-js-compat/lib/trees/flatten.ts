// Raw JS-tree utilities (`me.utils.flatten` / `unflatten*` / `match`), backed by
// the wasm ports which take/return the JSON tree encoding.
import wasm from "../_wasm";

function viaWasm(fn, tree) {
  if (!Array.isArray(tree)) return tree;
  const out = fn(JSON.stringify(tree));
  return out === undefined ? tree : JSON.parse(out);
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
  const associative = ["+", "*", "and", "or", "union", "intersect"].includes(op);
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

/** Default-mode template match; `false` when it does not match. */
export function match(tree, pattern) {
  const res = wasm.match_template(JSON.stringify(tree), JSON.stringify(pattern));
  return res === undefined ? false : JSON.parse(res);
}
