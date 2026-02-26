import { get_tree } from "../trees/util.js";
import { clean } from "../expression/simplify.js";

function add(expr_or_tree1, expr_or_tree2) {
  var result = ["+", get_tree(expr_or_tree1), get_tree(expr_or_tree2)];
  return clean(result);
}

function subtract(expr_or_tree1, expr_or_tree2) {
  var result = ["+", get_tree(expr_or_tree1), ["-", get_tree(expr_or_tree2)]];
  return clean(result);
}

function multiply(expr_or_tree1, expr_or_tree2) {
  var result = ["*", get_tree(expr_or_tree1), get_tree(expr_or_tree2)];
  return clean(result);
}

function divide(expr_or_tree1, expr_or_tree2) {
  var result = ["/", get_tree(expr_or_tree1), get_tree(expr_or_tree2)];
  return clean(result);
}

function pow(expr_or_tree1, expr_or_tree2) {
  var result = ["^", get_tree(expr_or_tree1), get_tree(expr_or_tree2)];
  return clean(result);
}

function mod(expr_or_tree1, expr_or_tree2) {
  var result = [
    "apply",
    "mod",
    ["tuple", get_tree(expr_or_tree1), get_tree(expr_or_tree2)],
  ];
  return clean(result);
}

function copy(expr_or_tree) {
  return get_tree(expr_or_tree);
}

export { add, subtract, multiply, divide, pow, mod, copy };
