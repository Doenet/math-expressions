var parser = require('./parser');

function Expression (ast) {
    this.tree = this._clean_ast(ast);
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

exports.from = function (expr) {
    if (Array.isArray(expr) || (typeof expr === 'number')) {
	return new Expression(expr);
    }
    else if (typeof expr === 'string') {
	try {
	    return new Expression( parser.text.to.ast(expr) );
	}
	catch(e_text) {
	    try {
		return new Expression( parser.latex.to.ast(expr) );
	    }
	    catch(e_latex){
		try {
		    return new Expression( parser.mml.to.ast(expr) );
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
    return new Expression( parser.text.to.ast(string) );
};

function parseLatex(string) {
    return new Expression( parser.latex.to.ast(string) );
};

function parseMml(string) {
    return new Expression( parser.mml.to.ast(string) );
};

exports.fromText = parseText;
exports.parse = parseText;
exports.fromLaTeX = parseLatex;
exports.fromLatex = parseLatex;
exports.fromTeX = parseLatex;
exports.fromTex = parseLatex;
exports.fromMml = parseMml;
exports.parse_tex = parseLatex;

exports.fromAst = function(ast) {
    return new Expression( ast );
};

extend( exports,
	require('./expression' )
      );


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
