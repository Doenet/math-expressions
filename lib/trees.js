var _ = require('underscore');

function deepClone(s) {
    return JSON.parse(JSON.stringify(s));
}

// MIT license'd code
// credit: http://stackoverflow.com/questions/9960908/permutations-in-javascript
function anyPermutation(permutation, callback) {
    var length = permutation.length,
	c = Array(length).fill(0),
	i = 1;
    
    if (callback(permutation))
	return true;
    
    while (i < length) {
	if (c[i] < i) {
	    var k = (i % 2) ? c[i] : 0,
		p = permutation[i];
	    permutation[i] = permutation[k];
	    permutation[k] = p;
	    ++c[i];
	    i = 1;
	    if (callback(permutation))
		return true;
	} else {
	    c[i] = 0;
	    ++i;
	}
    }

    return false;
}

exports.equal = function(left, right) {
    if ((typeof left === 'number') || (typeof right === 'number')) {
	if ((typeof right !== 'number') || (typeof right !== 'number')) {	
	    return false;
	}

	return (left === right);
    }    

    if ((typeof left === 'string') || (typeof right === 'string')) {
	if ((typeof right !== 'string') || (typeof right !== 'string')) {
	    return false;
	}

	return (left === right);
    }    
    
    var leftOperator = left[0];
    var leftOperands = left.slice(1);

    var rightOperator = right[0];
    var rightOperands = right.slice(1);    

    if (leftOperator != rightOperator)
	return false;
    var operator = leftOperator;

    if (leftOperands.length != rightOperands.length)
	return false;

    // We do permit permutations
    if ((operator === '+') || (operator === '*')) {
	return anyPermutation( leftOperands, function(permutedOperands) {
	    return (_.every( _.zip( permutedOperands, rightOperands ),
			     function(pair) {
				 return exports.equal( pair[0], pair[1] );
			     }));
	});
    }
    
    return (_.every( _.zip( leftOperands, rightOperands ),
		     function(pair) {
			 return exports.equal( pair[0], pair[1] );
		     }));
}

exports.match = function( tree, pattern ) {
    if ((typeof tree === 'number') && (typeof pattern === 'number')) {
	if (tree == pattern)
	    return {};
    }

    if (typeof pattern === 'string') {
	if (pattern.match(/^[a-zA-Z]$/)) {
	    var result = {};
	    result[pattern] = tree;
	    return result;
	}
	
	if (tree === pattern)
	    return {};
    }
    
    if ((typeof tree === 'object') && (typeof pattern === 'object')) {
	if ((tree[0] === pattern[0]) && (tree.length === pattern.length)) {
	    var matches = {};
	    
	    for( var i=1; i<tree.length; i++ ) {
		var m = exports.match( tree[i], pattern[i] );
		
		if (m) {
		    // Check consistency of bindings
		    if (_.every( _.intersection( Object.keys( m ), Object.keys( matches ) ),
				 function(k) {
				     return exports.equal( m[k], matches[k] );
				 })) {
			Object.assign( matches, m );
		    } else
			return false;			
		} else
		    return false;
	    }
	    
	    return matches;
	}
    }
    
    return false;
};

exports.associate = function( tree, op ) {
    if (typeof tree === 'number') {
	return tree;
    }

    if (typeof tree === 'string') {
	return tree;
    }    
    
    var operator = tree[0];
    var operands = tree.slice(1);

    operands = operands.map( function(v,i) {
	return exports.associate(v, op); } );
    
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
};

exports.deassociate = function( tree, op ) {
    if (typeof tree === 'number') {
	return tree;
    }

    if (typeof tree === 'string') {
	return tree;
    }    
    
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

exports.substitute = function( pattern, bindings ) {
    if (typeof pattern === 'number') {
	return pattern;
    }

    if (typeof pattern === 'string') {
	if (bindings[pattern])
	    return deepClone(bindings[pattern]);

	return pattern;
    }
    
    if (typeof pattern === 'object') {
	return [pattern[0]].concat( pattern.slice(1).map( function(p) {
	    return exports.substitute(p, bindings);
	}) );
    }

    return [];
};

exports.search = function( tree, callback, done, root ) {
    if (root === undefined)
	root = tree;

    var toFinish = 0;
    var finish = function() {
	toFinish--;

	if (toFinish <= 0) {
	    callback(tree, root);
	    done();
	}
    };
    
    if (typeof tree === 'object') {
	toFinish = tree.length - 1;
	
	for( var i=1; i<tree.length; i++ ) {
	    exports.search( tree[i], callback, finish, root );
	}
    } else {
	finish();
    }

    return;
};

exports.replace = function( root, tree, replacement ) {
    if (root === tree)
	return deepClone(replacement);

    if (typeof root === 'object') {
	var result = [];
	
	for( var i=0; i<root.length; i++ ) {
	    result.push( exports.replace( root[i], tree, replacement ) );
	}

	return result;
    } else {
	return root;
    }
};

exports.applyTransformation = function( tree, pattern, replacement, callback ) {
    var results = [];
    
    exports.search( tree,
		    function(subtree,root) {
			var m = exports.match(subtree, pattern);
			if (m) {
			    var replacedSubtree = exports.substitute( replacement, m );
			    var newRoot = exports.replace( root, subtree, replacedSubtree );
			    results.push( newRoot );
			}
		    },
		    function() {
			callback( null, results );
		    });
};


exports.patternTransformer = function( pattern, replacement ) {
    return function( tree, callback ) {
	exports.applyTransformation( tree, pattern, replacement, callback );
    };
};

exports.equalAfterTransformations = function(left, right, transformers, depth, comparer) {
    if (depth === undefined)
	depth = 5;
    
    if (comparer === undefined)
	comparer = exports.equal;

    var leftQueue = [left];
    var rightQueue = [right];

    var evolve = function(queue) {
	var toAppend = [];
	for( var item of queue ) {
	    for( var transformer of transformers ) {
		transformer(item, function(err, results ) {
		    for( var result of results ) {
			if (_.every( queue, function( other ) { return ! comparer( result, other ); }))
			    toAppend.push( result );			    
		    }
		});
	    }
	}

	toAppend.forEach( function(result) {
	    queue.push( result );
	});

	return toAppend.length > 0;
    };

    for( ; depth > 0; depth-- ) {
	var noMoreLeft = ! evolve(leftQueue);
	var noMoreRight = ! evolve(rightQueue);

	for (var a of leftQueue) {
	    for (var b of rightQueue) {
		if (comparer(a,b)) {
		    return true;
		}
	    }
	}
	
	if (noMoreLeft && noMoreRight) {
	    return false;
	}
    }

    return undefined;
};
