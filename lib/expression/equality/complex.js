var math=require('../../mathjs');

function randomBindings(variables) {
    var result = {};
    
    variables.forEach( function(v) {
	result[v] = math.random() * 20.0 - 10.0;
    });

    return result;
};

function randomComplexBindings(variables) {
    var result = {};
    
    variables.forEach( function(v) {
	result[v] = math.complex( math.random() * 20.0 - 10.0,  math.random() * 20.0 - 10.0 );
    });

    return result;
};

function randomComplexBindingsBall(variables,real,imag) {
    var result = {};
    
    variables.forEach( function(v) {
	result[v] = math.complex( real+math.random()-.5, imag +math.random()-.5);
    });

    return result;
};

function randomIntegerBindings(variables) {
    var result = {};
    variables.forEach( function(v) {
        result[v]=math.floor(math.random()*30);
    });
    return result;
};

exports.equals = function(expr, other) {
    var finite_tries = 0;
    var epsilon = 0.001;
    var sum_of_differences = 0;
    var sum = 0;
    
    // Get set of variables mentioned in at least one of the two expressions
    var variables = [ expr.variables(), other.variables() ];
    variables = variables.reduce( function(a,b) { return a.concat(b); } )
    variables = variables.reduce(function(p, c) {
        if (p.indexOf(c) < 0) p.push(c);
        return p;
    }, []);
    
    for (var i=0;i<variables.length;i++)
    { 
        if (variables[i]=='n') 
        {
            for (var i=1;i<11;i++)
            {
                var bindings = randomIntegerBindings(variables);
	        var expr_evaluated = expr.evaluate(bindings);
	        var other_evaluated = other.evaluate(bindings);
	        if (isFinite(math.re(expr_evaluated)) && isFinite(math.re(other_evaluated)) &&
		    isFinite(math.im(expr_evaluated)) && isFinite(math.im(other_evaluated)))
                {
		    finite_tries++;
                    sum_of_differences = sum_of_differences + math.abs(math.subtract(expr_evaluated,other_evaluated));
		    sum = sum + math.abs(other_evaluated);
                    
                } 
            }
            if (finite_tries<1)
            {return false}
	    
	    
	    if (sum_of_differences < epsilon*sum+(epsilon*epsilon))
            {return true;}
            else
            {return false;} 
        } 
    }
    
    //end integer case      
    
    //converts a variable name to a small offset, for use in the complex case above, via ascii code.
    
    function varToOffset(s){
	return (s.charCodeAt(0)-100)*0.3;
    }
    
    //begin complex case 
    var points=[]
    
    for( var i=-10; i < 11; i=i+2)
    {
        for (var j=-10; j<11; j=j+2)
        {
            var bindings = {};
            variables.forEach( function(v) {
	        bindings[v] = math.complex(i + varToOffset(v),j+varToOffset(v));
	    });
	    var expr_evaluated = expr.evaluate(bindings);
	    var other_evaluated = other.evaluate(bindings);
	    if (isFinite(math.re(expr_evaluated)) && isFinite(math.re(other_evaluated)) &&
		isFinite(math.im(expr_evaluated)) && isFinite(math.im(other_evaluated))) 
            {
                var difference=math.abs(math.subtract(expr_evaluated,other_evaluated));
		if(isFinite(difference)) {
		    finite_tries++;
                    sum_of_differences = sum_of_differences + difference ;
		    sum = sum + math.abs(other_evaluated);
                    if (difference<.00001 && points.length<3)
                    {points.push([i,j]);}
		}
	    }
        }
        
    }
    //console.log('first grid check');
    //console.log(bindings);
    //console.log(sum_of_differences)
    //console.log(points)
    if (finite_tries<1)
    {return false}
    if (sum_of_differences < epsilon*sum+(epsilon*epsilon))
    {return true;}
    else
    {
        //console.log('bad branch case');
        for (i=0;i<points.length;i++)
        {
            var ballsum=0;
            var sum=0;
            for (j=0;j<20;j++)
            {
                var bindings= randomComplexBindingsBall(variables,points[i][0],points[i][1]);
                var expr_evaluated = expr.evaluate(bindings);
	        var other_evaluated = other.evaluate(bindings);
                sum=sum+math.abs(math.subtract(expr_evaluated,other_evaluated));
            }
            //console.log(sum);
            if (sum<.0001)
            {return true}
            
        }
        return false;
    }  
    
};
