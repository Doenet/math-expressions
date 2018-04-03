import { get_tree } from '../trees/util';
import { clean } from '../expression/simplify';

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
    var result = ['/', get_tree(expr_or_tree1), get_tree(expr_or_tree2)];
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

export { add, subtract, multiply, divide, pow, mod };
