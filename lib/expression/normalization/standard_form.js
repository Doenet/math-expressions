var function_normalizations = {
    ln: 'log',
    arccos: 'acos',
    arccosh: 'acosh',
    arcsin: 'asin',
    arcsinh: 'asinh',
    arctan: 'atan',
    arctanh: 'atanh',
    arcsec: 'asec',
    arcsech: 'asech',
    arccsc: 'acsc',
    arccsch: 'acsch',
    arccot: 'acot',
    arccoth: 'acoth',
    cosec: 'csc',
}

function normalize_function_names(tree) {
    // replace "ln" with "log"
    // "arccos" with "acos", etc.
    // e^x with exp(x)
    // sqrt(x) with x^2

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
	if(operands[0] === 'sqrt') {
	    return ['^', normalize_function_names(operands[1]), 0.5];
	}
	
	var result = normalize_function_names_sub(operands[0]);
	result = ['apply', result];

	var args = operands.slice(1).map(function(v) {
	    return normalize_function_names(v);});

	if(args.length > 1)
	    args = ['tuple'].concat(args);
	else
	    args = args[0];

	result.push(args);

	return result;
    }
    if (operator === '^' && operands[0] === 'e')
	return ['apply', 'exp', normalize_function_names(operands[1])];

    return [operator].concat(operands.map(function (v) {
	return normalize_function_names(v)}));
}

function normalize_function_names_sub(tree) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	if(tree in function_normalizations)
	    return function_normalizations[tree];
	return tree;
    }    
    
    if (typeof tree === 'boolean') {
	return tree;
    }    
    
    var operator = tree[0];
    var operands = tree.slice(1);

    var result = [operator].concat(operands.map(function (v) {
	return normalize_function_names_sub(v);
    }));
    
    return result;
}

    

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


function substitute_abs(tree) {
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

    if(operator === "apply" && operands[0] === 'abs') {
	return ['^', ['^', substitute_abs(operands[1]), 2], 0.5];
    }

    return [operator].concat(operands.map( function (v) {
	return substitute_abs(v); } ) );
}

exports._normalize_function_names_ast = normalize_function_names;
exports.normalize_function_names = function(expr) {
    return expr.context.from(normalize_function_names(expr.tree));
};
exports._normalize_applied_functions_ast = normalize_applied_functions;
exports.normalize_applied_functions = function(expr) {
    return expr.context.from(normalize_applied_functions(expr.tree));
};

exports._substitute_abs_ast = substitute_abs;
exports.substitute_abs = function(expr) {
    return expr.context.from(substitute_abs(expr.tree));
};

//[ '+', 1, [ '*', [ '-', 'x' ], 'y' ] ]
//versus
//[ '+', 1, [ '-', [ '*', 'x', 'y' ] ] ]
