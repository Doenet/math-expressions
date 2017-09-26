function associate_ast( tree, op ) {
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
    operands = operands.map( function(v,i) { 
	return associate_ast(v, op); } );

    if (operator == op) {
	var result = [];
	
	for( var i=0; i<operands.length; i++ ) {
	    if ((typeof operands[i] !== 'number') && (typeof operands[i] !== 'string') && (operands[i][0] === op)) {
		result = result.concat( operands[i].slice(1) );
	    } else {
		result.push( operands[i] );
	    }
	}

	operands = result;
    }

    return [operator].concat( operands );
}

function remove_identity( tree, op, identity ) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	return tree;
    }    

    var operator = tree[0];
    var operands = tree.slice(1);
    operands = operands.map( function(v,i) { return remove_identity(v, op, identity); } );

    if (operator == op) {
	operands = operands.filter( function (a) { return a != identity; });
	if (operands.length == 0)
	    operands = [identity];

	if (operands.length == 1)
	    return operands[0];
    }

    return [operator].concat( operands );
}

function remove_zeroes( tree ) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	return tree;
    }    

    var operator = tree[0];
    var operands = tree.slice(1);
    operands = operands.map( function(v,i) { return remove_zeroes(v); } );

    if (operator === "*") {
	for( var i=0; i<operands.length; i++ ) {
	    if (operands[i] === 0)
		return 0;
	}
    }

    return [operator].concat( operands );
}

function collapse_unary_minus( tree ) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	return tree;
    }    

    var operator = tree[0];
    var operands = tree.slice(1);
    operands = operands.map( function(v,i) { return collapse_unary_minus(v); } );

    if (operator == "~") {
	if (typeof operands[0] === 'number')
	    return -operands[0];
    }

    return [operator].concat( operands );
}

function clean_ast(tree) {
    tree = associate_ast( tree, '+' );
    tree = associate_ast( tree, '*' );
    tree = associate_ast( tree, '=' );
    tree = associate_ast( tree, 'and' );
    tree = associate_ast( tree, 'or' );
    return tree;
}
    
function simplify_ast(tree) {
    tree = clean_ast(tree);
    tree = remove_identity( tree, '*', 1 );
    tree = collapse_unary_minus( tree );
    tree = remove_zeroes( tree );
    tree = remove_identity( tree, '+', 0 );
    return tree;
}

function simplify(expr) {
    return expr.from(simplify_ast(expr.tree));
}

function clean(expr) {
    return expr.from(clean_ast(expr.tree));
}

exports._associate_ast = associate_ast;
exports._remove_identity = remove_identity;
exports._collapse_unary_minus = collapse_unary_minus;
exports._remove_identity = remove_identity;

exports._clean_ast = clean_ast;
exports._simplify_ast = simplify_ast;

exports.clean = clean;
exports.simplify = simplify;
