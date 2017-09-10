/*
 * convert syntax trees back to string representations
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

var output_unicode = true;

var operators = {
    "+": function(operands) { return operands.join( ' ' ); },
    "-": function(operands) { return "- " + operands[0]; },
    "*": function(operands) { return operands.join( " " ); },
    "/": function(operands) { return operands[0] + "/" + operands[1]; },
    "^": function(operands) { return operands[0]  + "^" + operands[1]; },
    "_": function(operands) { return operands[0]  + "_" + operands[1]; },
    "abs": function(operands) { return "|" + operands[0] + "|"; },
    "factorial": function(operands) { return operands[0] + "!"; },
    "prime": function(operands) { return operands[0] + "'"; },
    "tuple": function(operands) { return '( ' + operands.join( ', ' ) + ' )';},
    "list": function(operands) { return '[ ' + operands.join( ', ' ) + ' ]';},
    "set": function(operands) { return '{ ' + operands.join( ', ' ) + ' }';},
    "and": function(operands) { return operands.join( ' and ' );},
    "or": function(operands) { return operands.join( ' or ' );},
    "not": function(operands) { return 'not ' + operands[0]; },
    "=": function(operands) { return operands.join( ' = ' );},
    "<": function(operands) { return operands.join( ' < ' );},
    ">": function(operands) { return operands.join( ' > ' );},
};

if(output_unicode) {
    operators["le"] = function(operands) { return operands.join( ' ≤ ' );};
    operators["ge"] = function(operands) { return operands.join( ' ≥ ' );};
    operators["ne"] = function(operands) { return operands.join( ' ≠ ' );};
    operators["in"] = function(operands) { return operands[0] + "∈" + operaands[1]; };
    operators["notin"] = function(operands) { return operands[0] + "∉" + operaands[1]; };
    operators["ni"] = function(operands) { return operands[0] + "∋" + operaands[1]; };
    operators["notni"] = function(operands) { return operands[0] + "∌" + operaands[1]; };
    operators["subset"] = function(operands) { return operands[0] + "⊂" + operaands[1]; };
    operators["notsubset"] = function(operands) { return operands[0] + "⊄" + operaands[1]; };
    operators["superset"] = function(operands) { return operands[0] + "⊃" + operaands[1]; },
    operators["notsuperset"] = function(operands) { return operands[0] + "⊅" + operaands[1]; };
    operators["union"] = function (operands) { return operands.join(' ∪ '); },
    operators["intersect"] = function (operands) { return operands.join(' ∩ '); };
}
else {
    operators["le"] = function(operands) { return operands.join( ' <= ' );};
    operators["ge"] = function(operands) { return operands.join( ' >= ' );};
    operators["ne"] = function(operands) { return operands.join( ' ne ' );};
    operators["in"] = function(operands) { return operands[0] + " elementof " + operaands[1]; };
    operators["notin"] = function(operands) { return operands[0] + " notelementof " + operaands[1]; };
    operators["ni"] = function(operands) { return operands[0] + " containselement " + operaands[1]; };
    operators["notni"] = function(operands) { return operands[0] + " notcontainselement " + operaands[1]; };
    operators["subset"] = function(operands) { return operands[0] + " subset " + operaands[1]; };
    operators["notsubset"] = function(operands) { return operands[0] + " notsubset " + operaands[1]; };
    operators["superset"] = function(operands) { return operands[0] + " superset " + operaands[1]; },
    operators["notsuperset"] = function(operands) { return operands[0] + " notsuperset " + operaands[1]; };
    operators["union"] = function (operands) { return operands.join(' union '); },
    operators["intersect"] = function (operands) { return operands.join(' intersect '); };
}


function statement(tree) {
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
	return single_statement(tree);
    }

    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator === 'and' || operator === 'or')  {
	return operators[operator]( operands.map( function(v,i) {
	    var result = single_statement(v);
	    // for clarity, add parenthesis unless result is
	    // single quantity (with no spaces) or already has parens
	    if (result.toString().match(/ /)
		&& (!(result.toString().match(/^\(.*\)$/))))
		return '(' + result  + ')';
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
		&& (!(result.toString().match(/^\(.*\)$/))))
		return '(' + result  + ')';
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

	var result = expression(args[0]);
	for(var i=0; i< args.length-1; i++) {
	    if(strict[i]) {
		if(operator == 'lts')
		    result += " < ";
		else
		    result += " > ";
	    }
	    else {
		if(operator == 'lts') {
		    if(output_unicode)
			result += " ≤ ";
		    else
			result += " <= ";
		}
		else {
		    if(output_unicode)
			result += " ≥ ";
		    else
			result += " >= ";
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

    if (operator == '*') {
	return operators[operator]( operands.map( function(v,i) {
	    var result;
	    if(i > 0) {
		result = factorWithParenthesesIfNegated(v);
		if (result.toString().match( /^[0-9]/ ))
		    return '* ' + result;
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

function symbolConvert( symbol) {
    var symbolConversions= {
	'infinity': '∞',
	'alpha': 'α',
	'beta': 'β',
	'Gamma': 'Γ',
	'gamma': 'γ',
	'Delta': 'Δ',
	'delta': 'δ',
	'epsilon': 'ε',
	'zeta': 'ζ',
	'eta': 'η',
	'Theta': 'ϴ',
	'theta': 'θ',
	'iota': 'ι',
	'kappa': 'κ',
	'Lambda': 'Λ',
	'lambda': 'λ',
	'mu': 'μ',
	'nu': 'ν',
	'Xi': 'Ξ',
	'xi': 'ξ',
	'Pi': 'Π',
	'pi': 'π',
	'rho': 'ρ',
	'Sigma': 'Σ',
	'sigma': 'σ',
	'tau': 'τ',
	'Upsilon': 'Υ',
	'upsilon': 'υ',
	'Phi': 'Φ',
	'phi': 'ϕ',
	'Psi': 'Ψ',
	'psi': 'ψ',
	'Omega': 'Ω',
	'omega': 'ω',
    }
    if (output_unicode && (symbol in symbolConversions))
	return symbolConversions[symbol];
    else
	return symbol
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
	|| result.toString().match( /^\(.*\)$/)
       )
	return true;
    else
	return false
}


function factor(tree) {
    if (typeof tree === 'string') {
	return symbolConvert(tree);
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
    
    else if (operator === "factorial") {
	var result = factor(operands[0]);
	if(simple_factor_or_function_or_parens(operands[0]) ||
	   (operands[0][0] == '_' &&  (typeof operands[0][1] == 'string'))
	  )
	    return result + "!";
	else
	    return '(' + result.toString() + ')!';
	
    }
    else if (operator === "^") {
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
	    operand0 = '(' + operand0.toString() + ')';
	
	var operand1 = factor(operands[1]);
	if(!(simple_factor_or_function_or_parens(operands[1])))
	    operand1 = '(' + operand1.toString() + ')';

	return operand0 + '^' + operand1;
    }
    else if (operator === "_") {
	return operators[operator]( operands.map( function(v,i) {
	    var result = factor(v);
	    if(simple_factor_or_function_or_parens(v))
		return result;
	    else
		return '(' + result.toString() + ')';
	}));
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
	    result = '(' + result.toString() + ')';
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
    else if(operator === 'tuple' || operator === 'list'
	    || operator === 'set') {
	return operators[operator]( operands.map( function(v,i) {
	    return statement(v);
	}));
	
    }
    else if(operator === 'interval') {

	var interval_type = "open"

	var closed = operands[1];

	var result = statement(operands[0][0]) + ", "
	    + statement(operands[0][1]);

	if(closed[0])
	    result = '[ ' + result;
	else
	    result = '( ' + result;
	
	if(closed[1])
	    result = result + ' ]';
	else
	    result = result + ' )';

	return result;

    }
    else if(operator == 'apply'){

	var f = factor(operands[0]);
	var f_args = operands[1];
	
	var argstring = f_args.map(function (v,i) {return expression(v)})
	    .join(", ");
	if(argstring)
	    return f + "(" + argstring + ")";
	else
	    return f;
    }
    else {
	return '(' + statement(tree) + ')';
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
    return statement(tree);
}

exports.astToString = astToString;
