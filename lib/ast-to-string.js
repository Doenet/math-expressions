/*
 * convert syntax trees back to string representations
 *
 * Copyright 2014-2015 by Jim Fowler <kisonecat@gmail.com>
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



var operators = {
    "+": function(operands) { return operands.join( ' ' ); },
    "~": function(operands) { return "- " + operands[0]; },
    "*": function(operands) { return operands.join( " " ); },
    "/": function(operands) { return operands[0] + "/" + operands[1]; },
    "^": function(operands) { return operands[0]  + "^" + operands[1]; },
    "_": function(operands) { return operands[0]  + "_" + operands[1]; },
    "abs": function(operands) { return "|" + operands[0] + "|"; },
    "_factorial": function(operands) { return operands[0] + "!"; },
    "_prime": function(operands) { return operands[0] + "'"; },
};


function expression(tree) {
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
	return term(tree);	
    }
    
    var operator = tree[0];
    var operands = tree.slice(1);
    
    if (operator == '+') {
	return operators[operator]( operands.map( function(v,i) {
	    if(i>0) 
		return termWithPlusIfNotNegated(v);
	    else
		return  term(v);
	} ));
    }
    
    return term(tree);
}

function term(tree) {
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
	return factor(tree);	
    }
    
    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator == '*') {
	return operators[operator]( operands.map( function(v,i) {
	    var result;
	    if(i > 0) {
		result = factorWithParenthesesIfNegated(v);
		if (result.toString().match( /^[0-9]/ ))
		    return ' * ' + result;
		else
		    return result
	    }
	    else
		return factor(v);
	}));
    }
    
    if (operator == '/') {
	return operators[operator]( operands.map( function(v,i) { return factor(v); } ) );
    }
    
    return factor(tree);	
}



function isNonFunctionOperator( symbol ) {
    var nonFunctionOperators = ["+", "-", "~", "*", "/", "^", "_", "_factorial", "_prime", "and", "or", "eq", "ne", "in", "subset", "superset", "intersection", "union"];
    return (nonFunctionOperators.indexOf(symbol) != -1);
}

function factor(tree) {
    if (typeof tree === 'string') {
	return tree;
    }    
    
    if (typeof tree === 'number') {
	return tree;
    }
    
    var operator = tree[0];
    var operands = tree.slice(1);	

    // Absolute value doesn't need any special parentheses handling, but its operand is really an expression
    if (operator === "abs") {
	return operators[operator]( operands.map( function(v,i) { return expression(v); } ));
    }
    
    else if (operator === "^" || operator === "_" || operator === "_factorial"
	     || operator === "_prime") {
	return operators[operator]( operands.map( function(v,i) {
	    // operands don't get parens if they are a single character,
	    // or a number or a function call or in parens
	    var result = factor(v);
	    if (result.toString().length == 1
		|| result.toString().match( /^[0-9]*$/ )
		|| (!(isNonFunctionOperator(v[0])))
		|| result.toString().match( /^\(.*\)$/)
	       ) {
		return result;
	    }
	    else
		return '(' + result.toString() + ')';
	}));
    }
    else if(operator === "~") {
	return operators[operator]( operands.map( function(v,i) {
	    return factor(v);
	}));
    }
    else if (isNonFunctionOperator(operator)) {
	// any operator that isn't a function requires parentheses
	// and starting over at expression
	return '(' + expression(tree) + ')';
    }
    else {
	// operator must be a function
	// return operator as function name with operands
	// as expression in parentheses

	var argstring = operands.map(function (v,i) {return expression(v)})
	    .join(" ,");
	return operator + "(" + argstring + ")";
    }
}

function factorWithParenthesesIfNegated(tree)
{
    var result = factor(tree);

    if (result.toString().match( /^-/ ))
	return '(' + result.toString() + ')';

    // else
    return result;
}

function termWithPlusIfNotNegated(tree)
{
    var result = term(tree);

    if (!result.toString().match( /^-/ ))
	return '+ ' + result.toString();

    // else
    return result;
}


function astToString(tree) {
    return expression(tree);
}

exports.astToString = astToString;
