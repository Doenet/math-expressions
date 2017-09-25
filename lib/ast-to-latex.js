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

"use strict";

var operators = {
    "+": function(operands) { return operands.join( ' ' ); },
    "-": function(operands) { return "- " + operands[0]; },
    "*": function(operands) { return operands.join( " " ); },
    "/": function(operands) { return "\\frac{" + operands[0] + "}{" + operands[1] + "}"; },
    "_": function(operands) { return operands[0]  + "_{" + operands[1] + "}"; },
    "^": function(operands) { return operands[0]  + "^{" + operands[1] + "}"; },
    "prime": function(operands) { return operands[0] + "'"; },
    "tuple": function(operands) { return '\\left( ' + operands.join( ', ' ) + ' \\right)';},
    "array": function(operands) { return '\\left[ ' + operands.join( ', ' ) + ' \\right]';},
    "set": function(operands) { return '\\left\\{ ' + operands.join( ', ' ) + ' \\right\\}';},
    "vector": function(operands) { return '\\left( ' + operands.join( ', ' ) + ' \\right)';},
    "interval": function(operands) { return '\\left( ' + operands.join( ', ' ) + ' \\right)';},
    "and": function(operands) { return operands.join( ' \\land ' );},
    "or": function(operands) { return operands.join( ' \\lor ' );},
    "not": function(operands) { return '\\lnot ' + operands[0]; },
    "=": function(operands) { return operands.join( ' = ' );},
    "<": function(operands) { return operands.join( ' < ' );},
    ">": function(operands) { return operands.join( ' > ' );},
    "lts": function(operands) { return operands.join( ' < ' );},
    "gts": function(operands) { return operands.join( ' > ' );},
    "le": function(operands) { return operands.join( ' \\le ' );},
    "ge": function(operands) { return operands.join( ' \\ge ' );},
    "ne": function(operands) { return operands.join( ' \\ne ' );},
    "in": function(operands) { return operands[0] + " \\in " + operands[1]; },
    "notin": function(operands) { return operands[0] + " \\notin " + operands[1]; },
    "ni": function(operands) { return operands[0] + " \\ni " + operands[1]; },
    "notni": function(operands) { return operands[0] + " \\not\\ni " + operands[1]; },
    "subset": function(operands) { return operands[0] + " \\subset " + operands[1]; },
    "notsubset": function(operands) { return operands[0] + " \\not\\subset " + operands[1]; },
    "superset": function(operands) { return operands[0] + " \\supset " + operands[1]; },
    "notsuperset": function(operands) { return operands[0] + " \\not\\supset " + operands[1]; },
    "union": function (operands) { return operands.join(' \\cup '); },
    "intersect": function (operands) { return operands.join(' \\cap '); },
};


function statement(tree) {
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
	return single_statement(tree);
    }

    var operator = tree[0];
    var operands = tree.slice(1);

    if((!(operator in operators)) && operator!=="apply")
	throw new Error("Badly formed ast: operator " + operator + " not recognized.");
    
    if (operator === 'and' || operator === 'or')  {
	return operators[operator]( operands.map( function(v,i) {
	    var result = single_statement(v);
	    // for clarity, add parenthesis unless result is
	    // single quantity (with no spaces) or already has parens
	    if (result.toString().match(/ /)
		&& (!(result.toString().match(/^\\left\(.*\\right\)$/))))
		return '\\left(' + result  + '\\right)';
	    else
		return result;
	}));
    }
    return single_statement(tree);
}

function single_statement(tree) {
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
	return expression(tree);
    }

    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator == 'not') {
	return operators[operator]( operands.map( function(v,i) {
	    var result = single_statement(v);
	    // for clarity, add parenthesis unless result is
	    // single quantity (with no spaces) or already has parens
	    if (result.toString().match(/ /)
		&& (!(result.toString().match(/^\\left\(.*\\right\)$/))))
		return '\\left(' + result  + '\\right)';
	    else
		return result;
	}));
    }

    if((operator == '=') || (operator == 'ne')
       || (operator == '<') || (operator == '>')
       || (operator == 'le') || (operator == 'ge')
       || (operator == 'in') || (operator == 'notin')
       || (operator == 'ni') || (operator == 'notni')
       || (operator == 'subset') || (operator == 'notsubset')
       || (operator == 'superset') || (operator == 'notsuperset')) {
	return operators[operator]( operands.map( function(v,i) {
	    return expression(v);
	}));
    }

    if(operator == 'lts' || operator == 'gts') {
	var args = operands[0]
	var strict = operands[1];

	if(args[0] != 'tuple' || strict[0] != 'tuple')
	    // something wrong if args or strict are not tuples
	    throw new Error("Badly formed ast");

	var result = expression(args[1]);
	for(var i=1; i< args.length-1; i++) {
	    if(strict[i]) {
		if(operator == 'lts')
		    result += " < ";
		else
		    result += " > ";
	    }
	    else {
		if(operator == 'lts') {
		    result += " \\le ";
		}
		else {
		    result += " \\ge ";
		}
	    }
	    result += expression(args[i+1]);
	}
	return result;
    }
    
    return expression(tree);
}


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
		return term(v);
	} ));
    }
    
    if ((operator == 'union') || (operator == 'intersect')) {
	return operators[operator]( operands.map( function(v,i) {
	    return term(v);
	}));
    }

    return term(tree);
}

function term(tree) {
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
	return factor(tree);	
    }
    
    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator == '-') {
	return operators[operator]( operands.map( function(v,i) {
	    return term(v);
	}));
    }
    if (operator == '*') {
	return operators[operator]( operands.map( function(v,i) {
	    var result;
	    if(i > 0) {
		result = factorWithParenthesesIfNegated(v);
		if (result.toString().match( /^[0-9]/ ))
		    return '\\cdot ' + result;
		else
		    return '\\, ' + result
	    }
	    else
		return factor(v);
	}));
    }
    
    if (operator == '/') {
	return operators[operator]( operands.map( function(v,i) { return expression(v); } ) );
    }
    
    return factor(tree);	
}

function simple_factor_or_function_or_parens(tree) {
    // return true if
    // factor(tree) is a single character
    // or tree is a number
    // or tree is a string
    // or tree is a function call
    // or factor(tree) is in parens

    var result=factor(tree);

    if (result.toString().length == 1
	|| (typeof tree == 'number')
	|| (typeof tree == 'string')
	|| (tree[0] == 'apply')
	|| result.toString().match( /^\\left\(.*\\right\)$/)
       )
	return true;
    else
	return false
}


function factor(tree) {
    if (typeof tree === 'string') {
	if (tree == "infinity") return "\\infty";
	if (tree.length > 1) return "\\" + tree;
	return tree;
    }
    
    if (typeof tree === 'number') {
	return tree;
    }
    
    var operator = tree[0];
    var operands = tree.slice(1);

    
    if (operator === "^") {
	var operand0 = factor(operands[0]);

	// so that f_(st)'^2(x) doesn't get extra parentheses
	// (and no longer recognized as function call)
	// check for simple factor after removing primes
	var remove_primes = operands[0];
	while(remove_primes[0] == 'prime') {
	    remove_primes=remove_primes[1];
	}
	
	if(!(simple_factor_or_function_or_parens(remove_primes) ||
	     (remove_primes[0] == '_' &&  (typeof remove_primes[1] == 'string'))
	    ))
	    operand0 = '\\left(' + operand0.toString() + '\\right)';
	
	return operand0 + '^{' + statement(operands[1]) + '}';
    }
    else if (operator === "_") {
	var operand0 = factor(operands[0]);
	if(!(simple_factor_or_function_or_parens(operands[0])))
	    operand0 = '\\left(' + operand0.toString() + '\\right)';
	
	return operand0 + '_{' + statement(operands[1]) + '}';
    }
    else if(operator === "prime") {
	var op = operands[0];

	var n_primes=1;
	while(op[0] === "prime") {
	    n_primes+=1;
	    op=op[1];
	}

	var result = factor(op);
	
	if (!(simple_factor_or_function_or_parens(op) ||
	      (op[0] == '_' &&  (typeof op[1] == 'string'))
	     ))
	    result = '\\left(' + result.toString() + '\\right)';
	for(var i=0; i<n_primes; i++) {
	    result += "'";
	}
	return result;
    }
    else if(operator === "-") {
	return operators[operator]( operands.map( function(v,i) {
	    return factor(v);
	}));
    }
    else if(operator === 'tuple' || operator === 'array'
	    || operator === 'set' || operator === 'vector') {
	return operators[operator]( operands.map( function(v,i) {
	    return statement(v);
	}));
	
    }
    else if(operator === 'interval') {

	var args = operands[0];
	var closed = operands[1];
	if(args[0] !== 'tuple' || closed[0] !== 'tuple')
	    throw new Error("Badly formed ast");

	var result = statement(args[1]) + ", "
	    + statement(args[2]);

	if(closed[1])
	    result = '\\left[ ' + result;
	else
	    result = '\\left( ' + result;
	
	if(closed[2])
	    result = result + ' \\right]';
	else
	    result = result + ' \\right)';

	return result;

    }
    else if(operator == 'apply'){

	if(operands[0] === 'abs') {
	    return '\\left|' + statement(operands[1]) + '\\right|';
	}

	if (operands[0] === "factorial") {
	    var result = factor(operands[1]);
	    if(simple_factor_or_function_or_parens(operands[1]) ||
	       (operands[1][0] == '_' &&  (typeof operands[1][1] == 'string'))
	      )
		return result + "!";
	    else
		return '\\left(' + result.toString() + '\\right)!';
	}

	if(operands[0] == 'sqrt') {
	    return '\\sqrt{' + statement(operands[1]) + '}';
	}
	
	var f = factor(operands[0]);
	var f_args = statement(operands[1]);

	if(operands[1][0] != 'tuple')
	    f_args = "\\left(" + f_args + "\\right)";

	return f+f_args;
    }
    else {
	return '\\left(' + statement(tree) + '\\right)';
    }
}

function factorWithParenthesesIfNegated(tree)
{
    var result = factor(tree);

    if (result.toString().match( /^-/ ))
	return '\\left(' + result.toString() + '\\right)';

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

function astToLatex(tree) {
    return statement(tree);
}

exports.astToLatex = astToLatex;
