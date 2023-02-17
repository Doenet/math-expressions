import { get_tree } from '../trees/util';
import astToMathjsObj from '../converters/ast-to-mathjs';
import math from '../mathjs';
import * as normalize from './normalization/standard_form';

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

const evaluate_to_constant = function (expr_or_tree) {
  // evaluate to number by converting tree to number
  // and calling without arguments

  // return null if couldn't evaluate to constant (e.g., contains a variable)
  // otherwise returns constant
  // NOTE: constant could be a math.js complex number object

  var tree = get_tree(expr_or_tree);

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
    return null;
  }

  if (!Array.isArray(tree)) {
    return null;
  }

  var num = null;
  try {
    var the_f = f(expr_or_tree);
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
