import { equals as numerical_equals } from "./numerical.js";

function randomRealBindings(rng, variables, radius, centers) {
  var result = {};

  if (centers === undefined) {
    variables.forEach(function (v) {
      result[v] = rng() * 2 * radius - radius;
    });
  } else {
    variables.forEach(function (v) {
      result[v] = centers[v] + rng() * 2 * radius - radius;
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
  } = {},
) {
  // don't use real equality if not analytic expression
  if (!expr.isAnalytic() || !other.isAnalytic()) return false;

  let rng = seedrandom("real_seed");

  return numerical_equals({
    expr: expr,
    other: other,
    randomBindings: randomRealBindings,
    expr_context: expr.context,
    other_content: other.context,
    relative_tolerance,
    absolute_tolerance,
    tolerance_for_zero,
    allowed_error_in_numbers,
    include_error_in_number_exponents,
    allowed_error_is_absolute,
    rng,
  });
};
