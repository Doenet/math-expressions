// `get_tree` unwraps an expression object to its raw AST, and is what every
// compat entry point that accepts "an Expression or a tree" calls first.
export const get_tree = function (expr_or_tree: any): any {
  if (expr_or_tree === undefined || expr_or_tree === null) return undefined;

  var tree;
  if (expr_or_tree.tree !== undefined) tree = expr_or_tree.tree;
  else tree = expr_or_tree;

  return tree;
};
