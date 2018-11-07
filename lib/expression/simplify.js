import * as flatten from '../trees/flatten';
import { default_order } from '../trees/default_order';
import { is_nonzero_ast as is_nonzero } from '../assumptions/element_of_sets.js';
import { is_positive_ast as is_positive } from '../assumptions/element_of_sets.js';
import { is_negative_ast as is_negative } from '../assumptions/element_of_sets.js';
import math from '../mathjs';
import * as trans from '../trees/basic.js';
import { get_tree } from '../trees/util';
import { evaluate_to_constant } from './evaluation';
import { toExponential } from 'mathjs/lib/utils/number';
import textToAstObj from '../converters/text-to-ast.js';

var textToAst = new textToAstObj();

function clean(expr_or_tree) {
    var tree=get_tree(expr_or_tree);
    return flatten.flatten(tree);
}

function evalf(x, n) {
    return parseFloat(toExponential(x,n));
}

function collapse_unary_minus( expr_or_tree) {
    var tree=get_tree(expr_or_tree);

    if(!Array.isArray(tree))
	return tree;

    var operator = tree[0];
    var operands = tree.slice(1);
    operands = operands.map( v => collapse_unary_minus(v));

    if (operator === "-") {
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

function simplify(expr_or_tree, assumptions, max_digits) {
    var tree = get_tree(expr_or_tree);

    if(assumptions===undefined && expr_or_tree.context !== undefined
       && expr_or_tree.context.get_assumptions !== undefined)
	assumptions = expr_or_tree.context.get_assumptions(
	    [expr_or_tree.variables()]);

    tree = evaluate_numbers(tree, assumptions, max_digits);
    tree = simplify_logical(tree, assumptions);
    tree = collect_like_terms_factors(tree, assumptions, max_digits);

    return tree;
}

function simplify_logical(expr_or_tree, assumptions) {
    var tree = get_tree(expr_or_tree);

    if(assumptions===undefined && expr_or_tree.context !== undefined
       && expr_or_tree.context.get_assumptions !== undefined)
	assumptions = expr_or_tree.context.get_assumptions(
	    [expr_or_tree.variables()]);

    tree = evaluate_numbers(tree, assumptions);

    tree = flatten.unflattenRight(tree);

    var transformations = [];
    transformations.push([textToAst.convert("not (not a)"), textToAst.convert("a")]);
    transformations.push([textToAst.convert("not (a and b)"), textToAst.convert("(not a) or (not b)")]);
    transformations.push([textToAst.convert("not (a or b)"), textToAst.convert("(not a) and (not b)")]);
    transformations.push([textToAst.convert("not (a = b)"), textToAst.convert("a != b")]);
    transformations.push([textToAst.convert("not (a != b)"), textToAst.convert("a = b")]);
    transformations.push([textToAst.convert("not (a < b)"), textToAst.convert("b <= a")]);
    transformations.push([textToAst.convert("not (a <= b)"), textToAst.convert("b < a")]);
    transformations.push([textToAst.convert("not (a elementof b)"), textToAst.convert("a notelementof b")]);
    transformations.push([textToAst.convert("not (a subset b)"), textToAst.convert("a notsubset b")]);

    tree = trans.applyAllTransformations(tree, transformations, 20);

    tree = flatten.flatten(tree);

    return tree;
}

function evaluate_numbers_sub(tree, assumptions, max_digits) {
    // assume that tree has been sorted to default order (while flattened)
    // and then unflattened_right
    // returns unflattened tree

    if(tree===undefined)
	return tree;

    if(typeof tree === 'number')
	return tree;

  var c = evaluate_to_constant(tree);

  if(c !== null) {
    if(typeof c === 'number') {
      if(Number.isFinite(c)) {
	if(max_digits === Infinity)
	  return c;
	let c_minround = evalf(c, 14);
	let c_round = evalf(c, max_digits);
	if(c_round === c_minround)
	  return c;
	
	let c_frac = math.fraction(c);
	let c_frac_d_round = evalf(c_frac.d, 3);

	if(c_frac.n < 1E4 || (c_frac_d_round === c_frac.d)) {
	  let c_reconstruct = evalf(c_frac.s*c_frac.n/c_frac.d, 14);
	  if(c_reconstruct === c_minround)
	    return ['/', c_frac.s*c_frac.n, c_frac.d];
	}
      }
      else if(!Number.isNaN(c))
	return c;
    }
  }

    if(!Array.isArray(tree))
	return tree;

    var operator = tree[0];
    var operands = tree.slice(1).map(v => evaluate_numbers_sub(
	v, assumptions, max_digits));

    if(operator === '+') {
	let left = operands[0];
	let right = operands[1];

	if(right === undefined)
	    return left;

	if(typeof left === 'number') {
	    if(left === 0)
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
	    if(right === 0)
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
	let left = operands[0];
	let right = operands[1];

	if(right === undefined)
	    return left;

	if(typeof left === 'number') {
	    if(isNaN(left))
		return NaN;

	    if(typeof right === 'number')
	    	return left * right;

	    if(!isFinite(left)) {
	      if((left === Infinity && is_negative(right))
		 || (left === -Infinity && is_positive(right)))
		return -Infinity
	      if(is_nonzero(right) === false)
		return NaN;
	      return Infinity;
	    }
	    if(left === 0) {
		return 0;
	    }
	    if(left === 1)
		return right;

	    if(left === -1) {
		return ['-', right];
	    }
	    // check if right is a multiplication with that begins with
	    // a constant.  If so combine with left
	    if(Array.isArray(right) && right[0] === '*'
	       && (typeof right[1] === 'number')) {
		left = left*right[1];
		right = right[2];
		if(left === 1)
		    return right;
		if(left === -1)
		    return ['-', right];
		return ['*', left, right];
	    }

	}
	if(typeof right === 'number') {
	    if(isNaN(right))
		return NaN;
	    if(!isFinite(right)) {
	      if((right === Infinity && is_negative(left))
		 || (right === -Infinity && is_positive(left)))
		return -Infinity
	      if(is_nonzero(left) === false)
		return NaN;
	      return Infinity;
	    }
	    if(right === 0) {
		return 0;
	    }
	    if(right === 1)
		return left;
	    if(right === -1) {
		return ['-', left];
	    }
	    // check if left is a multiplication with that begins with
	    // a constant.  If so combine with right
	    if(Array.isArray(left) && left[0] === '*'
	       && (typeof left[1] === 'number')) {
		right = right*left[1];
		left = left[2];
		if(right === 1)
		    return left;
		if(right === -1)
		    return ['-', left];
		return ['*', left, right];
	    }
	}

	return [operator].concat(operands);
    }

    if(operator === '/') {

	let numer = operands[0];
	let denom = operands[1];

	if(typeof numer === 'number') {
	    if(numer === 0) {
		let denom_nonzero = is_nonzero(denom, assumptions);
		if(denom_nonzero)
		    return 0;
		if(denom_nonzero === false)
		    return NaN;  // 0/0
	    }

	    if(typeof denom === 'number') {
		let quotient = numer/denom;
		if(max_digits === Infinity
		   || math.round(quotient, max_digits) === quotient)
		    return quotient;
		else if (denom < 0)
		    return ['/', -numer, -denom];
	    }

	    // check if denom is a multiplication with that begins with
	    // a constant.  If so combine with numerator
	    if(Array.isArray(denom) && denom[0] === '*'
	       && (typeof denom[1] === 'number')) {
		let quotient = numer/denom[1];

		if(max_digits === Infinity
		   || math.round(quotient, max_digits) === quotient) {
		    return ['/', quotient, denom[2]];
		}
	    }
	}
	else if(typeof denom === 'number') {
	    // check if numer is a multiplication with that begins with
	    // a constant.  If so combine with denominator
	    if(Array.isArray(numer) && numer[0] === '*'
	       && (typeof numer[1] === 'number')) {
		let quotient = numer[1]/denom;
		if(max_digits === Infinity
		   || math.round(quotient, max_digits) === quotient) {
		    if(quotient === 1)
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

	let base = operands[0];
	let pow = operands[1];

	if(typeof pow === 'number') {
	    if(pow === 0) {
		if(!math.pow_strict)
		    return 1;
		let base_nonzero = is_nonzero(base, assumptions);
		if(base_nonzero && (base !== Infinity) && (base !== -Infinity))
		    return 1;
		if(base_nonzero === false)
		    return NaN;   // 0^0
	    }
	    else if(pow === 1) {
		return base;
	    }
	    else if(typeof base === 'number') {
		let result = math.pow(base, pow);
		if(max_digits === Infinity
		   || math.round(result, max_digits) === result)
		    return result;

	    }
	}
	return [operator].concat(operands);
    }

    return [operator].concat(operands);
}


function evaluate_numbers(expr_or_tree, assumptions, max_digits) {

    if(max_digits === undefined ||
       !(Number.isInteger(max_digits) || max_digits === Infinity))
	max_digits = 4;

    var tree=get_tree(expr_or_tree);


    if(assumptions===undefined && expr_or_tree.context !== undefined
       && expr_or_tree.context.get_assumptions !== undefined)
	assumptions = expr_or_tree.context.get_assumptions(
	    [expr_or_tree.variables()]);
  
    tree = flatten.unflattenRight(default_order(flatten.flatten(tree)));

    var result = default_order(evaluate_numbers_sub(
	tree, assumptions, max_digits));

    return flatten.flatten(result);
}

function collect_like_terms_factors(expr_or_tree, assumptions, max_digits) {

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
    transformations.push([textToAst.convert("x/y^a"), textToAst.convert("x*y^(-a)"),
			  {evaluate_numbers: true, max_digits: max_digits}]);
    transformations.push([textToAst.convert("x/y"), textToAst.convert("x*y^(-1)"),
			  {evaluate_numbers: true, max_digits: max_digits}]);
    tree = trans.applyAllTransformations(tree, transformations, 40);

    // collecting like terms and factors
    transformations = [];
    transformations.push(
	[textToAst.convert("x^n*x^m"), textToAst.convert("x^(n+m)"),
	 {variables: {x: v => is_nonzero(v, assumptions),
		      n: isNumber, m: isNumber},
	  evaluate_numbers: true, max_digits: max_digits,
	  allow_implicit_identities: ['m', 'n'],
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );
    transformations.push(
	[textToAst.convert("x^n*x^m"), textToAst.convert("x^(n+m)"),
	 {variables: {x: true,
		      n: v => isNumber(v) && is_positive(v, assumptions),
		      m: v => isNumber(v) && is_positive(v, assumptions)
		     },
	  evaluate_numbers: true, max_digits: max_digits,
	  allow_implicit_identities: ['m', 'n'],
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );
    transformations.push(
	[textToAst.convert("x^n*x^m"), textToAst.convert("x^(n+m)"),
	 {variables: {x: true,
		      n: v => isNumber(v) && is_negative(v, assumptions),
		      m: v => isNumber(v) && is_negative(v, assumptions)
		     },
	  evaluate_numbers: true, max_digits: max_digits,
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );
    transformations.push(
	[textToAst.convert("n*x + m*x"), textToAst.convert("(n+m)*x"),
	 {variables: {x: true,
		      n: isNumber, m: isNumber},
	  evaluate_numbers: true, max_digits: max_digits,
	  allow_implicit_identities: ['m', 'n'],
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );
    transformations.push(
    	[textToAst.convert("n*x - m*x"), textToAst.convert("(n-m)*x"),
    	 {variables: {x: true,
    		      n: isNumber, m: isNumber},
    	  evaluate_numbers: true, max_digits: max_digits,
    	  allow_implicit_identities: ['m', 'n'],
    	  allow_extended_match: true,
    	  allow_permutations: true,
    	  max_group: 1,
    	 }]
    );
    transformations.push(
	[textToAst.convert("(x*y)^a"), textToAst.convert("x^a*y^a"),
	 { allow_permutations: true,}]
    );
    transformations.push(
	[textToAst.convert("(x^n)^m"), textToAst.convert("x^(n*m)"),
	 {variables: {x: true,
		      n: isNumber, m: isNumber},
	  evaluate_numbers: true, max_digits: max_digits,
	  allow_permutations: true,
	 }]
    );
    transformations.push([textToAst.convert("-(a+b)"), textToAst.convert("-a-b")]);

    // evaluate any products
    // (required since evaluate_numbers needs to be applied separately
    // to complicated products to evaluate them as numbers)
    transformations.push(
	[textToAst.convert("x*y"), textToAst.convert("x*y"),
	 {variables: {x: isNumerical, y:isNumerical},
	  evaluate_numbers: true, max_digits: max_digits,
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );

    tree = trans.applyAllTransformations(tree, transformations, 40);

    transformations = [];
    // redo as division
    transformations.push([textToAst.convert("x*y^(-a)"),textToAst.convert("x/y^a"),
    			  {allow_extended_match: true,
    			   allow_permutations: true,
    			   evaluate_numbers: true, max_digits: max_digits,
			   max_group: 1,
    			  }]);
    transformations.push([textToAst.convert("x*y^n"), textToAst.convert("x/y^(-n)"),
    			  {variables: {x: true, y: true,
    				       n: isNegativeNumber},
    			   evaluate_numbers: true, max_digits: max_digits,
    			   allow_extended_match: true,
    			   allow_permutations: true,
			   max_group: 1,
    			  }]);
    tree = trans.applyAllTransformations(tree, transformations, 40);

    transformations = [];
    // redo as division, try 2
    transformations.push([textToAst.convert("y^n"), textToAst.convert("1/y^(-n)"),
    			  {variables: {y: true,
    				       n: isNegativeNumber},
    			   evaluate_numbers: true, max_digits: max_digits,
    			  }]);
    tree = trans.applyAllTransformations(tree, transformations, 40);

    transformations = [];
    // '*' before '/' and products in denominator
    transformations.push([textToAst.convert("x*(y/z)"), textToAst.convert("(x*y)/z"),
    			  {allow_extended_match: true,
    			   allow_permutations: true,
			   max_group: 1,
    			  }]);
    transformations.push([textToAst.convert("(x/y)/z"), textToAst.convert("x/(y*z)"),
    			  {allow_extended_match: true,
    			   allow_permutations: true,
    			  }]);
    transformations.push([textToAst.convert("x/(y/z)"), textToAst.convert("xz/y"),
    			  {allow_extended_match: true,
    			   allow_permutations: true,
    			  }]);
    tree = trans.applyAllTransformations(tree, transformations, 40);

    tree = evaluate_numbers(tree, assumptions, max_digits);

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

	if(numer_factors.length === 1)
	    numer = numer_factors[0];
	else
	    numer = ['*'].concat(numer_factors);
	if(denom_factors.length === 1)
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

export { clean, simplify, simplify_logical, evaluate_numbers, collect_like_terms_factors, collapse_unary_minus, simplify_ratios, default_order };
