
/*
 * convert AST to a expression tree from math.js
 *
 * Copyright 2014-2017 by
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


import mathjs from '../mathjs';

var math = mathjs;

const operators = {
  "+": function (operands) { return operands.length === 1 ? operands[0] : new math.OperatorNode('+', 'add', operands) },
  "*": function (operands) { return new math.OperatorNode('*', 'multiply', operands); },
  "/": function (operands) { return new math.OperatorNode('/', 'divide', operands); },
  "-": function (operands) { return new math.OperatorNode('-', 'unaryMinus', [operands[0]]); },
  "^": function (operands) { return new math.OperatorNode('^', 'pow', operands); },
  //"prime": function(operands) { return operands[0] + "'"; },
  //"tuple": function(operands) { return '\\left( ' + operands.join( ', ' ) + ' \\right)';},
  //"array": function(operands) { return '\\left[ ' + operands.join( ', ' ) + ' \\right]';},
  //"set": function(operands) { return '\\left\\{ ' + operands.join( ', ' ) + ' \\right\\}';},
  "vector": function (operands) { return new math.ArrayNode(operands); },
  //"interval": function(operands) { return '\\left( ' + operands.join( ', ' ) + ' \\right)';},
  "and": function (operands) { return new math.OperatorNode('and', 'and', operands); },
  "or": function (operands) { return new math.OperatorNode('or', 'or', operands); },
  "not": function (operands) { return new math.OperatorNode('not', 'not', [operands[0]]); },
  "<": function (operands) { return new math.OperatorNode('<', 'smaller', operands); },
  ">": function (operands) { return new math.OperatorNode('>', 'larger', operands); },
  "le": function (operands) { return new math.OperatorNode('<=', 'smallerEq', operands); },
  "ge": function (operands) { return new math.OperatorNode('>=', 'largerEq', operands); },
  "ne": function (operands) { return new math.OperatorNode('!=', 'unequal', operands); },
  //"union": function (operands) { return operands.join(' \\cup '); },
  //"intersect": function (operands) { return operands.join(' \\cap '); },
  "binom": function (operands) {
    const f = new math.SymbolNode("combinations");
    return new math.FunctionNode(f, operands);
  }
};

const functionConverstions = {
  nCr: "combinations",
  nPr: "permutations",
  binom: "combinations",
}

class astToMathjs {
  constructor({ mathjs = null } = {}) {
    if (mathjs)
      math = mathjs;
  }

  convert(tree) {
    if (typeof tree === 'number') {
      if (Number.isFinite(tree))
        return new math.ConstantNode(tree);
      if (Number.isNaN(tree))
        return new math.SymbolNode('NaN');
      if (tree < 0)
        return operators['-']([new math.SymbolNode('Infinity')]);
      return new math.SymbolNode('Infinity');
    }

    if (typeof tree === 'string') {
      return new math.SymbolNode(tree);
    }

    if (typeof tree === 'boolean')
      throw Error("no support for boolean");

    if (!Array.isArray(tree))
      throw Error("Invalid ast");

    const operator = tree[0];
    const operands = tree.slice(1);

    if (operator === "apply") {
      if (typeof operands[0] !== 'string')
        throw Error("Non string functions not implemented for conversion to mathjs");

      if (operands[0] === "factorial")
        return new math.OperatorNode('!', 'factorial', [this.convert(operands[1])]);

      let functionWord = functionConverstions[operands[0]];
      if (!functionWord) {
        functionWord = operands[0];
      }
      const f = new math.SymbolNode(functionWord);
      const args = operands[1];
      let f_args;

      if (args[0] === 'tuple')
        f_args = args.slice(1).map(function (v, i) { return this.convert(v); }.bind(this));
      else
        f_args = [this.convert(args)];

      return new math.FunctionNode(f, f_args);
    }

    if (operator === 'lts' || operator === 'gts') {
      const args = operands[0]
      const strict = operands[1];

      if (args[0] !== 'tuple' || strict[0] !== 'tuple')
        // something wrong if args or strict are not tuples
        throw new Error("Badly formed ast");

      const arg_nodes = args.slice(1).map(function (v, i) { return this.convert(v); }.bind(this));

      let comparisons = []
      for (let i = 1; i < args.length - 1; i++) {
        if (strict[i]) {
          if (operator === 'lts')
            comparisons.push(new math.OperatorNode('<', 'smaller', arg_nodes.slice(i - 1, i + 1)));
          else
            comparisons.push(new math.OperatorNode('>', 'larger', arg_nodes.slice(i - 1, i + 1)));
        } else {
          if (operator === 'lts')
            comparisons.push(new math.OperatorNode('<=', 'smallerEq', arg_nodes.slice(i - 1, i + 1)));
          else
            comparisons.push(new math.OperatorNode('>=', 'largerEq', arg_nodes.slice(i - 1, i + 1)));
        }
      }
      let result = new math.OperatorNode('and', 'and', comparisons.slice(0, 2));
      for (let i = 2; i < comparisons.length; i++)
        result = new math.OperatorNode('and', 'and', [result, comparisons[i]]);
      return result;
    }

    if (operator === '=') {

      let arg_nodes = operands.map(function (v, i) { return this.convert(v); }.bind(this));

      let comparisons = []
      for (let i = 1; i < arg_nodes.length; i++) {
        comparisons.push(new math.OperatorNode('==', 'equal', arg_nodes.slice(i - 1, i + 1)));
      }

      if (comparisons.length === 1)
        return comparisons[0];

      let result = new math.OperatorNode('and', 'and', comparisons.slice(0, 2));
      for (let i = 2; i < comparisons.length; i++)
        result = new math.OperatorNode('and', 'and', [result, comparisons[i]]);
      return result;
    }

    if (operator === 'in' || operator === 'notin' ||
      operator === 'ni' || operator === 'notni') {

      let x, interval;
      if (operator === 'in' || operator === 'notin') {
        x = operands[0];
        interval = operands[1];
      } else {
        x = operands[1];
        interval = operands[0];
      }
      if ((typeof x !== 'number') && (typeof x !== 'string'))
        throw Error("Set membership non-string variables not implemented for conversion to mathjs");
      x = this.convert(x);

      if (interval[0] !== 'interval')
        throw Error("Set membership in non-intervals not implemented for conversion to mathjs");

      let args = interval[1];
      let closed = interval[2];
      if (args[0] !== 'tuple' || closed[0] !== 'tuple')
        throw new Error("Badly formed ast");

      let a = this.convert(args[1]);
      let b = this.convert(args[2]);

      let comparisons = [];
      if (closed[1])
        comparisons.push(new math.OperatorNode('>=', 'largerEq', [x, a]));
      else
        comparisons.push(new math.OperatorNode('>', 'larger', [x, a]));
      if (closed[2])
        comparisons.push(new math.OperatorNode('<=', 'smallerEq', [x, b]));
      else
        comparisons.push(new math.OperatorNode('<', 'smaller', [x, b]));

      let result = new math.OperatorNode('and', 'and', comparisons);

      if (operator === 'notin' || operator === 'notni')
        result = new math.OperatorNode('not', 'not', [result]);

      return result;
    }

    if (operator === 'subset' || operator === 'notsubset' ||
      operator === 'superset' || operator === 'notsuperset') {

      let big, small;
      if (operator === 'subset' || operator === 'notsubset') {
        small = operands[0];
        big = operands[1];
      } else {
        small = operands[1];
        big = operands[0];
      }
      if (small[0] !== 'interval' || big[0] !== 'interval')
        throw Error("Set containment of non-intervals not implemented for conversion to mathjs");

      let small_args = small[1];
      let small_closed = small[2];
      let big_args = big[1];
      let big_closed = big[2];
      if (small_args[0] !== 'tuple' || small_closed[0] !== 'tuple' ||
        big_args[0] !== 'tuple' || big_closed[0] !== 'tuple')
        throw Error("Badly formed ast");

      let small_a = this.convert(small_args[1]);
      let small_b = this.convert(small_args[2]);
      let big_a = this.convert(big_args[1]);
      let big_b = this.convert(big_args[2]);

      let comparisons = [];
      if (small_closed[1] && !big_closed[1])
        comparisons.push(new math.OperatorNode('>', 'larger', [small_a, big_a]));
      else
        comparisons.push(new math.OperatorNode('>=', 'largerEq', [small_a, big_a]));

      if (small_closed[2] && !big_closed[2])
        comparisons.push(new math.OperatorNode('<', 'smaller', [small_b, big_b]));
      else
        comparisons.push(new math.OperatorNode('<=', 'smallerEq', [small_b, big_b]));

      let result = new math.OperatorNode('and', 'and', comparisons);

      if (operator === 'notsubset' || operator === 'notsuperset')
        result = new math.OperatorNode('not', 'not', [result]);

      return result;
    }

    if (operator === 'matrix') {
      // Convert matrices into nested array nodes
      // Will become matrix on eval

      let size = operands[0];
      let nrows = size[1];
      let ncols = size[2];

      let entries = operands[1];

      if (!Number.isInteger(nrows) || !Number.isInteger(ncols))
        throw Error('Matrix must have integer dimensions');

      let result = [];

      for (let i = 1; i <= nrows; i++) {
        let row = [];
        for (let j = 1; j <= ncols; j++) {
          row.push(this.convert(entries[i][j]));
        }
        result.push(new math.ArrayNode(row));
      }

      return new math.ArrayNode(result);

    }

    if (operator in operators) {
      return operators[operator](
        operands.map(function (v, i) { return this.convert(v); }.bind(this)));
    }

    throw Error("Operator " + operator + " not implemented for conversion to mathjs");

  }


}


export default astToMathjs;
