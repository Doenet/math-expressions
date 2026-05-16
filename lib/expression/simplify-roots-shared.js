/**
 * Helpers shared by the radical-simplification routines: `apply_root` and
 * `pull_number_outside_root`. They live in their own module so that both
 * `simplify_integers_in_roots` (in simplify.js) and `simplify_powers_in_roots`
 * (in simplify-powers-in-roots.js) can use them without a circular import.
 */
import math from "../mathjs.js";

/** Build the canonical AST for square, cube, or n-th roots. */
function apply_root(rootOperand, root) {
  if (root === 2) {
    return ["apply", "sqrt", rootOperand];
  } else if (root === 3) {
    return ["apply", "cbrt", rootOperand];
  } else {
    return ["apply", "nthroot", ["tuple", rootOperand, root]];
  }
}

/**
 * Factor n as n_outside^root * n_inside, where n_inside is not an integer
 * raised to the power of root. Returns an object { n_outside, n_inside }.
 */
function pull_number_outside_root(n, root = 2) {
  // From https://stackoverflow.com/a/10492893
  let n_inside = n;
  let n_outside = 1;
  let d = 2;
  let dPower = math.pow(d, root);
  while (dPower <= Math.abs(n_inside)) {
    if (n_inside % dPower === 0) {
      n_inside /= dPower;
      n_outside *= d;
    } else {
      d++;
      dPower = math.pow(d, root);
    }
  }
  return { n_outside, n_inside };
}

export { apply_root, pull_number_outside_root };
