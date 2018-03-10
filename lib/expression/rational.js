"use strict";

var trees = require("../trees/basic.js");
var transformation = require("./transformation");
var textToAst = require("../converters/parser.js").text.to.ast;
var get_tree = require('../trees/util').get_tree;
var simplify = require('./simplify');
var poly = require('../polynomial/polynomial');

function common_denominator(tree, assumptions) {

    tree = simplify.simplify(tree, assumptions);
    
    var transformations = [];
    transformations.push(
	[textToAst("a/c+b/c"), textToAst("(a+b)/c"),
	 {evaluate_numbers: true,
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );
    transformations.push(
	[textToAst("a/c-b/c"), textToAst("(a-b)/c"),
	 {evaluate_numbers: true,
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );
    transformations.push(
	[textToAst("a+b/c"), textToAst("(ac+b)/c"),
	 {evaluate_numbers: true,
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );
    transformations.push(
	[textToAst("a-b/c"), textToAst("(ac-b)/c"),
	 {evaluate_numbers: true,
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );
    transformations.push(
	[textToAst("a/d+b/c"), textToAst("(ac+bd)/(cd)"),
	 {evaluate_numbers: true,
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );
    transformations.push(
	[textToAst("a/d-b/c"), textToAst("(ac-bd)/(cd)"),
	 {evaluate_numbers: true,
	  allow_extended_match: true,
	  allow_permutations: true,
	  max_group: 1,
	 }]
    );
    transformations.push([textToAst("x*(y/z)"), textToAst("(x*y)/z"),
    			  {allow_extended_match: true,
    			   allow_permutations: true,
			   max_group: 1,
    			  }]);
    transformations.push([textToAst("(x/y)/z"), textToAst("x/(y*z)"),
    			  {allow_extended_match: true,
    			   allow_permutations: true,
    			  }]);
    transformations.push([textToAst("x/(y/z)"), textToAst("xz/y"),
    			  {allow_extended_match: true,
    			   allow_permutations: true,
    			  }]);

    tree = trees.applyAllTransformations(tree, transformations, 40);

    tree = transformation.expand(tree, true);
    
    return tree;
}


function as_num_denom(expr_or_tree) {
    // Return tuple containing numerator and denominator
    // after attempting to place over common denominator
    
    var tree = get_tree(expr_or_tree);

    tree = common_denominator(tree);

    if(tree[0] == '/')
	return [tree[1], tree[2]];
    else
	return [tree, 1];
}

function reduce_rational(expr_or_tree) {
    var tree = get_tree(expr_or_tree);

    var num_denom = as_num_denom(tree);

    if(num_denom[1] == 1)
	return tree;

    var poly_num = poly.expression_to_polynomial(num_denom[0]);
    var poly_denom = poly.expression_to_polynomial(num_denom[1]);

    var reduced_polys = poly.reduce_rational_expression(poly_num, poly_denom);

    var num_new = poly.polynomial_to_expression(reduced_polys[0]);
    var denom_new = poly.polynomial_to_expression(reduced_polys[1]);
    
    if(denom_new == 1)
	return num_new;
    else
	return ['/', num_new, denom_new];

}

exports.common_demoninator = common_denominator;
exports.reduce_rational = reduce_rational;
