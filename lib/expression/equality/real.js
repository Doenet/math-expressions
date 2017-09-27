var math=require('../../mathjs');
var numerical_equals = require('./numerical').numerical_equals;


function randomRealBindings(variables, radius, centers) {
    var result = {};

    if(centers === undefined) {
	variables.forEach( function(v) {
	    result[v] = math.random()*2*radius - radius;
	});
    }
    else {
	variables.forEach( function(v) {
	    result[v] =centers[v] + math.random()*2*radius - radius;
	});
    }
    
    return result;
};

exports.equals = function(expr, other) {
    return numerical_equals(expr, other, randomRealBindings);
}
