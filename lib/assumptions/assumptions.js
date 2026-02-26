import * as simplify from "../expression/simplify.js";
import { default_order } from "../trees/default_order.js";
import { variables as variables_in } from "../expression/variables.js";
import * as trees from "../trees/basic.js";
import { flatten } from "../trees/flatten.js";
import { expand_relations } from "../expression/transformation.js";
import { get_tree } from "../trees/util.js";
import * as solve from "../expression/solve.js";

function clean_assumptions(tree, known) {
  // normalize order and operators of assumptions in tree,
  // remove any duplicates or those in known
  // return ast or undefined if no assumptions found

  if (!Array.isArray(tree) || tree.length === 0) return tree;

  tree = flatten(
    default_order(simplify.simplify_logical(expand_relations(tree))),
  );

  // check for duplicates (within tree or already in known)
  var operator = tree[0];
  var operands = tree.slice(1);

  if (operator === "and" || operator === "or") {
    // remove duplicates, using trees.equal
    operands = operands.reduce(function (a, b) {
      if (
        a.every(function (v) {
          return !trees.equal(v, b);
        })
      )
        a.push(b);
      return a;
    }, []);

    // if known exists, filter out those
    if (operator === "and" && known && Array.isArray(known)) {
      let known_operands;
      if (known[0] === "and") known_operands = known.slice(1);
      else known_operands = [known];

      operands = operands.filter((v) =>
        known_operands.every((u) => !trees.equal(u, v)),
      );
    }

    if (operands.length === 1) tree = operands[0];
    else tree = [operator].concat(operands);
  }

  // check if whole thing is known
  if (operator !== "and" && known && Array.isArray(known)) {
    let known_operands;
    if (known[0] === "and") known_operands = known.slice(1);
    else known_operands = [known];

    if (!known_operands.every((u) => !trees.equal(u, tree))) return undefined;
  }

  return tree;
}

function calculate_derived_assumptions(assumptions, tree) {
  // Calculate all assumptions on variables within tree that
  // can be derived from the assumptions within tree,
  // eliminating any assumptions that are already recorded
  // in byvar or generic of assumptions
  //
  // if tree is undefined, calculate assumptions that can be
  // derived from all given assumptions

  if (tree === undefined) {
    tree = [];
    for (let v in assumptions.byvar) {
      let a = assumptions.byvar[v];
      if (a.length > 0) tree.push(a);
    }
    if (tree.length === 0) return {};

    if (tree.length === 1) tree = tree[0];
    else tree = ["and"].concat(tree);

    tree = clean_assumptions(tree);
  }

  if (!Array.isArray(tree) || tree.length === 0) return {};

  var operator = tree[0];
  var operands = tree.slice(1);

  if (operator === "and" || operator === "or") {
    let results = operands.map(function (v) {
      return calculate_derived_assumptions(assumptions, v);
    });

    // array of all vars found in at least one result
    let allvars = [
      ...new Set(results.reduce((a, b) => [...a, ...Object.keys(b)], [])),
    ];

    let derived = {};

    for (let v of allvars) {
      let res = results.reduce(function (a, b) {
        if (b[v] !== undefined) a.push(b[v]);
        return a;
      }, []);

      // for OR, add only if obtain result for each operand
      if (operator === "and" || res.length === results.length) {
        let new_derived = derived[v];
        if (new_derived === undefined) {
          if (res.length > 1) new_derived = [operator].concat(res);
          else new_derived = res[0];
        } else {
          if (res.length > 1)
            new_derived = ["and", new_derived, [operator].concat(res)];
          else new_derived = ["and", new_derived, res[0]];
        }

        derived[v] = clean_assumptions(
          new_derived,
          get_assumptions(assumptions, v, { omit_derived: true }),
        );
      }
    }

    return derived;
  }
  // Shouldn't get a NOT of (in)equality after simplifying logical
  // if(operator === 'not') {
  // 	let results = calculate_derived_assumptions(assumptions, operands[0]);
  // 	for(let v of Object.keys(results)) {
  // 	    derived[v] = ['not', results[v]];
  // 	}
  // 	return derived;
  // }

  let derived = {};

  if (
    operator === "=" ||
    operator === "ne" ||
    operator === "<" ||
    operator === "le" ||
    operator === "in" ||
    operator === "subset" ||
    operator === "notin" ||
    operator === "notsubset"
  ) {
    var addressed_assumption = false;

    // calculate derived if one side is equal to a variable
    for (let ind = 0; ind < 2; ind++) {
      let v = operands[ind];
      let other = operands[1 - ind];
      let other_var = variables_in(other);
      if (
        typeof v !== "string" ||
        other_var.length === 0 ||
        other_var.includes(v)
      )
        continue;

      addressed_assumption = true;

      // look for any assumptions that from other that
      // do not contain a v
      var adjusted_op = operator;
      if (ind === 1) {
        if (operator === "<") adjusted_op = ">";
        else if (operator === "le") adjusted_op = "ge";
        else if (operator === "in") adjusted_op = "ni";
        else if (operator === "subset") adjusted_op = "superset";
        else if (operator === "notin") adjusted_op = "notni";
        else if (operator === "notsubset") adjusted_op = "notsuperset";
      }
      let result = get_assumptions_for_expr(assumptions, other, [v]);

      // combine with results for expr, if compatible
      result = combine_assumptions(v, adjusted_op, other, result);

      if (result !== undefined) {
        let new_derived = derived[v];

        if (new_derived === undefined) {
          new_derived = result;
        } else {
          new_derived = ["and", new_derived, result];
        }

        derived[v] = clean_assumptions(
          new_derived,
          get_assumptions(assumptions, v, { omit_derived: true }),
        );
      }
    }
    if (addressed_assumption) return derived;
  }

  // if wasn't able to combine expressions, just add any assumptions
  // on the operands
  let results = [];

  for (let op of operands) {
    let res = get_assumptions_for_expr(assumptions, op, []);
    if (res !== undefined) results.push(res);
  }

  if (results.length === 0) return {};

  if (results.length === 1) results = results[0];
  else results = ["and"].concat(results);

  for (let v of variables_in(tree)) {
    derived[v] = clean_assumptions(
      results,
      get_assumptions(assumptions, v, { omit_derived: true }),
    );
  }

  return derived;
}

function get_assumptions_for_expr(assumptions, expr, exclude_variables) {
  // return any assumptions that can be calculated for expression expr
  // that don't include exclude_variables
  //
  // The assumptions will be given directly in terms of expr when possible.

  let variables = variables_in(expr);

  // filter out any of the excluded variables
  variables = variables.filter((v) => !exclude_variables.includes(v));

  if (variables.length === 0) return undefined;

  function isNumber(s) {
    if (typeof s === "number") return true;
    if (Array.isArray(s) && s[0] === "-" && typeof s[1] === "number")
      return true;
    return false;
  }

  // will proccess assumptions in case where variables are linear in expr
  var pattern = ["_b"];
  var implicit_identities = ["_b"];
  var match_vars = { _b: isNumber };
  var coeff_mapping = {};
  for (let i = 0; i < variables.length; i++) {
    let a = "_a" + i;
    coeff_mapping[variables[i]] = a;
    pattern.push(["*", a, variables[i]]);
    implicit_identities.push(a);
    match_vars[a] = isNumber;
  }

  pattern = ["+"].concat(pattern);

  var m = trees.match(expr, pattern, {
    variables: match_vars,
    allow_permutations: true,
    allow_extended_match: false,
    allow_implicit_identities: implicit_identities,
    max_group: 1,
  });

  if (!m) {
    // if not linear, get assumptions for each variable of expr
    let results = [];
    for (let v of variables_in(expr)) {
      let res = get_assumptions_for_expr(assumptions, v, exclude_variables);
      if (res !== undefined) results.push(res);
    }
    if (results.length === 0) return undefined;
    if (results.length === 1) return results[0];
    return ["and"].concat(results);
  }

  for (let v of variables) coeff_mapping[v] = m[coeff_mapping[v]];

  let identity = false;
  if (m["_b"] === 0 && m["_a0"] === 1 && variables.length === 1)
    identity = true;

  // find all assumptions involving variables but excluding exclude_variables
  var new_assumptions;
  new_assumptions = get_assumptions(assumptions, [variables], {
    exclude_variables: exclude_variables,
  });

  if (new_assumptions === undefined) return undefined;

  return clean_assumptions(process_additional_assumptions(new_assumptions));

  function process_additional_assumptions(new_as) {
    if (!Array.isArray(new_as)) return undefined;

    var operator = new_as[0];
    var operands = new_as.slice(1);

    if (operator === "and" || operator === "or") {
      let results = operands
        .map(process_additional_assumptions)
        .filter((v) => v !== undefined);

      if (results.length === 0) return undefined;
      if (operator === "or") {
        if (results.length === operands.length) return ["or"].concat(results);
        else return undefined;
      }
      if (results.length === 1) return results[0];
      else return ["and"].concat(results);
    }

    // can ignore NOTs, as simplify_logical should remove
    // any before (in)equalities or containments

    if (
      !(
        ["=", "ne", "<", "le"].includes(operator) ||
        (["in", "notin", "subset", "notsubset"].includes(operator) && identity)
      )
    ) {
      let new_exclude = exclude_variables.concat(variables_in(expr));
      let results = [];
      for (let v of variables_in(new_as)) {
        if (new_exclude.includes(v)) continue;
        let res = get_assumptions_for_expr(assumptions, v, new_exclude);
        if (res !== undefined) results.push(res);
      }
      if (results.length === 0) return new_as;
      if (results.length === 1) return ["and", new_as, results[0]];
      return ["and", new_as].concat(results);
    }

    let results = [];

    for (let ind = 0; ind <= 1; ind++) {
      let next_var = operands[ind];
      let next_rhs = operands[1 - ind];

      if (typeof next_var === "string" && variables.includes(next_var)) {
        var bindings = {};
        bindings[next_var] = next_rhs;
        var new_expr = simplify.simplify(trees.substitute(expr, bindings));

        // may need to flip operator if it is an inequality
        // Two factors could induce flipping
        // - coefficient from expr is negative
        // - switched sides in next inequality (ind === 1)
        // The factors could cancel each other out
        var flip = false;
        var operator_eff = operator;
        if (
          (ind === 1 && coeff_mapping[next_var] > 0) ||
          (ind === 0 && coeff_mapping[next_var] < 0)
        ) {
          if (operator === "<") {
            flip = true;
            operator_eff = ">";
          } else if (operator === "le") {
            flip = true;
            operator_eff = "ge";
          } else if (operator === "in") {
            flip = true;
            operator_eff = "ni";
          } else if (operator === "subset") {
            flip = true;
            operator_eff = "superset";
          } else if (operator === "notin") {
            flip = true;
            operator_eff = "notni";
          } else if (operator === "notsubset") {
            flip = true;
            operator_eff = "notsuperset";
          }
        }

        if (flip) results.push([operator, new_expr, expr]);
        else results.push([operator, expr, new_expr]);

        // look for more assumptions
        let new_exclude = exclude_variables.concat([next_var]);
        let res = get_assumptions_for_expr(assumptions, new_expr, new_exclude);
        // combine with results for expr, if compatible
        res = combine_assumptions(expr, operator_eff, new_expr, res);

        if (res !== undefined) results.push(res);
      }
    }

    if (results.length === 1) return results[0];
    else if (results.length > 1) return ["and"].concat(results);

    // didn't address assumption
    let new_exclude = exclude_variables.concat(variables_in(expr));
    results = [];
    for (let v of variables_in(new_as)) {
      if (new_exclude.includes(v)) continue;
      let res = get_assumptions_for_expr(assumptions, v, new_exclude);
      if (res !== undefined) results.push(res);
    }
    if (results.length === 0) return new_as;
    if (results.length === 1) return ["and", new_as, results[0]];
    return ["and", new_as].concat(results);
  }
}

function combine_assumptions(expr1, op1, expr2, new_as) {
  // given the assumption "expr1 op1 expr2" and assumptions from new_as
  // - return new assumptions involving expr1, if possible
  // - return undefined if new_as appears to not affect expr1
  // - return new_as if new_as appear to affect expr1 but cannot
  //   be distilled to assumptions on expr1

  if (
    ![
      "=",
      "ne",
      "<",
      "le",
      ">",
      "ge",
      "in",
      "notin",
      "ni",
      "notni",
      "subset",
      "notsubset",
      "superset",
      "notsuperset",
    ].includes(op1)
  )
    return new_as;

  if (!Array.isArray(new_as)) return undefined;

  var op2 = new_as[0];
  var operands2 = new_as.slice(1);

  if (op2 === "and" || op2 === "or") {
    let results = operands2
      .map((v) => combine_assumptions(expr1, op1, expr2, v))
      .filter((v) => v !== undefined);

    if (results.length === 0) return undefined;
    if (op2 === "or") {
      if (results.length === operands2.length) return [["or"].concat(results)];
      else return undefined;
    }
    if (results.length === 1) return results[0];
    else return ["and"].concat(results);
  }

  if (
    !["=", "ne", "<", "le", "in", "notin", "subset", "notsubset"].includes(op2)
  )
    return new_as;

  var op2_eff = op2;
  var rhs;
  if (trees.equal(operands2[0], expr2)) {
    rhs = operands2[1];
  } else if (trees.equal(operands2[1], expr2)) {
    rhs = operands2[0];
    if (op2 === "<") op2_eff = ">";
    else if (op2 === "le") op2_eff = "ge";
    else if (op2 === "in") op2_eff = "ni";
    else if (op2 === "notin") op2_eff = "notni";
    else if (op2 === "subset") op2_eff = "superset";
    else if (op2 === "notsubset") op2_eff = "notsuperset";
  } else {
    return new_as;
  }

  // determined operator of combined expression
  var combined_op;
  if (op1 === "=") combined_op = op2_eff;
  else if (op2_eff === "=") combined_op = op1;
  else if (op1 === "<") {
    if (op2_eff === "<" || op2_eff === "le") combined_op = "<";
    else if (op2_eff === "in" || op2_eff === "notin") return new_as;
    else return undefined; // incompatible operators
  } else if (op1 === "le") {
    if (op2_eff === "<") combined_op = "<";
    else if (op2_eff === "le") combined_op = "le";
    else if (op2_eff === "in" || op2_eff === "notin") return new_as;
    else return undefined; // incompatible operators
  } else if (op1 === ">") {
    if (op2_eff === ">" || op2_eff === "ge") combined_op = ">";
    else if (op2_eff === "in" || op2_eff === "notin") return new_as;
    else return undefined; // incompatible operators
  } else if (op1 === "ge") {
    if (op2_eff === ">") combined_op = ">";
    else if (op2_eff === "ge") combined_op = "ge";
    else if (op2_eff === "in" || op2_eff === "notin") return new_as;
    else return undefined; // incompatible operators
  } else if (op1 === "in") {
    if (op2_eff === "subset") combined_op = "in";
    else return undefined; // incompatible operators
  } else if (op1 === "notin") {
    if (op2_eff === "superset") combined_op = "notin";
    else return undefined; // incompatible operators
  } else if (op1 === "ni") {
    if (op2_eff === "notin") combined_op = "notsubset";
    else return undefined; // incompatible operators
  } else if (op1 === "notni") {
    if (op2_eff === "in") combined_op = "notsuperset";
    else return undefined; // incompatible operators
  } else if (op1 === "subset") {
    if (op2_eff === "subset") combined_op = "subset";
    else if (op2_eff === "notni") combined_op = "notni";
    else if (op2_eff === "notsuperset") combined_op = "notsuperset";
    else return undefined; // incompatible operators
  } else if (op1 === "notsubset") {
    if (op2_eff === "superset") combined_op = "notsubset";
    else return undefined; // incompatible operators
  } else if (op1 === "superset") {
    if (op2_eff === "superset") combined_op = "superset";
    else if (op2_eff === "ni") combined_op = "ni";
    else if (op2_eff === "notsubset") combined_op = "notsubset";
    else return undefined; // incompatible operators
  } else if (op1 === "notsuperset") {
    if (op2_eff === "subset") combined_op = "notsuperset";
    else return undefined; // incompatible operators
  } else return undefined;

  if (combined_op === ">") return ["<", rhs, expr1];
  else if (combined_op === "ge") return ["le", rhs, expr1];
  else if (combined_op === "ni") return ["in", rhs, expr1];
  else if (combined_op === "notni") return ["notin", rhs, expr1];
  else if (combined_op === "superset") return ["subset", rhs, expr1];
  else if (combined_op === "notsuperset") return ["notsubset", rhs, expr1];
  else return [combined_op, expr1, rhs];
}

function filter_assumptions_from_tree(tree, exclude_variables) {
  // return an ast if found in tree assumptions without exclude_variables
  // otherwise return undefined

  if (!Array.isArray(tree) || tree.length === 0) {
    return undefined;
  }

  if (!Array.isArray(exclude_variables))
    exclude_variables = [exclude_variables];

  var operator = tree[0];
  var operands = tree.slice(1);

  if (operator === "and") {
    var a = operands.map(function (v) {
      return filter_assumptions_from_tree(v, exclude_variables);
    });

    a = a.filter((v) => v !== undefined);

    if (a.length === 0) return undefined;
    else if (a.length === 1) return a[0];
    else return ["and"].concat(a);
  }

  // if no intersection between exclude variables and variables in tree
  // return tree
  var tree_variables = variables_in(tree);
  var contains_excluded =
    exclude_variables.filter((v) => tree_variables.includes(v)).length > 0;

  if (!contains_excluded) return tree;
  else return undefined;
}

function get_assumptions_sub(
  assumptions,
  variables,
  exclude_variables,
  omit_derived,
) {
  // return an ast if found assumptions involving variables
  // otherwise return undefined

  if (!Array.isArray(variables)) variables = [variables];

  var a = [];

  // add assumptions specified by each variable
  variables.forEach(function (v) {
    // get assumption from byvar and derived, if exist
    if (assumptions.byvar[v] || assumptions.derived[v]) {
      if (assumptions.byvar[v] && assumptions.byvar[v].length > 0) {
        // only get assumptions that don't contain
        // exclude variables
        var byvar = filter_assumptions_from_tree(
          assumptions.byvar[v],
          exclude_variables,
        );
        if (byvar !== undefined) a.push(byvar);
      }
      if (
        assumptions.derived[v] &&
        assumptions.derived[v].length > 0 &&
        !omit_derived
      ) {
        // only get derived assumptions that don't contain
        // exclude variables
        var da = filter_assumptions_from_tree(
          assumptions.derived[v],
          exclude_variables,
        );
        if (da !== undefined) a.push(da);
      }
    }
    // if byvar and derived are undefined,
    // then get assumptions from generic
    else if (assumptions["generic"].length > 0) {
      // if generic contains any variables other than x
      // don't substitute those back into generic
      if (v === "x" || !variables_in(assumptions["generic"]).includes(v))
        a.push(trees.substitute(assumptions["generic"], { x: v }));
    }
  });

  if (a.length === 1) a = a[0];
  else if (a.length > 1) a = ["and"].concat(a);

  if (a.length > 0) return clean_assumptions(a);
  else return undefined;
}

function get_assumptions(assumptions, variables_or_expr, params) {
  // return an ast if found assumptions
  // otherwise return undefined
  //
  // variables_or_expr
  // - if a string or an array of an array, find assumptions on
  //   each of the variables represented by those strings
  //   directly from byvar and derived or from generic
  // - if an ast, then
  //   - calculate assumptions of the expression itself, if possible, or
  //   - calculate assumptions on the variables of the expression.

  // include any additional assumptions
  // involving new variables found in assumptions

  if (params === undefined) params = {};

  var exclude_variables = params.exclude_variables;
  if (exclude_variables === undefined) exclude_variables = [];
  else if (!Array.isArray(exclude_variables))
    exclude_variables = [exclude_variables];

  var variables;
  var tree = get_tree(variables_or_expr);

  // if string, have a variable
  if (typeof tree === "string") variables = [tree];
  else if (!Array.isArray(tree)) return undefined;
  else if (Array.isArray(tree[0]))
    // if array containing array, is list of variables
    variables = tree[0];

  if (variables)
    return get_assumptions_sub(
      assumptions,
      variables,
      exclude_variables,
      params.omit_derived,
    );
  else return get_assumptions_for_expr(assumptions, tree, exclude_variables);
}

function add_assumption(assumptions, expr_or_tree, exclude_generic) {
  // add assumption in tree to assumptions
  // if !exclude_generic, then add any generic assumptions to
  // variables if they don't have previous assumptions
  // return 1 if added assumption or 0 otherwise

  var tree = get_tree(expr_or_tree);

  if (!Array.isArray(tree)) return 0;

  tree = clean_assumptions(simplify.simplify(tree, assumptions));

  var added = add_assumption_sub(assumptions, tree, exclude_generic);

  if (added) assumptions.derived = calculate_derived_assumptions(assumptions);

  return added;
}

function add_assumption_sub(assumptions, tree, exclude_generic) {
  // add assumption in tree to assumptions
  // if !exclude_generic, then add any generic assumptions to
  // variables if they don't have previous assumptions
  // return number of assumptions added

  // if tree is an 'and', call once for each operand
  // so that assumptions can be separated by variable
  if (tree[0] === "and") {
    var results = tree
      .slice(1)
      .map((v) => add_assumption_sub(assumptions, v, exclude_generic));
    return results.reduce(function (a, b) {
      return a + b;
    });
  }

  var variables = variables_in(tree);

  if (variables.length === 0) return 0;

  let n_added = 0;

  if (!exclude_generic && assumptions["generic"].length > 0) {
    // check to see if any assumptions already for each variable
    // if not, start by assigning generic assumptions
    variables.forEach(function (v) {
      if (assumptions.byvar[v] === undefined) {
        // no previous assumptions, so
        // include add assumption for v corresponding to generic
        // unless non-x v is explicitly in generic
        if (v === "x" || !variables_in(assumptions["generic"]).includes(v)) {
          add_assumption_sub(
            assumptions,
            trees.substitute(assumptions["generic"], { x: v }),
            true,
          );
          n_added += 1;
        }
      }
    });
  }

  // attempt to solve for each variable
  for (let variable of variables) {
    // solve using current state of assumptions
    let solved = solve.solve_linear(tree, variable, assumptions);

    let new_a = tree;
    if (solved) new_a = solved;

    let current_a = assumptions["byvar"][variable];

    if (current_a !== undefined && current_a.length !== 0)
      new_a = ["and", current_a, new_a];

    new_a = clean_assumptions(new_a);

    if (!trees.equal(new_a, current_a)) {
      assumptions["byvar"][variable] = new_a;
      n_added += 1;
    }
  }

  return n_added;
}

function add_generic_assumption(assumptions, expr_or_tree) {
  // add assumption in expr_or_tree to generic assumptions

  // tree must contain the variable x
  // the variable x represents any variable for which
  // assumptions aren't specifically assigned

  // return 1 if added assumption or 0 otherwise

  var tree = get_tree(expr_or_tree);

  if (!Array.isArray(tree)) return 0;

  tree = clean_assumptions(simplify.simplify(tree, assumptions));

  var added = add_generic_assumption_sub(assumptions, tree);

  if (added) assumptions.derived = calculate_derived_assumptions(assumptions);

  return added;
}

function add_generic_assumption_sub(assumptions, tree) {
  // if tree is an 'and', call once for each operand
  // so that assumptions involving one variable can be separated
  if (tree[0] === "and") {
    var results = tree
      .slice(1)
      .map((v) => add_generic_assumption_sub(assumptions, v));
    return results.reduce(function (a, b) {
      return a + b;
    });
  }

  var variables = variables_in(tree);

  if (!variables.includes("x")) return 0;

  // attempt to solve for x
  // solve using current state of assumptions
  let solved = solve.solve_linear(tree, "x", assumptions);

  let new_a = tree;
  if (solved) new_a = solved;

  let current_a = assumptions["generic"];

  if (current_a.length !== 0) new_a = ["and", current_a, new_a];

  new_a = clean_assumptions(new_a);

  if (trees.equal(new_a, current_a)) {
    return 0;
  }

  assumptions["generic"] = new_a;

  return 1;
}

function remove_assumption(assumptions, expr_or_tree) {
  var tree = get_tree(expr_or_tree);

  if (!Array.isArray(tree)) return 0;

  tree = clean_assumptions(simplify.simplify(tree, assumptions));

  var removed = remove_assumption_sub(assumptions, tree);

  if (removed) assumptions.derived = calculate_derived_assumptions(assumptions);

  return removed;
}

function remove_assumption_sub(assumptions, tree) {
  // if tree is an 'and', call once for each operand
  // so that assumptions can be separated by variable
  if (tree[0] === "and") {
    var results = tree
      .slice(1)
      .map((v) => remove_assumption_sub(assumptions, v));
    return results.reduce(function (a, b) {
      return a + b;
    });
  }

  var variables = variables_in(tree);

  if (variables.length === 0) return 0;

  var n_removed = 0;

  // attempt to solve for each variable
  for (let variable of variables) {
    // solve using current state of assumptions
    let solved = solve.solve_linear(tree, variable, assumptions);

    let current = assumptions["byvar"][variable];

    // didn't find any assumptions to remove
    if (!current || current.length === 0) {
      continue;
    }

    // remove any occurence of tree from current
    let operator = current[0];
    let operands = current.slice(1);

    let n_op = operands.length;

    let result;

    if (operator === "and") {
      // remove any match, using trees.equal
      operands = operands.filter(
        (v) => !(trees.equal(v, tree) || trees.equal(v, solved)),
      );

      if (operands.length === 0) {
        result = [];
      } else if (operands.length === 1) {
        result = operands[0];
      } else if (operands.length < n_op) {
        result = [operator].concat(operands);
      } else {
        // didn't find anything to remove
        continue;
      }
    } else {
      if (trees.equal(current, tree) || trees.equal(current, solved)) {
        result = [];
      } else {
        // didn't find anything to remove
        continue;
      }
    }

    n_removed += 1;
    assumptions["byvar"][variable] = result;
  }

  return n_removed;
}

function remove_generic_assumption(assumptions, expr_or_tree) {
  // remove assumption in expr_or_tree from generic assumptions

  // return 1 if removed assumption or 0 otherwise

  var tree = get_tree(expr_or_tree);

  if (!Array.isArray(tree)) return 0;

  tree = clean_assumptions(simplify.simplify(tree, assumptions));

  var removed = remove_generic_assumption_sub(assumptions, tree);

  if (removed) assumptions.derived = calculate_derived_assumptions(assumptions);

  return removed;
}

function remove_generic_assumption_sub(assumptions, tree) {
  // if tree is an 'and', call once for each operand
  // so that assumptions involving one variable can be separated
  if (tree[0] === "and") {
    var results = tree
      .slice(1)
      .map((v) => remove_generic_assumption_sub(assumptions, v));
    return results.reduce(function (a, b) {
      return a + b;
    });
  }

  var variables = variables_in(tree);

  if (!variables.includes("x")) return 0;

  var current = assumptions["generic"];

  if (current.length === 0) return 0;

  // solve using current state of assumptions
  let solved = solve.solve_linear(tree, "x", assumptions);

  // remove any occurence of tree from current
  var operator = current[0];
  var operands = current.slice(1);

  var n_op = operands.length;

  var result;

  if (operator === "and") {
    // remove any match, using trees.equal
    operands = operands.filter(
      (v) => !(trees.equal(v, tree) || trees.equal(v, solved)),
    );

    if (operands.length === 0) {
      result = [];
    } else if (operands.length === 1) {
      result = operands[0];
    } else if (operands.length < n_op) {
      result = [operator].concat(operands);
    } else {
      // didn't find anything to remove
      return 0;
    }
  } else {
    if (trees.equal(current, tree) || trees.equal(current, solved)) {
      result = [];
    } else {
      // didn't find anything to remove
      return 0;
    }
  }

  assumptions["generic"] = result;

  return 1;
}

function initialize_assumptions() {
  var assumptions = {};
  assumptions["byvar"] = {};
  assumptions["derived"] = {};
  assumptions["generic"] = [];
  assumptions["not_commutative"] = [];
  assumptions["get_assumptions"] = function (v, params) {
    return get_assumptions(assumptions, v, params);
  };
  assumptions["add_assumption"] = function (v, exclude_generic) {
    return add_assumption(assumptions, v, exclude_generic);
  };
  assumptions["add_generic_assumption"] = function (v) {
    return add_generic_assumption(assumptions, v);
  };
  assumptions["remove_assumption"] = function (v) {
    return remove_assumption(assumptions, v);
  };
  assumptions["remove_generic_assumption"] = function (v) {
    return remove_generic_assumption(assumptions, v);
  };

  return assumptions;
}

export {
  clean_assumptions,
  get_assumptions,
  initialize_assumptions,
  add_assumption,
  add_generic_assumption,
  remove_assumption,
  remove_generic_assumption,
};
