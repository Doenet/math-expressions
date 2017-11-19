var get_tree = require('../trees/util').get_tree;
var clean = require('../expression/simplify').clean;

function add(expr_or_tree1, expr_or_tree2) {
    var result = ['+', get_tree(expr_or_tree1), get_tree(expr_or_tree2)];
    return clean(result);
}

function subtract(expr_or_tree1, expr_or_tree2) {
    var result = ['+', get_tree(expr_or_tree1), ['-', get_tree(expr_or_tree2)]];
    return clean(result);
}

function multiply(expr_or_tree1, expr_or_tree2) {
    var result = ['*', get_tree(expr_or_tree1), get_tree(expr_or_tree2)];
    return clean(result);
}

function divide(expr_or_tree1, expr_or_tree2) {
    var result = ['/', get_tree(tree1), get_tree(tree2)];
    return clean(result);
}

function pow(expr_or_tree1, expr_or_tree2) {
    var result = ['^', get_tree(expr_or_tree1), get_tree(expr_or_tree2)];
    return clean(result);
}

function mod(expr_or_tree1, expr_or_tree2) {
    var result = ['apply', 'mod', ['tuple', get_tree(expr_or_tree1),
				   get_tree(expr_or_tree2)]];
    return clean(result);
}

exports.add = add
exports.subtract = subtract;
exports.multiply = multiply;
exports.divide = divide;
exports.pow = pow;
exports.mod = mod;
