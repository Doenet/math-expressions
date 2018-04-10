// check for equality by randomly sampling

import math from '../../mathjs';
import { is_integer_ast } from '../../assumptions/element_of_sets';

function generate_random_integer(minvalue, maxvalue) {
  minvalue = math.ceil(minvalue);
  maxvalue = math.floor(maxvalue);
  return math.floor(math.random()*(maxvalue-minvalue+1)) + minvalue;
}



export const equals = function(expr, other, randomBindings,
			       expr_context, other_context) {

  if(Array.isArray(expr.tree) && Array.isArray(other.tree)) {
    
    let expr_operator = expr.tree[0];
    let expr_operands = expr.tree.slice(1);
    let other_operator = other.tree[0];
    let other_operands = other.tree.slice(1);

    if(expr_operator === 'tuple' || expr_operator === 'vector'
       || expr_operator === 'list' || expr_operator === 'array'
       || expr_operator === 'matrix'
      ) {

      if(other_operator !== expr_operator)
	return false;

      if(other_operands.length !== expr_operands.length)
	return false;

      for(let i=0; i<expr_operands.length; i++) {
	if(!equals(expr_context.fromAst(expr_operands[i]),
		   other_context.fromAst(other_operands[i]),
		   randomBindings,
		   expr_context, other_context))
	  return false;
      }

      return true;  // each component is equal
    }
  }

  // if not special case, use standard numerical equality
  return component_equals(expr, other, randomBindings,
			  expr_context, other_context);

}


const component_equals = function(expr, other, randomBindings,
			       expr_context, other_context) {

  expr = expr.normalize_function_names();
  other = other.normalize_function_names();

  var max_value = Number.MAX_VALUE*1E-20;

  var epsilon = 1E-12;
  var minimum_matches = 10;

  // Get set of variables mentioned in at least one of the two expressions
  var variables = [ expr.variables(), other.variables() ];
  variables = variables.reduce( function(a,b) { return a.concat(b); } )
  variables = variables.reduce(function(p, c) {
    if (p.indexOf(c) < 0) p.push(c);
    return p;
  }, []);

  // determine if any of the variables are integers
  // consider integer if is integer in either expressions' assumptions
  var integer_variables = [];
  for(var i=0; i < variables.length; i++)
    if(is_integer_ast(variables[i], expr_context.assumptions)
       || is_integer_ast(variables[i], other_context.assumptions))
      integer_variables.push(variables[i]);


  var expr_f = expr.f();
  var other_f = other.f();

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
  // conclusion that expression are unequal. Instead, to be consider unequal either
  // A. the functions are unequal at many points, or B
  // B. the functions are equal at a point and then unequal at a nearby point


  for(var i=0; i<100; i++) {
    
    // Look for a location where the magnitudes of both expressions
    // are below max_value;
    try {
      var result = find_equality_region(binding_scales[scale_num]);
    }
    catch(e) {
      continue;
    }
    if(result.equality === true) {
      if(result.always_zero) {
	// functions equal but zero
	// try changing the scale and repeating
	scale_num +=1;
	if(scale_num >= binding_scales.length)
	  return true;  // were equal and zero at all scales
	else
	  continue
      }
      else
	return true;
    }
    else if(result.equality === false)
      return false;
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
    // 5. If find a point where the functions are not equality,
    //    then return { equality: false }
    // 6. If find that functions are equal at minimum_matches points
    //    then return { equality: true, always_zero: always_zero }
    //    where always_zero is true if both functions were always zero
    //    and is false otherwise
    // 7. If were unable to find sufficent points where both functions are findit
    //    return { sufficient_finite_values: false }
    
    
    var bindings= randomBindings(variables, noninteger_scale);

    // replace any integer variables with integer
    for(let i=0; i<integer_variables.length; i++) {
      bindings[integer_variables[i]] = generate_random_integer(-10,10);
    }

    var expr_evaluated = expr_f(bindings);
    var other_evaluated = other_f(bindings);

    var expr_abs = math.abs(expr_evaluated);
    var other_abs = math.abs(other_evaluated);

    if(expr_abs >= max_value || other_abs > max_value)
      return { out_of_bounds: true };
    
    // now that found a finite point,
    // check to see if expressions are nearly equal.

    var min_mag = math.min(expr_abs, other_abs);
    if(math.abs(math.subtract(expr_evaluated, other_evaluated))
       > min_mag * epsilon)
      return { equal_at_start: false };


    var always_zero = (min_mag == 0);
    
    // Look for a region around point
    var finite_tries = 0;
    for(let j=0; j<100; j++) {
      var bindings2 = randomBindings(
	variables, noninteger_binding_scale/100, bindings);

      // replace any integer variables with integer
      for(let k=0; k<integer_variables.length; k++) {
	bindings2[integer_variables[k]]
	  = generate_random_integer(-10,10);
      }

      try {
	expr_evaluated = expr_f(bindings2);
	other_evaluated = other_f(bindings2);
      }
      catch (e) {
	continue;
      }
      expr_abs = math.abs(expr_evaluated);
      other_abs = math.abs(other_evaluated);

      if(expr_abs < max_value && other_abs < max_value) {
	min_mag = math.min(expr_abs, other_abs);

	finite_tries++;

	if(math.abs(math.subtract(expr_evaluated, other_evaluated))
	   > min_mag * epsilon) {
	  return { equality: false };
	}

	always_zero = always_zero && (min_mag == 0);

	if(finite_tries >=minimum_matches) {
	  return { equality: true, always_zero: always_zero }
	}
      }
    }
    return { sufficient_finite_values: false };
  }

}
