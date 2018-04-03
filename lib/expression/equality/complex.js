import math from '../../mathjs';
import { equals as numerical_equals } from './numerical';
import { substitute_abs } from '../normalization/standard_form';

function randomComplexBindings(variables, radius, centers) {
    var result = {};

    if(centers === undefined) {
	variables.forEach( function(v) {
	    result[v] = math.complex( math.random()*2*radius - radius,
				      math.random()*2*radius - radius );
	});
    }
    else {
	variables.forEach( function(v) {
	    result[v] = math.complex(
		centers[v].re + math.random()*2*radius - radius,
		centers[v].im + math.random()*2*radius - radius );
	});
    }
    
    return result;
}

export const equals = function(expr, other) {
    
    //expr = expr.substitute_abs();
    //other = other.substitute_abs();
    
    // don't use complex equality if not analytic expression
    // except abs is OK
    if((!expr.isAnalytic({allow_abs: true})) ||
       (!other.isAnalytic({allow_abs: true})) )
	return false;
    
    return numerical_equals(expr, other, randomComplexBindings,
			    expr.context, other.context);
};
