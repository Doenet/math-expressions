/*
 * convert math.s tree to AST
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



import math from 'mathjs';
const node = math.expression.node;

const operators = {
    "+,add": function(operands) { return ['+'].concat(operands); },
    "*,multiply": function(operands) { return ['*'].concat(operands); },
    "/,divide": function(operands) { return ['/', operands[0], operands[1]]; },
    "-,unaryMinus": function(operands) { return ['-', operands[0]]; },
    "-,subtract": function(operands) { return ['+', operands[0], ['-', operands[1]]]; },
    "^,pow": function(operands) { return ['^', operands[0], operands[1]]; },
    "and,and": function(operands) { return ['and'].concat(operands); },
    "or,or": function(operands) { return ['or'].concat(operands); },
    "not,not": function(operands) { return ['not', operands[0]]; },
    "==,equal": function(operands) { return ['='].concat(operands); },
    "<,smaller": function(operands) { return ['<', operands[0], operands[1]]; },
    ">,larger": function(operands) { return ['>', operands[0], operands[1]]; },
    "<=,smallerEq": function(operands) { return ['le', operands[0], operands[1]]; },
    ">=,largerEq": function(operands) { return ['ge', operands[0], operands[1]]; },
    "!=,unequal": function(operands) { return ['ne', operands[0], operands[1]]; },
    "!,factorial": function(operands) { return ['apply', 'factorial', operands[0]];},
};

class mathjsToAst {

  convert(mathnode){
    if(mathnode.isConstantNode)
  return mathnode.value;
    if(mathnode.isSymbolNode)
  return mathnode.name;

    if(mathnode.isOperatorNode) {
  var key = [mathnode.op, mathnode.fn].join(',')
  if(key in operators)
      return operators[key](
    mathnode.args.map( function(v,i) { return this.convert(v); }.bind(this) ) );
  else
      throw Error("Unsupported operator: " + mathnode.op
      + ", " + mathnode.fn);
    }

    if(mathnode.isFunctionNode) {
  var args = mathnode.args.map(
      function(v,i) { return this.convert(v); }.bind(this) );

  if( args.length > 1)
      args = ["tuple"].concat(args);
  else
      args = args[0]

  var result = ["apply", mathnode.name];
  result.push(args);
  return result;

    }

    if(mathnode.isArrayNode) {
  return ["vector"].concat(mathnode.args.map(
   function(v,i) { return this.convert(v); }.bind(this) ) );
    }

    if(mathnode.isParenthesisNode)
  return this.convert(mathnode.content);

    throw Error("Unsupported node type: " + mathnode.type);

  }

}




export default mathjsToAst;
