import { equals as standardEquality } from "./equality.js";

const equalWithSignErrors = function (
  expr,
  other,
  { equalityFunction = standardEquality, max_sign_errors = 1 } = {},
) {
  if (equalityFunction(expr, other)) {
    return { matched: true, n_sign_errors: 0 };
  }

  for (let i = 1; i <= max_sign_errors; i++) {
    if (
      equalSpecifiedSignErrors(expr, other, {
        equalityFunction,
        n_sign_errors: i,
      })
    ) {
      return { matched: true, n_sign_errors: i };
    }
  }

  return { matched: false };
};

const equalSpecifiedSignErrors = function (
  expr,
  other,
  { equalityFunction = standardEquality, n_sign_errors = 1 } = {},
) {
  if (n_sign_errors === 0) {
    return equalityFunction(expr, other);
  } else if (!(Number.isInteger(n_sign_errors) && n_sign_errors > 0)) {
    throw Error(
      `Have not implemented equality check with ${n_sign_errors} sign errors.`,
    );
  }

  if (n_sign_errors > 1) {
    let oldEqualityFunction = equalityFunction;
    equalityFunction = function (expr, other) {
      return equalSpecifiedSignErrors(expr, other, {
        equalityFunction: oldEqualityFunction,
        n_sign_errors: n_sign_errors - 1,
      });
    };
  }

  var root = expr.tree;
  var stack = [[root]];
  var pointer = 0;
  var tree;
  var i;

  /* Unfortunately the root is handled separately */
  expr.tree = ["-", root];
  var equals = equalityFunction(expr, other);
  expr.tree = root;

  if (equals) return true;

  while ((tree = stack[pointer++])) {
    tree = tree[0];

    if (!Array.isArray(tree)) {
      continue;
    }

    for (i = 1; i < tree.length; i++) {
      stack.push([tree[i]]);
      tree[i] = ["-", tree[i]];
      equals = equalityFunction(expr, other);
      tree[i] = tree[i][1];

      if (equals) return true;
    }
  }

  return false;
};

export { equalSpecifiedSignErrors, equalWithSignErrors };
