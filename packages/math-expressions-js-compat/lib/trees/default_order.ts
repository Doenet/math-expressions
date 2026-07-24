// `default_order` was a standalone per-tree ordering pass. The Rust core folds
// ordering into `canonicalize`, with no separate tree-level entry point, so this
// is a compat stub: it returns the tree unchanged. Specs asserting a specific
// re-ordering will fail here (see JS_TEST_COVERAGE_AUDIT.md); the suite runs.
export function default_order(tree) {
  return tree;
}

export default default_order;
