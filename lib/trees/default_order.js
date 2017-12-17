var flatten = require('./flatten').flatten
var get_tree = require("./util").get_tree;

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


function normalize_negatives(expr_or_tree) {
    // Remove duplicate negatives and pull all negatives outside factors
    var tree = get_tree(expr_or_tree);
    
    tree = remove_duplicate_negatives(tree);
    tree = normalize_negatives_in_factors(tree);
    tree = remove_duplicate_negatives(tree);

    return tree;
}



function compare_function(a,b, params) {

    if(params === undefined)
	params = {};
    
    function sort_key(tree) {
	if (typeof tree === 'number') {
	    if(params.ignore_negatives)
		return [0, 'number', Math.abs(tree)];
	    return [0, 'number', tree];
	}
	if (typeof tree === 'string') {
	    // if string is a constant, return number with value?
	    return [1, 'symbol', tree];
	}
	if (typeof tree === 'boolean') {
	    return [1, 'boolean', tree];
	}


	if(!Array.isArray(tree))
	    return [-1, 'unknown', tree];
	
	
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
	    if(params.ignore_negatives)
		return factor_keys[0];
	    return [6, 'minus', n_factors, factor_keys];
	}

	
	return [7, operator, n_factors, factor_keys];
	
    }

    var key_a = sort_key(a);
    var key_b = sort_key(b);

    if(key_a < key_b)
	return -1;
    if(key_a > key_b)
	return 1;
    return 0;
}



function default_order(expr_or_tree, params) {

    if(params === undefined)
	params = {};
    
    tree = get_tree(expr_or_tree);

    tree = flatten(tree);
    tree = normalize_negatives(tree);

    function sort_ast(subTree) {
	if(!Array.isArray(subTree))
	    return subTree;

	var operator = subTree[0];
	var operands = subTree.slice(1);

	operands = operands.map(sort_ast);

	// TODO: determine if commutative
	if(operator === "*" || operator === "+" || operator === "="
	   || operator === "and" || operator === "or" || operator === "ne"
	   || operator === "union" || operator === "intersect") {
	    
	    // sort all operands of these arguments in default order
	    // determined by compare function
	    operands.sort((a,b) => compare_function(a,b,params));
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
	else if(operator === '-') {
	    // when negating a product with a numerical first factor
	    // put negative sign in that first factor
	    if(operands[0][0] === '*') {
		operands[0][1] = ['-', operands[0][1]];
		return operands[0];
	    }
	}
	
	return [operator].concat(operands);
    }

    return normalize_negatives(sort_ast(tree));
}

exports.normalize_negatives = normalize_negatives;
exports.compare_function = compare_function;
exports.default_order = default_order;
