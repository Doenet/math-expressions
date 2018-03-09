/*
 * convert AST to a real-valued javascript function
 *
 * Copyright 2014-2017 by
 * Jim Fowler <kisonecat@gmail.com>
 * Duane Nykamp <nykamp@umn.edu>
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


var math = require('../mathjs');
var astToMathTree = require('./ast-to-math-tree').astToMathTree

function astToFunction(tree) {
    // returns a mathjs compiled function
    // that can be called with argument of the variable bindings
    // in the format of an object, e.g., f({'x': 2})
    // All variable bindings must be numbers and all symbols in function
    // must have bindings
    
    var math_tree = astToMathTree(tree);

    math_tree=factorial_to_gamma_function(math_tree);

    var compiled_function = math_tree.compile();

    return compiled_function.eval;
}

function factorial_to_gamma_function(math_tree) {
    // convert factorial to gamma function
    // so that can evaluate at complex numbers
    var transformed = math_tree.transform(function (node, path, parent) {
    	if(node.isOperatorNode && node.op === "!" && node.fn == "factorial") {
    	    var args = [new math.expression.node.OperatorNode(
    		'+', 'add', [node.args[0],
    			     new math.expression.node.ConstantNode(1)])];
    	    return new math.expression.node.FunctionNode(
    		new math.expression.node.SymbolNode("gamma"),args);
    	}
    	else {
    	    return node;
    	}
    });
    return transformed;
}

exports.astToFunction = astToFunction;
