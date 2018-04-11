import math from 'mathjs';
import { equals as numerical_equals } from './numerical';


function randomRealBindings(variables, radius, centers) {
    var result = {};

    if(centers === undefined) {
	variables.forEach( function(v) {
	    result[v] = math.random()*2*radius - radius;
	});
    }
    else {
	variables.forEach( function(v) {
	    result[v] =centers[v] + math.random()*2*radius - radius;
	});
    }

    return result;
}

export const equals = function(expr, other) {

  // don't use real equality if not analytic expression
  if((!expr.isAnalytic()) || (!other.isAnalytic()))
    return false;

  let expr_normalized = expr.normalize_function_names()
      .normalize_applied_functions();
  let other_normalized = other.normalize_function_names()
      .normalize_applied_functions();
  
  return numerical_equals(expr_normalized, other_normalized,
			  randomRealBindings,
			  expr.context, other.context);
};
