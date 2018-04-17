import { get_tree } from '../../trees/util';

function tuples_to_vectors(expr_or_tree) {
    // convert tuple to vectors
    // except if tuple is argument of a function, gts, lts, or interval

    var tree = get_tree(expr_or_tree);

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
	let result = ['vector'].concat( operands.map( function(v,i) { return tuples_to_vectors(v); } ) );
	return result;
    }

    if (operator === 'apply') {
	if(operands[1][0] === 'tuple') {
	    // special case for function applied to tuple.
	    // preserve tuple
	    let f = tuples_to_vectors(operands[0]);
	    let f_operands = operands[1].slice(1);
	    let f_tuple = ['tuple'].concat( f_operands.map( function(v,i) { return tuples_to_vectors(v); } ) );
	    return ['apply', f, f_tuple];
	}
	// no special case for function applied to single argument
    }
    else if(operator === 'gts' || operator === 'lts' || operator === 'interval') {
	// don't change tuples of gts, lts, or interval
	let args = operands[0]
	let booleans = operands[1];

	if(args[0] !== 'tuple' || booleans[0] !== 'tuple')
	    // something wrong if args or strict are not tuples
	    throw new Error("Badly formed ast");

	let args2= ['tuple'].concat( args.slice(1).map( function(v,i) { return tuples_to_vectors(v); } ) );

	return [operator, args2, booleans];
    }

    var result = [operator].concat( operands.map( function(v,i) { return tuples_to_vectors(v); } ) );
    return result;
}

function to_intervals(expr_or_tree) {
    // convert tuple and arrays of two arguments to intervals
    // except if tuple is argument of a function, gts, lts, or interval

    var tree = get_tree(expr_or_tree);

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

    if(operator === 'tuple' && operands.length === 2) {
	// open interval
	let result = ['tuple'].concat( operands.map( function(v,i) { return to_intervals(v); } ) );
	result = ['interval', result, ['tuple', false, false]];
	return result;
    }
    if(operator === 'array' && operands.length === 2) {
	// closed interval
	let result = ['tuple'].concat( operands.map( function(v,i) { return to_intervals(v); } ) );
	result = ['interval', result, ['tuple', true, true]];
	return result;
    }

    if (operator === 'apply') {
	if(operands[1][0] === 'tuple') {
	    // special case for function applied to tuple.
	    // preserve tuple
	    let f = to_intervals(operands[0]);
	    let f_operands = operands[1].slice(1);
	    let f_tuple = ['tuple'].concat( f_operands.map( function(v,i) { return to_intervals(v); } ) );
	    return ['apply', f, f_tuple];
	}
	// no special case for function applied to single argument
    }
    else if(operator === 'gts' || operator === 'lts' || operator === 'interval') {
	// don't change tuples of gts, lts, or interval
	let args = operands[0]
	let booleans = operands[1];

	if(args[0] !== 'tuple' || booleans[0] !== 'tuple')
	    // something wrong if args or strict are not tuples
	    throw new Error("Badly formed ast");

	let args2= ['tuple'].concat( args.slice(1).map( function(v,i) { return to_intervals(v); } ) );

	return [operator, args2, booleans];
    }

    var result = [operator].concat( operands.map( function(v,i) { return to_intervals(v); } ) );
    return result;
}

export { tuples_to_vectors, to_intervals };
