exports.associate = function( tree, op ) {

    if(!Array.isArray(tree))
	return tree;

    var operator = tree[0];
    var operands = tree.slice(1);

    operands = operands.map( function(v,i) {
	return exports.associate(v, op); } );
    
    if (operator == op) {
	var result = [];
	
	for( var i=0; i<operands.length; i++ ) {
	    if (Array.isArray(operands[i]) && (operands[i][0] === op)) {
		result = result.concat( operands[i].slice(1) );
	    } else {
		result.push( operands[i] );
	    }
	}
	
	operands = result;
    }
    
    return [operator].concat( operands );
};

exports.deassociate = function( tree, op ) {

    if(!Array.isArray(tree))
	return tree;
    
    var operator = tree[0];
    var operands = tree.slice(1);

    operands = operands.map( function(v,i) {
	return exports.deassociate(v, op); } );
    
    if (operator == op) {
	var result = [op, operands[0], undefined];
	var next = result;
	
	for( var i=1; i<operands.length - 1; i++ ) {
	    next[2] = [op, operands[i], undefined];
	    next = next[2];
	}

	next[2] = operands[operands.length - 1];
	
	return result;
    }
    
    return [operator].concat( operands );
};


exports.associate_all = function(tree) {
    tree = exports.associate( tree, '+' );
    tree = exports.associate( tree, '*' );
    tree = exports.associate( tree, 'and' );
    tree = exports.associate( tree, 'or' );
    tree = exports.associate( tree, 'union' );
    tree = exports.associate( tree, 'intersect' );
    return tree;
}

exports.deassociate_all = function(tree) {
    tree = exports.deassociate( tree, '+' );
    tree = exports.deassociate( tree, '*' );
    tree = exports.deassociate( tree, 'and' );
    tree = exports.deassociate( tree, 'or' );
    tree = exports.deassociate( tree, 'union' );
    tree = exports.deassociate( tree, 'intersect' );
    return tree;
}
