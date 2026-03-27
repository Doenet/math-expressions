/**
 * Remove one leading negative factor from a node when possible.
 *
 * Supported shapes:
 * - negative numbers, e.g. -2 -> 2
 * - unary minus, e.g. ["-", "a"] -> "a"
 * - product with negative first factor,
 *   e.g. ["*", -2, "x"] -> ["*", 2, "x"]
 *
 * @param {*} node
 * @returns {[*, boolean]} Tuple of [updatedNode, changed]
 */
function remove_leading_negative_factor(node) {
  if (typeof node === "number" && node < 0) {
    return [-node, true];
  }

  if (!Array.isArray(node)) {
    return [node, false];
  }

  if (node[0] === "-" && node.length === 2) {
    return [node[1], true];
  }

  if (node[0] === "*" && node.length >= 2) {
    let first_factor = node[1];

    if (typeof first_factor === "number" && first_factor < 0) {
      return [["*", -first_factor, ...node.slice(2)], true];
    }

    if (
      Array.isArray(first_factor) &&
      first_factor[0] === "-" &&
      first_factor.length === 2
    ) {
      return [["*", first_factor[1], ...node.slice(2)], true];
    }
  }

  return [node, false];
}

/**
 * Recursively normalize display-form negative fractions in an AST.
 *
 * Converts fractions whose numerators begin with a negative factor into a
 * unary-minus-wrapped fraction. The transformation is skipped when the
 * fraction is the direct operand of unary minus.
 *
 * @param {*} tree
 * @param {{inside_unary_minus?: boolean}} [options]
 * @returns {*} Normalized AST node
 */
function normalize_display_negative_fractions_sub(
  tree,
  { inside_unary_minus = false } = {},
) {
  if (!Array.isArray(tree)) {
    return tree;
  }

  let operator = tree[0];
  let operands = tree.slice(1);

  if (operator === "-" && operands.length === 1) {
    return [
      "-",
      normalize_display_negative_fractions_sub(operands[0], {
        inside_unary_minus: true,
      }),
    ];
  }

  if (operator === "+") {
    return [
      "+",
      ...operands.map((v) =>
        normalize_display_negative_fractions_sub(v, {
          inside_unary_minus: false,
        }),
      ),
    ];
  }

  if (operator === "*") {
    if (operands.length === 0) {
      return tree;
    }

    let new_operands = [
      normalize_display_negative_fractions_sub(operands[0], {
        inside_unary_minus: false,
      }),
    ];

    for (let i = 1; i < operands.length; i += 1) {
      new_operands.push(
        normalize_display_negative_fractions_sub(operands[i], {
          inside_unary_minus: false,
        }),
      );
    }

    return ["*", ...new_operands];
  }

  if (operator === "/" && operands.length === 2) {
    let numerator = operands[0];
    let denominator = operands[1];

    if (!inside_unary_minus) {
      let [positive_numerator, changed] =
        remove_leading_negative_factor(numerator);

      if (changed) {
        return [
          "-",
          [
            "/",
            normalize_display_negative_fractions_sub(positive_numerator, {
              inside_unary_minus: false,
            }),
            normalize_display_negative_fractions_sub(denominator, {
              inside_unary_minus: false,
            }),
          ],
        ];
      }
    }

    return [
      "/",
      normalize_display_negative_fractions_sub(numerator, {
        inside_unary_minus: false,
      }),
      normalize_display_negative_fractions_sub(denominator, {
        inside_unary_minus: false,
      }),
    ];
  }

  return [
    operator,
    ...operands.map((v) =>
      normalize_display_negative_fractions_sub(v, {
        inside_unary_minus: false,
      }),
    ),
  ];
}

/**
 * Normalize negative fractions for display output without mutating semantics.
 *
 * This is intended for converter output formatting (text/latex), not canonical
 * algebraic normalization.
 *
 * @param {*} tree
 * @returns {*} Normalized AST
 */
function normalize_display_negative_fractions(tree) {
  return normalize_display_negative_fractions_sub(tree, {
    inside_unary_minus: false,
  });
}

export { normalize_display_negative_fractions };
