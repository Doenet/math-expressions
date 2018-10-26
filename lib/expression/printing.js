
import astToLatexObj from '../converters/ast-to-latex';
import astToTextObj from '../converters/ast-to-text';
import astToGuppyObj from '../converters/ast-to-guppy';
import astToGLSLObj from '../converters/ast-to-glsl';

var astToLatex = new astToLatexObj();
var astToText = new astToTextObj();
var astToGuppy = new astToGuppyObj();
var astToGLSL = new astToGLSLObj();

const tex = function(expr) {
    return astToLatex.convert( expr.tree );
};

const toLatex = tex;

const toString = function(expr) {
    return astToText.convert( expr.tree );
};

const toGLSL = function(expr) {
    return astToGLSL.convert( expr.tree );
};

const toXML = function(expr) {
    return astToGuppy.convert( expr.tree );
};

export { tex, toLatex, toString, toXML, toGLSL }
