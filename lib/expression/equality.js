import { equals as equalsViaComplex } from './equality/complex.js';
import { equals as equalsViaReal } from './equality/real.js';
import { equals as equalsViaSyntax } from './equality/syntax.js';

//var equalsViaFiniteField = require('./equality/finite-field.js').equals;
import { equals as equalsDiscreteInfinite } from './equality/discrete_infinite_set';


//exports.equalsViaFiniteField = equalsViaFiniteField;

const equals = function (expr, other, {
  tolerance = 1E-12, allowed_error_in_numbers = 0,
  include_error_in_number_exponents = false,
  allowed_error_is_absolute = false,
} = {}) {
  if (expr.variables().includes('\uFF3F') || other.variables().includes('\uFF3F')) {
    return false;
  }

  // first check with symbolic equality
  // converting all numbers and numerical quantities to floating point
  // and normalizing form of each expression
  let exprNormalized = expr.evaluate_numbers({ max_digits: Infinity })
    .normalize_function_names()
    .normalize_applied_functions()
    .simplify();
  let otherNormalized = other.evaluate_numbers({ max_digits: Infinity })
    .normalize_function_names()
    .normalize_applied_functions()
    .simplify();

  if (exprNormalized.equalsViaSyntax(otherNormalized, {
    allowed_error_in_numbers: allowed_error_in_numbers,
    include_error_in_number_exponents: include_error_in_number_exponents,
    allowed_error_is_absolute: allowed_error_is_absolute,
  })
  ) {
    return true;
  } else if (expr.equalsViaComplex(other, {
    tolerance: tolerance,
    allowed_error_in_numbers: allowed_error_in_numbers,
    include_error_in_number_exponents: include_error_in_number_exponents,
    allowed_error_is_absolute: allowed_error_is_absolute,
  })) {
    return true;
    // } else if (expr.equalsViaReal(other)) {
    //   	return true;
  } else if (equalsDiscreteInfinite(expr, other)) {
    return true;
  } else {
    return false;
  }
};

export { equals, equalsViaComplex, equalsViaReal, equalsViaSyntax, equalsDiscreteInfinite };
