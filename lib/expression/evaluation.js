import { get_tree } from "../trees/util.js";
import astToMathjsObj from "../converters/ast-to-mathjs.js";
import astToFiniteFieldObj from "../converters/ast-to-finite-field.js";
import math from "../mathjs.js";
import * as normalize from "./normalization/standard_form.js";
import { remove_units } from "./simplify.js";
import * as trans from "../trees/basic.js";

var astToMathjs = new astToMathjsObj({ mathjs: math });
var astToFiniteField = new astToFiniteFieldObj();

const f = function (expr_or_tree) {
  var tree = get_tree(expr_or_tree);

  var mt = factorial_to_gamma_function(
    astToMathjs.convert(
      normalize.log_subscript_to_two_arg_log(
        normalize.normalize_function_names(
          normalize.normalize_applied_functions(tree),
        ),
      ),
    ),
  );

  return mt.evaluate.bind(mt);
};

const evaluate = function (expr, bindings) {
  return f(expr)(bindings);
};

const finite_field_evaluate = function (expr, bindings, modulus) {
  return astToFiniteField.convert(expr.tree, bindings, modulus);
};

const evaluate_to_constant = function (
  expr_or_tree,
  {
    remove_units_first = true,
    scale_based_on_unit = true,
    nan_for_non_numeric = true,
  } = {},
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

  // if have a negative number x raised to m/n,
  // where m is an integer and n is an odd integer,
  // transform to nthRoot so that it returns the negative real root
  var transformations = [];
  transformations.push([
    ["^", "x", ["/", "m", "n"]],
    ["^", ["apply", "nthRoot", ["tuple", "x", "n"]], "m"],
    {
      variables: {
        x: (v) => v < 0,
        n: (v) => Number.isInteger(v) && Number.isInteger((v - 1) / 2),
        m: (v) => Number.isInteger(v),
      },
    },
  ]);
  transformations.push([
    ["^", "x", ["-", ["/", "m", "n"]]],
    ["^", ["apply", "nthRoot", ["tuple", "x", "n"]], ["-", "m"]],
    {
      variables: {
        x: (v) => v < 0,
        n: (v) => Number.isInteger(v) && Number.isInteger((v - 1) / 2),
        m: (v) => Number.isInteger(v),
      },
    },
  ]);
  tree = trans.applyAllTransformations(tree, transformations, 40);

  var num = nan_for_non_numeric ? NaN : null;
  try {
    var the_f = f(tree);
    num = the_f();
  } catch (e) {}

  if (
    !Number.isNaN(num) &&
    (typeof num === "number" ||
      (typeof num?.re === "number" && typeof num?.im === "number"))
  ) {
    if (
      num.re === Infinity ||
      num.re === -Infinity ||
      num.im === Infinity ||
      num.im === -Infinity
    ) {
      // if start with Infinity*i, result is Infinity+Infinity*i,
      // but if start with Infinity+Infinity*i, result is is NaN+NaN*i
      // To make sure passing through evaluate_to_constant a second time won't change the value,
      // evaluate a second time in this case

      let mathTree;
      if (typeof num?.re === "number" && typeof num?.im === "number") {
        if (num.im === 0) {
          mathTree = num.re;
        } else {
          let imPart;
          if (num.im === 1) {
            imPart = "i";
          } else if (num.im === -1) {
            imPart = ["-", "i"];
          } else {
            imPart = ["*", num.im, "i"];
          }
          if (num.re === 0) {
            mathTree = imPart;
          } else {
            mathTree = ["+", num.re, imPart];
          }
        }
      } else {
        mathTree = num;
      }

      num = nan_for_non_numeric ? NaN : null;
      try {
        var the_f = f(mathTree);
        num = the_f();
      } catch (e) {}
    } else if (num.im === 0) {
      num = num.re;
    }

    // return a numerical value
    return num;
  }

  // did not find a numerical value
  if (typeof num === "object") {
    num = nan_for_non_numeric ? NaN : null;
  }

  return num;
};

function factorial_to_gamma_function(math_tree) {
  // convert factorial to gamma function
  // so that can evaluate at complex numbers
  var transformed = math_tree.transform(function (node, path, parent) {
    if (node.isOperatorNode && node.op === "!" && node.fn === "factorial") {
      var args = [
        new math.OperatorNode("+", "add", [
          node.args[0],
          new math.ConstantNode(1),
        ]),
      ];
      return new math.FunctionNode(new math.SymbolNode("gamma"), args);
    } else {
      return node;
    }
  });
  return transformed;
}

export { f, evaluate, finite_field_evaluate, evaluate_to_constant };
