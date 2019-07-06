import math from '../../mathjs';
import { equals as numerical_equals } from './numerical';
//import { substitute_abs } from '../normalization/standard_form';

function randomComplexBindings(variables, radius, centers) {
  var result = {};

  if(centers === undefined) {
    variables.forEach( function(v) {
    result[v] = math.complex( math.random()*2*radius - radius,
                    math.random()*2*radius - radius );
    });
  }
  else {
    variables.forEach( function(v) {
      result[v] = math.complex(
      centers[v].re + math.random()*2*radius - radius,
      centers[v].im + math.random()*2*radius - radius );
    });
  }

  return result;
}

export const equals = function(expr, other,
  { tolerance = 1E-12, allowed_error_in_numbers = 0,
    include_error_in_number_exponents = false } = {}) {

  //expr = expr.substitute_abs();
  //other = other.substitute_abs();
  
  // don't use complex equality if not analytic expression
  // except abs is OK
  if((!expr.isAnalytic({allow_abs: true, allow_relation: true})) ||
      (!other.isAnalytic({allow_abs: true, allow_relation: true})) )
    return false;
  
  return numerical_equals({
    expr: expr,
    other: other,
    randomBindings: randomComplexBindings,
    expr_context: expr.context,
    other_context: other.context,
    tolerance: tolerance,
    allowed_error_in_numbers: allowed_error_in_numbers,
    include_error_in_number_exponents: include_error_in_number_exponents,
  });
};
