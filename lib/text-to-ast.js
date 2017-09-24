/*
 * recursive descent parser for math expressions 
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

/* Grammar:

   statement =
    statement 'OR' statement2 |
    statement2

   statement2 =
    statement2 'AND' relation |
    relation

   relation =
    'NOT' relation |
    '!' relation |
    relation '=' expression |
    relation 'NE' expression |
    relation '<' expression |
    relation '>' expression |
    relation 'LE' expression |
    relation 'GE' expression |
    relation 'IN' expression |
    relation 'NOTIN' expression |
    relation 'NI' expression |
    relation 'NOTNI' expression |
    relation 'SUBSET' expression |
    relation 'NOTSUBSET' expression |
    relation 'SUPERSET' expression |
    relation 'NOTSUPERSET' expression |
    expression

   expression =
    expression '+' term |
    expression '-' term |
    expression 'UNION' term |
    expression 'INTERSECT' term |
    term

   term =
    term '*' factor |
    term nonMinusFactor |
    term '/' factor |
    factor

   baseFactor =
    '(' statement_sequence ')' |
    '[' statement_sequence ']' |
    '{' statement_sequence '}' |
    '(' statement ',' statement ']' |
    '[' statement ',' statement ')' |
    number |
    variable |
    modified_function '(' statement_sequence ')' |
    modified_applied_function '(' statement_sequence ')' |
    modified_function |
    modified_applied_function factor |
    baseFactor '_' baseFactor |
    *** modified_applied_function factor
        allowed only if allow_simplified_function_application==true

   modified_function =
    function |
    function '_' baseFactor |
    function '_' baseFactor '^' factor |
    function '^' factor
    function "'"
    function '_' baseFactor "'"
    function '_' baseFactor "'" '^' factor
    function "'" '^' factor
    *** where the "'" after the functions can be repeated

   modified_applied_function =
    applied_function |
    applied_function '_' baseFactor |
    applied_function '_' baseFactor '^' factor |
    applied_function '^' factor
    applied_function "'"
    applied_function '_' baseFactor "'"
    applied_function '_' baseFactor "'" '^' factor
    applied_function "'" '^' factor
    *** where the "'" after the applied_functions can be repeated

   statement_sequence =
    statement_sequence ',' statement |
    statement

   nonMinusFactor =
    baseFactor |
    baseFactor '^' factor |
    baseFactor '!' and/or "'" |
    baseFactor '!' and/or "'"  '^' factor|
    *** where '!' and/or "'"  indicates arbitrary sequence of "!" and/or "'"

   factor =
    '-' factor |
    nonMinusFactor |
    '|' statement '|'

*/


"use strict";

/****************************************************************/
/* parameters for parser behavior */

// if true, allowed applied functions to omit parentheses around argument
// if false, omitting parentheses will lead to a Parse Error
var allow_simplified_function_application=true;

// symbols that won't be split into a product of letters
var unsplitSymbols = ['pi', 'theta', 'theta', 'Theta', 'alpha', 'nu', 'beta', 'xi', 'Xi', 'gamma', 'Gamma', 'delta', 'Delta', 'pi', 'Pi', 'epsilon', 'epsilon', 'rho', 'rho', 'zeta', 'sigma', 'Sigma', 'eta', 'tau', 'upsilon', 'Upsilon', 'iota', 'phi', 'phi', 'Phi', 'kappa', 'chi', 'lambda', 'Lambda', 'psi', 'Psi', 'omega', 'Omega'];

// Applied functions must be given an argument so that
// they are applied to the argument
var appliedFunctionSymbols = ['sin', 'cos', 'tan', 'csc', 'sec', 'cot', 'arcsin', 'arccos', 'arctan', 'arccsc', 'arcsec', 'arccot', 'log', 'ln', 'exp', 'sqrt', 'abs', 'asin', 'acos', 'atan'];

// Functions could have an argument, in which case they are applied
// or, if they don't have an argument in parentheses, then they are treated
// like a variable, except that trailing ^ and ' have higher precedence
var functionSymbols = ['f', 'g'];


/****************************************************************/
/* setup the lexer */

var ParseError = require('./error').ParseError;

var Parser = require('./lexers/text').Parser;
var lexer = new Parser();

lexer.parse('');
lexer = lexer.lexer;

var symbol = '';
var EOFsymbol = 4

function advance() {
    symbol = lexer.lex();

    if (symbol == 'INVALID') {
	if(yytext() == '_')
	    throw new ParseError("Invalid location of _", yyloc());
	else
	    throw new ParseError("Invalid symbol '" + yytext() + "'",
				 yyloc());
    }

    return symbol;
}

function yytext() {
    return lexer.yytext;
}
function yyloc() {
    return lexer.yylloc;
}

function isUnsplitSymbol( symbol ) {
    return (unsplitSymbols.indexOf(symbol) != -1);
}

function isAppliedFunctionSymbol( symbol ) {
    return (appliedFunctionSymbols.indexOf(symbol) != -1);
}

function isFunctionSymbol( symbol) {
    return (functionSymbols.indexOf(symbol) != -1);
}


/****************************************************************/
/* grammar */

function statement() {

    var lhs=statement2();

    while (symbol == 'OR') {

	var operation = symbol.toLowerCase();

	advance();

	var rhs = statement2();

	var lhs = [operation, lhs, rhs];
    }

    return lhs;
}

function statement2() {
    // split AND into second statement to give higher precedence than OR
    
    var lhs=relation();

    while (symbol == 'AND') {

	var operation = symbol.toLowerCase();

	advance();

	var rhs = relation();

	var lhs = [operation, lhs, rhs];
    }

    return lhs;
}

	
function relation() {

    if(symbol == 'NOT' || symbol == '!') {
	advance();
	return ['not', relation()];
    }

    var lhs = expression();

    while ((symbol == '=') || (symbol == 'NE')
	   || (symbol == '<') || (symbol == '>')
	   || (symbol == 'LE') || (symbol == 'GE')
	   || (symbol == 'IN') || (symbol == 'NOTIN')
	   || (symbol == 'NI') || (symbol == 'NOTNI')
	   || (symbol == 'SUBSET') || (symbol == 'NOTSUBSET')
	   || (symbol == 'SUPERSET') || (symbol == 'NOTSUPERSET')) {

	var operation = symbol.toLowerCase();

	var inequality_sequence=0;
	
	if((symbol == '<') || (symbol == 'LE')) {
	    inequality_sequence = -1;
	}
	else if((symbol == '>') || (symbol == 'GE')) {
	    inequality_sequence = 1;
	}
	
	advance();
	var rhs = expression();

	if(inequality_sequence == -1) {
	    if((symbol == '<') || symbol == 'LE') {
		// sequence of multiple < or <=
		var strict = ['tuple'];
		if(operation == '<')
		    strict.push(true)
		else
		    strict.push(false)

		var args = ['tuple', lhs, rhs];
		while((symbol == '<') || symbol == 'LE') {
		    if(symbol == '<')
			strict.push(true)
		    else
			strict.push(false)

		    advance();
		    args.push(expression());
		}
		lhs = ['lts', args, strict];
	    }
	    else {
		lhs = [operation, lhs, rhs];
	    }
	    
	}
	else if(inequality_sequence == 1) {
	    if((symbol == '>') || symbol == 'GE') {
		// sequence of multiple > or >=
		var strict = ['tuple'];
		if(operation == '>')
		    strict.push(true)
		else
		    strict.push(false)

		var args = ['tuple', lhs, rhs];
		while((symbol == '>') || symbol == 'GE') {
		    if(symbol == '>')
			strict.push(true)
		    else
			strict.push(false)

		    advance();
		    args.push(expression());
		}
		lhs = ['gts', args, strict];
	    }
	    else {
		lhs = [operation, lhs, rhs];
	    }

	}
	else {

	    lhs = [operation, lhs, rhs];
	}

    }

    return lhs;
}


function expression() {
    var lhs = term();
    while ((symbol == '+') || (symbol == '-') || (symbol == 'UNION')
	   || (symbol == 'INTERSECT')) {
	
	var operation = symbol.toLowerCase();
	var negative = false;
	
	if (symbol == '-') {
	    operation = '+';
	    negative = true;
	    advance();
	}
	else  {
	    advance();
	}
	var rhs = term();
	if(negative) {
	    rhs = ['-', rhs];
	}
	
	lhs = [operation, lhs, rhs];
    }
    
    return lhs;
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
	    var rhs = nonMinusFactor();
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
	return ['-', factor()];
    }

    if (symbol == '|') {
	advance();
	
	var result = statement();
	result = ['apply', 'abs', result];
	    
	if (symbol != '|') {
	    throw new ParseError('Expected |', yyloc());
	}
	advance();
	return result;
    }

    var result = nonMinusFactor();

    if(result === false) {
	if (symbol == EOFsymbol) {
	    throw new ParseError("Unexpected end of input", yyloc());
	}
	else {
	    throw new ParseError("Invalid location of '" + yytext() + "'",
				 yyloc());
	}
    }
    else {
	return result;
    }
    
}

function nonMinusFactor() {
    
    var result = baseFactor();
    
    // allow arbitrary sequence of factorials
    if (symbol == '!' || symbol == "'") {
	if(result === false)
	    throw new ParseError("Invalid location of " + symbol, yyloc());
	while(symbol == '!' || symbol == "'") {
	    if(symbol == '!')
		result = ['apply', 'factorial', result]
	    else
		result = ['prime', result];
	    advance();
	}
    }

    if (symbol == '^') {
	if(result === false) {
	    throw new ParseError("Invalid location of ^", yyloc());
	}
	advance();
	return ['^', result, factor()];
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

	if (isAppliedFunctionSymbol(result) || isFunctionSymbol(result))  {
	    var must_apply=false
	    if(isAppliedFunctionSymbol(result))
		must_apply = true;
	    
	    result = result.toLowerCase();
	    advance();

	    if(symbol=='_') {
		advance();
		var subresult =  baseFactor();

		// since baseFactor could return false, must check
		if(subresult === false) {
		    if (symbol == EOFsymbol) {
			throw new ParseError("Unexpected end of input",
					     yyloc());
		    }
		    else {
			throw new ParseError("Invalid location of '" + yytext()
					     + "'", yyloc()) ;
		    }
		}
		result = ['_', result, subresult];
	    }

	    var n_primes=0;
	    while(symbol == "'") {
		n_primes += 1;
		result = ['prime', result];
		advance();
	    }

	    if(symbol=='^') {
		advance();
		result = ['^', result, factor()];
	    }
	    
	    if (symbol == '(') {
		advance();
		var parameters = [statement()];
		while(symbol == ",") {
		    advance();
		    parameters.push(statement());
		}
		if (symbol != ')') {
		    throw new ParseError('Expected )', yyloc());
		}
		advance();

		if(parameters.length > 1)
		    parameters = ['tuple'].concat(parameters);
		else
		    parameters = parameters[0];
		
		result = ['apply', result, parameters];
	    }
	    else {
		// if was an applied function symbol,
		// cannot omit argument
		if(must_apply) {
		    if(!allow_simplified_function_application)
			throw new ParseError("Expected ( after function",
					     yyloc());

		    // if allow simplied function application
		    // let the argument be the next factor
		    result = ['apply', result, factor()];
		}
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
		// so that each character gets processed separately
		// put all characters back on the input
		// but with spaces
		// then process again

		for(var i=result.length-1; i>=0; i--) {
		    lexer.unput(" ");
		    lexer.unput(result[i]);
		}
		advance();

		return baseFactor();
	    }
	    else {
		advance();
	    }
	}
    } else if (symbol == '(' || symbol == '[') {
	var symbol_left = symbol;
	var expected_right, other_right;
	if(symbol == '(') {
	    expected_right = ')';
	    other_right = ']';
	}
	else {
	    expected_right = ']';
	    other_right = ')';
	}
	
	advance();
	result = [statement()];

	var n_elements = 1;
	while(symbol == ",") {
	    advance();
	    result.push(statement());
	    n_elements += 1;
	}
	
	if (symbol != expected_right) {
	    if(n_elements != 2) {
		throw new ParseError('Expected ' + expected_right, yyloc());
	    }
	    else if (symbol != other_right) {
		throw new ParseError('Expected ) or ]', yyloc());
	    }
	}
	
	if (n_elements == 1) {
	    result = result[0];
	}
	else if(n_elements==2 && symbol != expected_right) {
	    result = ['interval', ['tuple'].concat(result)];
	    var closed;
	    if(symbol_left == '(')
		closed = ['tuple', false, true];
	    else
		closed = ['tuple', true, false];
	    result.push(closed);
	}
	else {
	    if(symbol_left == '(') {
		result = ['tuple'].concat(result);
	    }
	    else {
		result = ['array'].concat(result);
	    }
	}
	
	advance();
	
    } else if (symbol == '{') {
	
	advance();
	result = [statement()];

	var n_elements = 1;
	while(symbol == ",") {
	    advance();
	    result.push(statement());
	    n_elements += 1;
	}
	
	if (symbol != '}') {
	    throw new ParseError('Expected }', yyloc());
	}
	
	result = ['set'].concat(result);

	advance();
    }
    
    if (symbol == '_') {
	if(result === false) {
	    throw new ParseError("Invalid location of _", yyloc());
	}
	advance();
	var subresult =  baseFactor();

	if(subresult === false) {
	    if (symbol == EOFsymbol) {
		throw new ParseError("Unexpected end of input", yyloc());
	    }
	    else {
		throw new ParseError("Invalid location of '" + yytext() + "'",
				     yyloc());
	    }
	}
	return ['_', result, subresult];
    }

    return result;
}


// Without reassociating, a string like "1+2+3" is parsed into "(1+2)+3" which doesn't display very well.
function associate_ast( tree, op ) {

    if(!Array.isArray(tree)) {
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
    tree = associate_ast( tree, '*' );
    tree = associate_ast( tree, '=' );
    tree = associate_ast( tree, 'and' );
    tree = associate_ast( tree, 'or' );
    return tree;
}

function parse(input) {
    lexer.setInput(input);
    advance();
    var result=statement()
    if (symbol != EOFsymbol) {
	throw new ParseError("Invalid location of '" + yytext() + "'", yyloc());
    }
    return clean_ast(result);
}

exports.textToAst = parse;
