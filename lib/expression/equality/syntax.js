import { equal as tree_equal } from "../../trees/basic.js";

export const equals = function (
  expr,
  other,
  {
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

  let exprNormalized = expr
    .normalize_function_names()
    .normalize_applied_functions()
    .normalize_negative_numbers()
    .normalize_angle_linesegment_arg_order();
  let otherNormalized = other
    .normalize_function_names()
    .normalize_applied_functions()
    .normalize_negative_numbers()
    .normalize_angle_linesegment_arg_order();

  return tree_equal(exprNormalized.tree, otherNormalized.tree, {
    allowed_error_in_numbers,
    include_error_in_number_exponents,
    allowed_error_is_absolute,
    coerce_tuples_arrays,
    coerce_vectors,
  });
};
