var math=require('../../mathjs');
var numerical_equals = require('./numerical').numerical_equals;


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
};

exports.equals = function(expr, other) {
    return numerical_equals(expr, other, randomComplexBindings);
}
