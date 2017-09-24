/****************************************************************/
// replace variables in an AST by another AST
function substitute_ast(tree, bindings) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	if (tree in bindings)
	    return bindings[tree];
	
	return tree;
    }    
    
    var operator = tree[0];
    var operands = tree.slice(1);
    
    var result = [operator].concat( operands.map( function(v,i) { return substitute_ast(v,bindings); } ) );
    return result;
};

function tree_match( haystack, needle ) {
    var match = {};

    if (typeof needle === 'string') {
	match[needle] = haystack;
	return match;
    }

    if (typeof haystack === 'number') {
	if (typeof needle === 'number') {
	    if (needle === haystack) {
		return {};
	    }
	}

	return null;
    }

    if (typeof haystack === 'string') {
	if (typeof needle === 'string') {
	    match[needle] = haystack;
	    return match;
	}

	return null;
    }

    var haystack_operator = haystack[0];
    var haystack_operands = haystack.slice(1);

    var needle_operator = needle[0];
    var needle_operands = needle.slice(1);

    if (haystack_operator === needle_operator) {
	if (haystack_operands.length >= needle_operands.length) {
	    var matches = {}

	    needle_operands.forEach( function(i) {
		var new_matches = tree_match( haystack_operands[i], needle_operands[i] );
		
		if (new_matches === null) {
		    matches = null;
		}

		if (matches != null) {
		    matches = $.extend( matches, new_matches );
		}
	    } );

	    if (matches != null) {
		matches = $.extend( matches, { remainder: haystack_operands.slice( needle_operands.length ) } );
	    }

	    return matches;
	}

	return null;
    }

    return null;
};

function subtree_matches(haystack, needle) {
    if (typeof haystack === 'number') {
	return (typeof needle === 'string');
    }    
    
    if (typeof haystack === 'string') {
	return (typeof needle === 'string');
    }    

    var match = tree_match( haystack, needle );
    if (match != null) {
	return true;
    }

    var operator = haystack[0];
    var operands = haystack.slice(1);

    var any_matches = false;

    $.each( operands, function(i) {
	if (subtree_matches(operands[i], needle))
	    any_matches = true;
    } );

    return any_matches;
};

function replace_subtree(haystack, needle, replacement) {
    if (typeof haystack === 'number') {
	return haystack;
    }    
    
    if (typeof haystack === 'string') {
	if (typeof needle === 'string')
	    if (needle === haystack)
		return replacement;
	
	return haystack;
    }    

    var match = tree_match( haystack, needle );
    if (match != null) {
	return substitute_ast( replacement, match ).concat( match.remainder );
    }

    var operator = haystack[0];
    var operands = haystack.slice(1);

    return [operator].concat( operands.map( function(v,i) { return replace_subtree(v, needle, replacement); } ) );
};

exports._substitute_ast = substitute_ast;
