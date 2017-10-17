function tuples_to_vectors(tree) {
    // convert tuple to vectors
    // except if tuple is argument of a function, gts, lts, or interval
    
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

    if(operator === 'tuple') {
	var result = ['vector'].concat( operands.map( function(v,i) { return tuples_to_vectors(v); } ) );
	return result;
    }
    
    if (operator === 'apply') {
	if(operands[1][0] === 'tuple') {
	    // special case for function applied to tuple.
	    // preserve tuple
	    var f = tuples_to_vectors(operands[0]);
	    var f_operands = operands[1].slice(1);
	    var f_tuple = ['tuple'].concat( f_operands.map( function(v,i) { return tuples_to_vectors(v); } ) );
	    return ['apply', f, f_tuple];
	}
	// no special case for function applied to single argument
    }
    else if(operator === 'gts' || operator === 'lts' || operator === 'interval') {
	// don't change tuples of gts, lts, or interval
	var args = operands[0]
	var booleans = operands[1];

	if(args[0] != 'tuple' || booleans[0] != 'tuple')
	    // something wrong if args or strict are not tuples
	    throw new Error("Badly formed ast");

	var args2= ['tuple'].concat( args.slice(1).map( function(v,i) { return tuples_to_vectors(v); } ) );

	return [operator, args2, booleans];
    }

    var result = [operator].concat( operands.map( function(v,i) { return tuples_to_vectors(v); } ) );
    return result;
}

function to_intervals(tree) {
    // convert tuple and arrays of two arguments to intervals
    // except if tuple is argument of a function, gts, lts, or interval
    
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

    if(operator === 'tuple' && operands.length==2) {
	// open interval
	var result = ['tuple'].concat( operands.map( function(v,i) { return to_intervals(v); } ) );
	result = ['interval', result, ['tuple', false, false]];
	return result;
    }
    if(operator === 'array' && operands.length==2) {
	// closed interval
	var result = ['tuple'].concat( operands.map( function(v,i) { return to_intervals(v); } ) );
	result = ['interval', result, ['tuple', true, true]];
	return result;
    }
    
    if (operator === 'apply') {
	if(operands[1][0] === 'tuple') {
	    // special case for function applied to tuple.
	    // preserve tuple
	    var f = to_intervals(operands[0]);
	    var f_operands = operands[1].slice(1);
	    var f_tuple = ['tuple'].concat( f_operands.map( function(v,i) { return to_intervals(v); } ) );
	    return ['apply', f, f_tuple];
	}
	// no special case for function applied to single argument
    }
    else if(operator === 'gts' || operator === 'lts' || operator === 'interval') {
	// don't change tuples of gts, lts, or interval
	var args = operands[0]
	var booleans = operands[1];

	if(args[0] != 'tuple' || booleans[0] != 'tuple')
	    // something wrong if args or strict are not tuples
	    throw new Error("Badly formed ast");

	var args2= ['tuple'].concat( args.slice(1).map( function(v,i) { return to_intervals(v); } ) );

	return [operator, args2, booleans];
    }

    var result = [operator].concat( operands.map( function(v,i) { return to_intervals(v); } ) );
    return result;
}

exports._tuples_to_vectors_ast = tuples_to_vectors;
exports._to_intervals_ast = to_intervals;

exports.tuples_to_vectors = function(expr) {
    return expr.context.from(tuples_to_vectors(expr.tree));
};

exports.to_intervals = function(expr) {
    return expr.context.from(to_intervals(expr.tree));
};
