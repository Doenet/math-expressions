/*
 * recursive descent parser for math expressions 
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

/* Grammar: 

   expression =
    expression '+' term | 
    expression '-' term |
    term

   term =
    term '*' factor |
    term nonMinusFactor |
    term '/' factor |
    factor

   baseFactor = 
    '(' expression ')' |
    number | 
    variable |
    function '(' expression ')' |
    baseFactor '_' baseFactor |

   nonMinusFactor =
    baseFactor |
    baseFactor '^' factor |
    baseFactor '!' |
    baseFactor "'"      (except allow arbitrary sequence of ! and ')

   factor = 
    '-' factor |
    nonMinusFactor |
    '|' expression '|'

*/



var Parser = require('./lexers/string').Parser;
var lexer = new Parser();

/****************************************************************/
/* setup the lexer */

lexer.parse('');
lexer = lexer.lexer;

var symbol = '';
var EOFsymbol = 4

function advance() {
    symbol = lexer.lex();

    if (symbol == 'INVALID') {
	if(yytext() == '_')
	    throw "Parse Error: Invalid location of _"
	else 
	    throw "Parse Error: Invalid symbol '" + yytext() + "'";
    }

    return symbol;
}

function yytext() {
    return lexer.yytext;
}

/****************************************************************/
/* grammar */

function expression() {
    var lhs = term();
    while ((symbol == '+') || (symbol == '-')) {
	
	if (symbol == '+')
	    advance();
	
	var rhs = term();

	lhs = ['+', lhs, rhs];
    }
    
    return lhs;
}

function isUnsplitSymbol( symbol )
{
    var unsplitSymbols = ['pi', 'theta', 'theta', 'Theta', 'alpha', 'nu', 'beta', 'xi', 'Xi', 'gamma', 'Gamma', 'delta', 'Delta', 'pi', 'Pi', 'epsilon', 'epsilon', 'rho', 'rho', 'zeta', 'sigma', 'Sigma', 'eta', 'tau', 'upsilon', 'Upsilon', 'iota', 'phi', 'phi', 'Phi', 'kappa', 'chi', 'lambda', 'Lambda', 'psi', 'Psi', 'omega', 'Omega'];
    return (unsplitSymbols.indexOf(symbol) != -1);
}

function isFunctionSymbol( symbol )
{
    var functionSymbols = ['sin', 'cos', 'tan', 'csc', 'sec', 'cot', 'arcsin', 'arccos', 'arctan', 'arccsc', 'arcsec', 'arccot', 'log', 'ln', 'exp', 'sqrt', 'abs', 'asin', 'acos', 'atan',];
    return (functionSymbols.indexOf(symbol) != -1);
}    

function term() {
    var lhs = factor();

    var keepGoing = false;
    
    do {
	keepGoing = false;
	
	if (symbol == '*') {
	    advance();
	    lhs = ['*', lhs, factor()];
	    keepGoing = true;
	} else if (symbol == '/') {
	    advance();
	    lhs = ['/', lhs, factor()];
	    keepGoing = true;
	} else {
	    rhs = nonMinusFactor();
	    if (rhs !== false) {
		lhs = ['*', lhs, rhs];
		keepGoing = true;
	    }
	}
    } while( keepGoing );
    
    return lhs;
}

function factor() {
    if (symbol == '-') {
	advance();
	return ['~', factor()];
    }

    if (symbol == '|') {
	advance();
	
	var result = expression();
	result = ['abs', result];
	    
	if (symbol != '|') {
	    throw 'Parse Error: Expected |';
	}
	advance();	    
	return result;
    }

    var result =  nonMinusFactor();
    
    if(result === false) {
	if (symbol == EOFsymbol) {
	    throw "Parse Error: Unexpected end of input";
	}
	else {
	    throw "Parse Error: Invalid location of '" + yytext() + "'" ;
	}
    }
    else {
	return result;
    }
    
}

function nonMinusFactor() {
    
    var result = baseFactor();
    
    if (symbol == '^') {
	if(result === false) {
	    throw "Parse Error: Invalid location of ^";
	}
	advance();
	return ['^', result, factor()];
    }

    // allow arbitrary sequence of primes and factorials
    if (symbol == '!' | symbol == "'") {
	if(result === false) {
	    if(symbol == '!') 
		throw "Parse Error: Invalid location of '!'";
	    else
		throw "Parse Error: Invalid location of '";
	}
	while(symbol == '!' | symbol == "'") {
	    if(symbol == '!')
		result = ['_factorial', result]
	    else
		result = ["_prime", result]
	    advance();
	}
	return result;
    }

    return result;
}


function baseFactor() {
    var result = false;
    
    if (symbol == 'NUMBER') {
	result = parseFloat( yytext() );
	advance();
    } else if (symbol == 'INFINITY') {
	result = 'infinity';
	advance();		
    } else if (symbol == 'VAR' || symbol == 'VARMULTICHAR') {
	result = yytext();

	if (isFunctionSymbol(result))  {
	    var functionName = result.toLowerCase();
	    advance();
	    if (symbol == '(') {
		advance();
		var parameter = expression();
		if (symbol != ')') {
		    throw 'Parse Error: Expected )';
		}
		advance();
		
		result = [functionName, parameter];
	    }
	    else {
		throw "Parse Error: Expected ( after function";
	    }
	}
	else {
	    // determine if should split text into single letter factors
	    var split = true

	    if(symbol == 'VARMULTICHAR' || isUnsplitSymbol(result)
	       || result.length == 1) {
		split = false;
	    }
	    else if(result.match(/[\d]/g)) {
		// don't split if has a number in it
		split = false;
	    }
	    
	    if (split) {
		var args=[]
		for(var i=0; i < result.length-1; i++) {
		    args.push(result[i])
		}

		// put the last character back on the input
		// and parse again 
		var last_arg = result[result.length-1];
		lexer.unput(last_arg);
		advance();

		last_arg = nonMinusFactor();
		args.push(last_arg)

		result = ["*"].concat(args);
	    }
	    else {
		advance();
	    }
	}
    } else if (symbol == '(') {
	advance();
	result = expression();
	if (symbol != ')') {
	    throw 'Parse Error: Expected )';	    
	}
	advance();
    }
    
    if (symbol == '_') {
	if(result === false) {
	    throw "Parse Error: Invalid location of _";
	}
	advance();
	var subresult =  baseFactor();

	if(subresult === false) {
	    if (symbol == EOFsymbol) {
		throw "Parse Error: Unexpected end of input";
	    }
	    else {
		throw "Parse Error: Invalid location of '" + yytext() + "'" ;
	    }
	}
	return ['_', result, subresult];
    }

    return result;
}


// Without reassociating, a string like "1+2+3" is parsed into "(1+2)+3" which doesn't display very well.
function associate_ast( tree, op ) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	return tree;
    }    
    
    var operator = tree[0];
    var operands = tree.slice(1);
    operands = operands.map( function(v,i) { 
	return associate_ast(v, op); } );
    
    if (operator == op) {
	var result = [];
	
	for( var i=0; i<operands.length; i++ ) {
	    if ((typeof operands[i] !== 'number') && (typeof operands[i] !== 'string') && (operands[i][0] === op)) {
		result = result.concat( operands[i].slice(1) );
	    } else {
		result.push( operands[i] );
	    }
	}
	
	operands = result;
    }
    
    return [operator].concat( operands );
}

function clean_ast( tree ) {
    tree = associate_ast( tree, '+' );
    tree = associate_ast( tree, '-' );
    tree = associate_ast( tree, '*' );
    return tree;
}

function parse(input) {
    lexer.setInput(input);
    advance();
    var result=expression()
    if (symbol != EOFsymbol) {
	throw "Parse Error: Invalid location of '" + yytext() + "'" ;
    }
    return clean_ast(result);
}

exports.stringToAst = parse;
