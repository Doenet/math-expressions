import { default_order } from '../../trees/default_order';
import math from '../../mathjs';
import { get_tree } from '../../trees/util';

var function_normalizations = {
  ln: 'log',
  arccos: 'acos',
  arccosh: 'acosh',
  arcsin: 'asin',
  arcsinh: 'asinh',
  arctan: 'atan',
  arctanh: 'atanh',
  arcsec: 'asec',
  arcsech: 'asech',
  arccsc: 'acsc',
  arccsch: 'acsch',
  arccot: 'acot',
  arccoth: 'acoth',
  cosec: 'csc',
}

var create_trig_inverses_for = ['cos', 'cosh', 'sin', 'sinh', 'tan', 'tanh', 'sec', 'sech', 'csc', 'csch', 'cot', 'coth']

function normalize_function_names(expr_or_tree) {
  // replace "ln" with "log"
  // "arccos" with "acos", etc.
  // e^x with exp(x)
  // sqrt(x) with x^0.5

  var tree = get_tree(expr_or_tree);

  if (!Array.isArray(tree))
    return tree;

  var operator = tree[0];
  var operands = tree.slice(1);

  if (operator === 'apply') {
    if (operands[0] === 'sqrt') {
      return ['^', normalize_function_names(operands[1]), 0.5];
    }

    var result = normalize_function_names_sub(operands[0]);
    result = ['apply', result];

    var args = operands.slice(1).map(function (v) {
      return normalize_function_names(v);
    });

    if (args.length > 1)
      args = ['tuple'].concat(args);
    else
      args = args[0];

    result.push(args);

    return result;
  }

  if (operator === '^' && operands[0] === 'e' && math.define_e)
    return ['apply', 'exp', normalize_function_names(operands[1])];

  return [operator].concat(operands.map(function (v) {
    return normalize_function_names(v)
  }));
}

function normalize_function_names_sub(tree) {

  if (typeof tree === 'string') {
    if (tree in function_normalizations)
      return function_normalizations[tree];
    return tree;
  }

  if (!Array.isArray(tree))
    return tree;

  var operator = tree[0];
  var operands = tree.slice(1);

  if (operator === "^" && operands.length === 2 && operands[1] === -1) {
    let operand0 = normalize_function_names_sub(operands[0]);

    if(create_trig_inverses_for.includes(operand0)) {
      return 'a' + operand0;
    }
  }

  var result = [operator].concat(operands.map(function (v) {
    return normalize_function_names_sub(v);
  }));

  return result;
}



function normalize_applied_functions(expr_or_tree) {
  // normalize applied functions
  // so that primes and powers occur outside function application
  // with the exception of an exponent of -1

  var tree = get_tree(expr_or_tree);

  if (!Array.isArray(tree))
    return tree;

  var operator = tree[0];
  var operands = tree.slice(1);

  if (operator === 'apply') {
    let result = strip_function_names(operands[0]);
    let f_applied;
    if(result.exponent === -1) {
      f_applied = ['apply', ['^', result.tree, -1], operands[1]]
    } else {
      f_applied = ['apply', result.tree, operands[1]];
    }
    for (let i = 0; i < result.n_primes; i++)
      f_applied = ['prime', f_applied];

    if (result.exponent !== undefined && result.exponent !== -1)
      f_applied = ['^', f_applied, result.exponent];

    return f_applied
  }

  var result = [operator].concat(operands.map(function (v, i) { return normalize_applied_functions(v); }));
  return result;
}


function strip_function_names(tree) {
  // strip primes and powers off tree

  if (!Array.isArray(tree))
    return { tree: tree, n_primes: 0 };

  var operator = tree[0];
  var operands = tree.slice(1);


  if (operator === '^') {
    let result = strip_function_names(operands[0]);
    let exponent = normalize_applied_functions(operands[1]);

    result.exponent = exponent;
    return result;
  }

  if (operator === "prime") {
    let result = strip_function_names(operands[0]);
    result.n_primes += 1;
    return result;
  }

  return { tree: normalize_applied_functions(tree), n_primes: 0 };
}


function substitute_abs(expr_or_tree) {

  var tree = get_tree(expr_or_tree);

  if (!Array.isArray(tree))
    return tree;

  var operator = tree[0];
  var operands = tree.slice(1);

  if (operator === "apply" && operands[0] === 'abs') {
    return ['^', ['^', substitute_abs(operands[1]), 2], 0.5];
  }

  return [operator].concat(operands.map(function (v) {
    return substitute_abs(v);
  }));
}


function constants_to_floats(expr_or_tree) {

  var tree = get_tree(expr_or_tree);

  if (!(math.define_e || math.define_pi)) {
    return tree;
  }
  if (typeof tree === "string") {
    if (tree === "e") {
      if (math.define_e) {
        return math.e;
      }
    } else if (tree === "pi") {
      if (math.define_pi) {
        return math.pi;
      }
    }
    return tree;
  }

  if (!Array.isArray(tree)) {
    return tree;
  }

  let operator = tree[0];
  let operands = tree.slice(1);

  // don't convert exponential function
  if (operator === "^" && operands[0] === "e") {
    return ["^", "e", constants_to_floats(operands[1])];
  }

  return [operator, ...operands.map(constants_to_floats)]

}

export {
  normalize_function_names, normalize_applied_functions,
  substitute_abs, default_order, constants_to_floats
};
