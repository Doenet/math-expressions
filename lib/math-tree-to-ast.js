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


var clean = require('./expression/simplify').clean;

var math = require('./mathjs');
var node = math.expression.node;

var operators = {
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

function convert_to_ast(mathnode) {
    if(mathnode.isConstantNode) {
	if (mathnode.valueType === 'number')
	    return parseFloat(mathnode.value);
	else if (mathnode.valueType === 'string') {
	    var result = parseFloat(mathnode.value);
	    if( isNaN(result) )
		return mathnode.value;
	    else
		return result;
	}
	else
	    throw Error("Unsupported ConstantNode valueType: "
			 + mathnode.valueType);
    }
    if(mathnode.isSymbolNode)
	return mathnode.name;

    if(mathnode.isOperatorNode) {
	var key = [mathnode.op, mathnode.fn].join(',')
	if(key in operators) 
	    return operators[key](
		mathnode.args.map( function(v,i) { return convert_to_ast(v); } ) );
	else
	    throw Error("Unsupported operator: " + mathnode.op
			+ ", " + mathnode.fn);
    }

    if(mathnode.isFunctionNode) {
	var args = mathnode.args.map(
	    function(v,i) { return convert_to_ast(v); } );

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
	    function(v,i) { return convert_to_ast(v); } ) );
    }

    if(mathnode.isParenthesisNode)
	return convert_to_ast(mathnode.content);

    throw Error("Unsupported node type: " + mathnode.type);

}


exports.mathTreeToAst = function(mathnode) {
    return clean(convert_to_ast(mathnode));
}
