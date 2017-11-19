// check for equality by randomly sampling 
"use strict";

var math=require('../../mathjs');
var is_integer_ast = require('../../assumptions/element_of_sets').is_integer_ast;

function generate_random_integer(minvalue, maxvalue) {
    minvalue = math.ceil(minvalue);
    maxvalue = math.floor(maxvalue);
    return math.floor(math.random()*(maxvalue-minvalue+1)) + minvalue;
}


exports.equals = function(expr, other, randomBindings,
			  expr_context, other_context) {

    expr = expr.normalize_function_names();
    other = other.normalize_function_names();
    
    var max_value = Number.MAX_VALUE*1E-20;

    var epsilon = 1E-12;
    var minimum_matches = 10;
    
    // Get set of variables mentioned in at least one of the two expressions
    var variables = [ expr.variables(), other.variables() ];
    variables = variables.reduce( function(a,b) { return a.concat(b); } )
    variables = variables.reduce(function(p, c) {
        if (p.indexOf(c) < 0) p.push(c);
        return p;
    }, []);

    // determine if any of the variables are integers
    // consider integer if is integer in either expressions' assumptions
    var integer_variables = [];
    for(var i=0; i < variables.length; i++)
	if(is_integer_ast(variables[i], expr_context.assumptions)
	   || is_integer_ast(variables[i], other_context.assumptions))
	    integer_variables.push(variables[i]);
    

    var expr_f = expr.f();
    var other_f = other.f();
    
    // find a location where the magnitudes of both expressions
    // are below max_value;
    for(var i=0; i<100; i++) {
	
	var bindings= randomBindings(variables, 10);

	// replace any integer variables with integer
	for(var j=0; j<integer_variables.length; j++) {
	    bindings[integer_variables[j]] = generate_random_integer(-10,10);
	}

	try {
	    var expr_evaluated = expr_f(bindings);
	    var other_evaluated = other_f(bindings);
	}
	catch (e) {
	    continue;
	}
	var expr_abs = math.abs(expr_evaluated);
	var other_abs = math.abs(other_evaluated);

	if(expr_abs < max_value && other_abs < max_value) {
	    // now that found a finite point,
	    // check to see if expressions are nearly equal.

	    var min_mag = math.min(expr_abs, other_abs);
	    if(math.abs(math.subtract(expr_evaluated, other_evaluated))
	       > min_mag * epsilon)
		continue;

	    // Look for a region around point
	    var found_large_difference=false
	    var finite_tries = 0;
	    for(var j=0; j<100; j++) {
		
		var bindings2 = randomBindings(variables, 0.1, bindings);
		
		// replace any integer variables with integer
		for(var j=0; j<integer_variables.length; j++) {
		    bindings2[integer_variables[j]]
			= generate_random_integer(-10,10);
		}
		
		try {
		    expr_evaluated = expr_f(bindings2);
		    other_evaluated = other_f(bindings2);
		}
		catch (e) {
		    continue;
		}
		expr_abs = math.abs(expr_evaluated);
		other_abs = math.abs(other_evaluated);
		
		if(expr_abs < max_value && other_abs < max_value) {
		    min_mag = math.min(expr_abs, other_abs);
		    
		    finite_tries++;
		    
		    if(math.abs(math.subtract(expr_evaluated, other_evaluated))
		       > min_mag * epsilon) {
			found_large_difference = true;
			break;
		    }

		    if(finite_tries >=minimum_matches)
			break;
		}
	    }

	    if(!found_large_difference && finite_tries >= minimum_matches) {
		return true;
	    }
	}
    }

    return false;

};
