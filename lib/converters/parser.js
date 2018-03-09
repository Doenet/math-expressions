/*
 * compute transitive closure of the many math expression parsers
 *
 * Copyright 2014-2015 by Jim Fowler <kisonecat@gmail.com>
 *
 * This file is part of a math-expressions library
 * 
 * math-expressions is free software: you can redistribute
 * it and/or modify it under the terms of the GNU General Public
 * License as published by the Free Software Foundation, either
 * version 3 of the License, or at your option any later version.
 * 
 * math-expressions is distributed in the hope that it
 * will be useful, but WITHOUT ANY WARRANTY; without even the implied
 * warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details.
 * 
 */

var mmlToLatex = require('./mml-to-latex').mmlToLatex;
var latexToAst = require('./latex-to-ast').latexToAst;
var textToAst = require('./text-to-ast').textToAst;
var mathTreeToAst = require('./math-tree-to-ast').mathTreeToAst;
var astToText = require('./ast-to-text').astToText;
var astToLatex = require('./ast-to-latex').astToLatex;
var astToGlsl = require('./ast-to-glsl').astToGlsl;
var astToGuppy = require('./ast-to-guppy').astToGuppy;
var astToFunction = require('./ast-to-function').astToFunction;
//var astToRealFunction = require('./ast-to-real-function').astToRealFunction;
var astToFiniteField = require('./ast-to-finite-field').astToFiniteField;
var astToMathTree = require('./ast-to-math-tree').astToMathTree;

kinds = ['mml', 'text', 'latex', 'ast', 'glsl', 'function', 'mathTree'];

// define the basic converters
converters = {
    mml: {
	to: {
	    latex: mmlToLatex,
	}
    },
    latex: {
	to: { 
	    ast: latexToAst,
	}
    },
    text: {
	to: { 
	    ast: textToAst,
	}
    },
    mathTree: {
	to: {
	    ast: mathTreeToAst,
	}
    },
    ast: {
	to: {
	    text: astToText,
	    latex: astToLatex,
	    glsl: astToGlsl,
	    guppy: astToGuppy,
	    function: astToFunction,
	    //realFunction: astToRealFunction,
	    finiteField: astToFiniteField,
	    mathTree: astToMathTree,
	}
    }
};

// compute the transitive closure
var foundNew = true;

while( foundNew ) {
    foundNew = false;
    
    kinds.forEach( function(a) {
	if (a in converters) {
	    kinds.forEach( function(b) {
		if ((b in converters) && (b in converters[a].to)) {
		    kinds.forEach( function(c) {
			if ((c in converters[b].to) && (!(c in converters[a].to))) {
			    foundNew = true;
			    converters[a].to[c] = function(x) { return (converters[b].to[c])( (converters[a].to[b])(x) ); };
			}
		    });
		}
	    });
	}
    });
}

// export the converters
kinds.forEach( function(a) {
    module.exports[a] = converters[a];
});
