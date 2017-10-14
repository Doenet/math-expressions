// check for equality by randomly sampling 
"use strict";

var math=require('../../mathjs');

exports.equals = function(expr, other, randomBindings) {

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


    var expr_f = expr.f();
    var other_f = other.f();
    
    // find a location where the magnitudes of both expressions
    // are below max_value;
    for(var i=0; i<100; i++) {
	
	var bindings= randomBindings(variables, 10);
	
	var expr_evaluated = expr_f(bindings);
	var other_evaluated = other_f(bindings);
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
		
		expr_evaluated = expr_f(bindings2);
		other_evaluated = other_f(bindings2);
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
