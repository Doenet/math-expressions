import { get_tree } from '../trees/util';
import astToMathjsObj from '../converters/ast-to-mathjs';
import math from '../mathjs';
import * as normalize from './normalization/standard_form';
import { remove_units } from './simplify';

var astToMathjs = new astToMathjsObj({ mathjs: math });

const f = function (expr_or_tree) {
  var tree = get_tree(expr_or_tree);

  var mt = factorial_to_gamma_function(
    astToMathjs.convert(
      normalize.log_subscript_to_two_arg_log(
        normalize.normalize_function_names(
          normalize.normalize_applied_functions(
            tree
          )
        )
      )
    )
  );

  return mt.evaluate.bind(mt);
};

const evaluate = function (expr, bindings) {
  return f(expr)(bindings);
};

// export const finite_field_evaluate = function(expr, bindings, modulus) {
//     return parser.ast.to.finiteField( expr.tree, modulus )( bindings );
// };

const evaluate_to_constant = function (expr_or_tree,
  { remove_units_first = true, scale_based_on_unit = true, nan_for_non_numeric = true } = {}
) {
  // evaluate to number by converting tree to number
  // and calling without arguments

  // return NaN (or null if nan_for_non_numeric is false) 
  // if couldn't evaluate to constant (e.g., contains a variable)
  // otherwise returns constant (which could be NaN for an expression like 0/0)
  // NOTE: constant could be a math.js complex number object

  var tree = get_tree(expr_or_tree);

  if (remove_units_first) {
    tree = remove_units(tree, scale_based_on_unit);
  }

  if (typeof tree === "number") {
    return tree;
  } else if (typeof tree === "string") {
    if (tree === "pi" && math.define_pi) {
      return Math.PI;
    } else if (tree === "e" && math.define_e) {
      return Math.E;
    } else if (tree === "i" && math.define_i) {
      return { re: 0, im: 1 };
    }
    return nan_for_non_numeric ? NaN : null;
  }

  if (!Array.isArray(tree)) {
    return nan_for_non_numeric ? NaN : null;
  }

  var num = nan_for_non_numeric ? NaN : null;
  try {
    var the_f = f(tree);
    num = the_f();
  }
  catch (e) { };

  return num;
};


function factorial_to_gamma_function(math_tree) {
  // convert factorial to gamma function
  // so that can evaluate at complex numbers
  var transformed = math_tree.transform(function (node, path, parent) {
    if (node.isOperatorNode && node.op === "!" && node.fn === "factorial") {
      var args = [new math.OperatorNode(
        '+', 'add', [node.args[0],
        new math.ConstantNode(1)])];
      return new math.FunctionNode(
        new math.SymbolNode("gamma"), args);
    }
    else {
      return node;
    }
  });
  return transformed;
}


export { f, evaluate, evaluate_to_constant };
