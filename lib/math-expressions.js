var parser = require('./parser');
var _ = require('underscore');

function Expression (ast) {
    this.tree = ast;
    this.simplify();
}

/* Load methods from various modules */
_.extend( Expression.prototype,
	  require('./expression/printing.js' ),
	  require('./expression/differentiation.js' ),
	  require('./expression/integration.js' ),
	  require('./expression/variables.js' ),
	  require('./expression/equality.js' ),
	  require('./expression/evaluation.js' ),
	  require('./expression/simplify.js' )
	);

/****************************************************************/
/* Factory methods */

function parseText(string) {
    return new Expression( parser.text.to.ast(string) );
};

function parseLatex(string) {
    return new Expression( parser.latex.to.ast(string) );
};

exports.fromText = parseText;
exports.parse = parseText;
exports.fromLaTeX = parseLatex;
exports.fromLatex = parseLatex;
exports.fromTeX = parseLatex;
exports.fromTex = parseLatex;
exports.parse_tex = parseLatex;

exports.fromAst = function(ast) {
    return new Expression( ast );
};
