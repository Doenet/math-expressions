// check for equality by randomly sampling

import math from '../../mathjs';
import { is_integer_ast } from '../../assumptions/element_of_sets';

function generate_random_integer(minvalue, maxvalue) {
  minvalue = math.ceil(minvalue);
  maxvalue = math.floor(maxvalue);
  return math.floor(math.random() * (maxvalue - minvalue + 1)) + minvalue;
}



export const equals = function ({ expr, other, randomBindings,
  expr_context, other_context,
  tolerance = 1E-12, allowed_error_in_numbers = 0,
  include_error_in_number_exponents = false,
}) {

  if (Array.isArray(expr.tree) && Array.isArray(other.tree)) {

    let expr_operator = expr.tree[0];
    let expr_operands = expr.tree.slice(1);
    let other_operator = other.tree[0];
    let other_operands = other.tree.slice(1);

    if (expr_operator === 'tuple' || expr_operator === 'vector'
      || expr_operator === 'list' || expr_operator === 'array'
      || expr_operator === 'matrix' || expr_operator === 'interval'
    ) {

      if (other_operator !== expr_operator)
        return false;

      if (other_operands.length !== expr_operands.length)
        return false;

      for (let i = 0; i < expr_operands.length; i++) {
        if (!equals({
          expr: expr_context.fromAst(expr_operands[i]),
          other: other_context.fromAst(other_operands[i]),
          randomBindings: randomBindings,
          expr_context: expr_context,
          other_context: other_context,
          tolerance: tolerance,
          allowed_error_in_numbers: allowed_error_in_numbers,
          include_error_in_number_exponents: include_error_in_number_exponents,
        }))
          return false;
      }

      return true;  // each component is equal
    }

    // check if a relation with two operands
    if (expr_operands.length === 2 && ["=", '>', '<', 'ge', 'le'].includes(expr_operator)) {
      if (other_operands.length !== 2) {
        return false;
      }
      //normalize operator
      if (expr_operator === ">") {
        expr_operator = "<";
        expr_operands = [expr_operands[1], expr_operands[0]];
      } else if (expr_operator === "ge") {
        expr_operator = "le";
        expr_operands = [expr_operands[1], expr_operands[0]];
      }
      if (other_operator === ">") {
        other_operator = "<";
        other_operands = [other_operands[1], other_operands[0]];
      } else if (other_operator === "ge") {
        other_operator = "le";
        other_operands = [other_operands[1], other_operands[0]];
      }

      if (expr_operator !== other_operator) {
        return false;
      }

      // put in standard form
      let expr_rhs = ['+', expr_operands[0], ['-', expr_operands[1]]];
      let other_rhs = ['+', other_operands[0], ['-', other_operands[1]]];
      let require_positive_proportion = (expr_operator !== "=");

      return component_equals({
        expr: expr_context.fromAst(expr_rhs),
        other: other_context.fromAst(other_rhs),
        randomBindings: randomBindings,
        expr_context: expr_context,
        other_context: other_context,
        allow_proportional: true,
        require_positive_proportion: require_positive_proportion,
        tolerance: tolerance,
        allowed_error_in_numbers: allowed_error_in_numbers,
        include_error_in_number_exponents: include_error_in_number_exponents,
      });

    }

  }

  // if not special case, use standard numerical equality
  return component_equals({
    expr: expr,
    other: other,
    randomBindings: randomBindings,
    expr_context: expr_context,
    other_context: other_context,
    tolerance: tolerance,
    allowed_error_in_numbers: allowed_error_in_numbers,
    include_error_in_number_exponents: include_error_in_number_exponents,
  });

}


const component_equals = function ({ expr, other, randomBindings,
  expr_context, other_context,
  allow_proportional = false, require_positive_proportion = false,
  tolerance, allowed_error_in_numbers, include_error_in_number_exponents,
}) {

  var max_value = Number.MAX_VALUE * 1E-20;
  var min_nonzero_value = 0;//1E-100; //Number.MIN_VALUE & 1E20;

  var epsilon = tolerance;
  var minimum_matches = 10;
  var number_tries = 100;
  // if (allowed_error_in_numbers > 0) {
  //   minimum_matches = 400;
  //   number_tries = 4000;
  // }

  // normalize function names, so in particular, e^x becomes exp(x)
  expr = expr.normalize_function_names();
  other = other.normalize_function_names();

  // Get set of variables mentioned in at least one of the two expressions
  var variables = [expr.variables(), other.variables()];
  variables = variables.reduce(function (a, b) { return a.concat(b); })
  variables = variables.reduce(function (p, c) {
    if (p.indexOf(c) < 0) p.push(c);
    return p;
  }, []);

  // pi, e, and i shouldn't be treated as a variable
  // for the purposes of equality if they are defined as having values
  if (math.define_pi) {
    variables = variables.filter(function (a) {
      return (a !== "pi");
    });
  }
  if (math.define_i) {
    variables = variables.filter(function (a) {
      return (a !== "i");
    });
  }
  if (math.define_e) {
    variables = variables.filter(function (a) {
      return (a !== "e");
    });
  }

  // determine if any of the variables are integers
  // consider integer if is integer in either expressions' assumptions
  var integer_variables = [];
  for (let i = 0; i < variables.length; i++)
    if (is_integer_ast(variables[i], expr_context.assumptions)
      || is_integer_ast(variables[i], other_context.assumptions))
      integer_variables.push(variables[i]);

  // determine if any of the variables are functions
  var functions = [expr.functions(), other.functions()];
  functions = functions.reduce(function (a, b) { return a.concat(b); })
  functions = functions.reduce(function (p, c) {
    if (p.indexOf(c) < 0) p.push(c);
    return p;
  }, []);
  functions = functions.filter(function (a) {
    return a.length == 1;
  });

  try {
    var expr_f = expr.f();
    var other_f = other.f();
  }
  catch (e) {
    // Can't convert to mathjs to create function
    // just check if equal via syntax
    return expr.equalsViaSyntax(other)
  }

  let expr_with_params, parameters_for_numbers;
  let tolerance_function;

  if (allowed_error_in_numbers > 0) {
    let result = replace_numbers_with_parameters({
      expr: expr,
      variables: variables,
      include_exponents: include_error_in_number_exponents,
    });
    expr_with_params = expr_context.fromAst(result.expr_with_params);
    parameters_for_numbers = result.parameters;

    let parameter_list = Object.keys(parameters_for_numbers);
    if (parameter_list.length > 0) {
      let derivative_sum = expr_with_params.derivative(parameter_list[0])
        .multiply(parameters_for_numbers[parameter_list[0]])

      if (parameter_list.length > 1) {
        for (let par of parameter_list.slice(1)) {
          derivative_sum = derivative_sum.add(expr_with_params.derivative(par)
            .multiply(parameters_for_numbers[par])
          )
        }
      }

      let tolerance_expression = derivative_sum.multiply(allowed_error_in_numbers);

      try {
        tolerance_function = tolerance_expression.f();
      } catch (e) {
        // can't create function out of derivative
        // so can't compute tolerance that would correspond
        // to the allowed error in numbers

        // Leave tolerance_function undefined

      }

    }

  }



  var noninteger_binding_scale = 1;

  var binding_scales = [10, 1, 100, 0.1, 1000, 0.01];
  var scale_num = 0;

  // Numerical test of equality
  // If can find a region of the complex plane where the functions are equal
  // at minimum_matches points, consider the functions equal
  // unless the functions were always zero, in which case
  // test at multiple scales to check for underflow

  // In order to account for possible branch cuts,
  // finding points where the functions are not equal does not lead to the
  // conclusion that expression are unequal. Instead, to be consider unequal
  // the functions must be unequal around many different points.

  let num_at_this_scale = 0;

  let always_zero = true;

  let num_finite_unequal = 0;

  for (let i = 0; i < 10 * number_tries; i++) {

    // Look for a location where the magnitudes of both expressions
    // are below max_value;
    try {
      var result = find_equality_region(binding_scales[scale_num]);
    }
    catch (e) {
      continue;
    }

    if (result.always_zero === false) {
      always_zero = false;
    }


    if (!result.equal && !result.out_of_bounds && !result.always_zero &&
      result.sufficient_finite_values !== false
    ) {
      num_finite_unequal++;
      if (num_finite_unequal > number_tries) {
        return false;
      }
    }

    if (result.equal) {
      if (result.always_zero) {
        if (!always_zero) {
          // if found always zero this time, but wasn't zero at a different point
          // don't count as equal
          continue;
        }
        // functions equal but zero
        // repeat to make sure (changing if continuing to be zero)
        num_at_this_scale += 1;
        if (num_at_this_scale > 5) {
          scale_num += 1;
          num_at_this_scale = 0;
        }
        if (scale_num >= binding_scales.length) {
          return true;  // were equal and zero at all scales
        } else
          continue
      }
      else {
        return true;
      }
    }
  }
  return false;



  function find_equality_region(noninteger_scale) {

    // Check if expr and other are equal in a region as follows
    // 1. Randomly select bindings (use noninteger scale for non-integer variables)
    //    and evaluate expr and other at that point
    // 2. If either value is too large, return { out_of_bounds: true }
    // 3. If values are not equal (within tolerance), return { equal_at_start: false }
    // 4. If functions are equal, then
    //    randomly select binding in neighborhood of that point
    //    (use non_integer scale/100 for non-integer variables)
    // 5. If find a point where the functions are not equal,
    //    then return { equal_in_middle: false }
    // 6. If find that functions are equal at minimum_matches points
    //    then return { equality: true, always_zero: always_zero }
    //    where always_zero is true if both functions were always zero
    //    and is false otherwise
    // 7. If were unable to find sufficent points where both functions are finite
    //    return { sufficient_finite_values: false }
    // If allow_proportional is true, then instead of return non-equal
    // in step 3, use the ratio of value of these first evaluations to set
    // the proportion, and base equality on remaining values having the
    // same proportion



    var bindings = randomBindings(variables, noninteger_scale);

    // replace any integer variables with integer
    for (let i = 0; i < integer_variables.length; i++) {
      bindings[integer_variables[i]] = generate_random_integer(-10, 10);
    }

    // replace any function variables with a function
    for (let i = 0; i < functions.length; i++) {
      var a = generate_random_integer(-10, 10);
      var b = generate_random_integer(-10, 10);
      var c = generate_random_integer(-10, 10);
      bindings[functions[i]] = function (x) {
        return math.add(math.multiply(math.add(math.multiply(a, x), b), x), c);
      };
    }


    var bindingsIncludingParameters;
    if (tolerance_function) {
      bindingsIncludingParameters = Object.assign({}, bindings, parameters_for_numbers)
    }

    var expr_evaluated = expr_f(bindings);
    var other_evaluated = other_f(bindings);

    var expr_abs = math.abs(expr_evaluated);
    var other_abs = math.abs(other_evaluated);

    if (!(expr_abs < max_value && other_abs < max_value))
      return { out_of_bounds: true, always_zero: false };

    if (!((expr_abs === 0 || expr_abs > min_nonzero_value) &&
      (other_abs === 0 || other_abs > min_nonzero_value)))
      return { out_of_bounds: true, always_zero: false };

    // now that found a finite point,
    // check to see if expressions are nearly equal.

    var min_mag = Math.min(expr_abs, other_abs);
    var max_mag = Math.max(expr_abs, other_abs);
    var proportion = 1;

    let tol = 0;
    if (tolerance_function) {
      try {
        tol = math.abs(tolerance_function(bindingsIncludingParameters));
      } catch (e) {
        return { equal_at_start: false, always_zero: false };
      }
      if (!Number.isFinite(tol)) {
        return { equal_at_start: false, always_zero: false };
      }
    }

    tol += min_mag * epsilon

    // never allow tol to get over 10% the min_mag
    tol = Math.min(tol, 0.1 * min_mag);

    if (!(
      max_mag === 0 ||
      math.abs(math.subtract(expr_evaluated, other_evaluated)) < tol
    )) {
      if (!allow_proportional) {
        return { equal_at_start: false, always_zero: false };
      }
      // at this point, know both are not zero
      if (expr_abs === 0 || other_abs === 0) {
        return { equal_at_start: false, always_zero: false };
      }

      proportion = math.divide(expr_evaluated, other_evaluated);
      if (require_positive_proportion && !(proportion > 0)) {
        return { equal_at_start: false, always_zero: false };
      }
    }


    var always_zero = (max_mag === 0);

    // Look for a region around point
    var finite_tries = 0;
    for (let j = 0; j < 100; j++) {
      var bindings2 = randomBindings(
        variables, noninteger_binding_scale / 100, bindings);

      // replace any integer variables with integer
      for (let k = 0; k < integer_variables.length; k++) {
        bindings2[integer_variables[k]]
          = generate_random_integer(-10, 10);
      }

      // replace any function variables with a function
      for (let i = 0; i < functions.length; i++) {
        var a = generate_random_integer(-10, 10);
        var b = generate_random_integer(-10, 10);
        var c = generate_random_integer(-10, 10);
        bindings2[functions[i]] = function (x) {
          return math.add(math.multiply(math.add(math.multiply(a, x), b), x), c);
        };
      }

      var bindings2IncludingParameters;
      if (tolerance_function) {
        bindings2IncludingParameters = Object.assign({}, bindings2, parameters_for_numbers)
      }

      try {
        expr_evaluated = expr_f(bindings2);
        other_evaluated = math.multiply(other_f(bindings2), proportion);
      }
      catch (e) {
        continue;
      }
      expr_abs = math.abs(expr_evaluated);
      other_abs = math.abs(other_evaluated);

      if (expr_abs < max_value && other_abs < max_value) {
        min_mag = Math.min(expr_abs, other_abs);
        max_mag = Math.max(expr_abs, other_abs);

        finite_tries++;

        let tol = 0;
        if (tolerance_function) {
          try {
            tol = math.abs(tolerance_function(bindings2IncludingParameters));
          } catch (e) {
            continue;
          }
          if (!Number.isFinite(tol)) {
            continue;
          }
        }
        tol += min_mag * epsilon

        // never allow tol to get over 10% the min_mag
        tol = Math.min(tol, 0.1 * min_mag);

        if (!(
          max_mag === 0 ||
          math.abs(math.subtract(expr_evaluated, other_evaluated)) < tol
        )) {
          return { equality_in_middle: false, always_zero: false };
        }

        always_zero = always_zero && (max_mag === 0);

        if (finite_tries >= minimum_matches) {
          return { equal: true, always_zero: always_zero }
        }
      }
    }
    return { sufficient_finite_values: false, always_zero: always_zero };
  }

}

export function replace_numbers_with_parameters({ expr, variables, include_exponents = false }) {

  // find all numbers, including pi and e, if defined as numerical
  let parameters = {};
  let lastParNum = 0;

  function get_new_parameter_name() {
    lastParNum++;
    let parName = "par" + lastParNum;
    while (variables.includes(parName)) {
      lastParNum++;
      parName = "par" + lastParNum;
    }

    // found a new parameter name that isn't a variable
    return parName;

  }

  function replace_number_sub(tree) {
    if (typeof tree === 'number') {
      if (tree === 0) {
        // since will compute bounds for relative error in numbers
        // can't include zero
        return tree;
      } else {
        let par = get_new_parameter_name();
        parameters[par] = tree;
        return par;
      }
    }

    if (typeof tree === "string") {
      if (tree === "pi") {
        if (math.define_pi) {
          let par = get_new_parameter_name();
          parameters[par] = math.PI;
          return par;
        }
      } else if (tree === "e") {
        if (math.define_e) {
          let par = get_new_parameter_name();
          parameters[par] = math.e;
          return par;
        }
      }
      return tree;
    }

    if (!Array.isArray(tree)) {
      return tree;
    }

    let operator = tree[0];
    let operands = tree.slice(1);
    if (operator === "^" && !include_exponents) {
      return [operator, replace_number_sub(operands[0]), operands[1]]
    } else {
      return [operator, ...operands.map(replace_number_sub)];
    }

  }

  // first evaluate numbers to combine then
  // and turn any numerical constants to floating points
  expr = expr.evaluate_numbers({ max_digits: Infinity });
  return {
    expr_with_params: replace_number_sub(expr.tree),
    parameters: parameters
  }

}