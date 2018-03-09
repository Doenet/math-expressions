var parser = require('../converters/parser');

exports.tex = function(expr) {
    return parser.ast.to.latex( expr.tree );
};

exports.toLatex = exports.tex;

exports.toString = function(expr) {
    return parser.ast.to.text( expr.tree );
};

exports.toXML = function(expr) {
    return parser.ast.to.guppy( expr.tree );
};
