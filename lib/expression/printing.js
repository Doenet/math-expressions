var parser = require('../parser');

exports.tex = function() {
    return parser.ast.to.latex( this.tree );
};

exports.toLatex = exports.tex;

exports.toString = function() {
    return parser.ast.to.text( this.tree );
};

exports.toXML = function() {
    return parser.ast.to.guppy( this.tree );
};
