
import astToLatexObj from '../converters/ast-to-latex';
import astToTextObj from '../converters/ast-to-text';
import astToGuppyObj from '../converters/ast-to-guppy';

var astToLatex = new astToLatexObj();
var astToText = new astToTextObj();
var astToGuppy = new astToGuppyObj();

export const tex = function(expr) {
    return astToLatex.convert( expr.tree );
};

export const toLatex = tex;

export const toString = function(expr) {
    return astToText.convert( expr.tree );
};

export const toXML = function(expr) {
    return astToGuppy.convert( expr.tree );
};
