import { equal as tree_equal } from '../../trees/basic';

export const equals = function (expr, other, {
  allowed_error_in_numbers = 0,
  include_error_in_number_exponents = false,
  allowed_error_is_absolute = false,
  allow_blanks = false,
} = {}) {

  if (!allow_blanks && (expr.variables().includes('\uFF3F') || other.variables().includes('\uFF3F'))) {
    return false;
  }

  let exprNormalized = expr
    .normalize_function_names()
    .normalize_applied_functions()
    .normalize_angle_arg_order();
  let otherNormalized = other
    .normalize_function_names()
    .normalize_applied_functions()
    .normalize_angle_arg_order();

  return tree_equal(exprNormalized.tree, otherNormalized.tree, {
    allowed_error_in_numbers: allowed_error_in_numbers,
    include_error_in_number_exponents: include_error_in_number_exponents,
    allowed_error_is_absolute: allowed_error_is_absolute,
  });
};
