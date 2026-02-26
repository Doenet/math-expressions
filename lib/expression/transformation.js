import * as flatten from "../trees/flatten.js";
import * as trans from "../trees/basic.js";
import { normalize_negatives } from "../trees/default_order.js";
import * as tuples from "./normalization/tuples.js";
import { get_tree } from "../trees/util.js";
import textToAstObj from "../converters/text-to-ast.js";
import {
  collect_like_terms_factors,
  evaluate_numbers,
  perform_vector_matrix_additions_scalar_multiplications,
} from "./simplify.js";
var textToAst = new textToAstObj();

function expand(expr_or_tree, no_division) {
  // Initial implementation of expand
  // Expands polynomials only up to degree 4

  var tree = get_tree(expr_or_tree);

  tree = perform_matrix_multiplications(tree);
  tree = perform_vector_matrix_additions_scalar_multiplications(tree);

  const no_dx = (v) =>
    !Array.isArray(v) ||
    !["d", "*"].includes(v[0]) ||
    (v[0] === "*" &&
      v
        .slice(1)
        .every((factor) => !Array.isArray(factor) || factor[0] !== "d"));

  var transformations = [];
  transformations.push([
    textToAst.convert("a*(b+c)"),
    textToAst.convert("a*b+a*c"),
    ["^", "x", ["+", "n", "m"]],
    {
      variables: {
        a: no_dx,
        b: true,
        c: true,
      },
      allow_extended_match: true,
    },
  ]);
  transformations.push([
    textToAst.convert("(a+b)*c"),
    textToAst.convert("a*c+b*c"),
    {
      variables: {
        a: true,
        b: true,
        c: no_dx,
      },
      allow_extended_match: true,
    },
  ]);
  if (!no_division)
    transformations.push([
      textToAst.convert("(a+b)/c"),
      textToAst.convert("a/c+b/c"),
    ]);
  transformations.push([
    textToAst.convert("-(a+b)"),
    textToAst.convert("-a-b"),
  ]);
  transformations.push([textToAst.convert("a(-b)"), textToAst.convert("-ab")]);
  transformations.push([
    textToAst.convert("(a+b)^2"),
    textToAst.convert("a^2+2ab+b^2"),
  ]);
  transformations.push([
    textToAst.convert("(a+b)^3"),
    textToAst.convert("a^3+3a^2b+3ab^2+b^3"),
  ]);
  transformations.push([
    textToAst.convert("(a+b)^4"),
    textToAst.convert("a^4+4a^3b+6a^2b^2+4ab^3+b^4"),
  ]);
  transformations.push([
    textToAst.convert("a*(b+c)*d"),
    textToAst.convert("(a*b+a*c)*d"),
  ]);

  tree = trans.applyAllTransformations(tree, transformations, 20);

  tree = flatten.flatten(tree);

  tree = evaluate_numbers(tree);

  tree = collect_like_terms_factors(tree);

  tree = normalize_negatives(tree);

  return tree;
}

function perform_matrix_multiplications(
  expr_or_tree,
  include_vectors = true,
  include_tuples = true,
) {
  var tree = get_tree(expr_or_tree);

  function matrix_mult(matrix1, matrix2) {
    let m = matrix1[1][1];
    let n = matrix1[1][2];
    let p = matrix2[1][2];

    if (matrix2[1][1] !== n) {
      return { success: false };
    }

    let matrix1Data = matrix1[2];
    let matrix2Data = matrix2[2];

    let newMatrixData = ["tuple"];

    for (let i = 0; i < m; i++) {
      let row = matrix1Data[i + 1].slice(1);

      let newRow = ["tuple"];

      for (let k = 0; k < p; k++) {
        let col = matrix2Data.slice(1).map((x) => x[k + 1]);

        if (row.length !== n || col.length !== n) {
          return subtree;
        }

        let newEntry = ["+", ...row.map((x, i) => ["*", x, col[i]])];

        newRow.push(newEntry);
      }

      newMatrixData.push(newRow);
    }

    let newMatrix = ["matrix", ["tuple", m, p], newMatrixData];

    return { success: true, newMatrix };
  }

  function matrix_power(matrix, p) {
    if (!Number.isInteger(p) || p < 1) {
      return { success: false };
    }

    if (p === 1) {
      return { success: true, newMatrix: matrix };
    }

    let n = matrix[1][1];

    if (matrix[1][2] !== n) {
      return { success: false };
    }

    let matrixData = matrix[2];

    let newMatrixData = JSON.parse(JSON.stringify(matrixData));

    for (let j = 1; j < p; j++) {
      const leftMatrixData = newMatrixData;
      newMatrixData = ["tuple"];

      for (let i = 0; i < n; i++) {
        let row = leftMatrixData[i + 1].slice(1);

        let newRow = ["tuple"];

        for (let k = 0; k < n; k++) {
          let col = matrixData.slice(1).map((x) => x[k + 1]);

          if (row.length !== n || col.length !== n) {
            return subtree;
          }

          let newEntry = ["+", ...row.map((x, i) => ["*", x, col[i]])];

          newRow.push(newEntry);
        }

        newMatrixData.push(newRow);
      }
    }

    let newMatrix = ["matrix", ["tuple", n, n], newMatrixData];

    return { success: true, newMatrix };
  }

  function mult_matrix_scalar(matrix, scalar) {
    let m = matrix[1][1];
    let n = matrix[1][2];

    let matrixData = matrix[2];

    let newMatrixData = ["tuple"];

    for (let i = 0; i < m; i++) {
      let row = ["tuple"];
      let oldRow = matrixData[i + 1];

      for (let j = 0; j < n; j++) {
        row.push(["*", scalar, oldRow[j + 1]]);
      }

      newMatrixData.push(row);
    }

    return ["matrix", ["tuple", m, n], newMatrixData];
  }

  function perform_matrix_integer_powers() {
    let pattern = ["^", ["matrix", "size", "matrixData"], "p"];

    let params = {
      variables: {
        p: (v) => Number.isInteger(v) && v > 0,
        size: true,
        matrixData: true,
      },
    };

    tree = trans.transform(tree, function (subtree) {
      let matchResults = trans.match(subtree, pattern, params);
      if (matchResults) {
        let newMatrix = ["matrix", matchResults.size, matchResults.matrixData];

        let result = matrix_power(newMatrix, matchResults.p);

        if (result.success) {
          return result.newMatrix;
        } else {
          return subtree;
        }
      } else {
        return subtree;
      }
    });
  }

  function perform_matrix_matrix_multiplications() {
    let pattern = ["*", "a", ["matrix", "size", "matrixData"], "b"];

    let params = {
      variables: {
        a: true,
        b: true,
        size: true,
        matrixData: true,
      },
      allow_implicit_identities: ["a", "b"],
    };

    tree = trans.transform(tree, function (subtree) {
      let matchResults = trans.match(subtree, pattern, params);
      if (matchResults) {
        let newMatrix = ["matrix", matchResults.size, matchResults.matrixData];

        let preFactors = [],
          postFactors = [];

        if (Array.isArray(matchResults.a) && matchResults.a[0] === "*") {
          preFactors.push(...matchResults.a.slice(1));
        } else {
          preFactors.push(matchResults.a);
        }

        if (Array.isArray(matchResults.b) && matchResults.b[0] === "*") {
          postFactors.push(...matchResults.b.slice(1));
        } else {
          postFactors.push(matchResults.b);
        }

        let nPreFactors = preFactors.length;

        for (let i = nPreFactors - 1; i >= 0; i--) {
          let operator = preFactors[i][0];
          if (operator === "matrix") {
            let result = matrix_mult(preFactors[i], newMatrix);

            if (result.success) {
              newMatrix = result.newMatrix;
              preFactors.pop();
            } else {
              break;
            }
          } else if (
            [
              "tuple",
              "list",
              "vector",
              "altvector",
              "interval",
              "set",
              "array",
            ].includes(operator)
          ) {
            break;
          } else {
            if (preFactors[i] !== 1) {
              newMatrix = mult_matrix_scalar(newMatrix, preFactors[i]);
            }
            preFactors.pop();
          }
        }

        while (postFactors.length > 0) {
          let operator = postFactors[0][0];
          if (operator === "matrix") {
            let result = matrix_mult(newMatrix, postFactors[0]);

            if (result.success) {
              newMatrix = result.newMatrix;
              postFactors.splice(0, 1);
            } else {
              break;
            }
          } else if (
            [
              "tuple",
              "list",
              "vector",
              "altvector",
              "interval",
              "set",
              "array",
            ].includes(operator)
          ) {
            break;
          } else {
            if (postFactors[0] !== 1) {
              newMatrix = mult_matrix_scalar(newMatrix, postFactors[0]);
            }
            postFactors.splice(0, 1);
          }
        }

        if (preFactors.length > 0 || postFactors.length > 0) {
          return ["*", ...preFactors, newMatrix, ...postFactors];
        } else {
          return newMatrix;
        }
      } else {
        return subtree;
      }
    });
  }

  function perform_matrix_vector_multiplications() {
    let pattern = ["*", ["matrix", ["tuple", "m", "n"], "matrixData"], "b"];

    let params = {
      variables: {
        n: Number.isInteger,
        m: Number.isInteger,
        matrixData: true,
        b: true,
      },
    };

    tree = trans.transform(tree, function (subtree) {
      let matchResults = trans.match(subtree, pattern, params);
      if (matchResults) {
        let m = matchResults.m,
          n = matchResults.n,
          b = matchResults.b;
        let matrixData = matchResults.matrixData;

        let vectorOperators = [];

        if (include_vectors) {
          vectorOperators.push("vector");
          vectorOperators.push("altvector");
        }
        if (include_tuples) {
          vectorOperators.push("tuple");
        }

        let vectorData;
        let otherFactors = [];

        if (vectorOperators.includes(b[0])) {
          vectorData = b;
        } else if (
          Array.isArray(b) &&
          b[0] === "*" &&
          vectorOperators.includes(b[1][0])
        ) {
          vectorData = b[1];
          otherFactors = b.slice(2);
        } else {
          return subtree;
        }

        let vectorValues = vectorData.slice(1);

        if (vectorValues.length !== n) {
          return subtree;
        }

        let newVector = [vectorData[0]];

        for (let i = 0; i < m; i++) {
          let row = matrixData[i + 1].slice(1);

          let newEntry = ["+", ...row.map((x, i) => ["*", x, vectorValues[i]])];

          newVector.push(newEntry);
        }

        if (otherFactors.length > 0) {
          return ["*", newVector, ...otherFactors];
        } else {
          return newVector;
        }
      } else {
        return subtree;
      }
    });
  }

  perform_matrix_integer_powers();

  perform_matrix_matrix_multiplications();

  if (include_vectors || include_tuples) {
    perform_matrix_vector_multiplications();
  }

  return tree;
}

function perform_vector_scalar_multiplications(
  expr_or_tree,
  include_tuples = true,
) {
  var tree = get_tree(expr_or_tree);

  let vector_scalar_mult_transform = function (ast, operator) {
    let pattern = ["*", "a", [operator, "vectorData"], "b"];

    let params = {
      allow_implicit_identities: ["a", "b"],
    };

    ast = trans.transform(ast, function (subtree) {
      let matchResults = trans.match(subtree, pattern, params);
      if (matchResults) {
        if (matchResults.a === 1 && matchResults.b === 1) {
          return subtree;
        }

        let vectorData = matchResults.vectorData;

        // vector data is either an array beginning with operator
        // or a singleton representing a vector of one component
        if (Array.isArray(vectorData) && vectorData[0] === operator) {
          vectorData = vectorData.slice(1);
        } else {
          vectorData = [vectorData];
        }

        let preFactors = [],
          postFactors = [];

        if (Array.isArray(matchResults.a) && matchResults.a[0] === "*") {
          preFactors.push(...matchResults.a.slice(1));
        } else {
          preFactors.push(matchResults.a);
        }

        if (Array.isArray(matchResults.b) && matchResults.b[0] === "*") {
          postFactors.push(...matchResults.b.slice(1));
        } else {
          postFactors.push(matchResults.b);
        }

        let nPreFactors = preFactors.length;

        for (let i = nPreFactors - 1; i >= 0; i--) {
          let operator = preFactors[i][0];
          if (
            [
              "tuple",
              "list",
              "vector",
              "altvector",
              "interval",
              "set",
              "array",
              "matrix",
            ].includes(operator)
          ) {
            break;
          } else {
            if (preFactors[i] !== 1) {
              vectorData = vectorData.map((x) => ["*", x, preFactors[i]]);
            }
            preFactors.pop();
          }
        }

        while (postFactors.length > 0) {
          let operator = postFactors[0][0];
          if (
            [
              "tuple",
              "list",
              "vector",
              "altvector",
              "interval",
              "set",
              "array",
              "matrix",
            ].includes(operator)
          ) {
            break;
          } else {
            if (postFactors[0] !== 1) {
              vectorData = vectorData.map((x) => ["*", x, postFactors[0]]);
            }
            postFactors.splice(0, 1);
          }
        }

        let newVector = [operator, ...vectorData];

        if (preFactors.length > 0 || postFactors.length > 0) {
          return ["*", ...preFactors, newVector, ...postFactors];
        } else {
          return newVector;
        }
      } else {
        return subtree;
      }
    });

    return ast;
  };

  tree = vector_scalar_mult_transform(tree, "vector");
  tree = vector_scalar_mult_transform(tree, "altvector");

  if (include_tuples) {
    tree = vector_scalar_mult_transform(tree, "tuple");
  }

  return tree;
}

function perform_matrix_scalar_multiplications(expr_or_tree) {
  var tree = get_tree(expr_or_tree);

  let pattern = ["*", "a", ["matrix", "size", "matrixData"], "b"];

  let params = {
    allow_implicit_identities: ["a", "b"],
  };

  tree = trans.transform(tree, function (subtree) {
    let matchResults = trans.match(subtree, pattern, params);
    if (matchResults) {
      if (matchResults.a === 1 && matchResults.b === 1) {
        return subtree;
      }

      let matrixData = matchResults.matrixData;

      let preFactors = [],
        postFactors = [];

      if (Array.isArray(matchResults.a) && matchResults.a[0] === "*") {
        preFactors.push(...matchResults.a.slice(1));
      } else {
        preFactors.push(matchResults.a);
      }

      if (Array.isArray(matchResults.b) && matchResults.b[0] === "*") {
        postFactors.push(...matchResults.b.slice(1));
      } else {
        postFactors.push(matchResults.b);
      }

      let nPreFactors = preFactors.length;

      for (let i = nPreFactors - 1; i >= 0; i--) {
        let operator = preFactors[i][0];
        if (
          [
            "tuple",
            "list",
            "vector",
            "altvector",
            "interval",
            "set",
            "array",
            "matrix",
          ].includes(operator)
        ) {
          break;
        } else {
          if (preFactors[i] !== 1) {
            matrixData = matrixData.map((x) =>
              x === "tuple"
                ? x
                : x.map((y) => (y === "tuple" ? y : ["*", y, preFactors[i]])),
            );
          }
          preFactors.pop();
        }
      }

      while (postFactors.length > 0) {
        let operator = postFactors[0][0];
        if (
          [
            "tuple",
            "list",
            "vector",
            "altvector",
            "interval",
            "set",
            "array",
            "matrix",
          ].includes(operator)
        ) {
          break;
        } else {
          if (postFactors[0] !== 1) {
            matrixData = matrixData.map((x) =>
              x === "tuple"
                ? x
                : x.map((y) => (y === "tuple" ? y : ["*", y, postFactors[0]])),
            );
          }
          postFactors.splice(0, 1);
        }
      }

      let newMatrix = ["matrix", matchResults.size, matrixData];

      if (preFactors.length > 0 || postFactors.length > 0) {
        return ["*", ...preFactors, newMatrix, ...postFactors];
      } else {
        return newMatrix;
      }
    } else {
      return subtree;
    }
  });

  return tree;
}

function expand_relations(expr_or_tree) {
  var tree = get_tree(expr_or_tree);
  return trans.transform(tree, expand_relations_transform);
}

function expand_relations_transform(ast) {
  if (!Array.isArray(ast)) {
    return ast;
  }

  var operator = ast[0];
  var operands = ast.slice(1);
  // since transforms in bottom up fashion,
  // operands have already been expanded

  if (operator === "=") {
    if (operands.length <= 2) return ast;
    let result = ["and"];
    for (let i = 0; i < operands.length - 1; i++) {
      result.push(["=", operands[i], operands[i + 1]]);
    }
    return result;
  }
  if (operator === "gts" || operator === "lts") {
    let args = operands[0];
    let strict = operands[1];

    if (args[0] !== "tuple" || strict[0] !== "tuple")
      // something wrong if args or strict are not tuples
      throw new Error("Badly formed ast");

    let comparisons = [];
    for (let i = 1; i < args.length - 1; i++) {
      let new_operator;
      if (strict[i]) {
        if (operator === "lts") new_operator = "<";
        else new_operator = ">";
      } else {
        if (operator === "lts") new_operator = "le";
        else new_operator = "ge";
      }
      comparisons.push([new_operator, args[i], args[i + 1]]);
    }

    let result = ["and", comparisons[0], comparisons[1]];
    for (let i = 2; i < comparisons.length; i++)
      result = ["and", result, comparisons[i]];
    return result;
  }

  // convert interval containment to inequalities
  if (
    operator === "in" ||
    operator === "notin" ||
    operator === "ni" ||
    operator === "notni"
  ) {
    let negate = false;
    if (operator === "notin" || operator === "notni") negate = true;

    let x, interval;
    if (operator === "in" || operator === "notin") {
      x = operands[0];
      interval = operands[1];
    } else {
      x = operands[1];
      interval = operands[0];
    }

    // convert any tuples/arrays of length two to intervals
    interval = tuples.to_intervals(interval);

    // if not interval, don't transform
    if (interval[0] !== "interval") return ast;

    let args = interval[1];
    let closed = interval[2];
    if (args[0] !== "tuple" || closed[0] !== "tuple")
      throw new Error("Badly formed ast");

    let a = args[1];
    let b = args[2];

    let comparisons = [];
    if (closed[1]) {
      if (negate) comparisons.push(["<", x, a]);
      else comparisons.push(["ge", x, a]);
    } else {
      if (negate) comparisons.push(["le", x, a]);
      else comparisons.push([">", x, a]);
    }
    if (closed[2]) {
      if (negate) comparisons.push([">", x, b]);
      else comparisons.push(["le", x, b]);
    } else {
      if (negate) comparisons.push(["ge", x, b]);
      else comparisons.push(["<", x, b]);
    }

    let result;
    if (negate) result = ["or"].concat(comparisons);
    else result = ["and"].concat(comparisons);

    return result;
  }

  // convert interval containment to inequalities
  if (
    operator === "subset" ||
    operator === "notsubset" ||
    operator === "superset" ||
    operator === "notsuperset"
  ) {
    let negate = false;
    if (operator === "notsubset" || operator === "notsuperset") negate = true;

    let small, big;
    if (operator === "subset" || operator === "notsubset") {
      small = operands[0];
      big = operands[1];
    } else {
      small = operands[1];
      big = operands[0];
    }

    // convert any tuples/arrays of length two to intervals
    small = tuples.to_intervals(small);
    big = tuples.to_intervals(big);

    // if not interval, don't transform
    if (small[0] !== "interval" || big[0] !== "interval") return ast;

    let small_args = small[1];
    let small_closed = small[2];
    let big_args = big[1];
    let big_closed = big[2];
    if (
      small_args[0] !== "tuple" ||
      small_closed[0] !== "tuple" ||
      big_args[0] !== "tuple" ||
      big_closed[0] !== "tuple"
    )
      throw new Error("Badly formed ast");

    let small_a = small_args[1];
    let small_b = small_args[2];
    let big_a = big_args[1];
    let big_b = big_args[2];

    let comparisons = [];
    if (small_closed[1] && !big_closed[1]) {
      if (negate) comparisons.push(["le", small_a, big_a]);
      else comparisons.push([">", small_a, big_a]);
    } else {
      if (negate) comparisons.push(["<", small_a, big_a]);
      else comparisons.push(["ge", small_a, big_a]);
    }
    if (small_closed[2] && !big_closed[2]) {
      if (negate) comparisons.push(["ge", small_b, big_b]);
      else comparisons.push(["<", small_b, big_b]);
    } else {
      if (negate) comparisons.push([">", small_b, big_b]);
      else comparisons.push(["le", small_b, big_b]);
    }
    let result;
    if (negate) result = ["or"].concat(comparisons);
    else result = ["and"].concat(comparisons);

    return result;
  }

  return ast;
}

function substitute(pattern, bindings) {
  var pattern_tree = get_tree(pattern);

  var bindings_tree = {};
  for (let b in bindings) {
    bindings_tree[b] = get_tree(bindings[b]);
  }

  return trans.substitute(pattern_tree, bindings_tree);
}

function substitute_component(pattern, component, value) {
  let pattern_tree = get_tree(pattern);
  let value_tree = get_tree(value);

  if (typeof component === "number") {
    component = [component];
  } else if (!Array.isArray(component)) {
    throw Error("Invalid substitute_component: " + component);
  }

  let container_operators = ["list", "tuple", "vector", "altvector", "array"];

  return substitute_component_sub(pattern_tree, component, value_tree);

  function substitute_component_sub(tree, component, value_tree) {
    if (component.length === 0) {
      return value;
    }
    if (!Array.isArray(tree)) {
      throw Error(
        "Invalid substitute_component: expected list, tuple, vector, or array",
      );
    }

    let operator = tree[0];
    let operands = tree.slice(1);

    if (!container_operators.includes(operator)) {
      throw Error(
        "Invalid substitute_component: expected list, tuple, vector, or array",
      );
    }

    let ind = component[0];
    if (ind < 0 || ind > operands.length - 1) {
      throw Error("Invalid substitute_component: component out of range");
    }
    let new_components = component.slice(1);
    let result = substitute_component_sub(
      operands[ind],
      new_components,
      value_tree,
    );

    return [
      operator,
      ...operands.slice(0, ind),
      result,
      ...operands.slice(ind + 1),
    ];
  }
}

function get_component(pattern, component) {
  let pattern_tree = get_tree(pattern);

  if (typeof component === "number") {
    component = [component];
  } else if (!Array.isArray(component)) {
    throw Error("Invalid get_component: " + component);
  }

  let container_operators = ["list", "tuple", "vector", "altvector", "array"];

  return get_component_sub(pattern_tree, component);

  function get_component_sub(tree, component) {
    if (component.length === 0) {
      return tree;
    }

    if (!Array.isArray(tree)) {
      throw Error(
        "Invalid get_component: expected list, tuple, vector, or array",
      );
    }

    let operator = tree[0];
    let operands = tree.slice(1);

    if (!container_operators.includes(operator)) {
      throw Error(
        "Invalid get_component: expected list, tuple, vector, or array",
      );
    }

    let ind = component[0];
    if (ind < 0 || ind > operands.length - 1) {
      throw Error("Invalid get_component: component out of range");
    }
    let new_components = component.slice(1);
    return get_component_sub(operands[ind], new_components);
  }
}

export {
  expand,
  expand_relations,
  substitute,
  substitute_component,
  get_component,
  perform_vector_scalar_multiplications,
  perform_matrix_scalar_multiplications,
  perform_matrix_multiplications,
};
