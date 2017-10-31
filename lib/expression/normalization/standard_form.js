var clean_ast = require("../simplify")._clean_ast;

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


function remove_duplicate_negatives(tree) {
    // remove pairs of consecutive minus signs

    if(!Array.isArray(tree))
	return tree;

    var operator = tree[0];
    var operands = tree.slice(1);

    if(operator === '-' && operands[0][0] == '-') {
	return remove_duplicate_negatives(operands[0][1]);
    }

    operands = operands.map(remove_duplicate_negatives);
    
    return [operator].concat(operands);

}

function normalize_negatives_in_factors(tree) {
    // if any factors contain a negative,
    // place negative outside factor
    //
    // run remove_duplicates_negatives before and after
    // running this function to make sure all negatives are addressed
    
    if(!Array.isArray(tree))
	return tree;

    var operator = tree[0];
    var operands = tree.slice(1);

    operands = operands.map(normalize_negatives_in_factors);

    if(operator !== '*' && operator !== '/')
	return [operator].concat(operands);

    var sign = 1;
    var operands_no_negatives=[];

    for(var i = 0; i < operands.length; i++) {
	if(operands[i][0] === '-') {
	    sign *= -1;
	    operands_no_negatives.push(operands[i][1]);
	}
	else {
	    operands_no_negatives.push(operands[i]);
	}
    }
    var result = [operator].concat(operands_no_negatives);
    if(sign == -1)
	result = ['-', result];

    return result;
}


function normalize_negatives(tree) {
    // Remove duplicate negatives and pull all negatives outside factors
    
    tree = remove_duplicate_negatives(tree);
    tree = normalize_negatives_in_factors(tree);
    tree = remove_duplicate_negatives(tree);

    return tree;
}



function sort_key(tree) {
    if (typeof tree === 'number') {
	return [0, 'number', tree];
    }
    if (typeof tree === 'string') {
	// if string is a constant, return number with value?
	return [1, 'symbol', tree];
    }
    if (typeof tree === 'boolean') {
	return [1, 'boolean', tree];
    }


    var operator = tree[0];
    var operands = tree.slice(1);

    if(operator === 'apply') {
	var key = [2, 'function', operands[0]];

	var f_args = operands[1];
	
	var n_args = 1;

	var arg_keys = [];
	
	if(Array.isArray(f_args)) {

	    f_args = f_args.slice(1);  // remove vector operator
	    
	    n_args = f_args.length;

	    arg_keys = f_args.map(sort_key);
	    
	    
	}
	else {
	    arg_keys = [sort_key(f_args)];
	}

	key.push([n_args, arg_keys]);

	return key;
    }

    var n_factors = operands.length;
    
    var factor_keys = operands.map(sort_key);


    if(operator === "*") {
	return [4, 'product', n_factors, factor_keys];
    }

    if(operator === "/") {
	return [4, 'quotient', n_factors, factor_keys];
    }

    if(operator === "+") {
	return [5, 'sum', n_factors, factor_keys];
    }

    if(operator === "-") {
	return [6, 'minus', n_factors, factor_keys];
    }

    
    return [7, operator, n_factors, factor_keys];
    
}

function compare_function(a,b) {
    var key_a = sort_key(a);
    var key_b = sort_key(b);

    if(key_a < key_b)
	return -1;
    if(key_a > key_b)
	return 1;
    return 0;
}

function default_order_ast(tree) {

    // associates tree
    tree = clean_ast(tree);
    tree = normalize_negatives(tree);

    function sort_ast(subTree) {
	if(!Array.isArray(subTree))
	    return subTree;

	var operator = subTree[0];
	var operands = subTree.slice(1);

	operands = operands.map(sort_ast);

	if(operator === "*" || operator === "+" || operator === "="
	   || operator === "and" || operator === "or" || operator === "ne"
	   || operator === "union" || operator === "intersect") {
	    
	    // sort all operands of these arguments in default order
	    // determined by compare function
	    operands.sort(compare_function);
	}
	else if(operator === ">" || operator === "ge") {
	    // turn all greater thans to less thans
	    
	    operands = operands.reverse();
	    if(operator === ">") 
		operator = "<";
	    else
		operator = "le";
	}
	else if(operator === "gts")  {
	    // turn all greater thans to less thans
	    var args = operands[0];
	    var strict = operands[1];
	    
	    if(args[0] != 'tuple' || strict[0] != 'tuple')
		// something wrong if args or strict are not tuples
		throw new Error("Badly formed ast");

	    args = ['tuple'].concat(args.slice(1).reverse());
	    strict = ['tuple'].concat(strict.slice(1).reverse());
	    
	    operator = "lts";
	    operands = [args, strict];
	    
	}
	else if(operator === 'ni' || operator === 'notni'
		|| operator === 'superset' || operator === 'notsuperset') {
	    // turn all containment operators to have larger set at right

	    operands = operands.reverse();
	    if(operator === 'ni')
		operator = 'in';
	    else if(operator === 'notni') 
		operator = 'notin';
	    else if(operator === 'superset')
		operator = 'subset';
	    else
		operator = 'notsubset';
	}

	
	return [operator].concat(operands);
    }

    return sort_ast(tree);
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

//exports._sort_key = sort_key;
//exports._compare_function = compare_function;
exports._default_order_ast = default_order_ast;

exports.default_order = function(expr) {
    return expr.context.from(default_order_ast(expr.tree));
}

exports._normalize_negatives_ast = normalize_negatives;
