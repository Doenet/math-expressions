import * as flatten from "../trees/flatten";
import { default_order } from "../trees/default_order";
import { is_nonzero_ast as is_nonzero } from "../assumptions/element_of_sets.js";
import { is_positive_ast as is_positive } from "../assumptions/element_of_sets.js";
import { is_negative_ast as is_negative } from "../assumptions/element_of_sets.js";
import math from "../mathjs";
import * as trans from "../trees/basic.js";
import { get_tree } from "../trees/util";
import { evaluate_to_constant } from "./evaluation";
import {
  perform_matrix_scalar_multiplications,
  perform_vector_scalar_multiplications,
} from "./transformation";
import {
  get_all_units,
  get_unit_of_tree,
  get_unit_value_of_tree,
} from "./units";

function clean(expr_or_tree) {
  var tree = get_tree(expr_or_tree);
  return flatten.flatten(tree);
}

function evalf(x, n) {
  return parseFloat(math.format(x, { notation: "exponential", precision: n }));
}

function collapse_unary_minus(expr_or_tree) {
  var tree = get_tree(expr_or_tree);

  if (!Array.isArray(tree)) return tree;

  var operator = tree[0];
  var operands = tree.slice(1);
  operands = operands.map((v) => collapse_unary_minus(v));

  if (operator === "-") {
    if (typeof operands[0] === "number") return -operands[0];
    // check if operand is a multiplication with that begins with
    // a constant.  If so combine with constant
    if (
      Array.isArray(operands[0]) &&
      operands[0][0] === "*" &&
      typeof operands[0][1] === "number"
    ) {
      return ["*", -operands[0][1]].concat(operands[0].slice(2));
    }
    // check if operand is a division with that begins with
    // either
    /// (A) a constant or
    //  (B) a multiplication that begins with a constant.
    // If so. combine with constant
    if (Array.isArray(operands[0]) && operands[0][0] === "/") {
      if (typeof operands[0][1] === "number")
        return ["/", -operands[0][1], operands[0][2]];
      if (
        Array.isArray(operands[0][1]) &&
        operands[0][1][0] === "*" &&
        typeof operands[0][1][1] === "number"
      ) {
        return [
          "/",
          ["*", -operands[0][1][1]].concat(operands[0][1].slice(2)),
          operands[0][2],
        ];
      }
    }
  }

  return [operator].concat(operands);
}

function simplify(expr_or_tree, assumptions, max_digits) {
  var tree = get_tree(expr_or_tree);

  if (contains_blank(tree)) {
    return tree;
  }

  if (
    assumptions === undefined &&
    expr_or_tree.context !== undefined &&
    expr_or_tree.context.get_assumptions !== undefined
  )
    assumptions = expr_or_tree.context.get_assumptions([
      expr_or_tree.variables(),
    ]);

  tree = evaluate_numbers(tree, {
    assumptions: assumptions,
    max_digits: max_digits,
    evaluate_functions: true,
  });
  // if already have it down to a number or variable, no need for more simplification
  if (!Array.isArray(tree)) {
    return tree;
  }
  tree = simplify_logical(tree, assumptions);
  tree = perform_vector_matrix_additions_scalar_multiplications(tree);
  tree = simplify_integer_square_roots(tree);
  tree = collect_like_terms_factors(tree, assumptions, max_digits);

  return tree;
}

function simplify_logical(expr_or_tree, assumptions) {
  var tree = get_tree(expr_or_tree);

  if (contains_blank(tree)) {
    return tree;
  }

  if (
    assumptions === undefined &&
    expr_or_tree.context !== undefined &&
    expr_or_tree.context.get_assumptions !== undefined
  )
    assumptions = expr_or_tree.context.get_assumptions([
      expr_or_tree.variables(),
    ]);

  tree = evaluate_numbers(tree, { assumptions: assumptions });

  tree = flatten.unflattenRight(tree);

  var transformations = [];
  transformations.push([["not", ["not", "a"]], "a"]);
  transformations.push([
    ["not", ["and", "a", "b"]],
    ["or", ["not", "a"], ["not", "b"]],
  ]);
  transformations.push([
    ["not", ["or", "a", "b"]],
    ["and", ["not", "a"], ["not", "b"]],
  ]);
  transformations.push([
    ["not", ["=", "a", "b"]],
    ["ne", "a", "b"],
  ]);
  transformations.push([
    ["not", ["ne", "a", "b"]],
    ["=", "a", "b"],
  ]);
  transformations.push([
    ["not", ["<", "a", "b"]],
    ["le", "b", "a"],
  ]);
  transformations.push([
    ["not", ["le", "a", "b"]],
    ["not", ["le", "a", "b"]],
  ]);
  transformations.push([
    ["not", ["in", "a", "b"]],
    ["notin", "a", "b"],
  ]);
  transformations.push([
    ["not", ["subset", "a", "b"]],
    ["notsubset", "a", "b"],
  ]);

  tree = trans.applyAllTransformations(tree, transformations, 20);

  tree = flatten.flatten(tree);

  return tree;
}

function simplify_integer_square_roots(tree) {
  if (!Array.isArray(tree)) {
    return tree;
  }

  var operator = tree[0];
  var operands = tree.slice(1);

  if (
    operator === "apply" &&
    operands[0] === "sqrt" &&
    Number.isInteger(operands[1])
  ) {
    // From https://stackoverflow.com/a/10492893

    let n_inside = operands[1];
    let n_outside = 1;
    let d = 2;
    let d2 = 4;
    while (d2 <= Math.abs(n_inside)) {
      if (n_inside % d2 === 0) {
        n_inside /= d2;
        n_outside *= d;
      } else {
        d++;
        d2 = d * d;
      }
    }

    if (n_outside > 1) {
      return ["*", n_outside, ["apply", "sqrt", n_inside]];
    } else {
      return tree;
    }
  }

  return [operator, ...operands.map(simplify_integer_square_roots)];
}

function perform_vector_matrix_additions_scalar_multiplications(
  expr_or_tree,
  include_tuples = true,
) {
  var tree = get_tree(expr_or_tree);

  tree = perform_matrix_scalar_multiplications(tree);

  tree = perform_vector_scalar_multiplications(tree, include_tuples);

  let pattern = ["+", "a", "b"];
  tree = trans.transform(tree, function (subtree) {
    let matchResults = trans.match(subtree, pattern, {
      allow_permutations: true,
    });
    if (matchResults) {
      let vectorOperators = ["vector", "altvector"];
      if (include_tuples) {
        vectorOperators.push("tuple");
      }

      let vectorAddendsByLength = {};
      let matrixAddendsBySize = {};
      let nonMatrixVectorAddends = [];
      if (vectorOperators.includes(matchResults.a[0])) {
        let n = matchResults.a.length - 1;
        if (!vectorAddendsByLength[n]) {
          vectorAddendsByLength[n] = [];
        }
        vectorAddendsByLength[n].push(matchResults.a);
      } else if (matchResults.a[0] === "matrix") {
        let size = matchResults.a[1].slice(1).toString();
        if (!matrixAddendsBySize[size]) {
          matrixAddendsBySize[size] = [];
        }
        matrixAddendsBySize[size].push(matchResults.a);
      } else {
        nonMatrixVectorAddends.push(matchResults.a);
      }
      if (vectorOperators.includes(matchResults.b[0])) {
        let n = matchResults.b.length - 1;
        if (!vectorAddendsByLength[n]) {
          vectorAddendsByLength[n] = [];
        }
        vectorAddendsByLength[n].push(matchResults.b);
      } else if (matchResults.b[0] === "matrix") {
        let size = matchResults.b[1].slice(1).toString();
        if (!matrixAddendsBySize[size]) {
          matrixAddendsBySize[size] = [];
        }
        matrixAddendsBySize[size].push(matchResults.b);
      } else if (matchResults.b[0] === "+") {
        for (let addend of matchResults.b.slice(1)) {
          if (vectorOperators.includes(addend[0])) {
            let n = addend.length - 1;
            if (!vectorAddendsByLength[n]) {
              vectorAddendsByLength[n] = [];
            }
            vectorAddendsByLength[n].push(addend);
          } else if (addend[0] === "matrix") {
            let size = addend[1].slice(1).toString();
            if (!matrixAddendsBySize[size]) {
              matrixAddendsBySize[size] = [];
            }
            matrixAddendsBySize[size].push(addend);
          } else {
            nonMatrixVectorAddends.push(addend);
          }
        }
      } else {
        nonMatrixVectorAddends.push(matchResults.b);
      }

      if (
        Object.values(vectorAddendsByLength).every((x) => x.length < 2) &&
        Object.values(matrixAddendsBySize).every((x) => x.length < 2)
      ) {
        return subtree;
      }

      let newAddends = nonMatrixVectorAddends;

      for (let n in vectorAddendsByLength) {
        if (vectorAddendsByLength[n].length < 2) {
          newAddends.push(...vectorAddendsByLength[n]);
        } else {
          let foundVector = vectorAddendsByLength[n].some(
            (x) => x[0] === "vector",
          );
          let foundTuple = vectorAddendsByLength[n].some(
            (x) => x[0] === "tuple",
          );
          let foundAltVector = vectorAddendsByLength[n].some(
            (x) => x[0] === "altvector",
          );

          let newVector;
          if (foundVector || (foundTuple && foundAltVector)) {
            newVector = ["vector"];
          } else if (foundAltVector) {
            newVector = ["altvector"];
          } else {
            newVector = ["tuple"];
          }

          for (let i = 0; i < n; i++) {
            newVector.push([
              "+",
              ...vectorAddendsByLength[n].map((x) => x[i + 1]),
            ]);
          }

          newAddends.push(newVector);
        }
      }

      for (let size in matrixAddendsBySize) {
        if (matrixAddendsBySize[size].length < 2) {
          newAddends.push(...matrixAddendsBySize[size]);
        } else {
          let [m, n] = size.split(",").map(Number);

          let matrixData = ["tuple"];
          for (let i = 0; i < m; i++) {
            let row = ["tuple"];
            for (let j = 0; j < n; j++) {
              row.push([
                "+",
                ...matrixAddendsBySize[size].map((x) => x[2][i + 1][j + 1]),
              ]);
            }
            matrixData.push(row);
          }

          let newMatrix = ["matrix", ["tuple", m, n], matrixData];

          newAddends.push(newMatrix);
        }
      }

      if (newAddends.length === 1) {
        return newAddends[0];
      } else {
        return ["+", ...newAddends];
      }
    } else {
      return subtree;
    }
  });

  return tree;
}

function contains_decimal_number(tree) {
  if (typeof tree === "string") {
    return false;
  }
  if (typeof tree === "number") {
    if (Number.isFinite(tree) && !Number.isInteger(tree)) {
      return true;
    } else {
      return false;
    }
  }
  if (!Array.isArray(tree)) {
    return false;
  }
  return tree.slice(1).some((x) => contains_decimal_number(x));
}

function contains_only_numbers(
  tree,
  { include_number_symbols = false, evaluate_functions = false } = {},
) {
  if (typeof tree === "string") {
    if (include_number_symbols) {
      if (tree === "e" && math.define_e) {
        return true;
      }
      if (tree === "pi" && math.define_pi) {
        return true;
      }
    }
    if (tree === "i" && math.define_i) {
      return true;
    }
    return false;
  }
  if (typeof tree === "number") {
    return true;
  }
  if (!Array.isArray(tree)) {
    return false;
  }
  if (evaluate_functions && tree[0] === "apply") {
    return tree.slice(2).every((x) =>
      contains_only_numbers(x, {
        include_number_symbols: include_number_symbols,
      }),
    );
  }
  return tree.slice(1).every((x) =>
    contains_only_numbers(x, {
      include_number_symbols: include_number_symbols,
    }),
  );
}

function try_evaluate_sum_of_numbers(left, right, skip_ordering) {
  if (typeof left === "number") {
    if (left === 0) return right;
    if (typeof right === "number") return left + right;
    // check if right is an addition with that begins with
    // a constant.  If so combine with left
    if (
      Array.isArray(right) &&
      right[0] === "+" &&
      typeof right[1] === "number"
    ) {
      return ["+", left + right[1], right[2]];
    }
    // check if right is an addition with that ends with
    // a constant.  If so combine with left
    if (
      !skip_ordering &&
      Array.isArray(right) &&
      right[0] === "+" &&
      typeof right[2] === "number"
    ) {
      return ["+", left + right[2], right[1]];
    }
  }
  if (typeof right === "number") if (right === 0) return left;

  return null;
}

function try_evaluate_negate_number(tree) {
  if (typeof tree === "number") return -tree;
  // check if operand is a multiplication with that begins with
  // a constant.  If so combine with constant
  if (Array.isArray(tree) && tree[0] === "*" && typeof tree[1] === "number") {
    return ["*", -tree[1]].concat(tree.slice(2));
  }
  // check if operand is a division with that begins with
  // either
  /// (A) a constant or
  //  (B) a multiplication that begins with a constant.
  // If so. combine with constant
  if (Array.isArray(tree) && tree[0] === "/") {
    if (typeof tree[1] === "number") return ["/", -tree[1], tree[2]];
    if (
      Array.isArray(tree[1]) &&
      tree[1][0] === "*" &&
      typeof tree[1][1] === "number"
    ) {
      return ["/", ["*", -tree[1][1]].concat(tree[1].slice(2)), tree[2]];
    }
  }

  return null;
}

function try_evaluate_product_of_numbers(left, right) {
  if (typeof left === "number") {
    if (isNaN(left)) return NaN;

    if (typeof right === "number") return left * right;

    if (!isFinite(left)) {
      if (
        (left === Infinity && is_negative(right)) ||
        (left === -Infinity && is_positive(right))
      )
        return -Infinity;
      if (is_nonzero(right) === false) return NaN;
      return Infinity;
    }
    if (left === 0) {
      return 0;
    }
    if (left === 1) return right;

    if (left === -1) {
      return ["-", right];
    }
    // check if right is a multiplication with that begins with
    // a constant.  If so combine with left
    if (
      Array.isArray(right) &&
      right[0] === "*" &&
      typeof right[1] === "number"
    ) {
      left = left * right[1];
      right = right[2];
      if (left === 1) return right;
      if (left === -1) return ["-", right];
      return ["*", left, right];
    }
  }
  if (typeof right === "number") {
    if (isNaN(right)) return NaN;
    if (!isFinite(right)) {
      if (
        (right === Infinity && is_negative(left)) ||
        (right === -Infinity && is_positive(left))
      )
        return -Infinity;
      if (is_nonzero(left) === false) return NaN;
      return Infinity;
    }
    if (right === 0) {
      return 0;
    }
    if (right === 1) return left;
    if (right === -1) {
      return ["-", left];
    }
    // check if left is a multiplication with that begins with
    // a constant.  If so combine with right
    if (Array.isArray(left) && left[0] === "*" && typeof left[1] === "number") {
      right = right * left[1];
      left = left[2];
      if (right === 1) return left;
      if (right === -1) return ["-", left];
      return ["*", left, right];
    }
  }

  if (math.define_i && left === "i") {
    if (right === "i") {
      return -1;
    }

    // check if right is a multiplication with that begins with i
    // If so combine with left
    if (Array.isArray(right) && right[0] === "*" && right[1] === "i") {
      return ["-", right[2]];
    }
  } else if (math.define_i && right === "i") {
    // check if left is a multiplication with that begins with i
    // a constant.  If so combine with right
    if (Array.isArray(left) && left[0] === "*" && left[1] === "i") {
      return ["-", left[2]];
    }
  }

  return null;
}

function try_evaluate_quotient_of_numbers(
  numer,
  denom,
  assumptions,
  max_digits,
) {
  if (typeof numer === "number") {
    if (numer === 0) {
      let denom_nonzero = is_nonzero(denom, assumptions);
      if (denom_nonzero) return 0;
      if (denom_nonzero === false) return NaN; // 0/0
    }

    if (typeof denom === "number") {
      let quotient = numer / denom;
      if (
        max_digits === Infinity ||
        math.round(quotient, max_digits) === quotient
      )
        return quotient;
      else if (denom < 0) return ["/", -numer, -denom];
    }

    // check if denom is a multiplication with that begins with
    // a constant.  If so combine with numerator
    if (
      Array.isArray(denom) &&
      denom[0] === "*" &&
      typeof denom[1] === "number"
    ) {
      let quotient = numer / denom[1];

      if (
        max_digits === Infinity ||
        math.round(quotient, max_digits) === quotient
      ) {
        return ["/", quotient, denom[2]];
      }
    }
  } else if (typeof denom === "number") {
    // check if numer is a multiplication with that begins with
    // a constant.  If so combine with denominator
    if (
      Array.isArray(numer) &&
      numer[0] === "*" &&
      typeof numer[1] === "number"
    ) {
      let quotient = numer[1] / denom;
      if (
        max_digits === Infinity ||
        math.round(quotient, max_digits) === quotient
      ) {
        if (quotient === 1) return numer[2];
        else return ["*", quotient, numer[2]];
      }
      // if denom is negative move negative to number
      if (denom < 0) return ["/", ["*", -numer[1], numer[2]], -denom];
    }
    let reciprocal = 1 / denom;
    if (
      max_digits === Infinity ||
      math.round(reciprocal, max_digits) === reciprocal
    ) {
      return ["*", reciprocal, numer];
    }
    // if denominator is negative, negate whole fraction
    if (denom < 0) {
      if (Array.isArray(numer) && numer[0] === "-")
        return ["/", numer[1], -denom];
      else return ["-", ["/", numer, -denom]];
    }
  }

  return null;
}

function set_small_zero(expr_or_tree, threshold = 1e-14) {
  var tree = get_tree(expr_or_tree);

  if (typeof tree === "number") {
    if (math.abs(tree) < threshold) {
      return 0;
    } else {
      return tree;
    }
  }

  if (!Array.isArray(tree)) {
    return tree;
  }

  return [tree[0], ...tree.slice(1).map((v) => set_small_zero(v, threshold))];
}

function evaluate_numbers_sub(
  tree,
  assumptions,
  max_digits,
  skip_ordering,
  evaluate_functions,
  set_small_zero,
) {
  // assume that tree has been sorted to default order (while flattened)
  // and then unflattened_right
  // returns unflattened tree

  if (tree === undefined) return tree;

  if (typeof tree === "number") {
    if (set_small_zero > 0 && math.abs(tree) < set_small_zero) {
      return 0;
    }
    return tree;
  }

  if (
    contains_only_numbers(tree, {
      include_number_symbols: true,
      evaluate_functions,
    })
  ) {
    let simplify_number = function (c) {
      if (set_small_zero > 0 && math.abs(c) < set_small_zero) {
        return 0;
      }
      if (max_digits === Infinity) return c;
      if (Number.isInteger(c)) {
        return c;
      }

      let c_minround = evalf(c, 14);
      let c_round = evalf(c, max_digits);
      if (max_digits === 0) {
        // interpret 0 max_digits as only accepting integers
        // (even though positive max_digits is number of significant digits)
        c_round = math.round(c);
      }
      if (c_round === c_minround) {
        return c;
      }

      // if expression already contained a decimal,
      // and contains only numbers (no constants like pi)
      // return the number
      if (
        contains_decimal_number(tree) &&
        contains_only_numbers(tree, {
          include_number_symbols: true,
          evaluate_functions,
        })
      ) {
        return c;
      }

      let c_frac_pre = math.fraction(c);
      let c_frac = {
        n: math.number(c_frac_pre.n),
        d: math.number(c_frac_pre.d),
        s: math.number(c_frac_pre.s),
      };
      let c_frac_d_round = evalf(c_frac.d, 3);

      if (c_frac.n < 1e4 || c_frac_d_round === c_frac.d) {
        let c_reconstruct = evalf((c_frac.s * c_frac.n) / c_frac.d, 14);
        if (c_reconstruct === c_minround) {
          if (c_frac.d === 1) {
            return c_frac.s * c_frac.n;
          } else {
            return ["/", c_frac.s * c_frac.n, c_frac.d];
          }
        }
      }

      return null;
    };

    var c = evaluate_to_constant(tree, { remove_units_first: false });

    if (!Number.isNaN(c)) {
      if (typeof c === "number") {
        if (Number.isFinite(c)) {
          let n = simplify_number(c);
          if (n !== null) {
            return n;
          }
        }
      } else if (c && Number.isFinite(c.re) && Number.isFinite(c.im)) {
        let re = simplify_number(c.re);
        let im = simplify_number(c.im);

        if (re !== null && im !== null) {
          if (im === 0) {
            return re;
          } else {
            let imPart;
            if (im === 1) {
              imPart = "i";
            } else if (im === -1) {
              imPart = ["-", "i"];
            } else {
              imPart = ["*", im, "i"];
            }
            if (re === 0) {
              return imPart;
            } else {
              return ["+", re, imPart];
            }
          }
        }
      }
    }
  }

  if (!Array.isArray(tree)) return tree;

  var operator = tree[0];
  var operands = tree
    .slice(1)
    .map((v) =>
      evaluate_numbers_sub(
        v,
        assumptions,
        max_digits,
        skip_ordering,
        evaluate_functions,
        set_small_zero,
      ),
    );

  if (operator === "+") {
    let left = operands[0];
    let right = operands[1];

    if (right === undefined) return left;

    let result = try_evaluate_sum_of_numbers(left, right, skip_ordering);

    if (result !== null) {
      return result;
    }

    if (left[0] === "unit" && right[0] === "unit") {
      let left_unit = get_unit_of_tree(left);
      let right_unit = get_unit_of_tree(right);

      if (left_unit && left_unit === right_unit) {
        let left_without_unit = remove_units(left, false);
        let right_without_unit = remove_units(right, false);

        let result = try_evaluate_sum_of_numbers(
          left_without_unit,
          right_without_unit,
          skip_ordering,
        );

        if (result !== null) {
          return add_unit(result, left_unit);
        }
      }
    }

    return [operator].concat(operands);
  }
  if (operator === "-") {
    let result = try_evaluate_negate_number(operands[0]);

    if (result !== null) {
      return result;
    }

    if (operands[0][0] === "unit") {
      let unit = get_unit_of_tree(operands[0]);

      if (unit) {
        let without_unit = remove_units(operands[0], false);

        let result = try_evaluate_negate_number(without_unit);

        if (result !== null) {
          return add_unit(result, unit);
        }
      }
    }

    return [operator].concat(operands);
  }
  if (operator === "*") {
    let left = operands[0];
    let right = operands[1];

    if (right === undefined) return left;

    let result = try_evaluate_product_of_numbers(left, right);

    if (result !== null) {
      return result;
    }

    if (left[0] === "unit" && right[0] !== "unit") {
      let unit = get_unit_of_tree(left);

      if (unit) {
        let left_without_unit = remove_units(left, false);

        let result = try_evaluate_product_of_numbers(left_without_unit, right);

        if (result !== null) {
          return add_unit(result, unit);
        }
      }
    }

    if (left[0] !== "unit" && right[0] === "unit") {
      let unit = get_unit_of_tree(right);

      if (unit) {
        let right_without_unit = remove_units(right, false);

        let result = try_evaluate_product_of_numbers(left, right_without_unit);

        if (result !== null) {
          return add_unit(result, unit);
        }
      }
    }

    return [operator].concat(operands);
  }

  if (operator === "/") {
    let numer = operands[0];
    let denom = operands[1];

    let result = try_evaluate_quotient_of_numbers(
      numer,
      denom,
      assumptions,
      max_digits,
    );

    if (result !== null) {
      return result;
    }

    if (numer[0] === "unit" && denom[0] !== "unit") {
      let unit = get_unit_of_tree(numer);

      if (unit) {
        let numer_without_unit = remove_units(numer, false);

        let result = try_evaluate_quotient_of_numbers(
          numer_without_unit,
          denom,
          assumptions,
          max_digits,
        );

        if (result !== null) {
          return add_unit(result, unit);
        }
      }
    }

    return [operator].concat(operands);
  }

  if (operator === "^") {
    let base = operands[0];
    let pow = operands[1];

    if (typeof pow === "number") {
      if (pow === 0) {
        if (!math.pow_strict) return 1;
        let base_nonzero = is_nonzero(base, assumptions);
        if (base_nonzero && base !== Infinity && base !== -Infinity) return 1;
        if (base_nonzero === false) return NaN; // 0^0
      } else if (pow === 1) {
        return base;
      } else if (typeof base === "number") {
        let result = math.pow(base, pow);
        if (
          max_digits === Infinity ||
          math.round(result, max_digits) === result
        )
          return result;
      }
    } else if (base === 1) {
      return 1;
    }
    return [operator].concat(operands);
  }

  return [operator].concat(operands);
}

function evaluate_numbers(
  expr_or_tree,
  {
    assumptions,
    max_digits,
    skip_ordering = false,
    evaluate_functions = false,
    set_small_zero = 0,
  } = {},
) {
  if (
    max_digits === undefined ||
    !(Number.isInteger(max_digits) || max_digits === Infinity)
  )
    max_digits = 0;

  if (set_small_zero === true) {
    set_small_zero = 1e-14;
  }

  var tree = get_tree(expr_or_tree);

  if (contains_blank(tree)) {
    return tree;
  }

  if (
    assumptions === undefined &&
    expr_or_tree.context !== undefined &&
    expr_or_tree.context.get_assumptions !== undefined
  )
    assumptions = expr_or_tree.context.get_assumptions([
      expr_or_tree.variables(),
    ]);

  var result;
  if (skip_ordering) {
    tree = flatten.unflattenRight(flatten.flatten(tree));
    result = evaluate_numbers_sub(
      tree,
      assumptions,
      max_digits,
      skip_ordering,
      evaluate_functions,
      set_small_zero,
    );
  } else {
    tree = move_units_outside_products(tree);
    tree = flatten.unflattenRight(default_order(flatten.flatten(tree)));
    result = default_order(
      evaluate_numbers_sub(
        tree,
        assumptions,
        max_digits,
        skip_ordering,
        evaluate_functions,
        set_small_zero,
      ),
    );
    // TODO: determine how often have to repeat
    result = default_order(
      evaluate_numbers_sub(
        flatten.unflattenRight(result),
        assumptions,
        max_digits,
        skip_ordering,
        evaluate_functions,
        set_small_zero,
      ),
    );
  }

  result = set_negative_zeros_to_zero(result);

  return flatten.flatten(result);
}

function set_negative_zeros_to_zero(tree) {
  if (tree === 0) {
    // so -0 is 0
    return 0;
  }

  if (Array.isArray(tree)) {
    return [tree[0], ...tree.slice(1).map(set_negative_zeros_to_zero)];
  } else {
    return tree;
  }
}

function collect_like_terms_factors(expr_or_tree, assumptions, max_digits) {
  function isNumber(s) {
    if (typeof s === "number") return true;
    if (Array.isArray(s) && s[0] === "-" && typeof s[1] === "number")
      return true;
    return false;
  }
  function isNegativeNumber(s) {
    if (typeof s === "number" && s < 0) return true;
    if (Array.isArray(s) && s[0] === "-" && typeof s[1] === "number")
      return true;
    return false;
  }
  function isNumerical(s) {
    if (typeof s === "number") return true;
    if (Array.isArray(s) && s[0] === "-" && typeof s[1] === "number")
      return true;
    var c = evaluate_to_constant(s, { remove_units_first: false });
    if (Number.isFinite(c)) return true;

    return false;
  }

  var tree = get_tree(expr_or_tree);

  if (contains_blank(tree)) {
    return tree;
  }

  if (
    assumptions === undefined &&
    expr_or_tree.context !== undefined &&
    expr_or_tree.context.get_assumptions !== undefined
  )
    assumptions = expr_or_tree.context.get_assumptions([
      expr_or_tree.variables(),
    ]);

  tree = evaluate_numbers(tree, {
    assumptions: assumptions,
    max_digits: max_digits,
    evaluate_functions: true,
  });

  var transformations = [];

  // preliminary transformations
  transformations.push([
    ["/", "x", ["^", "y", "a"]],
    ["*", "x", ["^", "y", ["-", "a"]]],
    { evaluate_numbers: true, max_digits: max_digits },
  ]);
  transformations.push([
    ["/", "x", ["apply", "exp", "a"]],
    ["*", "x", ["apply", "exp", ["-", "a"]]],
    {
      evaluate_numbers: true,
      max_digits: max_digits,
      variables: { x: true, a: true },
    },
  ]);
  transformations.push([
    ["/", "x", "y"],
    ["*", "x", ["^", "y", ["-", 1]]],
    { evaluate_numbers: true, max_digits: max_digits },
  ]);
  tree = trans.applyAllTransformations(tree, transformations, 40);

  // collecting like terms and factors
  transformations = [];

  for (let unit in all_units) {
    let unit_object = all_units[unit];

    if (unit_object.prefix) {
      transformations.push([
        ["+", ["unit", unit, "x"], ["unit", unit, "y"]],
        ["unit", unit, ["+", "x", "y"]],
        {
          variables: {
            x: true,
            y: true,
          },
          evaluate_numbers: true,
          max_digits: max_digits,
          allow_extended_match: true,
          allow_permutations: true,
          max_group: 1,
        },
      ]);
      transformations.push([
        ["+", ["unit", unit, "x"], ["-", ["unit", unit, "y"]]],
        ["unit", unit, ["+", "x", ["-", "y"]]],
        {
          variables: {
            x: true,
            y: true,
          },
          evaluate_numbers: true,
          max_digits: max_digits,
          allow_extended_match: true,
          allow_permutations: true,
          max_group: 1,
        },
      ]);
    } else {
      transformations.push([
        ["+", ["unit", "x", unit], ["unit", "y", unit]],
        ["unit", ["+", "x", "y"], unit],
        {
          variables: {
            x: true,
            y: true,
          },
          evaluate_numbers: true,
          max_digits: max_digits,
          allow_extended_match: true,
          allow_permutations: true,
          max_group: 1,
        },
      ]);
      transformations.push([
        ["+", ["unit", "x", unit], ["-", ["unit", "y", unit]]],
        ["unit", ["+", "x", ["-", "y"]], unit],
        {
          variables: {
            x: true,
            y: true,
          },
          evaluate_numbers: true,
          max_digits: max_digits,
          allow_extended_match: true,
          allow_permutations: true,
          max_group: 1,
        },
      ]);
    }
  }

  transformations.push([
    ["*", ["^", "x", "n"], ["^", "x", "m"]],
    ["^", "x", ["+", "n", "m"]],
    {
      variables: {
        x: (v) => is_nonzero(v, assumptions),
        n: isNumber,
        m: isNumber,
      },
      evaluate_numbers: true,
      max_digits: max_digits,
      allow_implicit_identities: ["m", "n"],
      allow_extended_match: true,
      allow_permutations: true,
      max_group: 1,
    },
  ]);
  transformations.push([
    ["*", ["^", "x", "n"], ["^", "x", "m"]],
    ["^", "x", ["+", "n", "m"]],
    {
      variables: {
        x: true,
        n: (v) => isNumber(v) && is_positive(v, assumptions),
        m: (v) => isNumber(v) && is_positive(v, assumptions),
      },
      evaluate_numbers: true,
      max_digits: max_digits,
      allow_implicit_identities: ["m", "n"],
      allow_extended_match: true,
      allow_permutations: true,
      max_group: 1,
    },
  ]);
  transformations.push([
    ["*", ["^", "x", "n"], ["^", "x", "m"]],
    ["^", "x", ["+", "n", "m"]],
    {
      variables: {
        x: true,
        n: (v) => isNumber(v) && is_negative(v, assumptions),
        m: (v) => isNumber(v) && is_negative(v, assumptions),
      },
      evaluate_numbers: true,
      max_digits: max_digits,
      allow_extended_match: true,
      allow_permutations: true,
      max_group: 1,
    },
  ]);
  transformations.push([
    ["*", ["apply", "exp", "n"], ["apply", "exp", "m"]],
    ["apply", "exp", ["+", "n", "m"]],
    {
      variables: {
        n: isNumber,
        m: isNumber,
      },
      evaluate_numbers: true,
      max_digits: max_digits,
      allow_implicit_identities: ["m", "n"],
      allow_extended_match: true,
      allow_permutations: true,
      max_group: 1,
    },
  ]);
  transformations.push([
    ["+", ["*", "n", "x"], ["*", "m", "x"]],
    ["*", ["+", "n", "m"], "x"],
    {
      variables: {
        x: true,
        n: isNumber,
        m: isNumber,
      },
      evaluate_numbers: true,
      max_digits: max_digits,
      allow_implicit_identities: ["m", "n"],
      allow_extended_match: true,
      allow_permutations: true,
      max_group: 1,
    },
  ]);
  transformations.push([
    ["+", ["*", "n1", "x"], ["*", ["/", "n2", "m2"], "x"]],
    ["*", ["+", "n1", ["/", "n2", "m2"]], "x"],
    {
      variables: {
        x: true,
        n1: isNumber,
        n2: isNumber,
        m2: isNumber,
      },
      evaluate_numbers: true,
      max_digits: max_digits,
      allow_implicit_identities: ["m2", "n1", "n2"],
      allow_extended_match: true,
      allow_permutations: true,
      max_group: 1,
    },
  ]);
  transformations.push([
    ["+", ["*", "n1", "x"], ["*", ["-", ["/", "n2", "m2"]], "x"]],
    ["*", ["+", "n1", ["-", ["/", "n2", "m2"]]], "x"],
    {
      variables: {
        x: true,
        n1: isNumber,
        n2: isNumber,
        m2: isNumber,
      },
      evaluate_numbers: true,
      max_digits: max_digits,
      allow_implicit_identities: ["m2", "n1", "n2"],
      allow_extended_match: true,
      allow_permutations: true,
      max_group: 1,
    },
  ]);
  transformations.push([
    ["+", ["*", ["/", "n1", "m1"], "x"], ["*", ["/", "n2", "m2"], "x"]],
    ["*", ["+", ["/", "n1", "m1"], ["/", "n2", "m2"]], "x"],
    {
      variables: {
        x: true,
        n1: isNumber,
        m1: isNumber,
        n2: isNumber,
        m2: isNumber,
      },
      evaluate_numbers: true,
      max_digits: max_digits,
      allow_implicit_identities: ["m1", "m2", "n1", "n2"],
      allow_extended_match: true,
      allow_permutations: true,
      max_group: 1,
    },
  ]);
  transformations.push([
    ["+", ["*", ["/", "n1", "m1"], "x"], ["*", ["-", ["/", "n2", "m2"]], "x"]],
    ["*", ["+", ["/", "n1", "m1"], ["-", ["/", "n2", "m2"]]], "x"],
    {
      variables: {
        x: true,
        n1: isNumber,
        m1: isNumber,
        n2: isNumber,
        m2: isNumber,
      },
      evaluate_numbers: true,
      max_digits: max_digits,
      allow_implicit_identities: ["m1", "m2", "n1", "n2"],
      allow_extended_match: true,
      allow_permutations: true,
      max_group: 1,
    },
  ]);
  transformations.push([
    ["+", ["*", "n", "x"], ["-", ["*", "m", "x"]]],
    ["*", ["+", "n", ["-", "m"]], "x"],
    {
      variables: {
        x: true,
        n: isNumber,
        m: isNumber,
      },
      evaluate_numbers: true,
      max_digits: max_digits,
      allow_implicit_identities: ["m", "n"],
      allow_extended_match: true,
      allow_permutations: true,
      max_group: 1,
    },
  ]);
  transformations.push([
    ["^", ["*", "x", "y"], "a"],
    ["*", ["^", "x", "a"], ["^", "y", "a"]],
    { allow_permutations: true },
  ]);
  transformations.push([
    ["^", ["^", "x", "n"], "m"],
    ["^", "x", ["*", "n", "m"]],
    {
      variables: {
        x: true,
        n: isNumber,
        m: isNumber,
      },
      evaluate_numbers: true,
      max_digits: max_digits,
      allow_permutations: true,
    },
  ]);
  transformations.push([
    ["-", ["+", "a", "b"]],
    ["+", ["-", "a"], ["-", "b"]],
  ]);

  // evaluate any products
  // (required since evaluate_numbers needs to be applied separately
  // to complicated products to evaluate them as numbers)
  transformations.push([
    ["*", "x", "y"],
    ["*", "x", "y"],
    {
      variables: { x: isNumerical, y: isNumerical },
      evaluate_numbers: true,
      max_digits: max_digits,
      allow_extended_match: true,
      allow_permutations: true,
      max_group: 1,
    },
  ]);

  tree = trans.applyAllTransformations(tree, transformations, 40);

  transformations = [];
  // redo as division
  transformations.push([
    ["*", "x", ["^", "y", ["-", "a"]]],
    ["/", "x", ["^", "y", "a"]],
    {
      allow_extended_match: true,
      allow_permutations: true,
      evaluate_numbers: true,
      max_digits: max_digits,
      max_group: 1,
    },
  ]);
  transformations.push([
    ["*", "x", ["apply", "exp", ["-", "a"]]],
    ["/", "x", ["apply", "exp", "a"]],
    {
      allow_extended_match: true,
      allow_permutations: true,
      evaluate_numbers: true,
      max_digits: max_digits,
      max_group: 1,
      variables: { x: true, a: true },
    },
  ]);
  transformations.push([
    ["*", "x", ["^", "y", "n"]],
    ["/", "x", ["^", "y", ["-", "n"]]],
    {
      variables: {
        x: true,
        y: true,
        n: isNegativeNumber,
      },
      evaluate_numbers: true,
      max_digits: max_digits,
      allow_extended_match: true,
      allow_permutations: true,
      max_group: 1,
    },
  ]);
  transformations.push([
    ["*", "x", ["apply", "exp", "n"]],
    ["/", "x", ["apply", "exp", ["-", "n"]]],
    {
      variables: {
        x: true,
        n: isNegativeNumber,
      },
      evaluate_numbers: true,
      max_digits: max_digits,
      allow_extended_match: true,
      allow_permutations: true,
      max_group: 1,
    },
  ]);
  tree = trans.applyAllTransformations(tree, transformations, 40);

  transformations = [];
  // redo as division, try 2
  transformations.push([
    ["^", "y", "n"],
    ["/", 1, ["^", "y", ["-", "n"]]],
    {
      variables: {
        y: true,
        n: isNegativeNumber,
      },
      evaluate_numbers: true,
      max_digits: max_digits,
    },
  ]);
  transformations.push([
    ["apply", "exp", "n"],
    ["/", 1, ["apply", "exp", ["-", "n"]]],
    {
      variables: {
        n: isNegativeNumber,
      },
      evaluate_numbers: true,
      max_digits: max_digits,
    },
  ]);
  tree = trans.applyAllTransformations(tree, transformations, 40);

  transformations = [];
  // '*' before '/' and products in denominator
  transformations.push([
    ["*", "x", ["/", "y", "z"]],
    ["/", ["*", "x", "y"], "z"],
    {
      allow_extended_match: true,
      allow_permutations: true,
      max_group: 1,
    },
  ]);
  transformations.push([
    ["/", ["/", "x", "y"], "z"],
    ["/", "x", ["*", "y", "z"]],
    {
      allow_extended_match: true,
      allow_permutations: true,
    },
  ]);
  transformations.push([
    ["/", "x", ["/", "y", "z"]],
    ["/", ["*", "x", "z"], "y"],
    {
      allow_extended_match: true,
      allow_permutations: true,
    },
  ]);
  tree = trans.applyAllTransformations(tree, transformations, 40);

  tree = evaluate_numbers(tree, {
    assumptions: assumptions,
    max_digits: max_digits,
  });

  return tree;
}

function simplify_ratios(expr_or_tree, assumptions) {
  // TODO: actually factor numerator and denominator
  // for now, assume factored, other than minus sign

  function remove_negative_factors(factors) {
    var sign_change = 1;

    factors = factors.map(function (v) {
      if (typeof v === "number") {
        if (v < 0) {
          sign_change *= -1;
          return -v;
        }
        return v;
      }
      if (!Array.isArray(v)) return v;

      if (v[0] === "-") {
        sign_change *= -1;
        return v[1];
      }
      if (v[0] !== "+") return v;

      var negate = false;
      if (typeof v[1] === "number" && v[1] < 0) negate = true;
      else if (Array.isArray(v[1]) && v[1][0] === "-") negate = true;
      else if (Array.isArray(v[1]) && v[1][0] === "*" && Number(v[1][1]) < 0) {
        negate = true;
      }

      if (negate) {
        sign_change *= -1;
        var v_ops = v.slice(1).map((x) => ["-", x]);
        return evaluate_numbers(["+"].concat(v_ops));
      } else return v;
    });

    return { factors: factors, sign_change: sign_change };
  }

  function simplify_ratios_sub(tree, negated) {
    if (!Array.isArray(tree)) {
      if (negated) {
        return ["-", tree];
      } else {
        return tree;
      }
    }

    var operator = tree[0];
    if (operator === "-") {
      return simplify_ratios_sub(tree[1], (negated = true));
    }
    var operands = tree.slice(1).map((v) => simplify_ratios_sub(v));

    if (operator !== "/") {
      if (negated) {
        return ["-", [operator, ...operands]];
      } else {
        return [operator, ...operands];
      }
    }

    var numer = operands[0];
    var denom = operands[1];

    // factor a minus sign from each factor in numerator and denominator
    // if it is negative or it is a sum with a negative first term
    // (when terms are sorted as though they were not negative)

    numer = default_order(numer, { ignore_negatives: true });
    var numer_factors;
    if (Array.isArray(numer) && numer[0] === "*")
      numer_factors = numer.slice(1);
    else numer_factors = [numer];
    var result_n = remove_negative_factors(numer_factors);
    numer_factors = result_n["factors"];
    if (negated) {
      result_n["sign_change"] *= -1;
    }

    denom = default_order(denom, { ignore_negatives: true });
    var denom_factors;
    if (Array.isArray(denom) && denom[0] === "*")
      denom_factors = denom.slice(1);
    else denom_factors = [denom];
    var result_d = remove_negative_factors(denom_factors);
    denom_factors = result_d["factors"];

    if (result_n["sign_change"] * result_d["sign_change"] < 0)
      numer_factors[0] = ["-", numer_factors[0]];

    if (numer_factors.length === 1) numer = numer_factors[0];
    else numer = ["*"].concat(numer_factors);
    if (denom_factors.length === 1) denom = denom_factors[0];
    else denom = ["*"].concat(denom_factors);

    return ["/", numer, denom];
  }

  var tree = get_tree(expr_or_tree);

  if (contains_blank(tree)) {
    return tree;
  }

  if (
    assumptions === undefined &&
    expr_or_tree.context !== undefined &&
    expr_or_tree.context.get_assumptions !== undefined
  )
    assumptions = expr_or_tree.context.get_assumptions([
      expr_or_tree.variables(),
    ]);

  return simplify_ratios_sub(tree);
}

function contains_blank(tree) {
  if (tree === "\uff3f") {
    return true;
  }
  if (!Array.isArray(tree)) {
    return false;
  }
  return tree.some(contains_blank);
}

let all_units = get_all_units();

function remove_units(expr_or_tree, scale_based_on_unit = true) {
  var tree = get_tree(expr_or_tree);

  if (!Array.isArray(tree)) {
    return tree;
  }

  let operator = tree[0];
  let operands = tree.slice(1);

  if (operator === "unit") {
    let unit_object = all_units[get_unit_of_tree(tree)];
    if (!unit_object) {
      return tree;
    }
    let value = remove_units(
      unit_object.prefix ? operands[1] : operands[0],
      scale_based_on_unit,
    );
    if (scale_based_on_unit) {
      return unit_object.scale(value);
    } else {
      return value;
    }
  } else {
    return [
      operator,
      ...operands.map((x) => remove_units(x, scale_based_on_unit)),
    ];
  }
}

function remove_scaling_units(expr_or_tree) {
  var tree = get_tree(expr_or_tree);

  if (!Array.isArray(tree)) {
    return tree;
  }

  let operator = tree[0];
  let operands = tree.slice(1);

  if (operator === "unit") {
    let unit_object = all_units[get_unit_of_tree(tree)];
    if (!(unit_object && unit_object.only_scales)) {
      return tree;
    }
    let value = remove_scaling_units(
      unit_object.prefix ? operands[1] : operands[0],
    );
    return unit_object.scale(value);
  } else {
    return [operator, ...operands.map((x) => remove_scaling_units(x))];
  }
}

function add_unit(expr_or_tree, unit) {
  var tree = get_tree(expr_or_tree);

  let unit_object = all_units[unit];
  if (!unit_object) {
    return tree;
  }

  if (unit_object.prefix) {
    return ["unit", unit, tree];
  } else {
    return ["unit", tree, unit];
  }
}

function move_units_outside_products(tree) {
  if (!Array.isArray(tree)) {
    return tree;
  }

  let operator = tree[0];
  let operands = tree.slice(1);

  if (operator === "*") {
    let units_found = [];
    let factors_without_units = [];

    for (let op of operands) {
      let result = get_unit_value_of_tree(op);

      if (result) {
        units_found.push(result[0]);
        factors_without_units.push(result[1]);
      } else {
        factors_without_units.push(op);
      }
    }

    let product = ["*", ...factors_without_units];

    for (let unit of units_found) {
      product = add_unit(product, unit);
    }

    return product;
  } else {
    return [operator, ...operands.map(move_units_outside_products)];
  }
}

export {
  clean,
  simplify,
  simplify_logical,
  set_small_zero,
  evaluate_numbers,
  collect_like_terms_factors,
  collapse_unary_minus,
  simplify_ratios,
  default_order,
  perform_vector_matrix_additions_scalar_multiplications,
  remove_units,
  add_unit,
  remove_scaling_units,
};
