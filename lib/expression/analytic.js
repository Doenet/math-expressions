import { get_tree } from "../trees/util.js";
import { operators } from "./variables.js";
import { functions } from "./variables.js";
import {
  normalize_function_names,
  normalize_applied_functions,
  subscripts_to_strings,
} from "./normalization/index.js";

var analytic_operators = [
  "+",
  "-",
  "*",
  "/",
  "^",
  "tuple",
  "vector",
  "altvector",
  "list",
  "array",
  "matrix",
  "interval",
  "vec",
];
var non_analytic_functions = ["abs", "sign", "arg"];

var relation_operators = ["=", "le", "ge", "<", ">"];

function isAnalytic(
  expr_or_tree,
  { allow_abs = false, allow_arg = false, allow_relation = false } = {},
) {
  var tree = normalize_applied_functions(
    normalize_function_names(expr_or_tree),
  );

  tree = subscripts_to_strings(tree);

  var operators_found = operators(tree);
  for (let i = 0; i < operators_found.length; i++) {
    let oper = operators_found[i];
    if (analytic_operators.indexOf(oper) === -1) {
      if (allow_relation) {
        if (relation_operators.indexOf(oper) === -1) {
          return false;
        }
      } else {
        return false;
      }
    }
  }

  var functions_found = functions(tree);
  for (let i = 0; i < functions_found.length; i++) {
    let fun = functions_found[i];
    if (non_analytic_functions.includes(fun)) {
      if (!((allow_abs && fun == "abs") || (allow_arg && fun == "arg"))) {
        return false;
      }
    }
  }

  return true;
}

export { isAnalytic };
