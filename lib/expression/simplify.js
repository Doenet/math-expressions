'use strict';

var flatten = require('../trees/flatten');
var default_order = require('../trees/default_order').default_order;
var is_nonzero = require('../assumptions/element_of_sets.js').is_nonzero_ast;
var is_positive = require('../assumptions/element_of_sets.js').is_positive_ast;
var is_negative = require('../assumptions/element_of_sets.js').is_negative_ast;
var math = require('../mathjs');
var trans = require("../trees/basic.js");
var parser = require("../parser.js");
var get_tree = require("../trees/util").get_tree;
var evaluate_to_constant = require("./evaluation").evaluate_to_constant;

function clean(expr_or_tree) {
    var tree=get_tree(expr_or_tree);
    return flatten.flatten(tree);
}

function collapse_unary_minus( expr_or_tree) {
    var tree=get_tree(expr_or_tree);

    if(!Array.isArray(tree))
	return tree;

    var operator = tree[0];
    var operands = tree.slice(1);
    operands = operands.map( v => collapse_unary_minus(v));

    if (operator == "-") {
	if (typeof operands[0] === 'number')
	    return -operands[0];
	// check if operand is a multiplication with that begins with
	// a constant.  If so combine with constant
	if(Array.isArray(operands[0]) && operands[0][0] === '*'
	   && (typeof operands[0][1] === 'number')) {
	    return ['*', -operands[0][1]].concat(operands[0].slice(2));
	}
	// check if operand is a division with that begins with
	// either
	/// (A) a constant or
	//  (B) a multiplication that begins with a constant.
	// If so. combine with constant
	if(Array.isArray(operands[0]) && operands[0][0] === '/') {
	    if(typeof operands[0][1] === 'number')
		return ['/', -operands[0][1], operands[0][2]];
	    if(Array.isArray(operands[0][1]) && operands[0][1][0] === '*'
	       && (typeof operands[0][1][1] === 'number')) {
		return ['/', [
		    '*', -operands[0][1][1]].concat(operands[0][1].slice(2)),
			operands[0][2]];
	    }
	}	
    }

    return [operator].concat( operands );
}

function simplify(expr_or_tree, assumptions, max_decimals) {
    var tree = get_tree(expr_or_tree);

    if(assumptions===undefined && expr_or_tree.context !== undefined
       && expr_or_tree.context.get_assumptions !== undefined)
	assumptions = expr_or_tree.context.get_assumptions(
	    [expr_or_tree.variables()]);

    tree = evaluate_numbers(tree, assumptions, max_decimals);
    tree = simplify_logical(tree, assumptions);
    tree = collect_like_terms_factors(tree, assumptions);
    return tree;
}

function simplify_logical(expr_or_tree, assumptions) {
    var tree = get_tree(expr_or_tree);

    if(assumptions===undefined && expr_or_tree.context !== undefined
       && expr_or_tree.context.get_assumptions !== undefined)
	assumptions = expr_or_tree.context.get_assumptions(
	    [expr_or_tree.variables()]);

    tree = evaluate_numbers(tree, assumptions);
    
    var textToAst = parser.text.to.ast;
    tree = flatten.unflattenRight(tree);

    var transformations = [];
    transformations.push([textToAst("not (not a)"), textToAst("a")]);
    transformations.push([textToAst("not (a and b)"), textToAst("(not a) or (not b)")]);
    transformations.push([textToAst("not (a or b)"), textToAst("(not a) and (not b)")]);
    transformations.push([textToAst("not (a = b)"), textToAst("a != b")]);
    transformations.push([textToAst("not (a != b)"), textToAst("a = b")]);
    transformations.push([textToAst("not (a < b)"), textToAst("b <= a")]);
    transformations.push([textToAst("not (a <= b)"), textToAst("b < a")]);
    transformations.push([textToAst("not (a elementof b)"), textToAst("a notelementof b")]);
    transformations.push([textToAst("not (a subset b)"), textToAst("a notsubset b")]);

    tree = trans.applyAllTransformations(tree, transformations, 20);
    
    tree = flatten.flatten(tree);

    return tree;
}

function evaluate_numbers_sub(tree, assumptions, max_decimals) {
    // assume that tree has been sorted to default order (while flattened)
    // and then unflattened_right
    // returns unflattened tree

    if(tree===undefined)
	return tree;

    var c = evaluate_to_constant(tree);
    if(c !== null) {
	if(typeof c === 'number') {
	    if(Number.isFinite(c)) {
		if(max_decimals == Infinity || math.round(c,max_decimals)==c)
		    return c;
	    }
	}
    }
    
    if(!Array.isArray(tree))
	return tree;

    var operator = tree[0];
    var operands = tree.slice(1).map(v => evaluate_numbers_sub(
	v, assumptions, max_decimals));
    
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
	    if(Array.isArray(right) && right[0] === '+'
	       && (typeof right[1] === 'number')) {
		return ['+', left+right[1], right[2]];
	    }
	    // check if right is an addition with that begins with
	    // a constant.  If so combine with left
	    if(Array.isArray(right) && right[0] === '+'
	       && (typeof right[2] === 'number')) {
		return ['+', left+right[2], right[1]];
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
	// check if operand is a multiplication with that begins with
	// a constant.  If so combine with constant
	if(Array.isArray(operands[0]) && operands[0][0] === '*'
	   && (typeof operands[0][1] === 'number')) {
	    return ['*', -operands[0][1]].concat(operands[0].slice(2));
	}
	// check if operand is a division with that begins with
	// either
	/// (A) a constant or
	//  (B) a multiplication that begins with a constant.
	// If so. combine with constant
	if(Array.isArray(operands[0]) && operands[0][0] === '/') {
	    if(typeof operands[0][1] === 'number')
		return ['/', -operands[0][1], operands[0][2]];
	    if(Array.isArray(operands[0][1]) && operands[0][1][0] === '*'
	       && (typeof operands[0][1][1] === 'number')) {
		return ['/', [
		    '*', -operands[0][1][1]].concat(operands[0][1].slice(2)),
			operands[0][2]];
		
	    }
	    
	}
	
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
	    if(Array.isArray(right) && right[0] === '*'
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
	    if(right == -1)
		return ['-', left];
	    // check if left is a multiplication with that begins with
	    // a constant.  If so combine with right
	    if(Array.isArray(left) && left[0] === '*'
	       && (typeof left[1] === 'number')) {
		right = right*left[1];
		left = left[2];
		if(right == 1)
		    return left;
		if(right == -1)
		    return ['-', left];
		return ['*', left, right];
	    }
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
		if(max_decimals == Infinity
		   || math.round(quotient, max_decimals) == quotient)
		    return quotient;
		else if (denom < 0)
		    return ['/', -numer, -denom];
	    }
	    
	    // check if denom is a multiplication with that begins with
	    // a constant.  If so combine with numerator
	    if(Array.isArray(denom) && denom[0] === '*'
	       && (typeof denom[1] === 'number')) {
		var quotient = numer/denom[1];
		
		if(max_decimals == Infinity
		   || math.round(quotient, max_decimals) == quotient) {
		    return ['/', quotient, denom[2]];
		}
	    }
	}
	else if(typeof denom === 'number') {
	    // check if numer is a multiplication with that begins with
	    // a constant.  If so combine with denominator
	    if(Array.isArray(numer) && numer[0] === '*'
	       && (typeof numer[1] === 'number')) {
		var quotient = numer[1]/denom;
		if(max_decimals == Infinity
		   || math.round(quotient, max_decimals) == quotient) {
		    if(quotient == 1)
			return numer[2];
		    else
			return ['*', quotient, numer[2]];
		}
		// if denom is negative move negative to number 
		if(denom < 0)
		    return ['/', ['*', -numer[1], numer[2]], -denom];
	    }
	    // if denonimator is negative, negate whole fraction
	    if(denom < 0) {
		if(Array.isArray(numer) && numer[0] === '-')
		    return ['/', numer[1], -denom];
		else
		    return ['-', ['/', numer, -denom]];
		
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
		if(base_nonzero && (base !== Infinity) && (base !== -Infinity))
		    return 1;
		if(base_nonzero === false)
		    return NaN;   // 0^0
	    }
	    else if(pow == 1) {
		return base;
	    }
	    else if(typeof base === 'number') {
		var result = math.pow(base, pow);
		if(max_decimals == Infinity
		   || math.round(result, max_decimals) == result)
		    return result;
		
	    }
	}
	return [operator].concat(operands);
    }

    return [operator].concat(operands);
}


function evaluate_numbers(expr_or_tree, assumptions, max_decimals) {

    if(max_decimals === undefined ||
       !(Number.isInteger(max_decimals) || max_decimals == Infinity))
	max_decimals = 4;
    
    var tree=get_tree(expr_or_tree);

    if(assumptions===undefined && expr_or_tree.context !== undefined
       && expr_or_tree.context.get_assumptions !== undefined)
	assumptions = expr_or_tree.context.get_assumptions(
	    [expr_or_tree.variables()]);
    
    tree = flatten.unflattenRight(default_order(tree));
    
    var result = default_order(evaluate_numbers_sub(
	tree, assumptions, max_decimals));

    return flatten.flatten(result);
}

function collect_like_terms_factors(expr_or_tree, assumptions) {
    var textToAst = parser.text.to.ast;

    function isNumber(s) {
	if (typeof s === 'number')
	    return true;
	if (Array.isArray(s) && s[0] === '-' && (typeof s[1] === 'number'))
	    return true;
	return false;
    }
    function isNegativeNumber(s) {
	if (typeof s === 'number' && s < 0)
	    return true;
	if (Array.isArray(s) && s[0] === '-' && (typeof s[1] === 'number'))
	    return true;
	return false;
    }
    function isNumerical(s) {
	if (typeof s === 'number')
	    return true;
	if (Array.isArray(s) && s[0] === '-' && (typeof s[1] === 'number'))
	    return true;
	let c = evaluate_to_constant(s);
	if(typeof c === 'number' && Number.isFinite(c))
	    return true;

	return false;

    }

    
    var tree=get_tree(expr_or_tree);

    if(assumptions===undefined && expr_or_tree.context !== undefined
       && expr_or_tree.context.get_assumptions !== undefined)
	assumptions = expr_or_tree.context.get_assumptions(
	    [expr_or_tree.variables()]);

    var transformations = [];

    // preliminary transformations
    transformations.push([textToAst("x/y^a"), textToAst("x*y^(-a)"),
			  {evaluate_numbers: true}]);
    transformations.push([textToAst("x/y"), textToAst("x*y^(-1)"),
			  {evaluate_numbers: true}]);
    tree = trans.applyAllTransformations(tree, transformations, 40);

    // collecting like terms and factors
    transformations = [];
    transformations.push(
	[textToAst("x^n*x^m"), textToAst("x^(n+m)"),
	 {variables: {x: v => is_nonzero(v, assumptions),
		      n: isNumber, m: isNumber},
	  evaluate_numbers: true,
	  allow_implicit_identities: ['m', 'n'],
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );
    transformations.push(
	[textToAst("x^n*x^m"), textToAst("x^(n+m)"),
	 {variables: {x: true, 
		      n: v => isNumber(v) && is_positive(v, assumptions),
		      m: v => isNumber(v) && is_positive(v, assumptions)
		     },
	  evaluate_numbers: true,
	  allow_implicit_identities: ['m', 'n'],
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );
    transformations.push(
	[textToAst("x^n*x^m"), textToAst("x^(n+m)"),
	 {variables: {x: true, 
		      n: v => isNumber(v) && is_negative(v, assumptions),
		      m: v => isNumber(v) && is_negative(v, assumptions)
		     },
	  evaluate_numbers: true,
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );
    transformations.push(
	[textToAst("n*x + m*x"), textToAst("(n+m)*x"),
	 {variables: {x: true,
		      n: isNumber, m: isNumber},
	  evaluate_numbers: true,
	  allow_implicit_identities: ['m', 'n'],
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );
    transformations.push(
	[textToAst("(x*y)^a"), textToAst("x^a*y^a"),
	 { allow_permutations: true,}]
    );
    transformations.push(
	[textToAst("(x^n)^m"), textToAst("x^(n*m)"),
	 {variables: {x: true,
		      n: isNumber, m: isNumber},
	  evaluate_numbers: true,
	  allow_permutations: true,
	 }]
    );
    transformations.push([textToAst("-(a+b)"), textToAst("-a-b")]);

    // evaluate any products
    // (required since evaluate_numbers needs to be applied separately
    // to complicated products to evaluate them as numbers)
    transformations.push(
	[textToAst("x*y"), textToAst("x*y"),
	 {variables: {x: isNumerical, y:isNumerical},
	  evaluate_numbers: true,
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );

    tree = trans.applyAllTransformations(tree, transformations, 40);

    transformations = [];
    // redo as division
    transformations.push([textToAst("x*y^(-a)"),textToAst("x/y^a"),
    			  {allow_extended_match: true,
    			   allow_permutations: true,
    			   evaluate_numbers: true,
			   max_group: 1,
    			  }]);
    transformations.push([textToAst("x*y^n"), textToAst("x/y^(-n)"),
    			  {variables: {x: true, y: true,
    				       n: isNegativeNumber},
    			   evaluate_numbers: true,
    			   allow_extended_match: true,
    			   allow_permutations: true,
			   max_group: 1,
    			  }]);
    tree = trans.applyAllTransformations(tree, transformations, 40);

    transformations = [];
    // redo as division, try 2
    transformations.push([textToAst("y^n"), textToAst("1/y^(-n)"),
    			  {variables: {y: true,
    				       n: isNegativeNumber},
    			   evaluate_numbers: true,
    			  }]);
    tree = trans.applyAllTransformations(tree, transformations, 40);
    
    transformations = [];
    // '*' before '/' and products in denominator
    transformations.push([textToAst("x*(y/z)"), textToAst("(x*y)/z"),
    			  {allow_extended_match: true,
    			   allow_permutations: true,
			   max_group: 1,
    			  }]);
    transformations.push([textToAst("(x/y)/z"), textToAst("x/(y*z)"),
    			  {allow_extended_match: true,
    			   allow_permutations: true,
    			  }]);
    tree = trans.applyAllTransformations(tree, transformations, 40);

    tree = evaluate_numbers(tree, assumptions);

    return tree;
    
}

function simplify_ratios(expr_or_tree, assumptions) {

    // TODO: actually factor numerator and denominator
    // for now, assume factored, other than minus sign

    function remove_negative_factors(factors) {

	var sign_change = 1;
	
	factors = factors.map(function (v) {
	    if(typeof v === "number") {
		if(v<0) {
		    sign_change *= -1;
		    return -v;
		}
		return v;
	    }
	    if(!Array.isArray(v))
		return v;

	    if(v[0] === '-') {
		sign_change *= -1;
		return v[1];
	    }
	    if(v[0] !== '+')
		return v;

	    var negate=false;
	    if((typeof v[1] === "number") && v[1] < 0)
		negate = true;
	    else if(Array.isArray(v[1]) && v[1][0] === '-')
		negate = true;

	    if(negate) {
		sign_change *= -1 ;
		var v_ops = v.slice(1).map(x => ['-', x]);
		return evaluate_numbers(['+'].concat(v_ops));
	    }
	    else
		return v;
	});

	return {factors: factors, sign_change: sign_change};
    }
	
    function simplify_ratios_sub(tree) {

	if(!Array.isArray(tree))
	    return tree;

	var operator = tree[0];
	var operands = tree.slice(1).map(v => simplify_ratios_sub(v));

	if(operator !== '/')
	    return [operator].concat(operands);

	var numer = operands[0];
	var denom = operands[1];

	// factor a minus sign from each factor in numerator and denominator
	// if it is negative or it is a sum with a negative first term
	// (when terms are sorted as though they were not negative)
	
	numer = default_order(numer, {ignore_negatives: true});
	var numer_factors;
	if(Array.isArray(numer) && numer[0] === '*')
	    numer_factors = numer.slice(1);
	else
	    numer_factors = [numer];
	var result_n = remove_negative_factors(numer_factors);
	numer_factors = result_n["factors"];
	
	denom = default_order(denom, {ignore_negatives: true});
	var denom_factors;
	if(Array.isArray(denom) && denom[0] === '*')
	    denom_factors = denom.slice(1);
	else
	    denom_factors = [denom];
	var result_d = remove_negative_factors(denom_factors);
	denom_factors = result_d["factors"];
	
	if(result_n["sign_change"]*result_d["sign_change"] < 0)
	    numer_factors[0] = ['-', numer_factors[0]];
	    
	if(numer_factors.length == 1)
	    numer = numer_factors[0];
	else
	    numer = ['*'].concat(numer_factors);
	if(denom_factors.length == 1)
	    denom = denom_factors[0];
	else
	    denom = ['*'].concat(denom_factors);
    
	return ['/', numer, denom];
	
    }

    
    var tree = get_tree(expr_or_tree);
    
    if(assumptions===undefined && expr_or_tree.context !== undefined
       && expr_or_tree.context.get_assumptions !== undefined)
	assumptions = expr_or_tree.context.get_assumptions(
	    [expr_or_tree.variables()]);

    return simplify_ratios_sub(tree);
    
}

exports.clean = clean;
exports.simplify = simplify;
exports.simplify_logical = simplify_logical;

exports.evaluate_numbers = evaluate_numbers;
exports.collect_like_terms_factors = collect_like_terms_factors;
exports.collapse_unary_minus = collapse_unary_minus;
exports.simplify_ratios = simplify_ratios;
