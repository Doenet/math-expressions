var clean_ast = require('../expression/simplify')._clean_ast;

function add_ast(tree1, tree2) {
    var result = ['+', tree1, tree2];
    return clean_ast(result);
}

function subtract_ast(tree1, tree2) {
    var result = ['+', tree1, ['-', tree2]];
    return clean_ast(result);
}

function multiply_ast(tree1, tree2) {
    var result = ['*', tree1, tree2];
    return clean_ast(result);
}

function divide_ast(tree1, tree2) {
    var result = ['/', tree1, tree2];
    return clean_ast(result);
}

function pow_ast(tree1, tree2) {
    var result = ['^', tree1, tree2];
    return clean_ast(result);
}

function mod_ast(tree1, tree2) {
    var result = ['apply', 'mod', ['tuple', tree1, tree2]];
    return clean_ast(result);
}

exports._add_ast = add_ast;
exports._subtract_ast = subtract_ast;
exports._multiply_ast = multiply_ast;
exports._divide_ast = divide_ast;
exports._pow_ast = pow_ast;
exports._mod_ast = mod_ast;

exports.add = function (expr1, expr2) {
    return expr1.context.from(add_ast(expr1.tree, expr2.tree));
}
exports.subtract = function (expr1, expr2) {
    return expr1.context.from(subtract_ast(expr1.tree, expr2.tree));
}
exports.multiply = function (expr1, expr2) {
    return expr1.context.from(multiply_ast(expr1.tree, expr2.tree));
}
exports.divide = function (expr1, expr2) {
    return expr1.context.from(divide_ast(expr1.tree, expr2.tree));
}
exports.pow = function (expr1, expr2) {
    return expr1.context.from(pow_ast(expr1.tree, expr2.tree));
}
exports.mod = function (expr1, expr2) {
    return expr1.context.from(mod_ast(expr1.tree, expr2.tree));
}

