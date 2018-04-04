
import astToLatexObj from '../converters/ast-to-latex';
import astToTextObj from '../converters/ast-to-text';
import astToGuppyObj from '../converters/ast-to-guppy';

var astToLatex = new astToLatexObj();
var astToText = new astToTextObj();
var astToGuppy = new astToGuppyObj();

const tex = function(expr) {
    return astToLatex.convert( expr.tree );
};

const toLatex = tex;

const toString = function(expr) {
    return astToText.convert( expr.tree );
};

const toXML = function(expr) {
    return astToGuppy.convert( expr.tree );
};

export default { tex, toLatex, toString, toXML }
