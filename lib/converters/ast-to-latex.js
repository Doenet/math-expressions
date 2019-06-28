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


const operators = {
  "+": function(operands) {
    return operands.join(' ');
  },
  "-": function(operands) {
    return "- " + operands[0];
  },
  "*": function(operands) {
    return operands.join(" ");
  },
  "/": function(operands) {
    return "\\frac{" + operands[0] + "}{" + operands[1] + "}";
  },
  "_": function(operands) {
    return operands[0] + "_{" + operands[1] + "}";
  },
  "^": function(operands) {
    return operands[0] + "^{" + operands[1] + "}";
  },
  "prime": function(operands) {
    return operands[0] + "'";
  },
  "tuple": function(operands) {
    return '\\left( ' + operands.join(', ') + ' \\right)';
  },
  "array": function(operands) {
    return '\\left[ ' + operands.join(', ') + ' \\right]';
  },
  "list": function(operands) {
    return operands.join(', ');
  },
  "set": function(operands) {
    return '\\left\\{ ' + operands.join(', ') + ' \\right\\}';
  },
  "vector": function(operands) {
    return '\\left( ' + operands.join(', ') + ' \\right)';
  },
  "interval": function(operands) {
    return '\\left( ' + operands.join(', ') + ' \\right)';
  },
  "matrix": function(operands) {
    return '\\left( ' + operands.join(', ') + ' \\right)';
  },
  "and": function(operands) {
    return operands.join(' \\land ');
  },
  "or": function(operands) {
    return operands.join(' \\lor ');
  },
  "not": function(operands) {
    return '\\lnot ' + operands[0];
  },
  "=": function(operands) {
    return operands.join(' = ');
  },
  "<": function(operands) {
    return operands.join(' < ');
  },
  ">": function(operands) {
    return operands.join(' > ');
  },
  "lts": function(operands) {
    return operands.join(' < ');
  },
  "gts": function(operands) {
    return operands.join(' > ');
  },
  "le": function(operands) {
    return operands.join(' \\le ');
  },
  "ge": function(operands) {
    return operands.join(' \\ge ');
  },
  "ne": function(operands) {
    return operands.join(' \\ne ');
  },
  "in": function(operands) {
    return operands[0] + " \\in " + operands[1];
  },
  "notin": function(operands) {
    return operands[0] + " \\notin " + operands[1];
  },
  "ni": function(operands) {
    return operands[0] + " \\ni " + operands[1];
  },
  "notni": function(operands) {
    return operands[0] + " \\not\\ni " + operands[1];
  },
  "subset": function(operands) {
    return operands[0] + " \\subset " + operands[1];
  },
  "notsubset": function(operands) {
    return operands[0] + " \\not\\subset " + operands[1];
  },
  "superset": function(operands) {
    return operands[0] + " \\supset " + operands[1];
  },
  "notsuperset": function(operands) {
    return operands[0] + " \\not\\supset " + operands[1];
  },
  "union": function(operands) {
    return operands.join(' \\cup ');
  },
  "intersect": function(operands) {
    return operands.join(' \\cap ');
  },
  "derivative_leibniz": function (operands) {
    return "\\frac{d" + operands[0] + "}{d" + operands[1] + "}";
  },
  "partial_derivative_leibniz": function (operands) {
    return "\\frac{d" + operands[0] + "}{d" + operands[1] + "}";
  },
  "|": function (operands) {
    return operands[0] + " \\mid " + operands[1];
  },
  ":": function (operands) {
    return operands[0] + " : " + operands[1];
  },
};

// defaults for parsers if not overridden by context


// allowed multicharacter latex symbols
// in addition to the below applied function symbols
const allowedLatexSymbolsDefault = ['alpha', 'beta', 'gamma', 'Gamma', 'delta', 'Delta', 'epsilon', 'zeta', 'eta', 'theta', 'Theta', 'iota', 'kappa', 'lambda', 'Lambda', 'mu', 'nu', 'xi', 'Xi', 'pi', 'Pi', 'rho', 'sigma', 'Sigma', 'tau', 'Tau', 'upsilon', 'Upsilon', 'phi', 'Phi', 'chi', 'psi', 'Psi', 'omega', 'Omega', 'partial', "abs", "exp", "log", "ln", "log10", "sign", "sqrt", "erf", "acos", "acosh", "acot", "acoth", "acsc", "acsch", "asec", "asech", "asin", "asinh", "atan", "atanh", "cos", "cosh", "cot", "coth", "csc", "csch", "sec", "sech", "sin", "sinh", "tan", "tanh", 'arcsin', 'arccos', 'arctan', 'arccsc', 'arcsec', 'arccot', 'cosec', 'arg'];

const matrixEnvironmentDefault = 'bmatrix';

class astToLatex {

  constructor({
    allowedLatexSymbols=allowedLatexSymbolsDefault,
    matrixEnvironment=matrixEnvironmentDefault,
  } = {}){
    this.allowedLatexSymbols = allowedLatexSymbols;
    this.matrixEnvironment = matrixEnvironment;
  }

  convert(tree) {
    return this.statement(tree);
  }

  statement(tree) {
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
      return this.single_statement(tree);
    }

    var operator = tree[0];
    var operands = tree.slice(1);

    if(operator === 'ldots')
      return '\\ldots';

    if ((!(operator in operators)) && operator !== "apply")
      throw new Error("Badly formed ast: operator " + operator + " not recognized.");

    if (operator === 'and' || operator === 'or') {
      return operators[operator](operands.map(function(v, i) {
        let result = this.single_statement(v);
        // for clarity, add parenthesis unless result is
        // single quantity (with no spaces) or already has parens
        if (result.toString().match(/ /) &&
          (!(result.toString().match(/^\\left\(.*\\right\)$/))))
          return '\\left(' + result + '\\right)';
        else
          return result;
      }.bind(this)));
    }
    return this.single_statement(tree);
  }

  single_statement(tree) {
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
      return this.expression(tree);
    }

    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator === 'not') {
      return operators[operator](operands.map(function(v, i) {
        let result = this.single_statement(v);
        // for clarity, add parenthesis unless result is
        // single quantity (with no spaces) or already has parens
        if (result.toString().match(/ /) &&
          (!(result.toString().match(/^\\left\(.*\\right\)$/))))
          return '\\left(' + result + '\\right)';
        else
          return result;
      }.bind(this)));
    }

    if ((operator === '=') || (operator === 'ne') ||
      (operator === '<') || (operator === '>') ||
      (operator === 'le') || (operator === 'ge') ||
      (operator === 'in') || (operator === 'notin') ||
      (operator === 'ni') || (operator === 'notni') ||
      (operator === 'subset') || (operator === 'notsubset') ||
      (operator === 'superset') || (operator === 'notsuperset')) {
      return operators[operator](operands.map(function(v, i) {
        return this.expression(v);
      }.bind(this)));
    }

    if (operator === 'lts' || operator === 'gts') {
      let args = operands[0]
      let strict = operands[1];

      if (args[0] !== 'tuple' || strict[0] !== 'tuple')
        // something wrong if args or strict are not tuples
        throw new Error("Badly formed ast");

      let result = this.expression(args[1]);
      for (let i = 1; i < args.length - 1; i++) {
        if (strict[i]) {
          if (operator === 'lts')
            result += " < ";
          else
            result += " > ";
        }
        else {
          if (operator === 'lts') {
            result += " \\le ";
          }
          else {
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
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
      return this.term(tree);
    }

    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator === '+') {
      return operators[operator](operands.map(function(v, i) {
        if (i > 0)
          return this.termWithPlusIfNotNegated(v);
        else
          return this.term(v);
      }.bind(this)));
    }

    if ((operator === 'union') || (operator === 'intersect')) {
      return operators[operator](operands.map(function(v, i) {
        return this.term(v);
      }.bind(this)));
    }

    return this.term(tree);
  }

  term(tree) {
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
      return this.factor(tree);
    }

    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator === '-') {
      return operators[operator](operands.map(function(v, i) {
        return this.term(v);
      }.bind(this)));
    }
    if (operator === '*') {
      return operators[operator](operands.map(function(v, i) {
        let result;
        if (i > 0) {
          result = this.factorWithParenthesesIfNegated(v);
          if (result.toString().match(/^[0-9]/))
            return '\\cdot ' + result;
          else
            return '\\, ' + result
        }
        else
          return this.factor(v);
      }.bind(this)));
    }

    if (operator === '/') {
      return operators[operator](operands.map(function(v, i) {
        return this.expression(v);
      }.bind(this)));
    }

    return this.factor(tree);
  }

  simple_factor_or_function_or_parens(tree) {
    // return true if
    // factor(tree) is a single character
    // or tree is a number
    // or tree is a string
    // or tree is a function call other than sqrt
    // or factor(tree) is in parens

    var result = this.factor(tree);

    if (result.toString().length === 1 ||
      (typeof tree === 'number') ||
      (typeof tree === 'string') ||
      (tree[0] === 'apply' && tree[1] !== "sqrt") ||
      result.toString().match(/^\\left\(.*\\right\)$/)
    )
      return true;
    else
      return false
  }

  stringConvert(string) {
    if (string.length > 1) {
      if(this.allowedLatexSymbols.includes(string))
	return "\\" + string;
      else
	return "\\var{" + string + '}';
    }
    return string;
  }

  factor(tree) {
    if (typeof tree === 'string') {
      return this.stringConvert(tree);
    }

    if (typeof tree === 'number') {
      if(tree === Infinity)
        return "\\infty";
      else if(tree === -Infinity)
        return "-\\infty";
      else
        return tree.toString();
    }

    var operator = tree[0];
    var operands = tree.slice(1);


    if (operator === "^") {
      let operand0 = this.factor(operands[0]);

      // so that f_(st)'^2(x) doesn't get extra parentheses
      // (and no longer recognized as function call)
      // check for simple factor after removing primes
      let remove_primes = operands[0];
      while (remove_primes[0] === 'prime') {
        remove_primes = remove_primes[1];
      }

      if (!(this.simple_factor_or_function_or_parens(remove_primes) ||
          (remove_primes[0] === '_' && (typeof remove_primes[1] === 'string'))
        ))
        operand0 = '\\left(' + operand0.toString() + '\\right)';

      return operand0 + '^{' + this.statement(operands[1]) + '}';
    }
    else if (operator === "_") {
      let operand0 = this.factor(operands[0]);
      if (!(this.simple_factor_or_function_or_parens(operands[0])))
        operand0 = '\\left(' + operand0.toString() + '\\right)';

      return operand0 + '_{' + this.statement(operands[1]) + '}';
    }
    else if (operator === "prime") {
      let op = operands[0];

      let n_primes = 1;
      while (op[0] === "prime") {
        n_primes += 1;
        op = op[1];
      }

      let result = this.factor(op);

      if (!(this.simple_factor_or_function_or_parens(op) ||
          (op[0] === '_' && (typeof op[1] === 'string'))
        ))
        result = '\\left(' + result.toString() + '\\right)';
      for (let i = 0; i < n_primes; i++) {
        result += "'";
      }
      return result;
    }
    else if (operator === "-") {
      return operators[operator](operands.map(function(v, i) {
        return this.factor(v);
      }.bind(this)));
    }
    else if (operator === 'tuple' || operator === 'array' ||
      operator === 'list' ||
	     operator === 'set' || operator === 'vector' ||
	     operator === '|' || operator === ':') {
      return operators[operator](operands.map(function(v, i) {
        return this.statement(v);
      }.bind(this)));

    }
    else if (operator === 'interval') {

      let args = operands[0];
      let closed = operands[1];
      if (args[0] !== 'tuple' || closed[0] !== 'tuple')
        throw new Error("Badly formed ast");

      let result = this.statement(args[1]) + ", " +
        this.statement(args[2]);

      if (closed[1])
        result = '\\left[ ' + result;
      else
        result = '\\left( ' + result;

      if (closed[2])
        result = result + ' \\right]';
      else
        result = result + ' \\right)';

      return result;

    }
    else if (operator === 'matrix') {
      let size = operands[0];
      let args = operands[1];

      let result = '\\begin{' + this.matrixEnvironment + '} ';

      for(let row = 0; row < size[1]; row += 1) {
	for(let col = 0; col < size[2]; col += 1) {
	  result = result + this.statement(args[row+1][col+1]);
	  if(col < size[2]-1)
	    result = result + ' & ';
	}
	if(row < size[1]-1)
	  result = result + ' \\\\ ';
      }
      result = result + ' \\end{' + this.matrixEnvironment + '}';

      return result;

    }
    else if(operator === 'derivative_leibniz' || operator === 'partial_derivative_leibniz') {
      let deriv_symbol = "d";
      if(operator === 'partial_derivative_leibniz')
	deriv_symbol = "\\partial ";

      let num = operands[0];
      let denom = operands[1];

      let n_deriv = 1;
      let var1 = "";
      if(Array.isArray(num)) {
	var1 = num[1];
	n_deriv = num[2];
      }
      else
	var1 = num;

      let result = deriv_symbol;
      if(n_deriv > 1)
	result = result.trimRight() + "^{" + n_deriv + "}" + this.stringConvert(var1);
      else {
	result = result + this.stringConvert(var1);
      }

      result = "\\frac{ " +  result + " }{ ";

      let n_denom = 1;
      if(Array.isArray(denom)) {
	n_denom = denom.length-1;
      }

      for(let i=1; i <= n_denom; i++) {
	let denom_part = denom[i];

       let exponent = 1;
	let var2 = "";
	if(Array.isArray(denom_part)) {
	  var2 = denom_part[1];
	  exponent = denom_part[2];
	}
	else
	  var2 = denom_part;

	result = result + deriv_symbol + this.stringConvert(var2);

	if(exponent > 1)
	  result = result + "^{" + exponent + "}";

	result = result + " ";

      }
      result = result + "}";
      return result;

    }
    else if (operator === 'apply') {

      if (operands[0] === 'abs') {
        return '\\left|' + this.statement(operands[1]) + '\\right|';
      }

      if (operands[0] === "factorial") {
        let result = this.factor(operands[1]);
        if (this.simple_factor_or_function_or_parens(operands[1]) ||
          (operands[1][0] === '_' && (typeof operands[1][1] === 'string'))
        )
          return result + "!";
        else
          return '\\left(' + result.toString() + '\\right)!';
      }

      if (operands[0] === 'sqrt') {
        return '\\sqrt{' + this.statement(operands[1]) + '}';
      }

      let f = this.factor(operands[0]);
      let f_args = this.statement(operands[1]);

      if (operands[1][0] !== 'tuple')
        f_args = "\\left(" + f_args + "\\right)";

      return f + f_args;
    }
    else {
      return '\\left(' + this.statement(tree) + '\\right)';
    }
  }

  factorWithParenthesesIfNegated(tree) {
    var result = this.factor(tree);

    if (result.toString().match(/^-/))
      return '\\left(' + result.toString() + '\\right)';

    // else
    return result;
  }

  termWithPlusIfNotNegated(tree) {
    var result = this.term(tree);

    if (!result.toString().match(/^-/))
      return '+ ' + result.toString();

    // else
    return result;
  }

}


export default astToLatex;
