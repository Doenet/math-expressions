import { get_tree } from '../trees/util';
import { operators } from './variables';
import { functions } from './variables';
import { normalize_function_names, normalize_applied_functions, subscripts_to_strings } from './normalization';

var analytic_operators = ['+', '-', '*', '/', '^', 'tuple', 'vector', 'list', 'array', 'matrix', 'interval'];
var analytic_functions = ["exp", "log", "log10", "sqrt", "factorial", "gamma", "erf", "acos", "acosh", "acot", "acoth", "acsc", "acsch", "asec", "asech", "asin", "asinh", "atan", "atanh", "cos", "cosh", "cot", "coth", "csc", "csch", "sec", "sech", "sin", "sinh", "tan", "tanh", 'arcsin', 'arccos', 'arctan', 'arccsc', 'arcsec', 'arccot', 'cosec'];
var relation_operators = ['=', 'le', 'ge', '<', '>'];

function isAnalytic(expr_or_tree, { allow_abs = false, allow_relation = false } = {}) {

  var tree = normalize_applied_functions(
    normalize_function_names(expr_or_tree));

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
    if (analytic_functions.indexOf(fun) === -1) {
      if ((!allow_abs) || fun !== "abs")
        return false;
    }
  }

  return true;
}

export { isAnalytic };
