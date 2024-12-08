import { get_unit_value_of_tree } from "../expression/units";
import { flatten } from "./flatten";
import { get_tree } from "./util";

function remove_duplicate_negatives(tree) {
  // remove pairs of consecutive minus signs

  if (!Array.isArray(tree)) return tree;

  var operator = tree[0];
  var operands = tree.slice(1);

  if (
    operator === "-" &&
    Array.isArray(operands[0]) &&
    operands[0][0] === "-"
  ) {
    return remove_duplicate_negatives(operands[0][1]);
  }

  operands = operands.map(remove_duplicate_negatives);

  return [operator].concat(operands);
}

function normalize_negatives_in_factors(tree) {
  // if any factors contain a negative,
  // place negative outside factor
  //
  // run remove_duplicates_negatives before and after
  // running this function to make sure all negatives are addressed

  if (!Array.isArray(tree)) return tree;

  var operator = tree[0];
  var operands = tree.slice(1);

  operands = operands.map(normalize_negatives_in_factors);

  if (operator !== "*" && operator !== "/") return [operator].concat(operands);

  var sign = 1;
  var operands_no_negatives = [];

  for (var i = 0; i < operands.length; i++) {
    if (Array.isArray(operands[i]) && operands[i][0] === "-") {
      sign *= -1;
      operands_no_negatives.push(operands[i][1]);
    } else {
      operands_no_negatives.push(operands[i]);
    }
  }
  var result = [operator].concat(operands_no_negatives);
  if (sign === -1) result = ["-", result];

  return result;
}

function normalize_negatives(expr_or_tree) {
  // Remove duplicate negatives and pull all negatives outside factors
  var tree = get_tree(expr_or_tree);

  tree = remove_duplicate_negatives(tree);
  tree = normalize_negatives_in_factors(tree);
  tree = remove_duplicate_negatives(tree);

  return tree;
}

function sort_key(tree, params = {}) {
  if (typeof tree === "number") {
    if (params.ignore_negatives) return [0, "number", Math.abs(tree)];
    return [0, "number", tree];
  }
  if (typeof tree === "string") {
    // if string is a constant, return number with value?
    if (tree === "-" || tree === "+") {
      return [8, "plus_minus_string", tree];
    }
    return [1, "symbol", tree];
  }
  if (typeof tree === "boolean") {
    return [1, "boolean", tree];
  }

  if (!Array.isArray(tree)) return [-1, "unknown", tree];

  var operator = tree[0];
  var operands = tree.slice(1);

  if (operator === "apply") {
    var key = [2, "function", operands[0]];

    var f_args = operands[1];

    var n_args = 1;

    var arg_keys = [];

    if (Array.isArray(f_args)) {
      f_args = f_args.slice(1); // remove vector operator

      n_args = f_args.length;

      arg_keys = f_args.map((x) => sort_key(x, params));
    } else {
      arg_keys = [sort_key(f_args, params)];
    }

    key.push([n_args, arg_keys]);

    return key;
  } else if (operator === "unit") {
    let [unit, value] = get_unit_value_of_tree(tree);

    if (unit) {
      let key = sort_key(value, params);
      key[1] += "_" + unit;
      return key;
    }
  }

  var n_factors = operands.length;

  var factor_keys = operands.map(sort_key, params);

  if (operator === "*") {
    return [4, "product", n_factors, factor_keys];
  }

  if (operator === "/") {
    return [4, "quotient", n_factors, factor_keys];
  }

  if (operator === "+") {
    return [5, "sum", n_factors, factor_keys];
  }

  if (operator === "-") {
    if (params.ignore_negatives) return factor_keys[0];
    return [6, "minus", n_factors, factor_keys];
  }

  // desired order so that items that look similar and can be coerced into each other are grouped together
  // - tuple, vector, altvector, open interval
  // - half-open intervals
  // - closed interval, array

  if (["tuple", "vector", "altvector"].includes(operator)) {
    // don't include operator so they sort independent of which operator it is
    return [7, n_factors, factor_keys];
  } else if (operator === "interval") {
    if (operands[1][1] === false) {
      if (operands[1][2] === false) {
        // open interval: sort key is just from the values so it it the same as a tuple or vector
        return [7, ...factor_keys[0].slice(1)];
      } else {
        // half open interval
        return [8, n_factors, factor_keys];
      }
    } else if (operands[1][2] === false) {
      // half open interval
      return [8, n_factors, factor_keys];
    } else {
      // closed interval: sort key is just from the values so it is the same as an array
      return [9, ...factor_keys[0].slice(1)];
    }
  } else if (operator === "array") {
    return [9, n_factors, factor_keys];
  }

  return [10, operator, n_factors, factor_keys];
}

function arrayCompare(a, b) {
  if (Array.isArray(a)) {
    if (Array.isArray(b)) {
      let minLength = Math.min(a.length, b.length);
      for (let i = 0; i < minLength; i++) {
        let comp = arrayCompare(a[i], b[i]);
        if (comp !== 0) {
          return comp;
        }
      }

      // shorter array comes first
      return a.length < b.length ? -1 : a.length > b.length ? 1 : 0;
    } else {
      // non array comes before array
      // a is the array
      return 1;
    }
  } else {
    if (Array.isArray(b)) {
      // non-array comes before array
      // b is the array
      return -1;
    } else {
      // got to two scalar
      return a < b ? -1 : a > b ? 1 : 0;
    }
  }
}

function compare_function(a, b, params = {}) {
  var key_a = sort_key(a, params);
  var key_b = sort_key(b, params);

  return arrayCompare(key_a, key_b);
}

function coeff_factors_from_term(term, string_factors) {
  if (typeof term === "string") {
    let ind = string_factors.indexOf(term);
    if (ind === -1) {
      string_factors.push(term);
      ind = string_factors.length - 1;
    }
    let f = [];
    f[ind] = 1;
    return { factor_contains: f, coeff: 1 };
  } else if (Array.isArray(term)) {
    let operator = term[0];
    let operands = term.slice(1);
    if (operator === "*") {
      let coeff = [];
      let f = [];
      for (let factor of operands) {
        if (typeof factor === "string") {
          let ind = string_factors.indexOf(factor);
          if (ind === -1) {
            string_factors.push(factor);
            ind = string_factors.length - 1;
          }
          if (f[ind] === undefined) {
            f[ind] = 0;
          }
          f[ind]++;
          continue;
        } else if (
          Array.isArray(factor) &&
          (factor[0] === "^" || factor[0] === "-")
        ) {
          let result = coeff_factors_from_term(factor, string_factors);
          for (let ind in result.factor_contains) {
            if (f[ind] === undefined) {
              f[ind] = 0;
            }
            f[ind] += result.factor_contains[ind];
          }
          if (result.coeff !== 1) {
            coeff.push(result.coeff);
          }
          continue;
        }
        coeff.push(factor);
      }

      if (coeff.length === 0) {
        coeff = 1;
      } else if (coeff.length === 1) {
        coeff = coeff[0];
      } else {
        coeff = ["*", ...coeff];
      }
      return { factor_contains: f, coeff: coeff };
    } else if (operator === "^") {
      let base = operands[0];
      let exponent = operands[1];
      let f = [];
      if (typeof base === "string" && Number.isFinite(exponent)) {
        let ind = string_factors.indexOf(base);
        if (ind === -1) {
          string_factors.push(base);
          ind = string_factors.length - 1;
        }
        f[ind] = exponent;
        return { factor_contains: f, coeff: 1 };
      }
    } else if (operator === "-") {
      let result = coeff_factors_from_term(operands[0], string_factors);
      let coeff = -1;
      if (typeof result.coeff === "number") {
        coeff *= result.coeff;
      } else {
        coeff = ["-", result.coeff];
      }
      return { factor_contains: result.factor_contains, coeff: coeff };
    } else if (operator === "/") {
      let result = coeff_factors_from_term(operands[0], string_factors);
      let coeff = ["/", result.coeff, operands[1]];
      return { factor_contains: result.factor_contains, coeff: coeff };
    }
  }

  return { factor_contains: [], coeff: term };
}

function default_order(expr_or_tree, params) {
  if (params === undefined) params = {};

  var tree = get_tree(expr_or_tree);

  tree = flatten(tree);
  tree = normalize_negatives(tree);

  function sort_ast(subTree) {
    if (!Array.isArray(subTree)) return subTree;

    var operator = subTree[0];
    var operands = subTree.slice(1);

    operands = operands.map(sort_ast);

    if (operator === "+") {
      // kludge to get sort order closer to lexographic order

      // TODO: clean this up

      // find all string factors
      let string_factors = [];
      let factors_by_term = [];
      let coeffs_by_term = [];
      for (let term of operands) {
        let result = coeff_factors_from_term(term, string_factors);
        factors_by_term.push(result.factor_contains);
        coeffs_by_term.push(result.coeff);
      }

      // factors_by_term = factors_by_term.map(x => Array.from(x, item => item || 0))

      let variableInfo = [];
      for (let [ind, varname] of string_factors.entries()) {
        let thisvar = { varname: varname, exponents_in_term: [] };
        for (let j = 0; j < factors_by_term.length; j++) {
          thisvar.exponents_in_term.push(factors_by_term[j][ind] || 0);
        }
        variableInfo.push(thisvar);
      }

      variableInfo.sort((a, b) => (a.varname < b.varname ? -1 : 1));

      let sort_keys_by_term = [];

      for (let i = 0; i < coeffs_by_term.length; i++) {
        let this_sort_key = variableInfo.reduce(
          (a, c) => [...a, -c.exponents_in_term[i]],
          [],
        );
        this_sort_key.push(sort_key(coeffs_by_term[i], params));
        sort_keys_by_term.push(this_sort_key);
      }

      let terms_with_sort_key = [];

      for (let [ind, term] of operands.entries()) {
        terms_with_sort_key.push({
          term: term,
          sort_key: sort_keys_by_term[ind],
        });
      }

      terms_with_sort_key.sort((a, b) => arrayCompare(a.sort_key, b.sort_key));

      operands = terms_with_sort_key.map((x) => x.term);

      // sort all operands of these arguments in default order
      // determined by compare function
      // operands.sort((a, b) => compare_function(a, b, params));
    } else if (operator === "*") {
      // preserve order of all operands that are non-commutative
      // and put them at the end
      let operandsToSort = [];
      let operandsToPreserveOrder = [];
      for (let op of operands) {
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
          ].includes(op[0])
        ) {
          operandsToPreserveOrder.push(op);
        } else {
          operandsToSort.push(op);
        }
      }

      operandsToSort.sort((a, b) => compare_function(a, b, params));
      operands = [...operandsToSort, ...operandsToPreserveOrder];
    } else if (
      operator === "=" ||
      operator === "and" ||
      operator === "or" ||
      operator === "ne" ||
      operator === "union" ||
      operator === "intersect"
    ) {
      // TODO: determine if commutative

      // sort all operands of these arguments in default order
      // determined by compare function
      operands.sort((a, b) => compare_function(a, b, params));
    } else if (operator === ">" || operator === "ge") {
      // turn all greater thans to less thans

      operands = operands.reverse();
      if (operator === ">") operator = "<";
      else operator = "le";
    } else if (operator === "gts") {
      // turn all greater thans to less thans
      var args = operands[0];
      var strict = operands[1];

      if (args[0] !== "tuple" || strict[0] !== "tuple")
        // something wrong if args or strict are not tuples
        throw new Error("Badly formed ast");

      args = ["tuple"].concat(args.slice(1).reverse());
      strict = ["tuple"].concat(strict.slice(1).reverse());

      operator = "lts";
      operands = [args, strict];
    } else if (
      operator === "ni" ||
      operator === "notni" ||
      operator === "superset" ||
      operator === "notsuperset" ||
      operator === "superseteq" ||
      operator === "notsuperseteq"
    ) {
      // turn all containment operators to have larger set at right

      operands = operands.reverse();
      if (operator === "ni") operator = "in";
      else if (operator === "notni") operator = "notin";
      else if (operator === "superset") operator = "subset";
      else if (operator === "notsuperset") operator = "notsubset";
      else if (operator === "superseteq") operator = "subseteq";
      else if (operator === "notsuperseteq") operator = "notsubseteq";
    } else if (operator === "-") {
      // when negating a product with a numerical first factor
      // put negative sign in that first factor
      if (operands[0][0] === "*") {
        operands[0][1] = ["-", operands[0][1]];
        return operands[0];
      }
    }

    return [operator].concat(operands);
  }

  return normalize_negatives(sort_ast(tree));
}

export { normalize_negatives, compare_function, default_order };
