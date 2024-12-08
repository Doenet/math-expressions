const nonTupleVectorOperators = ["vector", "altvector"];

/**
 * Coerce tuple, vector, or array operators of one expression
 * to match vector or interval operators of the other.
 *
 * If `coerce_tuples_array` is `true`, then
 * - if one operator is a "tuple" and the other operator is a "vector" or "altvector",
 *   then change the "tuple" to match the "vector" or "altvector"
 * - if one expression is a 2D "tuple" and the other expression is an open interval,
 *   then change the "tuple" expression to be an open interval (changes both expression and operator)
 * - if one expression is a 2D "array" and the other expression is a closed interval,
 *   then change the "tuple" expression to be a closed interval (changes both expression and operator)
 *
 * If `coerce_vectors` is `true` then
 * - if one operators is a "vector" and the other operator is an "altvector",
 *   then change both operators to be "vector"
 *
 * @returns the potentially changed operators and operands
 */
export function coerce_tuple_array_vectors({
  operator1,
  operator2,
  operands1,
  operands2,
  coerce_tuples_arrays,
  coerce_vectors,
}) {
  let operators_match = operator1 === operator2;

  if (!operators_match) {
    if (coerce_tuples_arrays) {
      // match tuple to vectors or open intervals
      // match arrays to closed intervals
      if (operator1 === "tuple") {
        if (nonTupleVectorOperators.includes(operator2)) {
          operator1 = operator2;
          operators_match = true;
        } else if (operator2 === "interval" && operands1.length === 2) {
          // check if open interval
          let closedInfo = operands2[1];
          if (!(closedInfo[1] || closedInfo[2])) {
            operators_match = true;
            operator1 = "interval";
            operands1 = [
              ["tuple", ...operands1],
              ["tuple", false, false],
            ];
          }
        }
      } else if (operator2 === "tuple") {
        if (nonTupleVectorOperators.includes(operator1)) {
          operator2 = operator1;
          operators_match = true;
        } else if (operator1 === "interval" && operands2.length === 2) {
          // check if open interval
          let closedInfo = operands1[1];
          if (!(closedInfo[1] || closedInfo[2])) {
            operators_match = true;
            operator2 = "interval";
            operands2 = [
              ["tuple", ...operands2],
              ["tuple", false, false],
            ];
          }
        }
      }
      if (!operators_match) {
        if (operator1 === "array" && operands1.length === 2) {
          if (operator2 === "interval") {
            // check if closed interval
            let closedInfo = operands2[1];
            if (closedInfo[1] && closedInfo[2]) {
              operators_match = true;
              operator1 = "interval";
              operands1 = [
                ["tuple", ...operands1],
                ["tuple", true, true],
              ];
            }
          }
        } else if (operator2 === "array" && operands2.length === 2) {
          if (operator1 === "interval") {
            // check if closed interval
            let closedInfo = operands1[1];
            if (closedInfo[1] && closedInfo[2]) {
              operators_match = true;
              operator2 = "interval";
              operands2 = [
                ["tuple", ...operands2],
                ["tuple", true, true],
              ];
            }
          }
        }
      }
    }

    if (!operators_match && coerce_vectors) {
      // match vectors and altVectors to each other
      if (
        nonTupleVectorOperators.includes(operator1) &&
        nonTupleVectorOperators.includes(operator2)
      ) {
        operators_match = true;
        operator1 = operator2 = "vector";
      }
    }
  }

  return { operator1, operator2, operands1, operands2 };
}
