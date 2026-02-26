import { get_tree } from "../trees/util.js";
import * as trees from "../trees/basic.js";
import { variables as variables_in } from "./variables.js";
import * as simplify from "./simplify.js";
import * as elements from "../assumptions/element_of_sets.js";

function solve_linear(expr_or_tree, variable, assumptions) {
  // assume expr is linear in variable

  if (!(typeof variable === "string")) return undefined;

  if (
    assumptions === undefined &&
    expr_or_tree.context !== undefined &&
    expr_or_tree.context.get_assumptions !== undefined
  )
    assumptions = expr_or_tree.context.get_assumptions([
      expr_or_tree.variables(),
    ]);

  var tree = simplify.simplify(get_tree(expr_or_tree), assumptions);
  //var tree = get_tree(expr_or_tree);

  if (!Array.isArray(tree)) return undefined;

  var operator = tree[0];
  var operands = tree.slice(1);

  if (
    !(
      operator === "=" ||
      operator === "ne" ||
      operator === "<" ||
      operator === "le" ||
      operator === ">" ||
      operator === "ge"
    )
  )
    return undefined;

  // set equal to zero, as lhs = 0
  var lhs = simplify.simplify(
    ["+", operands[0], ["-", operands[1]]],
    assumptions,
  );

  var no_var = (tree) => !variables_in(tree).includes(variable);

  // factor out variable
  var transformation = [
    ["+", ["*", "_a", variable], ["*", "_b", variable]],
    ["*", ["+", "_a", "_b"], variable],
    {
      variables: { _a: no_var, _b: no_var },
      allow_permutations: true,
      allow_extended_match: true,
      allow_implicit_identities: ["_a", "_b"],
      evaluate_numbers: true,
    },
  ];

  lhs = simplify.simplify(
    trees.applyAllTransformations(lhs, [transformation], 20),
  );

  if (!variables_in(lhs).includes(variable)) return undefined;

  var pattern = ["+", ["*", "_a", variable], "_b"];

  var params = {
    variables: { _a: no_var, _b: no_var },
    allow_permutations: true,
    allow_implicit_identities: ["_a", "_b"],
  };

  var match = trees.match(lhs, pattern, params);

  if (!match) return undefined; // not linear in variable

  var a = simplify.simplify(match["_a"]);
  var b = simplify.simplify(match["_b"]);

  if (!elements.is_nonzero_ast(a, assumptions)) return undefined; // can't confirm that there is a variable

  // equality or inequality with positive coefficient
  if (
    operator === "=" ||
    operator === "ne" ||
    elements.is_positive_ast(a, assumptions)
  ) {
    let result = simplify.simplify(["/", ["-", b], a]);
    return [operator, variable, result];
  }

  if (!elements.is_negative_ast(a, assumptions)) return undefined; // couldn't determined sign and have inequality

  // have inequality with negative coefficient
  var result = simplify.simplify(["/", ["-", b], a]);
  if (operator === "<") operator = ">";
  else if (operator === "le") operator = "ge";
  else if (operator === ">") operator = "<";
  else operator = "le";

  return [operator, variable, result];
}

export { solve_linear };
