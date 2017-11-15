var parser = require('./parser');
var parser_defaults = require('./parser-defaults');
var assumptions = require('./assumptions/assumptions');
var math=require('./mathjs');

function Expression (ast, context) {
    this.tree = this._clean_ast(ast);
    this.context = context;
}

function extend(object) {
    // arguments object is NOT an array
    var args = flatten(Array.prototype.slice.call(arguments, 1));

    args.forEach( 
	function(rhs) {
            if (rhs) {
		for (var property in rhs) {
                    object[property] = rhs[property];
		}
            }
	});
    
    return object;
}

function extend_prototype(object) {
    // if property of an argument begins with _
    // then add as a property to object
    // otherwise add as a property to object prepending this as first argument
    
    // arguments object is NOT an array
    var args = flatten(Array.prototype.slice.call(arguments, 1));

    args.forEach( 
	function(rhs) {
            if (rhs) {
		for (var property in rhs) {
		    if (property[0] === "_")
			object[property] = rhs[property];
		    else
			// prepend this as first argument
			(function () {
			    var prop=property;
			    object[prop] =  function() {
				var arg2 = [this].concat(
				    Array.prototype.slice.call(arguments));
				return rhs[prop].apply(null, arg2);}
			})();
		}
            }
	});
    
    return object;
}



/****************************************************************/
/* Factory methods */

function create_from_multiple(expr) {
    var context = this;
    if (Array.isArray(expr) || (typeof expr === 'number')) {
	return new Expression(expr, context);
    }
    else if (typeof expr === 'string') {
	try {
	    return new Expression( parser.text.to.ast(expr, context), context );
	}
	catch(e_text) {
	    try {
		return new Expression( parser.latex.to.ast(expr, context), context );
	    }
	    catch(e_latex){
		try {
		    return new Expression( parser.mml.to.ast(expr, context), context );
		}
		catch(e_mml) {
		    if(expr.indexOf("\\") !== -1)
			throw(e_latex)
		    if(expr.indexOf("</") !== -1)
			throw(e_mml)
		    throw(e_text)
		}
	    }
	}
    }
}

function parseText(string) {
    var context = this;
    return new Expression( parser.text.to.ast(string, context), context);
};

function parseLatex(string) {
    var context = this;
    return new Expression( parser.latex.to.ast(string, context), context );
};

function parseMml(string) {
    var context = this;
    return new Expression( parser.mml.to.ast(string, context), context );
};


var Context = {
    assumptions: assumptions.initialize_assumptions(),
    parser_parameters: {},
    from: create_from_multiple,
    fromText: parseText,
    parse: parseText,
    fromLaTeX: parseLatex,
    fromLatex: parseLatex,
    fromTeX: parseLatex,
    fromTex: parseLatex,
    fromMml: parseMml,
    parse_tex: parseLatex,
    fromAst: function(ast) {
	return new Expression( ast, this );
    },
    set_to_default: function() {
	this.assumptions=assumptions.initialize_assumptions();
	this.parser_parameters = JSON.parse(JSON.stringify(parser_defaults));
    },
    get_assumptions: function(variables) {
	return assumptions.get_assumptions(this.assumptions, variables);
    },
    add_assumption: function(assumption) {
	return assumptions.add_assumption(this.assumptions,assumption);
    },
    add_generic_assumption: function(assumption) {
	return assumptions.add_generic_assumption(this.assumptions,assumption);
    },
    clear_assumptions: function() {
	this.assumptions = assumptions.initialize_assumptions();
    },

    math: math,
}


Context.set_to_default();


extend( Context,
	require('./expression' ),
	require('./functions' )
      );


module.exports = Context;


extend_prototype( Expression.prototype,
		  require('./expression' )
		);


// from https://stackoverflow.com/a/34757676
function flatten(ary, ret) {
    ret = ret === undefined ? [] : ret;
    for (var i = 0; i < ary.length; i++) {
        if (Array.isArray(ary[i])) {
            flatten(ary[i], ret);
        } else {
            ret.push(ary[i]);
        }
    }
    return ret;
}
