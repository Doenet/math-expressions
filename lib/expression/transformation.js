var assoc = require("../trees/associate.js");
var trans = require("../trees/basic.js");
var textToAst = require("../parser.js").text.to.ast;
var normalize_negatives = require("../trees/default_order").normalize_negatives;

function expand_ast(tree) {
    tree = assoc.deassociate(tree, "*");
    tree = assoc.deassociate(tree, "+");

    transformations = [];
    transformations.push([textToAst("a*(b+c)"), textToAst("a*b+a*c")]);
    transformations.push([textToAst("(a+b)*c"), textToAst("a*c+b*c")]);
    transformations.push([textToAst("-(a+b)"), textToAst("-a-b")]);

    tree = trans.applyAllTransformations(tree, transformations, 20);
    
    tree = assoc.associate(tree, "*");
    tree = assoc.associate(tree, "+");

    tree = normalize_negatives(tree);
    
    return tree;
}

function expand(expr) {
    return expr.context.from(expand_ast(expr.tree));
}


exports._expand_ast = expand_ast;

exports.expand = expand;

