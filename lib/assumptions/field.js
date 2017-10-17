var variables_in_ast = require('../expression/variables')._variables_in_ast;

function is_integer(variable, assumptions) {
    // returns true if assumptions explicitly state that
    // variable is an integer
    // otherwise, return undefined

    if(!Array.isArray(assumptions))
	return undefined;
    
    var operator = assumptions[0];
    var operands = assumptions.slice(1);

    if(operator === 'in')
	if(operands[0]===variable && operands[1] === 'Z')
	    return true;
    if(operator === 'ni')
	if(operands[1]===variable && operands[0] === 'Z')
	    return true;
    
    // if isn't a simple And, just give up
    if(operator !== 'and')
	return undefined;
    
    for(var i=0; i < operands.length; i++)
	if(is_integer(variable, assumptions[i]))
	    return true;

    return undefined;
}


function is_real(variable, assumptions) {
    // returns true if assumptions explicitly state that
    // variable is an integer or real
    // or include an inequality involving variable.
    // otherwise, return undefined

    if(!Array.isArray(assumptions))
	return undefined;

    if(is_integer(variable,assumptions))
	return true;
    
    var operator = assumptions[0];
    var operands = assumptions.slice(1);

    if(operator === 'in')
	if(operands[0]===variable && operands[1] === 'R')
	    return true;
    if(operator === 'ni')
	if(operands[1]===variable && operands[0] === 'R')
	    return true;

    // if assumptions is an inequality involving variable
    // then return true
    if(operator === '>' || operator === '>='
       || operator === '<' || operator === '<=') {
	var variables_in_inequality = variables_in_ast(assumptions);
	if(variables_in_inequality.indexOf(variable) !== -1)
	    return true;
    }
    
    // if isn't a simple And, just give up
    if(operator !== 'and')
	return undefined;
    
    for(var i=0; i < operands.length; i++)
	if(is_real(variable, operands[i]))
	    return true;

    return undefined;
}

exports.is_integer = is_integer;
exports.is_real = is_real;
