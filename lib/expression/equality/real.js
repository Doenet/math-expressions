import math from '../../mathjs';
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

export const equals = function(expr, other,
    { tolerance = 1E-12, allowed_error_in_numbers = 0,
      include_error_in_number_exponents = false } = {}) {

  // don't use real equality if not analytic expression
  if((!expr.isAnalytic()) || (!other.isAnalytic()))
    return false;

  return numerical_equals({
    expr: expr,
    other: other,
    randomBindings: randomRealBindings,
    expr_context: expr.context,
    other_content: other.context,
    tolerance: tolerance,
    allowed_error_in_numbers: allowed_error_in_numbers,
    include_error_in_number_exponents: include_error_in_number_exponents,
  });
};
