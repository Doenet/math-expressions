var _ = require('underscore');
var variables_in_ast = require('../expression/variables')._variables_in_ast
var default_order_ast = require('./default_order').default_order_ast;

var anyPermutation = require('./permutation').anyPermutation

function deepClone(s) {
    return JSON.parse(JSON.stringify(s));
}

exports.equal = function(left, right) {
    /*
     * Return true if left and right are syntactically equal.
     * 
     * Sorts operands of many operators to a default order before comparing.
     */

    // sort to default order
    left = default_order_ast(left);
    right = default_order_ast(right);
    
    if(!(Array.isArray(left) && Array.isArray(right))) {
	if((typeof left) !== (typeof right))
	    return false;

	return (left===right);
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
    // if ((operator === '+') || (operator === '*')) {
    // 	return anyPermutation( leftOperands, function(permutedOperands) {
    // 	    return (_.every( _.zip( permutedOperands, rightOperands ),
    // 			     function(pair) {
    // 				 return exports.equal( pair[0], pair[1] );
    // 			     }));
    // 	});
    // }

    return (_.every( _.zip( leftOperands, rightOperands ),
		     function(pair) {
			 return exports.equal( pair[0], pair[1] );
		     }));
}

exports.match = function( tree, pattern, variables, params) {

    /*
     * Attempt to match the entire tree to given pattern
     *
     * Returns
     * - object describing the bindings of pattern if the entire tree
     *   was matched with those bindings
     * - false if a match was not found
     *
     *
     * In a pattern:
     * - operators much match exactly
     * - strings that are designed as variables
     *   must be bound to a subtree
     * - numbers and other strings much exactly match
     *
     * variables, if defined, specifies which strings in pattern are
     * wildcards that can be matched to any subtree
     * If defined, variables must be an object with 
     *   key: string from pattern which is a wildcard
     *   values: must be one of the following
     *      - true: any subtree matches the wildcard
     *      - a regular expression: subtree must match regular expression
     *           (a non-string subtree is first passed to JSON.stringify)
     *      - a function: takes a tree as an argument and 
     *           returns whether or not that tree is a valid match
     *
     * If variables is not defined, then all variables from pattern
     * will be wildcards that match any subtree
     *
     * If defined, params is an object with keys
     *   - allow_permutations: if true, check all permutations of operators
     *   - allow_implicit_identities: an array of variables from pattern
     *       that can implicitly match the identity of their enclosing
     *       operator
     */

    if(params === undefined)
	params = {};
	
    if(variables === undefined) {
	variables = {};
	var vip = variables_in_ast(pattern);
	for(var i=0; i < vip.length;i++ ) {
	    variables[vip[i]] = true;
	}
    }

    if(pattern in variables) {
	
	// check if tree satisfies any conditions for pattern
	var condition = variables[pattern];
	if(condition !== true) {
	    if(condition instanceof RegExp) {
		if(typeof tree === 'string') {
		    if(!tree.match(condition))
			return false;
		}
		else {
		    if(!JSON.stringify(tree).match(condition))
			return false;
		}
	    }
	    else if(typeof variables[pattern] === 'function') {
		if(!variables[pattern](tree))
		    return false;
	    }
	    else {
		return false;
	    }
	}
	
	// record the whole tree as the match to pattern
	var result = {};
	result[pattern] = tree;
	return result;
    }

    if(params.allow_permutations) {
	// even though order doesn't matter with permutations
	// normalize to default order as it orients operators
	// such as inequalities and containments to a direction
	// that won't be affected by permutations
	tree = default_order_ast(tree);
	pattern = default_order_ast(pattern);
    }
    
    // if pattern isn't an array, the tree must be the pattern to match
    // (As there are no variables, there is no binding)
    if(!Array.isArray(pattern)) {
	if (tree === pattern)
	    return {};
	else
	    return false;
    }


    // Since pattern is an array, there is no match if tree isn't an array
    // of the same length with the same operator
    // (unless some pattern variables can be implicitly set to identities)
    if (!Array.isArray(tree) || (tree[0] !== pattern[0])
	|| (tree.length !== pattern.length)) {
	
	if(!Array.isArray(params.allow_implicit_identities))
	    return false;
	else
	    return matchImplicitIdentity(tree, pattern, variables, params);
	
    }

    var operator=pattern[0];
    var patternOperands = pattern.slice(1);
    var treeOperands = tree.slice(1);

    if(!params.allow_permutations ||
       !(operator === "*" || operator === "+" || operator === "="
	 || operator === "and" || operator === "or" || operator === "ne"
	 || operator === "union" || operator === "intersect")) {

	// case with no permutations
	return matchOperands(treeOperands, patternOperands, variables, params);
    }
	
    // case where we allow permutations
    return anyPermutation( patternOperands, function(permutedOperands) {
	return matchOperands( treeOperands, permutedOperands,
			      variables, params);
    });
			   
};


function matchOperands(treeOperands, patternOperands, variables, params) {
    
    var matches = {};

    // treeOperands will match patternOperands only if
    // - every tree operand can be matched by the
    //   corresponding pattern operand, and
    // - all the resulting bindings are consistent,
    //   meaning they assigned the same match to any
    //   repeated placeholder in pattern
    
    for( var i=0; i<treeOperands.length; i++ ) {
	var m = exports.match( treeOperands[i], patternOperands[i],
			       variables, params );
	
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


function matchImplicitIdentity(tree, pattern, variables, params) {
    
    var operator = pattern[0];
    var patternOperands = pattern.slice(1);
    
    // for now, implement implicit identities just
    // for addition and multiplication
    if(!(operator === '+' || operator === '*'))
	return false;

    // find any pattern operand that is allowed to be an implicit identity
    var implicit_identity = null;
    for(var i=0; i < patternOperands.length; i++) {
	var po = patternOperands[i];
	if(typeof po === 'string' &&
	   params.allow_implicit_identities.indexOf(po) != -1) {
	    implicit_identity = po;
	    break;
	}
    }
    
    if(implicit_identity === null)
	return false;

    var matches = {};

    // match implicit_identity to the identity of the operator
    if(operator === '+') 
	matches[implicit_identity] = 0;
    else
	matches[implicit_identity] = 1;

    // remove matched variable from pattern
    var matched_ind = patternOperands.indexOf(implicit_identity);
    patternOperands.splice(matched_ind,1);
    
    if(patternOperands.length == 1) {
	pattern = patternOperands[0];
    }
    else {
	pattern = [operator].concat(patternOperands);
    }

    var m = exports.match( tree, pattern, variables, params);

    if (m) {
	// Check consistency of bindings
	if(implicit_identity in m) {
	    if(!exports.equal(m[implicit_identity],
			      matches[implicit_identity]))
		return false;
	}
	Object.assign( matches, m);
    } else
	return false;

    return matches;
}


exports.substitute = function( pattern, bindings ) {
    if (typeof pattern === 'number') {
	return pattern;
    }

    if (typeof pattern === 'string') {
	if (bindings[pattern])
	    return deepClone(bindings[pattern]);

	return pattern;
    }
    
    if (Array.isArray(pattern)) {
	return [pattern[0]].concat( pattern.slice(1).map( function(p) {
	    return exports.substitute(p, bindings);
	}) );
    }

    return [];
};

exports.traverse = function( tree, callback,  root ) {
    /*
     * Traverse the tree and call the function callback
     * in a bottom-up fashion (calling at children before parents)
     */
    
    if (root === undefined)
	root = tree;

    if (Array.isArray(tree)) {
	for( var i=1; i<tree.length; i++ ) {
	    exports.traverse( tree[i], callback, root );
	}
    }
    callback(tree, root);

};

exports.transform = function( tree, F ) {
    /*
     * Transform the tree function F in a bottom-up fashion 
     * (calling F at children before parents)
     * 
     * F must be be a function that returns a tree
     */

    if (Array.isArray(tree)) {
	var new_tree = [tree[0]];
	for( var i=1; i<tree.length; i++ ) {
	    new_tree.push(exports.transform( tree[i], F ));
	}
	return F(new_tree);
    }
    
    return F(tree);
};


exports.replaceSubtree = function( root, tree, replacement ) {
    /*
     * Replaces subtree "tree" from "root" with "replacement"
     * and returns resulting tree
     *
     * The subtree tree must be an actual subtree of root
     * to be replaced.  Another tree with the exact same values
     * will not be replaced
     */
    
    if (root === tree)
	return deepClone(replacement);

    if (Array.isArray(root)) {
	var result = [];
	
	for( var i=0; i<root.length; i++ ) {
	    result.push( exports.replaceSubtree( root[i], tree, replacement ) );
	}

	return result;
    } else {
	return root;
    }
};

exports.applyAllTransformations = function( tree, transformations, depth ) {
    /*
     * Repeatedly apply all transformations from transformations
     * to tree and return the resulting tree.
     *
     * transformations must be an array of arrays of the form
     * [pattern, replacement]
     */

    if (depth === undefined)
	depth = 5;

    var new_tree = tree;
    var old_tree;
    for( ; depth > 0; depth-- ) {
	old_tree = new_tree;
	for(var i=0; i<transformations.length; i++) {
	    var pattern = transformations[i][0];
	    var replacement = transformations[i][1];
	    
	    new_tree = exports.transform(
		new_tree,
		function(subtree) {
		    var m = exports.match(subtree, pattern);
		    if (m) {
			return exports.substitute( replacement, m );
		    }
		    else {
			return subtree;
		    }
		}
	    );

	}

	if(exports.equal(old_tree, new_tree)) {
	    return new_tree;
	}
    }

    return new_tree;
};


exports.applyTransformationEachSubtree = function( tree, pattern, replacement ) {
    /*
     * Attempt to replace pattern with replacement on each subtree of tree.
     *
     * Return an array of trees, each of which had one subtree replaced.
     */
    
    var results = [];
    
    exports.traverse(
	tree,
	function(subtree,root) {
	    var m = exports.match(subtree, pattern);
	    if (m) {
		var replacedSubtree = exports.substitute( replacement, m );
		var newRoot = exports.replaceSubtree( root, subtree, replacedSubtree );
		results.push( newRoot );
	    }
	}
    );
    return results;
};


exports.patternTransformer = function( pattern, replacement ) {
    return function( tree ) {
	return exports.applyTransformationEachSubtree( tree, pattern, replacement );
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
		var results = transformer(item);
		for( var result of results ) {
		    if (_.every( queue, function( other ) { return ! comparer( result, other ); }))
			toAppend.push( result );
		}
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
