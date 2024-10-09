import { match as tree_match } from "../trees/basic";
import { get_tree } from "../trees/util";

export const match = function (expr_or_tree, pattern_expr_or_tree, params) {
  let tree = get_tree(expr_or_tree);
  let pattern = get_tree(pattern_expr_or_tree);

  return tree_match(tree, pattern, params);
};
