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


var math = require('./mathjs');
var node = math.expression.node;
var normalize = require('./expression/normalization/standard_form');

var operators = {
    "+": function(operands) { return new node.OperatorNode('+', 'add', operands);},
    "*": function(operands) { return new node.OperatorNode('*', 'multiply', operands);},
    "/": function(operands)  { return new node.OperatorNode('/', 'divide', operands);},
    "-": function(operands) { return new node.OperatorNode('-', 'unaryMinus', [operands[0]]);},
    "^": function(operands) { return new node.OperatorNode('^', 'pow', operands);},
    //"prime": function(operands) { return operands[0] + "'"; },
    //"tuple": function(operands) { return '\\left( ' + operands.join( ', ' ) + ' \\right)';},
    //"array": function(operands) { return '\\left[ ' + operands.join( ', ' ) + ' \\right]';},
    //"set": function(operands) { return '\\left\\{ ' + operands.join( ', ' ) + ' \\right\\}';},
    "vector": function(operands) { return new node.ArrayNode(operands);},
    //"interval": function(operands) { return '\\left( ' + operands.join( ', ' ) + ' \\right)';},
    "and": function(operands) { return new node.OperatorNode('and', 'and', operands);},
    "or": function(operands) { return new node.OperatorNode('or', 'or', operands);},
    "not": function(operands) { return new node.OperatorNode('not', 'not', [operands[0]]);},
    "=": function(operands) { return new node.OperatorNode('==', 'equal', operands);},
    "<": function(operands) { return new node.OperatorNode('<', 'smaller', operands);},
    ">": function(operands) { return new node.OperatorNode('>', 'larger', operands);},
    "le": function(operands) { return new node.OperatorNode('<=', 'smallerEq', operands);},
    "ge": function(operands) { return new node.OperatorNode('>=', 'largerEq', operands);},
    "ne": function(operands) { return new node.OperatorNode('!=', 'unequal', operands);},
    //"union": function (operands) { return operands.join(' \\cup '); },
    //"intersect": function (operands) { return operands.join(' \\cap '); },
};

function convert_ast(tree) {
    if (typeof tree === 'number' ) {
	return new node.ConstantNode(tree);
    }

    if (typeof tree === 'string') {
	return new node.SymbolNode(tree);
    }    
    
    var operator = tree[0];
    var operands = tree.slice(1);

    if(operator === "apply") {
	if(typeof operands[0] !== 'string')
	    return new node.SymbolNode('NaN');

	if(operands[0] === "factorial") {
	    return new node.OperatorNode('!', 'factorial',
					 [convert_ast(operands[1])]);
	}
	
	var f = new node.SymbolNode(operands[0]);

	var f_args =  operands.slice(1).map(
	    function(v,i) { return convert_ast(v); });

	return new node.FunctionNode(f, f_args);
    }
    
    if(operator === 'lts' || operator === 'gts') {
	var args = operands[0]
	var strict = operands[1];

	if(args[0] != 'tuple' || strict[0] != 'tuple')
	    // something wrong if args or strict are not tuples
	    throw new Error("Badly formed ast");
	
	var arg_nodes = args.slice(1).map(
	    function(v,i) { return convert_ast(v); } );

	var comparisons = []
	for(var i=1; i< args.length-1; i++) {
	    if(strict[i]) {
		if(operator == 'lts')
		    comparisons.push(new node.OperatorNode('<', 'smaller', arg_nodes.slice(i-1, i+1)));
		else
		    comparisons.push(new node.OperatorNode('>', 'larger', arg_nodes.slice(i-1, i+1)));
	    }
	    else {
		if(operator == 'lts')
		    comparisons.push(new node.OperatorNode('<=', 'smallerEq', arg_nodes.slice(i-1, i+1)));
		else
		    comparisons.push(new node.OperatorNode('>=', 'largerEq', arg_nodes.slice(i-1, i+1)));
	    }
	}
	var result = new node.OperatorNode('and', 'and', comparisons.slice(0,2));
	for(var i=2; i<comparisons.length; i++)
	    result = new node.OperatorNode('and', 'and', [result, comparisons[i]]);
	return result;
    }
    
    if(operator === 'in' || operator === 'notin' ||
       operator === 'ni' || operator === 'notni') {

	if(operator === 'in' || operator === 'notin') {
	    var x = operands[0];
	    var interval = operands[1];
	}
	else {
	    var x = operands[1];
	    var interval = operands[0];
	}
	if((typeof x !== 'number') && (typeof x != 'string'))
	    return new node.SymbolNode('NaN');
	var x = convert_ast(x);

	// at present, just implement for interval
	if(interval[0] !== 'interval')
	    return new node.SymbolNode('NaN');	

	var args = interval[1];
	var closed = interval[2];
	if(args[0] !== 'tuple' || closed[0] !== 'tuple')
	    throw new Error("Badly formed ast");

	var a = convert_ast(args[1]);
	var b = convert_ast(args[2]);

	var comparisons = [];
	if(closed[1])
	    comparisons.push(new node.OperatorNode('>=', 'largerEq', [x,a]));
	else
	    comparisons.push(new node.OperatorNode('>', 'larger', [x,a]));
	if(closed[2])
	    comparisons.push(new node.OperatorNode('<=', 'smallerEq', [x,b]));
	else
	    comparisons.push(new node.OperatorNode('<', 'smaller', [x,b]));
	
	var result =  new node.OperatorNode('and', 'and', comparisons);

	if(operator === 'notin' || operator === 'notni')
	    result = new node.OperatorNode('not', 'not', [result]);

	return result;
    }

    if(operator === 'subset' || operator === 'notsubset' ||
       operator === 'superset' || operator === 'notsuperset') {

	if(operator === 'subset' || operator === 'notsubset') {
	    var small = operands[0];
	    var big = operands[1];
	}
	else {
	    var small = operands[1];
	    var big = operands[0];
	}
	// at present, just implement for intervals
	if(small[0] !== 'interval' || big[0] !== 'interval')
	    return new node.SymbolNode('NaN');	

	var small_args = small[1];
	var small_closed = small[2];
	var big_args = big[1];
	var big_closed = big[2];
	if(small_args[0] !== 'tuple' || small_closed[0] !== 'tuple' ||
	   big_args[0] !== 'tuple' || big_closed[0] !== 'tuple')
	    throw new Error("Badly formed ast");

	var small_a = convert_ast(small_args[1]);
	var small_b = convert_ast(small_args[2]);
	var big_a = convert_ast(big_args[1]);
	var big_b = convert_ast(big_args[2]);

	var comparisons = [];
	if(small_closed[1] && !big_closed[1])
	    comparisons.push(new node.OperatorNode('>', 'larger',
						   [small_a,big_a]));
	else
	    comparisons.push(new node.OperatorNode('>=', 'largerEq',
						   [small_a,big_a]));

	if(small_closed[2] && !big_closed[2])
	    comparisons.push(new node.OperatorNode('<', 'smaller',
						   [small_b,big_b]));
	else
	    comparisons.push(new node.OperatorNode('<=', 'smallerEq',
						   [small_b,big_b]));
	
	var result =  new node.OperatorNode('and', 'and', comparisons);

	if(operator === 'notsubset' || operator === 'notsuperset')
	    result = new node.OperatorNode('not', 'not', [result]);

	return result;
	
    }

    if (operator in operators) {
	return operators[operator](
	    operands.map( function(v,i) { return convert_ast(v); } ) );
    }

    return new node.SymbolNode('NaN');

}

exports.astToMathTree = function (tree) {
    result = normalize._normalize_function_names_ast(
     	normalize._normalize_applied_functions_ast(tree));
    return convert_ast(result);
}
