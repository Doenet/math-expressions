function leaves( tree ) {
    if (typeof tree === 'number') {
	return [tree];
    }

    if (typeof tree === 'string') {
	return [tree];
    }    

    if (typeof tree === 'boolean') {
	return [tree];
    }    

    var operator = tree[0];
    var operands = tree.slice(1);

    if(operator === "apply") {
	operands = tree.slice(2);
    }

    return operands.map( function(v,i) { return leaves(v); } )
	.reduce( function(a,b) { return a.concat(b); } );

}

function variables_in_ast( tree ) {
    var result = leaves( tree );

    result = result.filter( function(v,i) {
	return (typeof v === 'string') && (v != "e") && (v != "pi");
    });

    result = result.filter(function(itm,i,a){
	return i==result.indexOf(itm);
    });
    
    return result;
}

function variables(expr) {
    return variables_in_ast( expr.tree );
}

function operators_list( tree ) {
    if (typeof tree === 'number') {
	return [];
    }

    if (typeof tree === 'string') {
	return [];
    }    

    if (typeof tree === 'boolean') {
	return [];
    }    

    var operator = tree[0];
    var operands = tree.slice(1);

    if(operator === "apply") {
	operands = tree.slice(2);
    }

    return [operator].concat(
	operands.map( function(v,i) { return operators_list(v); } )
	    .reduce( function(a,b) { return a.concat(b); } ));

}

function operators_in_ast( tree ) {
    var result = operators_list( tree );

    result = result.filter( function(v,i) {
	return (v !== 'apply');
    });

    result = result.filter(function(itm,i,a){
	return i==result.indexOf(itm);
    });
    
    return result;
}

function operators(expr) {
    return operators_in_ast( expr.tree );
}

function functions_list( tree ) {
    if (typeof tree === 'number') {
	return [];
    }

    if (typeof tree === 'string') {
	return [];
    }    

    if (typeof tree === 'boolean') {
	return [];
    }    

    var operator = tree[0];
    var operands = tree.slice(1);

    var functions = [];
    if(operator === "apply") {
	functions = [operands[0]];
	operands = tree.slice(2);
    }

    return functions.concat(
	operands.map( function(v,i) { return functions_list(v); } )
	    .reduce( function(a,b) { return a.concat(b); } ));

}

function functions_in_ast( tree ) {
    var result = functions_list( tree );

    result = result.filter(function(itm,i,a){
	return i==result.indexOf(itm);
    });
    
    return result;
}

function functions(expr) {
    return functions_in_ast( expr.tree );
}

exports._variables_in_ast = variables_in_ast;
exports.variables = variables;

exports._operators_in_ast = operators_in_ast;
exports.operators = operators;

exports._functions_in_ast = functions_in_ast;
exports.functions = functions;
