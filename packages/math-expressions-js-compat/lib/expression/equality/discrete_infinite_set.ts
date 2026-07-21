// Discrete-infinite-set equality. The Rust core exposes this via
// `discrete_infinite_set(...)` free function; a faithful tree-level `equals`
// port is future work, so this throws when used (test fails; suite runs).
export function equals() {
  throw new Error("math-expressions-js-compat: discrete_infinite_set.equals is not implemented");
}
export default { equals };
