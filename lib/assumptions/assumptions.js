var simplify=require('../expression/simplify')._simplify_ast;
var default_order_ast = require('../trees/default_order').default_order_ast;
var variables_in_ast = require('../expression/variables')._variables_in_ast;
var trees = require('../trees/basic');
var associate = require('../trees/associate');
var expand_relations = require('../expression/transformation')._expand_relations_ast;


function clean_assumptions(tree) {
    
    var tree= associate.associate_all(
	expand_relations(
	    default_order_ast(
		simplify(tree)
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
    var tree_variables = variables_in_ast(tree);
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

    var a = [];
    
    // add assumptions specified by each variable
    variables.forEach(function (v) {
	// get assumption from byvar, if exists
	if(assumptions.byvar[v]) {
	    a.push(assumptions.byvar[v]);
	}
	// else get assumptions from generic
	else if(assumptions['generic'].length > 0){
	    a.push(trees.substitute(assumptions['generic'], {x: v}));
	}
    });

    if(a.length==1)
	a=a[0];
    else if(a.length > 1)
	a=['and'].concat(a);
    
    // add any assumption from combined
    var ca = get_assumptions_from_tree(assumptions.combined, variables);

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

function get_assumptions(assumptions, variables) {
    // return an ast if found assumptions involving variables
    // otherwise return undefined
    // include any additional assumptions
    // involving variables found in assumptions

    var result=[];
    
    if(!Array.isArray(variables))
	variables = [variables];

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
	    
	    new_variables = variables_in_ast(new_result).filter(
		v => !variables.includes(v));
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
    
function add_assumption(assumptions, tree) {
    // add assumption in tree to assumptions

    // if tree contains one variable, add to appropriate byvar
    // otherwise add to combined

    if(tree===undefined)
	return

    // if expression (contains a tree), convert to tree
    if(tree.tree !== undefined)
	tree = tree.tree;

    var variables = variables_in_ast(tree);

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
	else
	    assumptions['combined'] = ast;

    }
    
}

function initialize_assumptions() {
    var assumptions = {};
    assumptions['byvar'] = {};
    assumptions['combined'] = [];
    assumptions['generic'] = [];
    assumptions['not_commutative'] = [];
    assumptions['get_assumptions'] = function(v) {
	return get_assumptions(assumptions, v);
    }
    assumptions['add_assumption'] = function(v) {
	return add_assumption(assumptions, v);
    }
    
    return assumptions;
}

exports.get_assumptions = get_assumptions;
exports.initialize_assumptions = initialize_assumptions;
exports.add_assumption = add_assumption;
