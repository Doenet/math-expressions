/*
 * convert syntax trees back to LaTeX code
 *
 * Copyright 2014-2017 by
 *  Jim Fowler <kisonecat@gmail.com>
 *  Duane Nykamp <nykamp@umn.edu>
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

import { padNumberStringToDigitsAndDecimals } from "./pad-numbers";

const operators = {
  "+": function (operands) {
    return operands.join(" ");
  },
  "-": function (operands) {
    return "-" + operands[0];
  },
  "*": function (operands) {
    return operands.join(" ");
  },
  "/": function (operands) {
    return "\\frac{" + operands[0] + "}{" + operands[1] + "}";
  },
  _: function (operands) {
    return operands[0] + "_{" + operands[1] + "}";
  },
  "^": function (operands) {
    return operands[0] + "^{" + operands[1] + "}";
  },
  prime: function (operands) {
    return operands[0] + "'";
  },
  tuple: function (operands) {
    return "\\left( " + operands.join(", ") + " \\right)";
  },
  array: function (operands) {
    return "\\left[ " + operands.join(", ") + " \\right]";
  },
  list: function (operands) {
    return operands.join(", ");
  },
  set: function (operands) {
    return "\\left\\{ " + operands.join(", ") + " \\right\\}";
  },
  vector: function (operands) {
    return "\\left( " + operands.join(", ") + " \\right)";
  },
  altvector: function (operands) {
    return "\\left\\langle " + operands.join(", ") + " \\right\\rangle";
  },
  interval: function (operands) {
    return "\\left( " + operands.join(", ") + " \\right)";
  },
  matrix: function (operands) {
    return "\\left( " + operands.join(", ") + " \\right)";
  },
  and: function (operands) {
    return operands.join(" \\land ");
  },
  or: function (operands) {
    return operands.join(" \\lor ");
  },
  not: function (operands) {
    return "\\lnot " + operands[0];
  },
  "=": function (operands) {
    return operands.join(" = ");
  },
  "<": function (operands) {
    return operands.join(" < ");
  },
  ">": function (operands) {
    return operands.join(" > ");
  },
  lts: function (operands) {
    return operands.join(" < ");
  },
  gts: function (operands) {
    return operands.join(" > ");
  },
  le: function (operands) {
    return operands.join(" \\le ");
  },
  ge: function (operands) {
    return operands.join(" \\ge ");
  },
  ne: function (operands) {
    return operands.join(" \\ne ");
  },
  forall: function (operands) {
    return "\\forall " + operands[0];
  },
  exists: function (operands) {
    return "\\exists " + operands[0];
  },
  in: function (operands) {
    return operands[0] + " \\in " + operands[1];
  },
  notin: function (operands) {
    return operands[0] + " \\notin " + operands[1];
  },
  ni: function (operands) {
    return operands[0] + " \\ni " + operands[1];
  },
  notni: function (operands) {
    return operands[0] + " \\not\\ni " + operands[1];
  },
  subset: function (operands) {
    return operands[0] + " \\subset " + operands[1];
  },
  subseteq: function (operands) {
    return operands[0] + " \\subseteq " + operands[1];
  },
  notsubset: function (operands) {
    return operands[0] + " \\not\\subset " + operands[1];
  },
  notsubseteq: function (operands) {
    return operands[0] + " \\not\\subseteq " + operands[1];
  },
  superset: function (operands) {
    return operands[0] + " \\supset " + operands[1];
  },
  superseteq: function (operands) {
    return operands[0] + " \\supseteq " + operands[1];
  },
  notsuperset: function (operands) {
    return operands[0] + " \\not\\supset " + operands[1];
  },
  notsuperseteq: function (operands) {
    return operands[0] + " \\not\\supseteq " + operands[1];
  },
  union: function (operands) {
    return operands.join(" \\cup ");
  },
  intersect: function (operands) {
    return operands.join(" \\cap ");
  },
  implies: function (operands) {
    return operands.join(" \\implies ");
  },
  impliedby: function (operands) {
    return operands.join(" \\impliedby ");
  },
  iff: function (operands) {
    return operands.join(" \\iff ");
  },
  leftarrow: function (operands) {
    return operands.join(" \\leftarrow ");
  },
  rightarrow: function (operands) {
    return operands.join(" \\rightarrow ");
  },
  leftrightarrow: function (operands) {
    return operands.join(" \\leftrightarrow ");
  },
  perp: function (operands) {
    return operands.join(" \\perp ");
  },
  parallel: function (operands) {
    return operands.join(" \\parallel ");
  },
  derivative_leibniz: function (operands) {
    return "\\frac{d" + operands[0] + "}{d" + operands[1] + "}";
  },
  partial_derivative_leibniz: function (operands) {
    return "\\frac{d" + operands[0] + "}{d" + operands[1] + "}";
  },
  "|": function (operands) {
    return operands[0] + " \\mid " + operands[1];
  },
  ":": function (operands) {
    return operands[0] + " : " + operands[1];
  },
  binom: function (operands) {
    return "\\binom{" + operands[0] + "}{" + operands[1] + "}";
  },
  vec: function (operands) {
    return "\\vec{" + operands[0] + "}";
  },
  linesegment: function (operands) {
    return "\\overline{" + operands.join(" ") + "}";
  },
  angle: function (operands, use_shorthand) {
    if (use_shorthand) {
      return "\\angle " + operands.join("");
    } else {
      return "\\angle\\left( " + operands.join(", ") + " \\right)";
    }
  },
  unit: function (operands) {
    if (operands[1] === "\\circ") {
      return operands[0] + "^{\\circ}";
    } else {
      return operands[0] + " " + operands[1];
    }
  },
};

// defaults for parsers if not overridden by context

// allowed multicharacter latex symbols
// in addition to the below applied function symbols
const allowedLatexSymbolsDefault = [
  "alpha",
  "beta",
  "gamma",
  "Gamma",
  "delta",
  "Delta",
  "epsilon",
  "zeta",
  "eta",
  "theta",
  "Theta",
  "iota",
  "kappa",
  "lambda",
  "Lambda",
  "mu",
  "nu",
  "xi",
  "Xi",
  "pi",
  "Pi",
  "rho",
  "sigma",
  "Sigma",
  "tau",
  "Tau",
  "upsilon",
  "Upsilon",
  "phi",
  "Phi",
  "chi",
  "psi",
  "Psi",
  "omega",
  "Omega",
  "partial",
  "abs",
  "exp",
  "log",
  "ln",
  "log10",
  "sign",
  "sqrt",
  "erf",
  "cos",
  "cosh",
  "cot",
  "coth",
  "csc",
  "csch",
  "sec",
  "sech",
  "sin",
  "sinh",
  "tan",
  "tanh",
  "arcsin",
  "arccos",
  "arctan",
  "arccsc",
  "arcsec",
  "arccot",
  "arg",
  "Re",
  "Im",
  "det",
  "angle",
  "perp",
  "circ",
  "%",
  "$",
  "int",
  "varnothing",
];

const convertLatexSymbolsDefault = {
  acos: "arccos",
  acosh: "arccosh",
  acot: "arccot",
  acoth: "arccoth",
  acsc: "arccsc",
  acsch: "arccsch",
  asec: "arcsec",
  asech: "arcsech",
  asin: "arcsin",
  asinh: "arcsinh",
  atan: "arctan",
  atanh: "arctanh",
  deg: "circ",
  emptyset: "varnothing",
};

const matrixEnvironmentDefault = "bmatrix";

class astToLatex {
  constructor({
    allowedLatexSymbols = allowedLatexSymbolsDefault,
    convertLatexSymbols = convertLatexSymbolsDefault,
    matrixEnvironment = matrixEnvironmentDefault,
    padToDigits = null,
    padToDecimals = null,
    showBlanks = true,
  } = {}) {
    this.allowedLatexSymbols = allowedLatexSymbols;
    this.convertLatexSymbols = convertLatexSymbols;
    this.matrixEnvironment = matrixEnvironment;
    this.padToDigits = padToDigits;
    this.padToDecimals = padToDecimals;
    this.showBlanks = showBlanks;
  }

  convert(tree) {
    return this.statement(tree);
  }

  statement(tree) {
    if (!Array.isArray(tree)) {
      return this.single_statement(tree);
    }

    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator === "ldots") return "\\ldots";

    if (!(operator in operators) && operator !== "apply")
      throw new Error(
        "Badly formed ast: operator " + operator + " not recognized.",
      );

    if (
      [
        "implies",
        "impliedby",
        "iff",
        "rightarrow",
        "leftarrow",
        "leftrightarrow",
      ].includes(operator)
    ) {
      return operators[operator](operands.map((v) => this.statement(v)));
    }

    if (operator === "and" || operator === "or") {
      return operators[operator](
        operands.map(
          function (v, i) {
            let result = this.single_statement(v);
            // for clarity, add parenthesis unless result is
            // single quantity (with no spaces) or already has parens
            if (result.match(/ /) && !result.match(/^\\left\(.*\\right\)$/))
              return "\\left(" + result + "\\right)";
            else return result;
          }.bind(this),
        ),
      );
    }
    return this.single_statement(tree);
  }

  single_statement(tree) {
    if (!Array.isArray(tree)) {
      return this.expression(tree);
    }

    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator === "not") {
      return operators[operator](
        operands.map(
          function (v, i) {
            let result = this.single_statement(v);
            // for clarity, add parenthesis unless result is
            // single quantity (with no spaces) or already has parens
            if (result.match(/ /) && !result.match(/^\\left\(.*\\right\)$/))
              return "\\left(" + result + "\\right)";
            else return result;
          }.bind(this),
        ),
      );
    }

    if (operator === "exists" || operator === "forall") {
      return operators[operator]([this.single_statement(operands[0])]);
    }

    if (
      operator === "=" ||
      operator === "ne" ||
      operator === "<" ||
      operator === ">" ||
      operator === "le" ||
      operator === "ge" ||
      operator === "in" ||
      operator === "notin" ||
      operator === "ni" ||
      operator === "notni" ||
      operator === "subset" ||
      operator === "notsubset" ||
      operator === "subseteq" ||
      operator === "notsubseteq" ||
      operator === "superset" ||
      operator === "notsuperset" ||
      operator === "superseteq" ||
      operator === "notsuperseteq"
    ) {
      return operators[operator](
        operands.map(
          function (v, i) {
            return this.expression(v);
          }.bind(this),
        ),
      );
    }

    if (operator === "lts" || operator === "gts") {
      let args = operands[0];
      let strict = operands[1];

      if (args[0] !== "tuple" || strict[0] !== "tuple")
        // something wrong if args or strict are not tuples
        throw new Error("Badly formed ast");

      let result = this.expression(args[1]);
      for (let i = 1; i < args.length - 1; i++) {
        if (strict[i]) {
          if (operator === "lts") result += " < ";
          else result += " > ";
        } else {
          if (operator === "lts") {
            result += " \\le ";
          } else {
            result += " \\ge ";
          }
        }
        result += this.expression(args[i + 1]);
      }
      return result;
    }

    return this.expression(tree);
  }

  expression(tree) {
    if (!Array.isArray(tree)) {
      return this.term(tree);
    }

    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator === "+") {
      if (operands.length === 1) {
        return "+" + this.term(operands[0]);
      } else {
        return operators[operator](
          operands.map(
            function (v, i) {
              if (i > 0) return this.termWithPlusIfNotNegated(v);
              else return this.term(v);
            }.bind(this),
          ),
        );
      }
    }

    if (["union", "intersect", "perp", "parallel"].includes(operator)) {
      return operators[operator](
        operands.map(
          function (v, i) {
            return this.term(v);
          }.bind(this),
        ),
      );
    }

    return this.term(tree);
  }

  term(tree) {
    if (!Array.isArray(tree)) {
      return this.factor(tree);
    }

    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator === "-") {
      return operators[operator](
        operands.map(
          function (v, i) {
            return this.term(v);
          }.bind(this),
        ),
      );
    }
    if (operator === "*") {
      return operators[operator](
        operands.map(
          function (v, i) {
            let result;
            if (i > 0) {
              result =
                this.factorWithParenthesesIfNegatedExplicitPlusOrFraction(v);
              if (result.match(/^[0-9]/)) {
                return "\\cdot " + result;
              } else if (v[0] === "angle" && operands.length > 1) {
                return "\\left( " + result + " \\right)";
              } else {
                return result;
              }
            } else {
              result = this.factor(v);
              if (v[0] === "angle" && operands.length > 1) {
                return "\\left( " + result + " \\right)";
              } else {
                return result;
              }
            }
          }.bind(this),
        ),
      );
    }

    if (operator === "/" || operator === "unit") {
      return operators[operator](
        operands.map(
          function (v, i) {
            return this.expression(v);
          }.bind(this),
        ),
      );
    }

    return this.factor(tree);
  }

  simple_factor_or_function_or_parens(tree) {
    // return true if
    // factor(tree) is a single character
    // or tree is a non-negative number not in scientific notation
    // or tree is a string
    // or tree is a function call other than sqrt
    // or factor(tree) is in parens/angles

    var result = this.factor(tree);

    if (
      result.length <= 1 ||
      (typeof tree === "string" && tree.match(/^\w+$/)) ||
      (Array.isArray(tree) && tree[0] === "apply" && tree[1] !== "sqrt") ||
      result.match(/^\\left\(.*\\right\)$/) ||
      result.match(/^\\left\\langle.*\\right\\rangle$/)
    ) {
      return true;
    } else if (typeof tree === "number") {
      if (tree >= 0 && !tree.toString().includes("e")) {
        return true;
      } else {
        return false;
      }
    } else {
      return false;
    }
  }

  stringConvert(string) {
    if (string.length > 1) {
      if (this.convertLatexSymbols[string]) {
        string = this.convertLatexSymbols[string];
      }
      if (this.allowedLatexSymbols.includes(string)) return "\\" + string;
      else return "\\operatorname{" + string + "}";
    } else if (this.allowedLatexSymbols.includes(string)) return "\\" + string;
    else if (string === "\uff3f" && !this.showBlanks) {
      return "";
    }
    return string;
  }

  factor(tree) {
    if (typeof tree === "string") {
      return this.stringConvert(tree);
    }

    if (typeof tree === "number") {
      if (tree === Infinity) return "\\infty";
      else if (tree === -Infinity) return "-\\infty";
      else if (Number.isNaN(tree)) return "NaN";
      else {
        let numberString = tree.toString();
        let eIndex = numberString.indexOf("e");
        if (eIndex === -1) {
          if (this.padToDigits !== null || this.padToDecimals !== null) {
            numberString = padNumberStringToDigitsAndDecimals(
              numberString,
              this.padToDigits,
              this.padToDecimals,
            );
          }
          return numberString;
        }
        let num = numberString.substring(0, eIndex);
        let exponent = numberString.substring(eIndex + 1);
        if (exponent[0] === "+") {
          exponent = exponent.slice(1);
        }
        if (this.padToDigits !== null || this.padToDecimals !== null) {
          let exponentNumber = Number(exponent);
          if (exponentNumber > 0 && this.padToDecimals !== null) {
            // if padding decimals on a number in scientific notation with positive exponent
            // don't use scientific notation, as it won't save any zeros
            numberString = tree.toLocaleString("fullwide", {
              useGrouping: false,
            });
            return padNumberStringToDigitsAndDecimals(
              numberString,
              this.padToDigits,
              this.padToDecimals,
            );
          } else {
            let padDecimals = null;
            if (this.padToDecimals !== null) {
              padDecimals = this.padToDecimals + exponentNumber;
            }
            num = padNumberStringToDigitsAndDecimals(
              num,
              this.padToDigits,
              padDecimals,
            );
          }
        }

        return num + " \\cdot 10^{" + exponent + "}";
      }
    }

    if (!Array.isArray(tree)) {
      return "";
    }

    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator === "^") {
      if (Number.isInteger(operands[0]) && Number.isInteger(operands[1])) {
        // have integer^integer, as in scientific notation
        // since don't want to pad these numbers with zeros, just return the result directly
        // without invoking factor or other conversion functions
        if (operands[0] < 0) {
          return (
            "\\left(" +
            operands[0].toString() +
            "\\right)^{" +
            operands[1].toString() +
            "}"
          );
        } else {
          return operands[0].toString() + "^{" + operands[1].toString() + "}";
        }
      }

      let operand0 = this.factor(operands[0]);

      // so that f_(st)'^2(x) doesn't get extra parentheses
      // (and no longer recognized as function call)
      // check for simple factor after removing primes
      let remove_primes = operands[0];
      while (remove_primes[0] === "prime") {
        remove_primes = remove_primes[1];
      }

      if (
        !(
          this.simple_factor_or_function_or_parens(remove_primes) ||
          (remove_primes[0] === "_" && typeof remove_primes[1] === "string")
        )
      )
        operand0 = "\\left(" + operand0.toString() + "\\right)";

      return operand0 + "^{" + this.statement(operands[1]) + "}";
    } else if (operator === "_") {
      let operand0 = this.factor(operands[0]);
      if (!this.simple_factor_or_function_or_parens(operands[0]))
        operand0 = "\\left(" + operand0.toString() + "\\right)";

      return operand0 + "_{" + this.statement(operands[1]) + "}";
    } else if (operator === "prime") {
      let op = operands[0];

      let n_primes = 1;
      while (op[0] === "prime") {
        n_primes += 1;
        op = op[1];
      }

      let result = this.factor(op);

      if (
        !(
          this.simple_factor_or_function_or_parens(op) ||
          (op[0] === "_" && typeof op[1] === "string")
        )
      )
        result = "\\left(" + result + "\\right)";
      for (let i = 0; i < n_primes; i++) {
        result += "'";
      }
      return result;
    } else if (operator === "/") {
      return this.term(tree);
    } else if (operator === "-") {
      return operators[operator](
        operands.map(
          function (v, i) {
            return this.factor(v);
          }.bind(this),
        ),
      );
    } else if (
      operator === "tuple" ||
      operator === "array" ||
      operator === "list" ||
      operator === "set" ||
      operator === "vector" ||
      operator === "altvector" ||
      operator === "|" ||
      operator === ":" ||
      operator === "binom" ||
      operator === "vec" ||
      operator === "linesegment"
    ) {
      return operators[operator](
        operands.map(
          function (v, i) {
            return this.statement(v);
          }.bind(this),
        ),
      );
    } else if (operator === "interval") {
      let args = operands[0];
      let closed = operands[1];
      if (args[0] !== "tuple" || closed[0] !== "tuple")
        throw new Error("Badly formed ast");

      let result = this.statement(args[1]) + ", " + this.statement(args[2]);

      if (closed[1]) result = "\\left[ " + result;
      else result = "\\left( " + result;

      if (closed[2]) result = result + " \\right]";
      else result = result + " \\right)";

      return result;
    } else if (operator === "matrix") {
      let size = operands[0];
      let args = operands[1];

      let result = "\\begin{" + this.matrixEnvironment + "} ";

      for (let row = 0; row < size[1]; row += 1) {
        for (let col = 0; col < size[2]; col += 1) {
          result = result + this.statement(args[row + 1][col + 1]);
          if (col < size[2] - 1) result = result + " & ";
        }
        if (row < size[1] - 1) result = result + " \\\\ ";
      }
      result = result + " \\end{" + this.matrixEnvironment + "}";

      return result;
    } else if (
      operator === "derivative_leibniz" ||
      operator === "partial_derivative_leibniz"
    ) {
      let deriv_symbol = "d";
      if (operator === "partial_derivative_leibniz")
        deriv_symbol = "\\partial ";

      let num = operands[0];
      let denom = operands[1];

      let n_deriv = 1;
      let var1 = "";
      if (Array.isArray(num)) {
        var1 = num[1];
        n_deriv = num[2];
      } else var1 = num;

      let result = deriv_symbol;
      if (n_deriv > 1)
        result =
          result.trimRight() + "^{" + n_deriv + "}" + this.stringConvert(var1);
      else {
        result = result + this.stringConvert(var1);
      }

      result = "\\frac{ " + result + " }{ ";

      let n_denom = 1;
      if (Array.isArray(denom)) {
        n_denom = denom.length - 1;
      }

      for (let i = 1; i <= n_denom; i++) {
        let denom_part = denom[i];

        let exponent = 1;
        let var2 = "";
        if (Array.isArray(denom_part)) {
          var2 = denom_part[1];
          exponent = denom_part[2];
        } else var2 = denom_part;

        result = result + deriv_symbol + this.stringConvert(var2);

        if (exponent > 1) result = result + "^{" + exponent + "}";

        result = result + " ";
      }
      result = result + "}";
      return result;
    } else if (operator === "apply") {
      if (operands[0] === "abs") {
        return "\\left|" + this.statement(operands[1]) + "\\right|";
      } else if (operands[0] === "floor") {
        return (
          "\\left\\lfloor " + this.statement(operands[1]) + " \\right\\rfloor"
        );
      } else if (operands[0] === "ceil") {
        return (
          "\\left\\lceil " + this.statement(operands[1]) + " \\right\\rceil"
        );
      } else if (operands[0] === "factorial") {
        let result = this.factor(operands[1]);
        if (
          this.simple_factor_or_function_or_parens(operands[1]) ||
          (operands[1][0] === "_" && typeof operands[1][1] === "string")
        )
          return result + "!";
        else return "\\left(" + result + "\\right)!";
      } else if (operands[0] === "sqrt") {
        return "\\sqrt{" + this.statement(operands[1]) + "}";
      }

      // check if have integral
      let fun = operands[0];
      if (fun[0] === "^") {
        fun = fun[1];
      }
      if (fun[0] === "_") {
        fun = fun[1];
      }
      if (fun === "int") {
        let integral = this.factor(operands[0]);
        let integrand_ast = operands[1];
        let integrand;
        if (Array.isArray(integrand_ast) && integrand_ast[0] === "*") {
          let ds = [];
          let integrand_ast2 = ["*"];
          for (let i = 1; i < integrand_ast.length; i++) {
            let factor = integrand_ast[i];
            if (Array.isArray(factor) && factor[0] === "d") {
              ds.push(factor);
            } else {
              integrand_ast2.push(factor);
            }
          }

          integrand = this.term(integrand_ast2);

          if (ds.length > 0) {
            if (integrand[integrand.length - 1].match(/[a-zA-Z]/)) {
              integrand += "\\,";
            }
            integrand += ds.map((x) => "d" + this.factor(x[1])).join("\\,");
          }
        }

        if (!integrand) {
          integrand = this.term(operands[1]);
        }

        return integral + " " + integrand;
      }

      let f;
      if (operands[0] === "log10") {
        f = "\\log_{10}";
      } else if (operands[0] === "re") {
        f = "\\Re";
      } else if (operands[0] === "im") {
        f = "\\Im";
      } else {
        f = this.factor(operands[0]);
      }

      let f_args = this.statement(operands[1]);

      if (operands[1][0] !== "tuple") f_args = "\\left(" + f_args + "\\right)";

      return f + f_args;
    } else if (operator === "angle") {
      // if all operands are single character strings, superscripts, subscripts or primes
      // then use shorthand notation without parens
      let use_shorthand = operands.every((x) => {
        if (typeof x === "string" && x.length === 1) {
          return true;
        }
        if (!Array.isArray(x)) {
          return false;
        }
        let oper = x[0];
        return oper === "_" || oper === "^" || oper === "prime";
      });

      return operators[operator](
        operands.map(
          function (v, i) {
            return this.statement(v);
          }.bind(this),
        ),
        use_shorthand,
      );
    } else if (operator === "+" && tree.length === 2) {
      return "+ " + this.factor(tree[1]);
    } else {
      return "\\left(" + this.statement(tree) + "\\right)";
    }
  }

  factorWithParenthesesIfNegatedExplicitPlusOrFraction(tree) {
    var result = this.factor(tree);

    if (result.match(/^-/) || result.match(/^\+/) || tree[0] === "/")
      return "\\left(" + result + "\\right)";

    // else
    return result;
  }

  termWithPlusIfNotNegated(tree) {
    var result = this.term(tree);

    if (!result.match(/^-/)) return "+ " + result;

    if (result.match(/^-[^ ]/)) return "- " + result.slice(1);

    // else
    return result;
  }
}

export default astToLatex;
