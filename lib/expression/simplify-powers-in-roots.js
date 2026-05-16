/**
 * Simplification of powers inside radicals (sqrt/cbrt/nthroot).
 *
 * `simplify_powers_in_roots` and its helpers pull perfect-power factors out
 * of roots in a single deterministic pass. They are kept separate from the
 * general-purpose simplification machinery in simplify.js because they exist
 * solely to handle powers of roots.
 */
import { is_positive_ast as is_positive } from "../assumptions/element_of_sets.js";
import { is_real_ast as is_real } from "../assumptions/element_of_sets.js";
import { is_nonzero_ast as is_nonzero } from "../assumptions/element_of_sets.js";
import {
  apply_root,
  pull_number_outside_root,
} from "./simplify-roots-shared.js";

/** Build base^exp, collapsing the trivial exponents 0 and 1. */
function make_power(base, exp) {
  if (exp === 0) return 1;
  if (exp === 1) return base;
  return ["^", base, exp];
}

/**
 * Flattens the multiplicative factors of `tree` into `factors`, descending
 * through nested products. A leading unary minus flips the returned sign,
 * and negative numeric factors are split into a sign and a positive
 * magnitude. Used to normalize a radicand before pulling out powers.
 */
function collect_radicand_factors(tree, factors) {
  let sign = 1;
  const recur = (node) => {
    if (Array.isArray(node) && node[0] === "*") {
      for (const op of node.slice(1)) recur(op);
    } else if (Array.isArray(node) && node[0] === "-") {
      sign = -sign;
      recur(node[1]);
    } else if (typeof node === "number") {
      if (node < 0) {
        sign = -sign;
        factors.push(-node);
      } else {
        factors.push(node);
      }
    } else {
      factors.push(node);
    }
  };
  recur(tree);
  return sign;
}

/**
 * Splits a flat list of multiplicative factors into a single number factor,
 * the summed integer exponents of each variable (keyed by variable name, in
 * first-seen order), and any remaining factors that cannot be merged.
 *
 * Combining positive powers (e.g. a * a -> a^2) is always sound and is
 * always done. Combining a negative or zero power into a positive one
 * (e.g. a / a, a * a^(-1)) cancels terms that are only equal when the base
 * is nonzero, so it is done only for bases that are provably nonzero. Other
 * factors are left in `others`, which keeps this pass idempotent with the
 * assumption-aware cancellation performed by the surrounding transformation
 * loop.
 */
function combine_factors(rawFactors, assumptions) {
  let numberFactor = 1;
  const expByVar = new Map();
  const nonzeroByVar = new Map();
  const varOrder = [];
  const others = [];
  for (const f of rawFactors) {
    if (typeof f === "number") {
      numberFactor *= f;
      continue;
    }
    let base, exp;
    if (typeof f === "string") {
      base = f;
      exp = 1;
    } else if (
      Array.isArray(f) &&
      f[0] === "^" &&
      typeof f[1] === "string" &&
      Number.isInteger(f[2])
    ) {
      base = f[1];
      exp = f[2];
    } else {
      others.push(f);
      continue;
    }
    if (!nonzeroByVar.has(base)) {
      nonzeroByVar.set(base, is_nonzero(base, assumptions) === true);
    }
    // a negative or zero exponent may only be merged into a positive one
    // when the base is known nonzero; otherwise the cancellation is unsound
    if (exp < 1 && !nonzeroByVar.get(base)) {
      others.push(f);
      continue;
    }
    if (expByVar.has(base)) {
      expByVar.set(base, expByVar.get(base) + exp);
    } else {
      varOrder.push(base);
      expByVar.set(base, exp);
    }
  }
  return { numberFactor, expByVar, varOrder, others };
}

/**
 * Splits base^exp across a radical of index `root`: base^floor(exp/root) is
 * appended to `outside` and base^(exp mod root) is appended to `inside`,
 * provided the sign rules permit pulling the factor out. Otherwise the whole
 * factor is left inside.
 *
 * The sign rules mirror the (removed) sqrt/cbrt/nthroot power transformations:
 *   - base provably positive: pull out base^q
 *   - root odd and base provably real: pull out base^q
 *   - root even and base provably real: pull out (abs base)^q
 *     (the absolute value is unnecessary when q is even)
 *   - otherwise: leave the factor inside the root
 */
function pull_factor_into_root(base, exp, root, assumptions, outside, inside) {
  const q = exp >= 1 ? Math.floor(exp / root) : 0;
  if (q === 0) {
    // nothing can be pulled out of this factor
    inside.push(make_power(base, exp));
    return;
  }

  let outsideBase;
  if (is_positive(base, assumptions, false)) {
    outsideBase = base;
  } else if (root % 2 === 1 && is_real(base, assumptions)) {
    outsideBase = base;
  } else if (root % 2 === 0 && is_real(base, assumptions)) {
    outsideBase = q % 2 === 0 ? base : ["apply", "abs", base];
  } else {
    // sign unknown or not provably real: cannot pull this factor out
    inside.push(make_power(base, exp));
    return;
  }

  outside.push(make_power(outsideBase, q));
  inside.push(make_power(base, exp % root));
}

/**
 * Recursively pulls perfect-power factors out of radicals.
 *
 * For a radical of integer index m, a factor base^n contributes
 * base^floor(n/m) outside the root and base^(n mod m) inside it. Every
 * factor of the radicand is handled in a single deterministic pass, which
 * avoids the premature termination that a one-factor-at-a-time pattern
 * transformation can suffer.
 *
 * The pass also combines repeated variable factors (so that, e.g., factors
 * exposed by simplifying a nested root, or pulled out beside an existing
 * factor, are recombined) without re-running the transformation loop.
 */
function simplify_powers_in_roots(tree, assumptions) {
  if (!Array.isArray(tree)) {
    return tree;
  }

  var operator = tree[0];
  var operands = tree.slice(1);

  if (operator === "*") {
    // simplify_powers_in_roots can turn roots into multiplications, so we
    // first simplify all the operands and flatten any resulting products,
    // then recombine repeated variable and number factors.
    const rawFactors = [];
    for (const operand of operands) {
      const simplified = simplify_powers_in_roots(operand, assumptions);
      if (Array.isArray(simplified) && simplified[0] === "*") {
        rawFactors.push(...simplified.slice(1));
      } else {
        rawFactors.push(simplified);
      }
    }

    const { numberFactor, expByVar, varOrder, others } = combine_factors(
      rawFactors,
      assumptions,
    );

    const result = [];
    if (numberFactor !== 1) result.push(numberFactor);
    for (const v of varOrder) {
      const p = make_power(v, expByVar.get(v));
      if (p !== 1) result.push(p);
    }
    result.push(...others);

    if (result.length === 0) return 1;
    if (result.length === 1) return result[0];
    return ["*", ...result];
  }

  if (
    operator === "apply" &&
    ["sqrt", "cbrt", "nthroot"].includes(operands[0])
  ) {
    const rootOperator = operands[0];
    let root = 2;
    let rootOperand = operands[1];
    if (rootOperator === "cbrt") {
      root = 3;
    } else if (rootOperator === "nthroot") {
      if (Array.isArray(operands[1]) && operands[1][0] === "tuple") {
        if (Number.isInteger(operands[1][2]) && operands[1][2] > 0) {
          root = operands[1][2];
          rootOperand = operands[1][1];
        } else {
          // we have an nth root with a non-integer or non-positive root, so quit
          return [
            "apply",
            "nthroot",
            ...operands
              .slice(1)
              .map((o) => simplify_powers_in_roots(o, assumptions)),
          ];
        }
      } else {
        root = 2;
      }
    }

    rootOperand = simplify_powers_in_roots(rootOperand, assumptions);

    // normalize the radicand into a sign and a flat list of factors,
    // combining repeated variables before pulling out powers
    const rawFactors = [];
    const sign = collect_radicand_factors(rootOperand, rawFactors);
    const { numberFactor, expByVar, varOrder, others } = combine_factors(
      rawFactors,
      assumptions,
    );

    // a zero number factor makes the whole radicand (and hence the root) zero
    if (numberFactor === 0) {
      return 0;
    }

    const outsideFactors = [];
    const insideFactors = [];

    // pull integer perfect powers out of the (positive) number factor
    if (Number.isInteger(numberFactor)) {
      const { n_outside, n_inside } = pull_number_outside_root(
        numberFactor,
        root,
      );
      if (n_outside !== 1) outsideFactors.push(n_outside);
      if (n_inside !== 1) insideFactors.push(n_inside);
    } else if (numberFactor !== 1) {
      insideFactors.push(numberFactor);
    }

    // pull perfect powers out of each variable factor
    for (const v of varOrder) {
      pull_factor_into_root(
        v,
        expByVar.get(v),
        root,
        assumptions,
        outsideFactors,
        insideFactors,
      );
    }

    // a remaining base^n factor (with non-variable base) can still
    // contribute a perfect power; anything else stays inside untouched
    for (const f of others) {
      if (Array.isArray(f) && f[0] === "^" && Number.isInteger(f[2])) {
        pull_factor_into_root(
          f[1],
          f[2],
          root,
          assumptions,
          outsideFactors,
          insideFactors,
        );
      } else {
        insideFactors.push(f);
      }
    }

    // reassemble, restoring the sign of the radicand
    const insideClean = insideFactors.filter((f) => f !== 1);
    let insideTree;
    if (insideClean.length === 0) {
      insideTree = sign < 0 ? -1 : 1;
    } else {
      const insideProduct =
        insideClean.length === 1 ? insideClean[0] : ["*", ...insideClean];
      insideTree = sign < 0 ? ["-", insideProduct] : insideProduct;
    }

    const resultFactors = outsideFactors.filter((f) => f !== 1);
    if (insideTree !== 1) {
      resultFactors.push(apply_root(insideTree, root));
    }

    if (resultFactors.length === 0) return 1;
    if (resultFactors.length === 1) return resultFactors[0];
    return ["*", ...resultFactors];
  }

  return [
    operator,
    ...operands.map((o) => simplify_powers_in_roots(o, assumptions)),
  ];
}

export { simplify_powers_in_roots };
