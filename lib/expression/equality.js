import { equals as equalsViaComplex } from "./equality/complex.js";
import { equals as equalsViaReal } from "./equality/real.js";
import { equals as equalsViaSyntax } from "./equality/syntax.js";
import { equals as equalsViaFiniteField } from "./equality/finite-field.js";
import { equals as equalsDiscreteInfinite } from "./equality/discrete_infinite_set.js";

const equals = function (
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

  // first check with symbolic equality
  // converting all numbers and numerical quantities to floating point
  // and normalizing form of each expression
  let exprNormalized = expr
    .evaluate_numbers({ max_digits: Infinity })
    .normalize_function_names()
    .normalize_applied_functions()
    .normalize_negative_numbers()
    .normalize_angle_linesegment_arg_order()
    .remove_scaling_units()
    .simplify();
  let otherNormalized = other
    .evaluate_numbers({ max_digits: Infinity })
    .normalize_function_names()
    .normalize_applied_functions()
    .normalize_negative_numbers()
    .normalize_angle_linesegment_arg_order()
    .remove_scaling_units()
    .simplify();

  if (
    exprNormalized.equalsViaSyntax(otherNormalized, {
      allowed_error_in_numbers,
      include_error_in_number_exponents,
      allowed_error_is_absolute,
      allow_blanks: true, // since already took care of blanks, save time by not checking again
      coerce_tuples_arrays,
      coerce_vectors,
    })
  ) {
    return true;
  } else if (
    allowed_error_in_numbers === 0 &&
    expr.equalsViaFiniteField(other, {
      allow_blanks: true,
      coerce_tuples_arrays,
      coerce_vectors,
    }) == false
  ) {
    return false;
  } else if (
    expr.equalsViaComplex(other, {
      relative_tolerance,
      absolute_tolerance,
      tolerance_for_zero,
      allowed_error_in_numbers,
      include_error_in_number_exponents,
      allowed_error_is_absolute,
      allow_blanks: true, // since already took care of blanks, save time by not checking again
      coerce_tuples_arrays,
      coerce_vectors,
    })
  ) {
    return true;
    // } else if (expr.equalsViaReal(other)) {
    //   	return true;
  } else if (equalsDiscreteInfinite(expr, other)) {
    return true;
  } else {
    return false;
  }
};

export {
  equals,
  equalsViaComplex,
  equalsViaReal,
  equalsViaSyntax,
  equalsViaFiniteField,
  equalsDiscreteInfinite,
};
