"use strict";

var get_tree = require('../trees/util').get_tree;
var simplify = require('../expression/simplify');
var math=require('../mathjs');
var operators_in = require('../expression/variables').operators;
var evaluate_to_constant = require("../expression/evaluation").evaluate_to_constant;
var default_order = require('../trees/default_order');


function expression_to_polynomial(expr_or_tree) {

    var tree = get_tree(expr_or_tree);
    
    if(typeof tree === 'string') {
	if((tree == 'pi' && math.define_pi)
	   || (tree == 'i' && math.define_i)
	   || (tree == 'e' && math.define_e))
	    return tree; // treat as number
	else
	    return ['polynomial', tree, {1: 1}];  // treat a polynomial variable
    }
    if(typeof tree === 'number')
	return tree;

    let c = evaluate_to_constant(tree);
    if(c !== null && Number.isFinite(c)) {
	return simplify.simplify(tree);
    }
    
    if(!Array.isArray(tree))
	return false;

    // if contains invalid operators, it's not a polynomial
    if(!operators_in(tree).every(
	v => ['+', '-', '*', '^', '/', '_', 'prime'].includes(v)))
	return false;

    var operator = tree[0];
    var operands = tree.slice(1);

    if(operator === '+') {
	let result = operands.map(expression_to_polynomial)

	// return false if any operand returned false
	if(!result.every(v => v !== false))
	    return false;

	return result.reduce((u,v) => polynomial_add(u,v));
    }	
    else if(operator === '-') {
	let result = expression_to_polynomial(operands[0]);

	if(!result)
	    return false;
	
	return polynomial_neg(result);
    }
    else if(operator === '*') {
	let result = operands.map(expression_to_polynomial)

	// return false if any operand returned false
	if(!result.every(v => v !== false ))
	    return false;

	return result.reduce((u,v) => polynomial_mul(u,v));
    }	
    else if(operator === '^') {

	let base = operands[0];
	let subresult = expression_to_polynomial(base);

	// if subresult itself is false, then don't have a polynomial
	if(subresult === false)
	    return false;
	
	let pow = simplify.simplify(operands[1]);
	
	// if pow isn't a literal nonnegative integer
	if((typeof pow !== 'number') || pow < 0 || !Number.isInteger(pow)) {

	    let pow_num = evaluate_to_constant(pow);
	    
	    // check if pow is a rational number with a small base
	    if(pow_num !== null || Number.isFinite(pow_num)) {
		let pow_fraction = math.fraction(pow_num);
		if(pow_fraction.d <= 100) {
		    if(pow_fraction.s < 0)
			base = ['^', base, ['/', -1, pow_fraction.d]];
		    else
			base = ['^', base, ['/', 1, pow_fraction.d]];
		    
		    var results = ['polynomial', simplify.simplify(base), {}];

		    results[2][pow_fraction.n] = 1;
		    
		    return results;

		}
	    }

	    // just return entire tree as a polynomial variable
	    return ["polynomial", tree, {1:1}];
	}

	if(pow==0) {
	    return 1;
	}
	if(pow==1) {
	    return subresult;
	}
	
	return polynomial_pow(subresult, pow);

    }
    else if(operator === '/') {
	var denom = operands[1];
	
	var denom_num = evaluate_to_constant(denom);

	if(denom_num === null || !Number.isFinite(denom_num)) {
	    // return entire tree as polynomial variable
	    return ['polynomial', tree, {1: 1}];
	}

	var numer_result = expression_to_polynomial(operands[0]);

	return polynomial_mul(numer_result, ['/', 1, denom_num]);
    }
	    
	
    else {
	// return entire tree as polynomial variable
	return ['polynomial', tree, {1: 1}];
    }

    
}


function polynomials_in_same_leading_variable(p,q) {
    // If both polynomials have same leading variable, return unchanged.
    // Else, rewrite the polymomial whose leading variable comes later
    // as a polynomial that is constant in leading variable of other
    
    if(p[1] !== q[1]) {
	if(default_order.compare_function(p[1], q[1]) < 0) {
	    // variable p[1] is earlier in default order
	    // so write q as a polynomial constant in p[1]
	    q = ["polynomial", p[1], {0: q}];
	}
	else {
	    // variable q[1] is earlier in default order
	    // so write p as a polynomial constant in q[1]
	    p = ["polynomial", q[1], {0: p}];
	}
    }

    return [p, q];
}


function polynomial_add(p,q) {

    if(p[0] !== "polynomial") {
	if(q[0] !== "polynomial")
	    return simplify.simplify(['+', p, q]);
	else {
	    // write p as a constant polynomial in q's first variable
	    p = ["polynomial", q[1], {0: p}];
	}
    }
    else {
	if (q[0] !== "polynomial") {
	    // write q as a constant polynomial in p's first variable
	    q = ["polynomial", p[1], {0: q}];
	}
	else {
	    // if needed, rewrite polynomials so have same first variable
	    let tmp = polynomials_in_same_leading_variable(p,q);
	    p = tmp[0];
	    q = tmp[1];
	}
    }

    // at this point, both q and p are polynomials with same first variable

    let sum = ["polynomial", p[1], {}];

    let p_terms = p[2];
    let q_terms = q[2];
    let sum_terms = sum[2];
    
    let inds = [... new Set(Object.keys(p_terms)
			    .concat(Object.keys(q_terms)))];
    
    for(let i of inds) {
	if(p_terms[i]) {
	    if(q_terms[i]) {
		let temp = polynomial_add(p_terms[i], q_terms[i]);
		if(temp)
		    sum_terms[i] = temp;
	    }
	    else
		sum_terms[i] = p_terms[i];
	}
	else if(q_terms[i])
	    sum_terms[i] = q_terms[i];
    }


    // all terms canceled
    if(Object.keys(sum_terms).length == 0)
	return 0;

    // only a term that is constant in leading variable is left
    if(Object.keys(sum_terms).length == 1 && 0 in sum_terms)
	return sum_terms[0];

    return sum;

}


function polynomial_neg(p) {

    if(p[0] !== "polynomial") {
	return simplify.simplify(['-', p ]);
    }

    let result = ["polynomial", p[1], {}];
    let p_terms = p[2];
    let result_terms = result[2];

    for(let i in p_terms) {
	if(p_terms[i])
	    result_terms[i] = polynomial_neg(p_terms[i]);
    }
    return result;
}


function polynomial_sub(p,q) {

    return polynomial_add(p, polynomial_neg(q));

}


function polynomial_mul(p,q) {

    if(p[0] !== "polynomial") {
	if(q[0] !== "polynomial") {
	    return simplify.simplify(['*', p, q]);
	}
	else if(p) {
	    let prod = ["polynomial", q[1], {}];
	    let q_terms = q[2];
	    let prod_terms = prod[2];
	    for(let i in q_terms) {
		if(q_terms[i])
		    prod_terms[i] = polynomial_mul(p, q_terms[i]);
	    }
	    return prod;
	}
    }
    else {
	if (q && q[0] !== "polynomial") {
	    let prod = ["polynomial", p[1], {}];
	    let p_terms = p[2];
	    let prod_terms = prod[2];
	    for(let i in p_terms) {
		if(p_terms[i])
		    prod_terms[i] = polynomial_mul(p_terms[i], q);
	    }
	    return prod;
	}
    }

    // two non-constant polynomials
    // if needed, rewrite polynomials so have same first variable
    let tmp = polynomials_in_same_leading_variable(p,q);
    p = tmp[0];
    q = tmp[1];

    let p_terms = p[2];
    let q_terms = q[2];

    let prod = ["polynomial", p[1], {}];
    let prod_terms = prod[2];
    
    for(let i in p_terms) {
	i = +i; // convert from string to number
	for(let j in q_terms) {
	    j = +j; // convert from string to number
	    if(p_terms[i] && q_terms[j]) {
		let tmp = polynomial_mul(p_terms[i], q_terms[j]);
		if(prod_terms[i+j]) {
		    prod_terms[i+j] = polynomial_add(prod_terms[i+j], tmp);
		    if(!prod_terms[i+j])
			delete prod_terms[i+j];
		}
		else
		    prod_terms[i+j] = tmp;
	    }
	}
    }
    return prod;
}


function polynomial_pow(p, e) {

    if(isNaN(e) || e < 0 || !Number.isInteger(e))
	return undefined;

    let res = 1;
    
    while(e > 0) {

	if(e & 1) {
	    // odd exponent
	    res = polynomial_mul(res, p);
	}

	p = polynomial_mul(p, p);

	e >>= 1; // divide by 2 and truncate

    }
    
    return res;
}


function polynomial_to_expression(p) {

    if(!Array.isArray(p) || p[0] !== "polynomial")
	return p;

    let x = p[1];
    let terms = p[2];
    
    let result = [];

    for(let i in terms) {
	i = +i;  // convert string to number
	if(terms[i]) {
	    if(i==0)
		result.push(polynomial_to_expression(terms[i]));
	    else if(i==1)
		result.push(['*', polynomial_to_expression(terms[i]), x]);
	    else
		result.push(['*', polynomial_to_expression(terms[i]),
			     ['^', x, i]]);
		
	}
    }

    if(result.length == 0)
	return 0;
    else if(result.length == 1)
	result = result[0];
    else
	result.unshift('+');

    return simplify.simplify(result);
}


function initial_term(p) {
    
    //need powers to be stored in ascending order for this to work!!!!!
    
    if (!Array.isArray(p) || p[0] !== "polynomial")
        return p;               //or error?
    
    let var_powers = [];
    
    while( Array.isArray(p) && p[0] == "polynomial"){
        let x = p[1];
        let terms = p[2];
        let exp = Object.keys(terms)[Object.keys(terms).length-1];
        p = terms[exp];
        var_powers.push([x,+exp]);
    }
    
    return ["monomial", p, var_powers];
}

function mono_less_than(left,right) {
    
    if (!Array.isArray(right) || right[0] !== "monomial")
        return false;           //if right is constant, always false
    
    if (!Array.isArray(left) || left[0] !== "monomial")
        return true;        //if left is constant and right is not, always true
    
    let left_vars = left[2];
    let right_vars = right[2];
    let left_length = left_vars.length;
    let right_length = right_vars.length;
    var shorter;
    if (left_length < right_length)
        shorter = left_length;
    else
        shorter = right_length;
    
    for ( var i = 0; i < shorter; i++ ){
        if(left_vars[i][0] !== right_vars[i][0]) {
            if(default_order.compare_function(left_vars[i][0], right_vars[i][0]) < 0) {
                // left variable is earlier in default order
                return false;
            }
            else {
                // right variable is earlier in default order
                return true;
            }
        }
        if(left_vars[i][1] < right_vars[i][1]) {
            // left power is lower
            return true;
        }
        if(left_vars[i][1] > right_vars[i][1]) {
            // right power is lower
            return false;
        }
    }
    if ( left_length == right_length || shorter == right_length ){
        // same monomial, except possibly coefficient, or same until left is longer
        return false;
        }
    else {
        // same until right is longer
        return true;
    }
}


function mono_gcd(left, right) {
    if (!Array.isArray(left) || !Array.isArray(right) || left[0] !== "monomial" || right[0] !== "monomial")
        return 1;               //if either is constant, gcd is 1
    
    let left_vars = left[2];
    let right_vars = right[2];
    let gcd_vars = [];
    let left_length = left_vars.length;
    let right_length = right_vars.length;
    
    let i = 0;
    let j = 0;
    while (i < left_length && j < right_length){
        if (left_vars[i][0] == right_vars[j][0]){
            if (left_vars[i][1] < right_vars[j][1]){
                gcd_vars.push(left_vars[i]);
            }
            else{
                gcd_vars.push(right_vars[j]);
            }
            i = i + 1;
            j = j + 1;
        }
        else if (default_order.compare_function(left_vars[i][0], right_vars[j][0]) < 0){
            i = i + 1;
        }
        else if (default_order.compare_function(right_vars[j][0], left_vars[i][0]) < 0){
            j = j + 1;
        }
    }
    
    if (gcd_vars.length == 0)
        return 1;           //if they have no common variables, gcd is 1
    
    return ["monomial", 1, gcd_vars];
}


function mono_div(top, bottom) {
    //assume bottom has been computed using gcd function, so has coefficient 1
    
    if ( !Array.isArray(bottom) || bottom[0] !== "monomial"){
        //if bottom is constant
        if (bottom == 1)
            return top;
        else
            return undefined;       //shouldn't be passing constants other than 1
    }
    
    if ( !Array.isArray(top) || top[0] !== "monomial")
        return undefined;
    
    let top_vars = top[2];
    let bottom_vars = bottom[2];
    let div_vars = [];
    let top_length = top_vars.length;
    let bottom_length = bottom_vars.length;
    
    let i = 0;
    let j = 0;
    while (i < top_length && j < bottom_length){
        if (top_vars[i][0] == bottom_vars[j][0]){
            if (top_vars[i][1] < bottom_vars[j][1]){
                return undefined;       //does not divide
            }
            else{
                let diff = top_vars[i][1] - bottom_vars[j][1];
                if (diff !== 0)
                    div_vars.push( [ top_vars[i][0] , diff ] );
            }
            i = i + 1;
            j = j + 1;
        }
        else if (default_order.compare_function(top_vars[i][0], bottom_vars[j][0]) < 0){
            div_vars.push( top_vars[i] );
            i = i + 1;
        }
        else if (default_order.compare_function(bottom_vars[j][0], top_vars[i][0]) < 0){
            return undefined;           //does not divide
        }
    }
    
    if (div_vars.length == 0)
        return top[1];           //everything canceled, return coefficient of the top
    
    return ["monomial", top[1], div_vars];
}

function mono_to_poly(mono){
    if ( !Array.isArray(mono) || mono[0] !== "monomial")
        return mono;            //if constant, just return itself
    
    let num_vars = mono[2].length;
    let i = num_vars-1;
    let result = mono[1];
    let index = 0;
    
    while ( i >= 0){
        var obj = {};
        obj[mono[2][i][1]] = result;
        result = ["polynomial", mono[2][i][0], obj];
        i=i-1;
    }
    
    return result;
}



exports.expression_to_polynomial = expression_to_polynomial;
exports.polynomial_add = polynomial_add;
exports.polynomial_neg = polynomial_neg;
exports.polynomial_sub = polynomial_sub;
exports.polynomial_mul = polynomial_mul;
exports.polynomial_pow = polynomial_pow;
exports.polynomial_to_expression = polynomial_to_expression;
exports.initial_term = initial_term;
exports.mono_less_than = mono_less_than;
exports.mono_gcd = mono_gcd;
exports.mono_div = mono_div;
exports.mono_to_poly = mono_to_poly;
