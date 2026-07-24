// `me.utils` tree basics: structural `equal`, `match`, `substitute`.
export { match } from "./flatten";

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
    return Object.prototype.hasOwnProperty.call(bindings, tree) ? bindings[tree] : tree;
  }
  if (Array.isArray(tree)) {
    // index 0 is the operator/head; never a substitutable variable
    return tree.map((t, i) => (i === 0 ? t : substitute(t, bindings)));
  }
  return tree;
}
