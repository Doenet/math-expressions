import math from '../../mathjs';
import { equals as numerical_equals } from './numerical';
import seedrandom from 'seedrandom';

//import { substitute_abs } from '../normalization/standard_form';

function randomComplexBindings(rng, variables, radius, centers) {
  var result = {};

  if (centers === undefined) {
    variables.forEach(function (v) {
      result[v] = math.complex(rng() * 2 * radius - radius,
        rng() * 2 * radius - radius);
    });
  }
  else {
    variables.forEach(function (v) {
      result[v] = math.complex(
        centers[v].re + rng() * 2 * radius - radius,
        centers[v].im + rng() * 2 * radius - radius);
    });
  }

  return result;
}

export const equals = function (expr, other,
  { relative_tolerance = 1E-12, absolute_tolerance = 0, tolerance_for_zero = 1E-15,
    allowed_error_in_numbers = 0,
    include_error_in_number_exponents = false,
    allowed_error_is_absolute = false,
    allow_blanks = false
  } = {}) {

  if (!allow_blanks && (expr.variables().includes('\uFF3F') || other.variables().includes('\uFF3F'))) {
    return false;
  }

  let rng = seedrandom('complex_seed');

  //expr = expr.substitute_abs();
  //other = other.substitute_abs();

  // don't use complex equality if not analytic expression
  // except abs is OK
  if ((!expr.isAnalytic({ allow_abs: true, allow_arg: true, allow_relation: true })) ||
    (!other.isAnalytic({ allow_abs: true, allow_arg: true, allow_relation: true })))
    return false;

  return numerical_equals({
    expr: expr,
    other: other,
    randomBindings: randomComplexBindings,
    expr_context: expr.context,
    other_context: other.context,
    relative_tolerance, absolute_tolerance, tolerance_for_zero,
    allowed_error_in_numbers,
    include_error_in_number_exponents,
    allowed_error_is_absolute,
    rng
  });
};
