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


var math = require('./mathjs');
var astToMathTree = require('./ast-to-math-tree').astToMathTree

function astToFunction(tree) {
    var math_tree = astToMathTree(tree).compile();
    return function(bindings) { return math_tree.eval(bindings ); };
}

exports.astToFunction = astToFunction;
