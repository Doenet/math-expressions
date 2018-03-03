"use strict";

var get_tree = require('../trees/util').get_tree;
var simplify = require('../expression/simplify');
var math=require('../mathjs');
var operators_in = require('../expression/variables').operators;
var evaluate_to_constant = require("../expression/evaluation").evaluate_to_constant;
var default_order = require('../trees/default_order');
var _ = require('underscore');


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
        if ( !Array.isArray(top) || top[0] !== "monomial")      //if top is constant
            return simplify.evaluate_numbers(['/', top, bottom]);
        else
            return [top[0], simplify.evaluate_numbers(['/', top[1], bottom]), top[2]];       //shouldn't be passing constants other than 1
    }
    
    if ( !Array.isArray(top) || top[0] !== "monomial")      //if top is constant and bottom is not
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
    
    if (j < bottom_length)
        return undefined;
    
    while (i < top_length){
        div_vars.push( top_vars[i]);
        i=i+1;
    }
    
    if (div_vars.length == 0){
        if (bottom[1] == 1)
            return top[1];           //everything canceled, return coefficient of the top
        else
            return simplify.evaluate_numbers(['/', top[1], bottom[1]]);
    }
    
    if (bottom[1] == 1)
        return ["monomial", top[1], div_vars];
    else
        return ["monomial", simplify.evaluate_numbers(['/', top[1], bottom[1]]), div_vars];
}

function mono_is_div(top, bottom) {
    //assume bottom has been computed using gcd function, so has coefficient 1
    
    if ( !Array.isArray(bottom) || bottom[0] !== "monomial"){   //if bottom is constant
        return true;
    }
    
    if ( !Array.isArray(top) || top[0] !== "monomial")      //if top is constant and bottom is not
        return false;
    
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
                return false;       //does not divide
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
            return false;           //does not divide
        }
    }
    
    if (j < bottom_length)
        return false;
    
    return true;
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

function max_div_init(f, monos){
    //f is a polynomial, monos is array of monomials. returns the largest term of f divisible by something
    //in monos, and the index of the divisor.
    if ( f == 0)
        return 0;
    
    let focus = f;
    let var_powers = [];
    
    while( Array.isArray(focus) && focus[0] == "polynomial"){
        let x = focus[1];
        let terms = focus[2];
        let exp = Object.keys(terms)[Object.keys(terms).length-1];
        focus = terms[exp];
        var_powers.push([x,+exp]);
    }
    
    let current_term = ["monomial", focus, var_powers];
    
    let monos_size = monos.length;
    for ( var i = 0; i < monos_size; i++ ){
        if (mono_is_div(current_term, monos[i]))
            return [current_term, i];
    }
    
    return max_div_init(polynomial_sub( f, mono_to_poly(current_term)), monos);
}

function poly_div(f, divs){
    let inits = [];
    let su_mu = [];
    let sp = [];
    let mp = [];
    let f_prime = f;
    
    for (var g of divs){
        if (g == 0)
            return undefined;           //don't divide by 0
        inits.push(initial_term(g));
    }
    
    let m = max_div_init(f_prime, inits);
    
    while (m !== 0){
        sp = m[1];
        mp = mono_div(m[0], inits[sp]);
        su_mu.push([sp, mp]);
        f_prime = polynomial_sub(f_prime, polynomial_mul(mono_to_poly(mp), divs[sp]));
        m = max_div_init(f_prime, inits);
    }
    
    return [su_mu, f_prime];
}

function prereduce(polys){
    let len = polys.length;
    let new_polys = [];
    
    //check for 0's, constants
    for (var j = 0; j < len; j++ ){
        if (polys[j] !== 0 && (!Array.isArray(polys[j]) || polys[j][0] !== "polynomial")){
            return [1];       //if there's a nonzero constant, return [1]
        }
        if (polys[j] !== 0){
            new_polys.push(polys[j]);
        }
    }
    
    if (new_polys.length == 0)
        return [0];
    
    return new_polys;
}

function reduce_ith(i, polys){
    if (!Array.isArray(polys) || i >= polys.length){
        return undefined;
    }
        
    let len = polys.length;
    let new_polys = [];
    
    //check for 0's, constants
    for (var j = 0; j < len; j++ ){
        if (polys[j] !== 0 && (!Array.isArray(polys[j]) || polys[j][0] !== "polynomial")){
            return 1;       //if there's a nonzero constant, return [1]
        }
        if (polys[j] !== 0){
            new_polys.push(polys[j]);
        }
    }
    
    len = new_polys.length;
    
    if (len == 0)
        return 0;         //if there were no nonzero polys, return [0]
    
    if (len == 1)
        return new_polys[0];           //if there's only one poly, don't need to reduce
    
    let others = [];
    for ( var j = 0; j < len; j++ ){
        if (j !== i)
            others.push(new_polys[j]);
    }
    
    return poly_div(new_polys[i], others)[1];
}

function reduce(polys){
    //this could be made more efficient with better bookkeeping if necessary - currently copying sub-arrays a lot, whenever call reduce_ith. Would need to track changes in sub-arrays.
    
    if (!Array.isArray(polys)){
        return undefined;
    }
    
    let i = 0;
    let h=[];
    let new_polys = prereduce(polys);
    let len = new_polys.length;
    
    if (len == 1)
        return new_polys;           //if there's only one poly, don't need to reduce
    
    while (i < len){
        h = reduce_ith(i, new_polys);
        if ( _.isEqual( h, new_polys[i] )){      //from underscore lib to compare arrays with objects
            i=i+1;
        }
        else{
            new_polys[i] = h;
            i = 0;
            new_polys = prereduce(new_polys);
            len = new_polys.length;
        }
    }
    
    i = 0;
    let init = [];
    let coeff = 0;
    while (i < len){            //leading coeffs should be 1
        init = initial_term(new_polys[i]);
        if (!Array.isArray(init) || init[0] !== "monomial")
            new_polys[i] = 1;
        else{
            coeff = init[1];
            if (coeff !== 1)
                new_polys[i] = polynomial_mul(new_polys[i], ['/', 1, coeff]);
        }
        i = i + 1;
    }
    
    return new_polys;
}

function hij(i, j, polys){
    let init_gi = initial_term(polys[i]);
    let init_gj = initial_term(polys[j]);
    let gcd = mono_gcd(init_gi, init_gj);
    let mij = mono_to_poly(mono_div(init_gi, gcd));
    let mji = mono_to_poly(mono_div(init_gj, gcd));
    let std_exp = poly_div(polynomial_sub(polynomial_mul(mji, polys[i]), polynomial_mul(mij, polys[j])), polys);
    return std_exp[1];
}

function reduced_grobner(polys){
    let new_polys = reduce(polys);
    let len = new_polys.length;
    let i = 0;
    let j = 1;
    let h = [];
    let trigger = false;
    
    while (j < len){
        while (i < j){
            h = hij(i, j, new_polys);
            if (h !== 0){
                new_polys.push(h);
                new_polys = reduce(new_polys);      //might be faster not to reduce this often
                len = new_polys.length;
                i = 0;
                j = 1;
            }
            else{
                i = i + 1;
            }
        }
        j = j + 1;
    }
    
    new_polys = reduce(new_polys);
    
    return new_polys;
}

function poly_lcm(f,g){
    let t = ["polynomial", "_t", {1:1}];
    let one_minus_t = ["polynomial", "_t", {0:1, 1:-1}];
    let grob = reduced_grobner([polynomial_mul(t, f), polynomial_mul(one_minus_t, g)]);
    
    //find term without _t
    let len = grob.length;
    for (var i = 0; i < len; i = i + 1){
        if (!Array.isArray(grob[i]) || grob[i][0] !== "polynomial"){
            return 1;           //if there is a constant in the grobner basis, return 1 (shouldn't have constants other than 1, so could probably just check for 1)
        }
        if (grob[i][1] !== "_t")
            return grob[i];
    }
    
    return undefined;       //this should never be reached, unless something bad happens?
}

function poly_gcd(f,g){
    let lcm = poly_lcm(f,g);
    let fg = polynomial_mul(f,g);
    let std_exp = poly_div(fg, [lcm]);

    let sum = 0;
    let len = std_exp[0].length;
    for (var i = 0; i < len; i = i+1){
        sum = polynomial_add(sum, mono_to_poly(std_exp[0][i][1]));
    }
    
    //divide by leading coeff, so leading coeff of gcd is 1
    let init = initial_term(sum);
    if (!Array.isArray(init) || init[0] !== "monomial")
        sum = 1;
    else{
        let coeff = init[1];
        if (coeff !== 1)
            sum = polynomial_mul(sum, ['/', 1, coeff]);
    }
    
    return sum;
}

function poly_by_divisor(f, d){         //divides f by d. This only works correctly if d evenly divides f
    let std_exp = poly_div(f, [d]);
    let sum = 0;
    let len = std_exp[0].length;
    for (var i = 0; i < len; i = i+1){
        sum = polynomial_add(sum, mono_to_poly(std_exp[0][i][1]));
    }
    return sum;
}

function reduce_rational_expression(top, bottom){
    //input: top and bottom of a rational expression. top and bottom should be polynomials, ["polynomial", ...]. returns an array with two entries: new_top and new_bottom, which are reduced (gcd of new_top and new_bottom is 1)
    
    let gcd = poly_gcd(top, bottom);
    let denom_coeff = (initial_term(bottom))[1];
    let div = polynomial_mul(gcd,denom_coeff);
    let new_top = poly_by_divisor(top, div);
    let new_bottom = poly_by_divisor(bottom, div);
    return [new_top, new_bottom];
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
exports.mono_is_div = mono_is_div;
exports.max_div_init = max_div_init;
exports.poly_div = poly_div;
exports.reduce_ith = reduce_ith;
exports.reduce = reduce;
exports.hij = hij;
exports.reduced_grobner = reduced_grobner;
exports.poly_lcm = poly_lcm;
exports.poly_gcd = poly_gcd;
exports.reduce_rational_expression = reduce_rational_expression
