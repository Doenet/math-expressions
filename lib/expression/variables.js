import math from "../mathjs.js";
import { get_tree } from "../trees/util.js";

function leaves(tree, include_subscripts) {
  if (!Array.isArray(tree)) return [tree];

  var operator = tree[0];
  var operands = tree.slice(1);

  if (include_subscripts && operator === "_") {
    if (
      typeof operands[0] === "string" &&
      (typeof operands[1] === "string" || typeof operands[1] === "number")
    )
      return [operands[0] + "_" + operands[1]];
  }

  if (operator === "apply") {
    operands = tree.slice(2);
  }
  if (operands.length === 0) return [];

  return operands
    .map(function (v, i) {
      return leaves(v, include_subscripts);
    })
    .reduce(function (a, b) {
      return a.concat(b);
    });
}

function variables(expr_or_tree, include_subscripts = false) {
  var tree = get_tree(expr_or_tree);

  var result = leaves(tree, include_subscripts);

  result = result.filter(function (v, i) {
    return (
      typeof v === "string" &&
      (math.define_e || v !== "e") &&
      (math.define_pi || v !== "pi") &&
      (math.define_i || v !== "i")
    );
  });

  result = result.filter(function (itm, i, a) {
    return i === result.indexOf(itm);
  });

  return result;
}

function operators_list(tree) {
  if (!Array.isArray(tree)) return [];

  var operator = tree[0];
  var operands = tree.slice(1);

  if (operator === "apply") {
    operands = tree.slice(2);
  }
  if (operands.length === 0) return [operator];

  return [operator].concat(
    operands
      .map(function (v, i) {
        return operators_list(v);
      })
      .reduce(function (a, b) {
        return a.concat(b);
      }),
  );
}

function operators(expr_or_tree) {
  var tree = get_tree(expr_or_tree);

  var result = operators_list(tree);

  result = result.filter(function (v, i) {
    return v !== "apply";
  });

  result = result.filter(function (itm, i, a) {
    return i === result.indexOf(itm);
  });

  return result;
}

function functions_list(tree) {
  if (!Array.isArray(tree)) {
    return [];
  }

  var operator = tree[0];
  var operands = tree.slice(1);

  var functions = [];
  if (operator === "apply") {
    functions = [operands[0]];
    operands = tree.slice(2);
  }

  return functions.concat(
    operands
      .map(function (v, i) {
        return functions_list(v);
      })
      .reduce(function (a, b) {
        return a.concat(b);
      }, []),
  );
}

function functions(expr_or_tree) {
  var tree = get_tree(expr_or_tree);

  var result = functions_list(tree);

  result = result.filter(function (itm, i, a) {
    return i === result.indexOf(itm);
  });

  return result;
}

export { variables, operators, functions };
