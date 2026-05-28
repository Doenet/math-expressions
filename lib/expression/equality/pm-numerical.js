// Numeric equality for expressions containing the `pm` (plus-minus) operator.
//
// `a \pm b` denotes the set `{a+b, a-b}`. Two expressions with `pm` are
// numerically equal when, at every random binding of free variables, the
// collections of values produced by the sign-expansions of each side coincide
// (within tolerance). `pm_equals_numerical` expands all `pm` operators into
// their 2^n sign assignments, then samples bindings and compares the resulting
// value collections via `value_multisets_match`.
//
// The generic sampling helpers (`prepare_exprs_numerical_eval`,
// `replace_numbers_with_parameters`, `generate_random_integer`, `random_cubic`)
// are shared with the non-pm path and imported from `./numerical.js`.

import math from "../../mathjs.js";
import { expand_pm_signs } from "../pm.js";
import {
  prepare_exprs_numerical_eval,
  replace_numbers_with_parameters,
  generate_random_integer,
  random_cubic,
} from "./numerical.js";

export function pm_equals_numerical({
  expr,
  other,
  randomBindings,
  expr_context,
  other_context,
  relative_tolerance,
  absolute_tolerance,
  tolerance_for_zero,
  allowed_error_in_numbers = 0,
  include_error_in_number_exponents = false,
  allowed_error_is_absolute = false,
  rng,
}) {
  // Normalize and disambiguate function/variable names on the originals
  // first; the resulting trees are then sign-expanded. None of the steps
  // in `prepare_exprs_numerical_eval` touch `pm` nodes, so expansion after
  // preparation is equivalent to expansion before, and this way every
  // variant inherits the same function-rename treatment.
  let variables, integer_variables, functions;
  let prepared_expr, prepared_other;
  try {
    ({
      variables,
      integer_variables,
      functions,
      expr: prepared_expr,
      other: prepared_other,
    } = prepare_exprs_numerical_eval(expr, other, expr_context, other_context));
  } catch (e) {
    return false;
  }

  let expr_variant_trees, other_variant_trees;
  try {
    expr_variant_trees = expand_pm_signs(prepared_expr.tree);
    other_variant_trees = expand_pm_signs(prepared_other.tree);
  } catch (e) {
    // too many pm operators to enumerate; cannot decide
    return false;
  }

  let expr_fs, other_fs;
  try {
    expr_fs = expr_variant_trees.map((t) => expr_context.fromAst(t).f());
    other_fs = other_variant_trees.map((t) => other_context.fromAst(t).f());
  } catch (e) {
    return false;
  }

  // Build a per-variant tolerance function from the numbers appearing in the
  // expression (mirroring `component_equals`). Different sign-variants have
  // different sensitivities to literal perturbations (e.g. for `5 ± 3` with
  // `allowed_error_in_numbers=0.01`, the `+` variant's derivative-based
  // tolerance is 0.08, while the `−` variant's is 0.02), so using a single
  // tolerance from variant 0 is too loose for other variants.
  //
  // To keep parameter names consistent across variants of the same side, we
  // first replace numeric literals with parameters on the prepared tree, then
  // expand pm on the parameterized form. Each variant then has the same
  // `parameters_for_numbers` map but its own derivative structure.
  let parameters_for_numbers;
  let expr_variant_tolerance_functions; // array, parallel to expr_variant_trees
  if (allowed_error_in_numbers > 0 && expr_variant_trees.length > 0) {
    try {
      let result = replace_numbers_with_parameters({
        expr: prepared_expr,
        variables: variables,
        include_exponents: include_error_in_number_exponents,
      });
      parameters_for_numbers = result.parameters;

      let parameter_list = Object.keys(parameters_for_numbers);
      if (parameter_list.length > 0) {
        let parametrized_variant_trees;
        try {
          parametrized_variant_trees = expand_pm_signs(result.expr_with_params);
        } catch (e) {
          // already validated above on prepared_expr.tree; shouldn't happen,
          // but if it does, skip per-variant tolerance.
          parametrized_variant_trees = null;
        }

        if (
          parametrized_variant_trees &&
          parametrized_variant_trees.length === expr_variant_trees.length
        ) {
          expr_variant_tolerance_functions = parametrized_variant_trees.map(
            (variant_tree) => {
              try {
                let expr_with_params = expr_context.fromAst(variant_tree);
                let derivative_sum = expr_with_params.derivative(
                  parameter_list[0],
                );
                if (!allowed_error_is_absolute) {
                  derivative_sum = derivative_sum.multiply(
                    parameters_for_numbers[parameter_list[0]],
                  );
                }
                for (let par of parameter_list.slice(1)) {
                  let term = expr_with_params.derivative(par);
                  if (!allowed_error_is_absolute) {
                    term = term.multiply(parameters_for_numbers[par]);
                  }
                  derivative_sum = derivative_sum.add(term);
                }

                let tolerance_expression = derivative_sum.multiply(
                  allowed_error_in_numbers,
                );
                return tolerance_expression.f();
              } catch (e) {
                // can't build tolerance function for this variant; mirror
                // `component_equals` and fall back to no numeric-error
                // tolerance for it.
                return undefined;
              }
            },
          );
        }
      }
    } catch (e) {
      // unable to build tolerance functions; fall back to plain tolerances
    }
  }

  const minimum_matches = 10;
  const number_tries = 100;
  const max_value = Number.MAX_VALUE * 1e-20;
  const binding_scales = [10, 1, 100, 0.1, 1000, 0.01];

  let matches = 0;

  for (let i = 0; i < 10 * number_tries; i++) {
    const scale = binding_scales[Math.floor(i / 20) % binding_scales.length];

    let bindings;
    try {
      bindings = randomBindings(rng, variables, scale);
    } catch (e) {
      continue;
    }

    for (let k = 0; k < integer_variables.length; k++) {
      bindings[integer_variables[k]] = generate_random_integer(-10, 10, rng);
    }
    for (let k = 0; k < functions.length; k++) {
      bindings[functions[k]] = random_cubic(rng);
    }

    // Compute a per-variant numeric-error tolerance for the expr side. The
    // `other` side gets no numeric-error tolerance (mirroring how
    // `component_equals` treats only the LHS literals as contributing to the
    // numeric-error term).
    let expr_variant_tolerances;
    if (expr_variant_tolerance_functions) {
      let bindingsIncludingParameters = Object.assign(
        {},
        bindings,
        parameters_for_numbers,
      );
      expr_variant_tolerances = new Array(
        expr_variant_tolerance_functions.length,
      ).fill(0);
      let any_failed_evaluation = false;
      for (let v = 0; v < expr_variant_tolerance_functions.length; v++) {
        const tf = expr_variant_tolerance_functions[v];
        if (!tf) continue; // fall back to 0 for this variant
        let tol;
        try {
          tol = math.abs(tf(bindingsIncludingParameters));
        } catch (e) {
          any_failed_evaluation = true;
          break;
        }
        if (!Number.isFinite(tol)) {
          any_failed_evaluation = true;
          break;
        }
        expr_variant_tolerances[v] = tol;
      }
      if (any_failed_evaluation) {
        continue;
      }
    }

    let expr_values, other_values;
    try {
      expr_values = expr_fs.map((f) => f(bindings));
      other_values = other_fs.map((f) => f(bindings));
    } catch (e) {
      continue;
    }

    // Reject bindings where any value is non-finite or too large.
    if (
      !expr_values.every(
        (v) =>
          typeof v !== "boolean" &&
          Number.isFinite(math.abs(v)) &&
          math.abs(v) < max_value,
      ) ||
      !other_values.every(
        (v) =>
          typeof v !== "boolean" &&
          Number.isFinite(math.abs(v)) &&
          math.abs(v) < max_value,
      )
    ) {
      continue;
    }

    if (
      !value_multisets_match(
        expr_values,
        other_values,
        relative_tolerance,
        absolute_tolerance,
        tolerance_for_zero,
        expr_variant_tolerances,
      )
    ) {
      return false;
    }

    matches += 1;
    if (matches >= minimum_matches) return true;
  }

  return false;
}

function value_multisets_match(
  values_a,
  values_b,
  relative_tolerance,
  absolute_tolerance,
  tolerance_for_zero,
  number_tolerances_a,
) {
  // `number_tolerances_a` is an optional array of per-element numeric-error
  // tolerances aligned with `values_a` (the expr/LHS side). Elements of
  // `values_b` (the other/RHS side) contribute no numeric-error tolerance,
  // mirroring `component_equals` (which derives its tolerance only from the
  // expr side's literals).
  const tol_for = (i) =>
    number_tolerances_a && number_tolerances_a[i] !== undefined
      ? number_tolerances_a[i]
      : 0;
  if (values_a.length !== values_b.length) {
    // Different cardinality happens when the two sides have different pm
    // counts (e.g. `5 ± 0` produces 2 variants vs `5` producing 1). Fall back
    // to set-equality (not multiset) so that duplicate values caused by a
    // vacuous ± choice don't break equality.
    return (
      values_a.every((va, i) =>
        values_b.some((vb) =>
          values_close(
            va,
            vb,
            relative_tolerance,
            absolute_tolerance,
            tolerance_for_zero,
            tol_for(i),
          ),
        ),
      ) &&
      values_b.every((vb) =>
        values_a.some((va, i) =>
          values_close(
            va,
            vb,
            relative_tolerance,
            absolute_tolerance,
            tolerance_for_zero,
            tol_for(i),
          ),
        ),
      )
    );
  }
  const used_b = new Array(values_b.length).fill(false);
  for (let i = 0; i < values_a.length; i++) {
    const va = values_a[i];
    let matched = false;
    for (let j = 0; j < values_b.length; j++) {
      if (used_b[j]) continue;
      if (
        values_close(
          va,
          values_b[j],
          relative_tolerance,
          absolute_tolerance,
          tolerance_for_zero,
          tol_for(i),
        )
      ) {
        used_b[j] = true;
        matched = true;
        break;
      }
    }
    if (!matched) return false;
  }
  return true;
}

function values_close(
  a,
  b,
  relative_tolerance,
  absolute_tolerance,
  tolerance_for_zero,
  number_tolerance = 0,
) {
  const a_abs = math.abs(a);
  const b_abs = math.abs(b);
  const min_mag = Math.min(a_abs, b_abs);
  const max_mag = Math.max(a_abs, b_abs);
  // Mirror `component_equals`: start with the numeric-error tolerance, add
  // relative tolerance, cap at 10% of min_mag, then add zero/absolute
  // tolerance depending on whether either value is exactly zero.
  let tol = number_tolerance;
  tol += min_mag * relative_tolerance;
  tol = Math.min(tol, 0.1 * min_mag);
  if (tol === 0 && (a === 0 || b === 0)) {
    tol += tolerance_for_zero;
  } else {
    tol += absolute_tolerance;
  }
  return max_mag === 0 || math.abs(math.subtract(a, b)) < tol;
}
