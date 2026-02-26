import math from "../../mathjs.js";
import seedrandom from "seedrandom";
import {
  normalize_for_equality_checking,
  prepare_exprs_numerical_eval,
} from "./numerical.js";
import { coerce_tuple_array_vectors } from "./coersion.js";
import { rationalApproximation } from "../../converters/ast-to-finite-field.js";

function randomFiniteFieldBindings(rng, variables, modulus) {
  var result = {};

  variables.forEach(function (v) {
    result[v] = Math.floor(rng() * modulus);
  });

  return result;
}

export const equals = function (
  expr,
  other,
  {
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

  let rng = seedrandom("finite_field_seed");

  if (Array.isArray(expr.tree) && Array.isArray(other.tree)) {
    let expr_operator = expr.tree[0];
    let expr_operands = expr.tree.slice(1);
    let other_operator = other.tree[0];
    let other_operands = other.tree.slice(1);

    if (
      expr_operator === "tuple" ||
      expr_operator === "vector" ||
      expr_operator === "altvector" ||
      expr_operator === "list" ||
      expr_operator === "array" ||
      expr_operator === "matrix" ||
      expr_operator === "interval"
    ) {
      if (other_operator !== expr_operator) {
        ({
          operator1: expr_operator,
          operator2: other_operator,
          operands1: expr_operands,
          operands2: other_operands,
        } = coerce_tuple_array_vectors({
          operator1: expr_operator,
          operator2: other_operator,
          operands1: expr_operands,
          operands2: other_operands,
          coerce_tuples_arrays,
          coerce_vectors,
        }));
      }

      if (other_operator !== expr_operator) return false;

      if (other_operands.length !== expr_operands.length) return false;

      for (let i = 0; i < expr_operands.length; i++) {
        if (
          equals(
            expr.context.fromAst(expr_operands[i]),
            other.context.fromAst(other_operands[i]),
            rng,
          ) === false
        )
          return false;
      }

      return undefined; // each component could be equal
    }

    // check if a relation with two operands
    if (
      expr_operands.length === 2 &&
      ["=", ">", "<", "ge", "le"].includes(expr_operator)
    ) {
      if (other_operands.length !== 2) {
        return false;
      }
      //normalize operator
      if (expr_operator === ">") {
        expr_operator = "<";
        expr_operands = [expr_operands[1], expr_operands[0]];
      } else if (expr_operator === "ge") {
        expr_operator = "le";
        expr_operands = [expr_operands[1], expr_operands[0]];
      }
      if (other_operator === ">") {
        other_operator = "<";
        other_operands = [other_operands[1], other_operands[0]];
      } else if (other_operator === "ge") {
        other_operator = "le";
        other_operands = [other_operands[1], other_operands[0]];
      }

      if (expr_operator !== other_operator) {
        return false;
      }

      // put in standard form
      let expr_rhs = ["+", expr_operands[0], ["-", expr_operands[1]]];
      let other_rhs = ["+", other_operands[0], ["-", other_operands[1]]];
      let require_positive_proportion = expr_operator !== "=";

      let normalized_expr = expr.context.fromAst(expr_rhs);
      let normalized_other = other.context.fromAst(other_rhs);

      // attempt to find the proportion between the expression
      try {
        let exprs_for_eval = prepare_exprs_numerical_eval(
          normalized_expr,
          normalized_other,
        );

        // haven't implemented case with symbolic functions
        if (exprs_for_eval.functions.length === 0) {
          var expr_f = exprs_for_eval.expr.f();
          var other_f = exprs_for_eval.other.f();

          let max_value = Number.MAX_VALUE * 1e-20;

          for (let i = 0; i < 10; i++) {
            var bindings = {};

            exprs_for_eval.variables.forEach(function (v) {
              bindings[v] = rng() * 2 - 1;
            });

            // replace any integer variables with integer
            for (let i = 0; i < exprs_for_eval.integer_variables.length; i++) {
              bindings[exprs_for_eval.integer_variables[i]] =
                math.floor(rng() * 21) - 10;
            }

            let expr_evaluated = expr_f(bindings);
            let other_evaluated = other_f(bindings);

            var expr_abs = math.abs(expr_evaluated);
            var other_abs = math.abs(other_evaluated);

            if (!(expr_abs < max_value && other_abs < max_value)) {
              continue;
            }
            if (expr_abs == 0 || other_abs === 0) {
              continue;
            }

            let proportion = math.divide(expr_evaluated, other_evaluated);
            if (require_positive_proportion && !(proportion > 0)) {
              return false;
            }

            if (proportion !== 1) {
              const r = rationalApproximation(proportion);

              normalized_other = other.context.fromAst([
                "*",
                ["/", r.numerator, r.denominator],
                other_rhs,
              ]);
            }

            break;
          }
        }
      } catch (e) {
        // if have an error, result is undetermined
        return undefined;
      }

      return component_equals(normalized_expr, normalized_other, rng);
    }
  }

  // if not special case, use standard equality
  return component_equals(expr, other, rng);
};

const component_equals = function (expr, other, rng) {
  ({ expr, other } = normalize_for_equality_checking(expr, other));

  let primes = [1181, 1187, 1193, 1201, 1213, 1217, 1223, 1229, 1231];

  var variables = [...new Set([...expr.variables(), ...other.variables()])];

  for (let prime of primes) {
    let bindings = randomFiniteFieldBindings(rng, variables, prime);
    try {
      let a = expr.finite_field_evaluate(bindings, prime);
      let b = other.finite_field_evaluate(bindings, prime);
      if (
        !a.approximate &&
        !b.approximate &&
        !a.isNaN() &&
        !b.isNaN() &&
        a.values.length > 0 &&
        b.values.length > 0 &&
        !a.equals(b)
      )
        return false;
    } catch (e) {
      return undefined;
    }
  }

  return undefined;
};
