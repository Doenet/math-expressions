var Expression = require('../../math-expressions');

function normalize_applied_functions(tree) {
    // normalize applied functions
    // so that primes and powers occur outside function application
    
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	return tree;
    }    
    
    if (typeof tree === 'boolean') {
	return tree;
    }    
    
    var operator = tree[0];
    var operands = tree.slice(1);
    
    if (operator === 'apply') {
	var result = strip_function_names(operands[0]);
	var f_applied = ['apply', result.tree, operands[1]];
	for(var i=0; i<result.n_primes; i++)
	    f_applied = ['prime', f_applied];

	if (result.exponent !== undefined)
	    f_applied = ['^', f_applied, result.exponent];

	return f_applied
    }

    var result = [operator].concat( operands.map( function(v,i) { return normalize_applied_functions(v); } ) );
    return result;
};


function strip_function_names(tree) {
    // strip primes and powers off tree
    
    if (typeof tree === 'number') {
	return {tree: tree, n_primes: 0};
    }    
    
    if (typeof tree === 'string') {
	return {tree: tree, n_primes: 0};
    }    

    var operator = tree[0];
    var operands = tree.slice(1);


    if (operator === '^') {
	var result = strip_function_names(operands[0]);
	var exponent = normalize_applied_functions(operands[1]);

	result.exponent=exponent;
	return result;
    }
    
    if (operator ==="prime") {
	var result = strip_function_names(operands[0]);
	result.n_primes += 1;
	return result;
    }
    
    return {tree: normalize_applied_functions(tree), n_primes: 0};
}

exports.normalize_applied_functions = function(expr) {
    return Expression.from(normalize_applied_functions(expr.tree));
};
