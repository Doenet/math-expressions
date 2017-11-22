"use strict";

var get_tree = require('../trees/util').get_tree;
var trees = require('../trees/basic');
var parser = require('../parser');
var variables_in = require('./variables').variables;
var simplify = require('./simplify');
var is_nonzero = require('../assumptions/element_of_sets.js').is_nonzero_ast;
var is_positive = require('../assumptions/element_of_sets.js').is_positive_ast;
var is_negative = require('../assumptions/element_of_sets.js').is_negative_ast;


function solve_linear(expr_or_tree, variable, assumptions) {
    // assume expr is linear in variable

    var textToAst = parser.text.to.ast;

    if(!(typeof variable === 'string'))
	return undefined;

    if(assumptions===undefined && expr_or_tree.context !== undefined
       && expr_or_tree.context.get_assumptions !== undefined)
	assumptions = expr_or_tree.context.get_assumptions(
	    expr_or_tree.variables());

    var tree = simplify.simplify(get_tree(expr_or_tree), assumptions);
    //var tree = get_tree(expr_or_tree);
    
    if(!Array.isArray(tree))
	return undefined;

    var operator = tree[0];
    var operands = tree.slice(1);

    if(!(operator === '=' || operator == 'ne'
	 || operator === '<' || operator == 'le'
	 || operator === '>' || operator === 'ge'))
	return undefined;


    // set equal to zero, as lhs = 0
    var lhs = simplify.simplify(['+', operands[0], ['-', operands[1]]],
				assumptions);

    var no_var = tree => !variables_in(tree).includes(variable);

    // factor out variable
    var transformation  = [
	['+', ['*', 'a', variable], ['*', 'b', variable]],
	['*', ['+', 'a', 'b'], variable],
	{variables: {a: no_var, b: no_var},
	 allow_permutations: true,
	 allow_extended_match: true,
	 allow_implicit_identities: ['a', 'b'],
	 evaluate_numbers: true,
	    
	}];

    lhs = simplify.simplify(
	trees.applyAllTransformations(lhs, [transformation], 20));

    if(!variables_in(lhs).includes(variable))
	return undefined;
    
    var pattern = ['+', ['*', 'a', variable], 'b'];

    var params = {
	variables: { a: no_var, b: no_var},
	allow_permutations: true,
	allow_implicit_identities: ['a', 'b'],
    }


    var match = trees.match(lhs, pattern, params);

    if(!match)
	return undefined;  // not linear in variable
    
    var a = simplify.simplify(match['a']);
    var b = simplify.simplify(match['b']);
    
    if(!is_nonzero(a, assumptions))
	return undefined;  // can't confirm that there is a variable

    var result;
    
    // equality or inequality with positive coefficient
    if(operator === '=' || operator === 'ne' || is_positive(a, assumptions)) {
	var result = simplify.simplify(['/', ['-', b], a]);
	return [operator, variable, result];
    }

    if(!is_negative(a, assumptions))
	return undefined;   // couldn't determined sign and have inequality

    // have inequality with negative coefficient
    result = simplify.simplify(['/', ['-', b], a]);
    if(operator === '<')
	operator = '>';
    else if(operator === 'le')
	operator = 'ge';
    else if(operator === '>')
	operator = '<';
    else
	operator = 'le';
    
    return [operator, variable, result];
}

exports.solve_linear = solve_linear;
