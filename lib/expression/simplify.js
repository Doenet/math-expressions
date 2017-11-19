'use strict';

var flatten = require('../trees/flatten');
var default_order = require('../trees/default_order').default_order;
var is_nonzero = require('../assumptions/element_of_sets.js').is_nonzero_ast;
var math = require('../mathjs');
var trans = require("../trees/basic.js");
var parsers = require("../parser.js");
var get_tree = require("../trees/util").get_tree;

function clean(expr_or_tree) {
    var tree=get_tree(expr_or_tree);
    return flatten.flatten(tree);
}
    
function simplify(expr_or_tree) {
    var tree = get_tree(expr_or_tree);
    
    tree = evaluate_numbers(tree);
    tree = simplify_logical(tree);
    return tree;
}

function simplify_logical(tree) {
    tree = flatten.unflattenRight(tree);

    var transformations = [];
    transformations.push([parsers.text.to.ast("not (not a)"), parsers.text.to.ast("a")]);
    transformations.push([parsers.text.to.ast("not (a and b)"), parsers.text.to.ast("(not a) or (not b)")]);
    transformations.push([parsers.text.to.ast("not (a or b)"), parsers.text.to.ast("(not a) and (not b)")]);

    tree = trans.applyAllTransformations(tree, transformations, 20);
    
    tree = flatten.flatten(tree);

    return tree;
}

function evaluate_numbers_sub(tree, assumptions) {
    // assume that tree has been sorted to default order (while flatten)
    // and then unflattened_right
    // returns unflattened tree

    if(tree===undefined)
	return tree;

    // in case an expression was passed in rather than a tree
    if(tree.tree !== undefined)
	tree=tree.tree;
    
    if(!Array.isArray(tree))
	return tree;

    var operator = tree[0];
    var operands = tree.slice(1).map(v => evaluate_numbers_sub(v));
    
    if(operator === '+') {
	var left = operands[0];
	var right = operands[1];

	if(right === undefined)
	    return left;
	
	if(typeof left === 'number') {
	    if(left==0)
		return right;
	    if(typeof right === 'number')
		return left + right;
	    // check if right is an addition with that begins with
	    // a constant.  If so combine with left
	    if(Array.isArray(right) && right[0] == '+'
	       && (typeof right[1] === 'number')) {
		return ['+', left+right[1], right[2]];
	    }

	}
	if(typeof right === 'number')
	    if(right == 0)
		return left;

	return [operator].concat(operands);
    }
    if(operator === '-') {
	if(typeof operands[0] === 'number')
	    return -operands[0];
	return [operator].concat(operands);
    }
    if(operator === '*') {
	var left = operands[0];
	var right = operands[1];

	if(right === undefined)
	    return left;

	if(typeof left === 'number') {
	    if(left == 0)
		return 0;
	    if(left == 1)
		return right;
	    if(typeof right === 'number')
		return left * right;
	    if(left == -1)
		return ['-', right];
	    // check if right is a multiplication with that begins with
	    // a constant.  If so combine with left
	    if(Array.isArray(right) && right[0] == '*'
	       && (typeof right[1] === 'number')) {
		left = left*right[1];
		right = right[2];
		if(left == 1)
		    return right;
		if(left == -1)
		    return ['-', right];
		return ['*', left, right];
	    }
	    
	}
	if(typeof right === 'number') {
	    if(right == 0)
		return 0;
	    if(right == 1)
		return left;
	}

	return [operator].concat(operands);
    }

    if(operator === '/') {

	var numer = operands[0];
	var denom = operands[1];
	
	if(typeof numer === 'number') {
	    if(numer == 0) {
		var denom_nonzero = is_nonzero(denom, assumptions);
		if(denom_nonzero)
		    return 0;
		if(denom_nonzero === false)
		    return NaN;  // 0/0
	    }

	    if(typeof denom === 'number') {
		var quotient = numer/denom;
		if(math.round(quotient,2) == quotient)
		    return quotient;
	    }
	}

	return [operator].concat(operands);

    }

    if(operator === '^') {

	var base = operands[0];
	var pow = operands[1];

	if(typeof pow === 'number') {
	    if(pow == 0) {
		if(!math.pow_strict)
		    return 1;
		var base_nonzero = is_nonzero(base, assumptions);
		if(base_nonzero)
		    return 1;
		if(base_nonzero === false)
		    return NaN;   // 0^0
	    }
	    else if(pow == 1) {
		return base;
	    }
	    else if(typeof base === 'number') {
		return math.pow(base, pow);
	    }
	}
	return [operator].concat(operands);
    }

    return [operator].concat(operands);
}


function evaluate_numbers(expr, assumptions) {

    var tree=get_tree(expr);
    
    tree = flatten.unflattenRight(default_order(tree));
    
    var result = evaluate_numbers_sub(tree, assumptions);

    return flatten.flatten(result);
}
   
exports.clean = clean;
exports.simplify = simplify;

exports.evaluate_numbers = evaluate_numbers;
exports.evaluate_numbers.append_to_expression = true;
exports.evaluate_numbers.takes_expression = true;
exports.evaluate_numbers.output_expression = true;

