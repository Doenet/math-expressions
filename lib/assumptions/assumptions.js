var simplify=require('../expression/simplify');
var default_order = require('../trees/default_order').default_order;
var variables_in = require('../expression/variables').variables;
var trees = require('../trees/basic');
var flatten = require('../trees/flatten').flatten;
var expand_relations = require('../expression/transformation').expand_relations;
var get_tree = require('../trees/util').get_tree;

function clean_assumptions(tree) {

    var tree= flatten(
	expand_relations(
	    default_order(
		simplify.simplify_logical(tree)
	    )
	)
    );
    
    // check for duplicates
    var operator=tree[0];
    var operands=tree.slice(1);

    if(operator === 'and') {
	// remove duplicates, using trees.equal
	operands = operands.reduce(function (a,b) {
	    if(a.every(function(v) { return !trees.equal(v,b)}))
		a.push(b);
	    return a;
	},[]);

	if(operands.length==1)
	    tree = operands[0];
	else
	    tree = [operator].concat(operands);
    }

    return tree;
}

function get_assumptions_from_tree(tree, variables) {
    // return an ast if found in tree assumptions involving variables
    // otherwise return undefined
    
    if(!Array.isArray(tree) || tree.length== 0) {
	return undefined;
    }
    
    if(!Array.isArray(variables))
	variables = [variables];

    var operator = tree[0];
    var operands = tree.slice(1);
    
    if(operator === 'and') {
	
	var a= operands.map(function (v) {
	    return get_assumptions_from_tree(v, variables);
	});

	a=a.filter(v => v !== undefined);

	if(a.length==0)
	    return undefined;
	else if(a.length==1)
	    return a[0];
	else
	    return ['and'].concat(a);
    }

    // if any intersection between variables and variables in tree
    // return tree
    var tree_variables = variables_in(tree);
    var intersection = variables.filter(
	(v) => tree_variables.includes(v));
    if(intersection.length > 0)
	return tree;
    else
	return undefined;
}


function get_assumptions_sub(assumptions, variables) {
    // return an ast if found assumptions involving variables
    // otherwise return undefined
    
    if(!Array.isArray(variables))
	variables = [variables];

    // get any assumption from combined
    var ca = get_assumptions_from_tree(assumptions.combined, variables);

    var a = [];
    
    // add assumptions specified by each variable
    variables.forEach(function (v) {
	// get assumption from byvar, if exists
	if(assumptions.byvar[v]) {
	    if(assumptions.byvar[v].length > 0)
		a.push(assumptions.byvar[v]);
	}
	// if byvar was undefined,
	// then get assumptions from generic if v isn't in combined assumptions
	else if(assumptions['generic'].length > 0) {
	    if(ca === undefined || !variables_in(ca).includes(v)){
		// if generic contains any variables other than x
		// don't substitute those back into generic
		if(v == 'x' ||
		   !variables_in(assumptions['generic']).includes(v))
		    a.push(trees.substitute(assumptions['generic'], {x: v}));
	    }
	}
    });

    if(a.length==1)
	a=a[0];
    else if(a.length > 1)
	a=['and'].concat(a);
    
    if(a.length > 0) {
	if(ca !== undefined)
	    return clean_assumptions(['and',a,ca]);
	else
	    return clean_assumptions(a);
    }
    else {
	if(ca !== undefined)
	    return clean_assumptions(ca);
	else
	    return undefined
    }

}

function get_assumptions(assumptions, variables, known_variables) {
    // return an ast if found assumptions involving variables
    // otherwise return undefined
    // include any additional assumptions
    // involving new variables found in assumptions
    // unless those new variablers are in known_variables

    var result=[];
    
    if(!Array.isArray(variables))
	variables = [variables];

    if(known_variables === undefined)
	known_variables = [];
    else if(!Array.isArray(known_variables))
	known_variables = [known_variables];

    var new_variables = variables;
    variables = [];
    
    while(new_variables.length > 0) {

	var new_result = get_assumptions_sub(assumptions, new_variables);

	variables = variables.concat(new_variables);
	
	if(new_result !== undefined) {
	    if(result.length > 0)
		result = clean_assumptions(['and', result, new_result]);
	    else
		result = new_result;
	    
	    new_variables = variables_in(new_result).filter(
		v => !variables.includes(v) && !known_variables.includes(v));
	}
	else {
	    new_variables = [];
	}
    }
    
    if(result.length > 0)
	return result;
    else
	return undefined;

}
    
function add_assumption(assumptions, expr_or_tree, exclude_generic) {
    // add assumption in tree to assumptions
    // if !exclude_generic, then add any generic assumptions to
    // variables if they don't have previous assumptions
    // return 1 if added assumption or 0 otherwise

    var tree = get_tree(expr_or_tree);

    if(!Array.isArray(tree))
	return 0;
    
    tree = clean_assumptions(simplify.simplify(tree));
    
    // if tree is an 'and', call once for each operand
    // so that assumptions involving one variable can be separated
    if(tree[0] === 'and') {
	var results = tree.slice(1).map(
	    v => add_assumption(assumptions, v, exclude_generic));
	return results.reduce(function (a,b) { return a > b ? a: b;});
    }
    
    // if tree contains one variable, add to appropriate byvar
    // otherwise add to combined

    var variables = variables_in(tree);

    if(!exclude_generic && assumptions['generic'].length > 0) {
	// check to see if any assumptions already for each variable
	// if not, start by assigning generic assumptions
	variables.forEach(function (v) {
	    if(!get_assumptions_from_tree(assumptions.combined, v)
	       && assumptions['byvar'][v] === undefined) {

		// no previous assumptions, so
		// include add assumption for v corresponding to generic
		// unless non-x v is explicitly in generic
		if(v == 'x' ||
		   !variables_in(assumptions['generic']).includes(v)) {
		    add_assumption(
			assumptions, 
			trees.substitute(assumptions['generic'], {x: v}),
			true);
		    
		}
		
	    }
	});
	
    }
	
    var ast = null;
    if(variables.length == 1) {
	ast = assumptions['byvar'][variables[0]];
	if(ast === undefined)
	    ast = [];
    }
    else if(variables.length > 1) {
	ast = assumptions['combined'];
    }

    if(ast) {
	if(ast.length == 0) {
	    ast = tree;
	}
	else {
	    ast = ['and', ast, tree];
	}

	ast = clean_assumptions(ast);

	if(variables.length == 1)
	    assumptions['byvar'][variables[0]] = ast;
	else {
	    assumptions['combined'] = ast;

	    // if any 'byvar' are empty, set to []
	    // so that generic won't appear if all assumptions deleted
	    variables.forEach(function (v) {
		if(!assumptions['byvar'][v])
		    assumptions['byvar'][v] = [];
	    });
	}
	
	return 1;
    }

    return 0;
}


function add_generic_assumption(assumptions, expr_or_tree) {
    // add assumption in expr_or_tree to generic assumptions

    // tree must contain the variable x
    // the variable x represents any variable for which
    // assumptions aren't specifically assigned

    // return 1 if added assumption or 0 otherwise

    var tree = get_tree(expr_or_tree);

    if(!Array.isArray(tree))
	return 0;
    
    tree = clean_assumptions(simplify.simplify(tree));

    var variables = variables_in(tree);

    if(!variables.includes('x'))
	return 0;

    ast = assumptions['generic'];
    
    if(ast.length == 0) {
	ast = tree;
    }
    else {
	ast = ['and', ast, tree];
    }
    
    ast = clean_assumptions(ast);

    assumptions['generic'] = ast;
    
    return 1;
}


function remove_assumption(assumptions, expr_or_tree) {

    var tree=get_tree(expr_or_tree);

    if(!Array.isArray(tree))
	return 0;
    
    tree = clean_assumptions(simplify.simplify(tree));
    
    // if tree is an 'and', call once for each operand
    // so that assumptions involving one variable can be separated
    if(tree[0] === 'and') {
	var results = tree.slice(1).map(v => remove_assumption(assumptions, v));
	return results.reduce(function (a,b) { return a > b ? a: b;});
    }
    
    
    // if tree contains one variable, attempt to find appropriate byvar
    // otherwise find from combined

    var variables = variables_in(tree);
    
    var current = null;
    if(variables.length == 1) {
	current = assumptions['byvar'][variables[0]];
    }
    else if(variables.length > 1) {
	current = assumptions['combined'];
    }

    // didn't find any assumptions to remove
    if(!current || current.length==0) {
	return 0;
    }
    
    // remove any occurence of tree from current
    var operator=current[0];
    var operands=current.slice(1);

    var n_op = operands.length;

    var result;
    
    if(operator === 'and') {
	// remove any match, using trees.equal
	operands = operands.filter(v => !trees.equal(v, tree));

	if(operands.length == 1) {
	    result = operands[0];
	}
	else if(operands.length < n_op) {
	    result = [operator].concat(operands);
	}
	else {
	    // didn't find anything to remove
	    return 0;
	}
    }
    else {
	if(trees.equal(current, tree)) {
	    result = [];
	}
	else {
	    // didn't find anything to remove
	    return 0;
	}
    }
    
    if(variables.length == 1) {
	assumptions['byvar'][variables[0]] = result;
    }
    else {
	assumptions['combined'] = result;
    }

    return 1;

}

function remove_generic_assumption(assumptions, expr_or_tree) {
    // remove assumption in expr_or_tree from generic assumptions

    // return 1 if removed assumption or 0 otherwise

    var current = assumptions['generic'];

    if(current.length == 0)
	return 0;
    
    var tree=get_tree(expr_or_tree);

    if(!Array.isArray(tree))
	return 0;
    
    tree = clean_assumptions(simplify.simplify(tree));
    
    // if tree is an 'and', call once for each operand
    // so that assumptions involving one variable can be separated
    if(tree[0] === 'and') {
	var results = tree.slice(1).map(v => remove_generic_assumption(
	    assumptions, v));
	return results.reduce(function (a,b) { return a > b ? a: b;});
    }
    
    
    // remove any occurence of tree from current
    var operator=current[0];
    var operands=current.slice(1);

    var n_op = operands.length;

    var result;
    
    if(operator === 'and') {
	// remove any match, using trees.equal
	operands = operands.filter(v => !trees.equal(v, tree));

	if(operands.length == 1) {
	    result = operands[0];
	}
	else if(operands.length < n_op) {
	    result = [operator].concat(operands);
	}
	else {
	    // didn't find anything to remove
	    return 0;
	}
    }
    else {
	if(trees.equal(current, tree)) {
	    result = [];
	}
	else {
	    // didn't find anything to remove
	    return 0;
	}
    }

    assumptions['generic'] = result;
    
    return 1;
}


function initialize_assumptions() {
    var assumptions = {};
    assumptions['byvar'] = {};
    assumptions['combined'] = [];
    assumptions['generic'] = [];
    assumptions['not_commutative'] = [];
    assumptions['get_assumptions'] = function(v, known_variables) {
	return get_assumptions(assumptions, v, known_variables);
    }
    assumptions['add_assumption'] = function(v, exclude_generic) {
	return add_assumption(assumptions, v, exclude_generic);
    }
    assumptions['add_generic_assumption'] = function(v) {
	return add_generic_assumption(assumptions, v);
    }
    assumptions['remove_assumption'] = function(v) {
	return remove_assumption(assumptions, v);
    }
    assumptions['remove_generic_assumption'] = function(v) {
	return remove_generic_assumption(assumptions, v);
    }
    
    return assumptions;
}

exports.get_assumptions = get_assumptions;
exports.initialize_assumptions = initialize_assumptions;
exports.add_assumption = add_assumption;
exports.add_generic_assumption = add_generic_assumption;
exports.remove_assumption = remove_assumption;
exports.remove_generic_assumption = remove_generic_assumption;
