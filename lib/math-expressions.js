import assumptions from './assumptions/assumptions';
import math from 'mathjs';
import flatten from './trees/flatten';
import { expression_to_tree } from './expression';
import { expression_to_other } from './expression';
import { expression_to_tree as functions_to_tree } from './functions';
import textToAstObj from './converters/text-to-ast';

var textToAst = new textToAstObj();



function Expression (ast, context) {
    this.tree = flatten.flatten(ast);
    this.context = context;
}

function extend(object, tree_to_expression) {
    // if tree_to_expression, convert ast to expression

    // arguments object is NOT an array
    var args = flatten_array(Array.prototype.slice.call(arguments, 2));

    args.forEach(
	function(rhs) {
            if (rhs) {
		for (var property in rhs) {
		    if(tree_to_expression) {
			(function () {
			    var prop=property;
			    object[prop] =  function() {
				return this.fromAst(
				    rhs[prop].apply(null, arguments));
			    }
			})();
		    }
		    else
			object[property] = rhs[property];
		}
            }
	});

    return object;
}

function extend_prototype(object, tree_to_expression) {
    // append a properties to object prepending this as first argument
    // if tree_to_expression, convert ast to expression

    // arguments object is NOT an array
    var args = flatten_array(Array.prototype.slice.call(arguments, 2));

    args.forEach(
	function(rhs) {
            if (rhs) {
		for (var property in rhs) {
		    // prepend this as first argument
		    (function () {
			var prop=property;
			object[prop] =  function() {
			    var arg2 = [this].concat(
				Array.prototype.slice.call(arguments));
			    // convert to expression if output_expression
			    if(tree_to_expression)
				return this.context.fromAst(
				    rhs[prop].apply(null, arg2));
			    else
				return rhs[prop].apply(null, arg2);
			}
		    })();
		}
            }
	});

    return object;
}



/****************************************************************/
/* Factory methods */

// function create_from_multiple(expr, pars) {
//     var context = this;
//     if (Array.isArray(expr) || (typeof expr === 'number')) {
// 	return new Expression(expr, context);
//     }
//     else if (typeof expr === 'string') {
// 	try {
// 	    return new Expression( textToAst.convert(expr));
// 	}
// 	catch(e_text) {
// 	    try {
// 		 return new Expression( parser.latex.to.ast(expr, context, pars), context );
// 	    }
// 	    catch(e_latex){
// 		try {
// 		    return new Expression( parser.mml.to.ast(expr, context, pars), context );
// 		}
// 		catch(e_mml) {
// 		    if(expr.indexOf("\\") !== -1)
// 			throw(e_latex)
// 		    if(expr.indexOf("</") !== -1)
// 			throw(e_mml)
// 		    throw(e_text)
// 		}
// 	    }
// 	}
//     }
// }

function parseText(string, pars) {
    var context = this;
    return new Expression( textToAst.convert(string));
}
//
// function parseLatex(string, pars) {
//     var context = this;
//     return new Expression( parser.latex.to.ast(string, context, pars), context);
// }
//
// function parseMml(string, pars) {
//     var context = this;
//     return new Expression( parser.mml.to.ast(string, context, pars), context);
// }


var Context = {
    assumptions: assumptions.initialize_assumptions(),
    parser_parameters: {},
    // from: create_from_multiple,
    fromText: parseText,
    // parse: parseText,
    // fromLaTeX: parseLatex,
    // fromLatex: parseLatex,
    // fromTeX: parseLatex,
    // fromTex: parseLatex,
    // fromMml: parseMml,
    // parse_tex: parseLatex,
    fromAst: function(ast) {
	return new Expression( ast, this );
    },
    set_to_default: function() {
	this.assumptions=assumptions.initialize_assumptions();
	// this.parser_parameters = JSON.parse(JSON.stringify(parser_defaults));
    },
    get_assumptions: function(variables_or_expr, params) {
	return assumptions.get_assumptions(this.assumptions, variables_or_expr,
					   params);
    },
    add_assumption: function(assumption, exclude_generic) {
	return assumptions.add_assumption(
	    this.assumptions, assumption, exclude_generic);
    },
    add_generic_assumption: function(assumption) {
	return assumptions.add_generic_assumption(this.assumptions,assumption);
    },
    remove_assumption: function(assumption) {
	return assumptions.remove_assumption(this.assumptions,assumption);
    },
    remove_generic_assumption: function(assumption) {
	return assumptions.remove_generic_assumption(this.assumptions,
						     assumption);
    },
    clear_assumptions: function() {
	this.assumptions = assumptions.initialize_assumptions();
    },

    math: math,
}


Context.set_to_default();


extend( Context, true,
	expression_to_tree,
	functions_to_tree,
      );
extend( Context, false,
	expression_to_other,
      );


export default Context;

extend_prototype( Expression.prototype, true,
		  expression_to_tree
		);
extend_prototype( Expression.prototype, false,
		  expression_to_other
		);


// from https://stackoverflow.com/a/34757676
function flatten_array(ary, ret) {
    ret = ret === undefined ? [] : ret;
    for (var i = 0; i < ary.length; i++) {
        if (Array.isArray(ary[i])) {
            flatten_array(ary[i], ret);
        } else {
            ret.push(ary[i]);
        }
    }
    return ret;
}
