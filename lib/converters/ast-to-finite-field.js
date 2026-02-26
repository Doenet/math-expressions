/*
 * convert AST to a finite field calculations
 *
 * Copyright 2014-2023 by
 * Jim Fowler <kisonecat@gmail.com>
 * Duane Nykamp <nykamp@umn.edu>
 *
 * This file is part of a math-expressions library
 *
 * math-expressions is free software: you can redistribute
 * it and/or modify it under the terms of the GNU General Public
 * License as published by the Free Software Foundation, either
 * version 3 of the License, or at your option any later version.
 *
 * math-expressions is distributed in the hope that it
 * will be useful, but WITHOUT ANY WARRANTY; without even the implied
 * warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details.
 *
 */

import numberTheory from "number-theory";
import ZmodN from "./z-mod-n.js";

var PRIME = 10739999; // a very safe prime

var levelFunctions = {
  "+": function (operands) {
    return operands.reduce(function (x, y) {
      return x.add(y);
    });
  },
  "*": function (operands) {
    return operands.reduce(function (x, y) {
      return x.multiply(y);
    });
  },
  "/": function (operands) {
    return operands.reduce(function (x, y) {
      return x.divide(y);
    });
  },
  "-": function (operands) {
    return operands[0].negate();
  },
  "~": function (operands) {
    return operands.reduce(
      function (x, y) {
        return x.subtract(y);
      },
      new ZmodN([0], operands[0].modulus),
    );
  },

  // These aren't implemented for Gonnet's finiteField
  factorial: function (operands) {
    return ZmodN([NaN], 1);
  },
  gamma: function (operands) {
    return ZmodN([NaN], 1);
  },
};

var deeperFunctions = {
  tan: function (operands) {
    return operands[0].tan();
  },
  arcsin: function (operands) {
    return operands[0].arcsin();
  },
  arccos: function (operands) {
    return operands[0].arccos();
  },
  arctan: function (operands) {
    return operands[0].arctan();
  },
  arccsc: function (operands) {
    return operands[0].reciprocal().arcsin();
  },
  arcsec: function (operands) {
    return operands[0].reciprocal().arccos();
  },
  arccot: function (operands) {
    return operands[0].reciprocal().arctan();
  },

  csc: function (operands) {
    return operands[0].csc();
  },
  sec: function (operands) {
    return operands[0].sec();
  },
  cot: function (operands) {
    return operands[0].cot();
  },

  log: function (operands) {
    return operands[0].log();
  },

  apply: function (operands) {
    return NaN;
  },
};

// Determine whether a is a primitive root of Z mod m
function isPrimitiveRoot(a, m) {
  var b = numberTheory.logMod(numberTheory.primitiveRoot(m), a, m);

  if (isNaN(b)) {
    return false;
  } else {
    return true;
  }
}

export function rationalApproximation(x, maxden = 1000000000000000) {
  // from https://www.ics.uci.edu/~eppstein/numth/frap.c

  /* initialize matrix */
  let m = [
    [1, 0],
    [0, 1],
  ];

  /* loop finding terms until denom gets too big */
  let ai;
  while (m[1][0] * (ai = Math.floor(x)) + m[1][1] <= maxden) {
    let t;
    t = m[0][0] * ai + m[0][1];
    m[0][1] = m[0][0];
    m[0][0] = t;
    t = m[1][0] * ai + m[1][1];
    m[1][1] = m[1][0];
    m[1][0] = t;
    if (x == ai) break; // AF: division by zero
    x = 1 / (x - ai);
    if (x > 0x7fffffff) break; // AF: representation failure
  }

  const approximate = m[1][0] * ai + m[1][1] > maxden;

  return { numerator: m[0][0], denominator: m[1][0], approximate };
}

function finite_field_evaluate_ast(tree, bindings, modulus, level) {
  if (typeof tree === "string") {
    if (tree === "e") {
      return new ZmodN([numberTheory.primitiveRoot(modulus)], modulus);
    }

    if (tree === "pi") {
      if (modulus % 2 == 0) return new ZmodN([modulus / 2], modulus);
      // Probably a really bad idea
      else return new ZmodN([NaN], modulus);
    }

    if (tree === "i") return new ZmodN([0], 1);

    if (tree in bindings) return new ZmodN([bindings[tree]], modulus);

    return tree;
  }

  if (typeof tree === "number") {
    if (Number.isInteger(tree)) {
      return new ZmodN([tree], modulus);
    } else {
      let r = rationalApproximation(tree);
      let numerator = new ZmodN([r.numerator], modulus, r.approximate);
      let denominator = new ZmodN([r.denominator], modulus, r.approximate);
      return numerator.divide(denominator);
    }
  }

  var operator = tree[0];
  var operands = tree.slice(1);

  if (operator === "apply") {
    if (typeof operands[0] !== "string")
      throw Error("Non string functions not implemented for finite fields");

    if (operands[0] === "exp") {
      var base = finite_field_evaluate_ast("e", {}, modulus, level);
      var exponent = finite_field_evaluate_ast(
        operands[1],
        bindings,
        numberTheory.eulerPhi(modulus),
        level + 1,
      );
      return base.power(exponent);
    }

    if (operands[0] === "sec") {
      return finite_field_evaluate_ast(
        ["/", 1, ["apply", "cos", operands[1]]],
        bindings,
        modulus,
        level,
      );
    }

    if (operands[0] === "csc") {
      return finite_field_evaluate_ast(
        ["/", 1, ["apply", "sin", operands[1]]],
        bindings,
        modulus,
        level,
      );
    }

    if (operands[0] === "cot") {
      return finite_field_evaluate_ast(
        ["/", 1, ["apply", "tan", operands[1]]],
        bindings,
        modulus,
        level,
      );
    }

    if (operands[0] === "tan") {
      var phi = numberTheory.eulerPhi(modulus);
      return finite_field_evaluate_ast(
        ["/", ["apply", "sin", operands[1]], ["apply", "cos", operands[1]]],
        bindings,
        modulus,
        level,
      );
    }

    if (operands[0] === "sin") {
      var root = numberTheory.primitiveRoot(modulus);
      var g = new ZmodN([root], modulus);
      var phi = numberTheory.eulerPhi(modulus);
      if (phi % 4 != 0) return new ZmodN([NaN], modulus);

      var i = new ZmodN(
        [numberTheory.powerMod(root, phi / 4, modulus)],
        modulus,
      );
      var x = finite_field_evaluate_ast(operands[1], bindings, phi, level + 1);
      return g.power(x).subtract(g.power(x.negate())).divide(i.add(i));
    }

    if (operands[0] === "cos") {
      var g = new ZmodN([numberTheory.primitiveRoot(modulus)], modulus);
      var x = finite_field_evaluate_ast(
        operands[1],
        bindings,
        numberTheory.eulerPhi(modulus),
        level + 1,
      );
      return g
        .power(x)
        .add(g.power(x.negate()))
        .divide(new ZmodN([2], modulus));
    }

    if (operands[0] === "log") {
      return new ZmodN([NaN], NaN);
    }

    if (operands[0] === "sqrt") {
      return finite_field_evaluate_ast(
        operands[1],
        bindings,
        modulus,
        level,
      ).sqrt();
    }

    if (operands[0] === "abs") {
      let operand1 = finite_field_evaluate_ast(
        operands[1],
        bindings,
        modulus,
        level,
      );
      return operand1.multiply(operand1).sqrt();
    }
  }

  if (operator == "^") {
    var base = finite_field_evaluate_ast(operands[0], bindings, modulus, level);
    var exponent = finite_field_evaluate_ast(
      operands[1],
      bindings,
      numberTheory.eulerPhi(modulus),
      level + 1,
    );
    return base.power(exponent);
  }

  if (operator in levelFunctions) {
    return levelFunctions[operator](
      operands.map(function (v, i) {
        return finite_field_evaluate_ast(v, bindings, modulus, level);
      }),
    );
  }

  return new ZmodN([NaN], modulus);
}

class astToFiniteField {
  constructor() {}

  convert(tree, bindings, modulus = PRIME) {
    return finite_field_evaluate_ast(tree, bindings, modulus, 0);
  }
}

export default astToFiniteField;
