var trees = require("../trees.js");
var textToAst = require("../parser.js").text.to.ast;

function expand_ast(tree) {
    tree = trees.deassociate(tree, "*");
    tree = trees.deassociate(tree, "+");

    transformations = [];
    transformations.push([textToAst("a*(b+c)"), textToAst("a*b+a*c")]);
    transformations.push([textToAst("(a+b)*c"), textToAst("a*c+b*c")]);

    tree = trees.applyAllTransformations(tree, transformations, 20);
    
    tree = trees.associate(tree, "*");
    tree = trees.associate(tree, "+");
    
    return tree;
}

function expand(expr) {
    return expr.context.from(expand_ast(expr.tree));
}

exports._expand_ast = expand_ast;

exports.expand = expand;
