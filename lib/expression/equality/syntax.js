import { equal as tree_equal } from '../../trees/basic';

export const equals = function (expr, other, {
  allowed_error_in_numbers = 0,
  include_error_in_number_exponents = false
} = {}) {
  return tree_equal(expr.tree, other.tree, {
    allowed_error_in_numbers: allowed_error_in_numbers,
    include_error_in_number_exponents: include_error_in_number_exponents
  });
};
