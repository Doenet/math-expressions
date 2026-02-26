import math from "../../mathjs.js";
import { equals as numerical_equals } from "./numerical.js";
import seedrandom from "seedrandom";

//import { substitute_abs } from '../normalization/standard_form';

function randomComplexBindings(rng, variables, radius, centers) {
  var result = {};

  if (centers === undefined) {
    variables.forEach(function (v) {
      result[v] = math.complex(
        rng() * 2 * radius - radius,
        rng() * 2 * radius - radius,
      );
    });
  } else {
    variables.forEach(function (v) {
      result[v] = math.complex(
        centers[v].re + rng() * 2 * radius - radius,
        centers[v].im + rng() * 2 * radius - radius,
      );
    });
  }

  return result;
}

export const equals = function (
  expr,
  other,
  {
    relative_tolerance = 1e-12,
    absolute_tolerance = 0,
    tolerance_for_zero = 1e-15,
    allowed_error_in_numbers = 0,
    include_error_in_number_exponents = false,
    allowed_error_is_absolute = false,
    allow_blanks = false,
    coerce_tuples_arrays = true,
    coerce_vectors = true,
  } = {},
) {
  if (
    !allow_blanks &&
    (expr.variables().includes("\uFF3F") ||
      other.variables().includes("\uFF3F"))
  ) {
    return false;
  }

  let rng = seedrandom("complex_seed");

  expr = expr.remove_scaling_units();
  other = other.remove_scaling_units();

  //expr = expr.substitute_abs();
  //other = other.substitute_abs();

  return numerical_equals({
    expr: expr,
    other: other,
    randomBindings: randomComplexBindings,
    expr_context: expr.context,
    other_context: other.context,
    relative_tolerance,
    absolute_tolerance,
    tolerance_for_zero,
    allowed_error_in_numbers,
    include_error_in_number_exponents,
    allowed_error_is_absolute,
    coerce_tuples_arrays,
    coerce_vectors,
    rng,
  });
};
