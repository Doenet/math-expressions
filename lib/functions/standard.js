// standard functions
// these functions are all defined in math.js

import { get_tree } from "../trees/util.js";

//"intersect", "cross", "det", "diag", "dot", "eye", " inv", " sort", " trace", " transpose", "max", "mean", "median", "min", "mode", "nthRoot"
// "ceil", "fix", "floor", "round"

export function abs(expr_or_tree) {
  return ["apply", "abs", get_tree(expr_or_tree)];
}
export function exp(expr_or_tree) {
  return ["apply", "exp", get_tree(expr_or_tree)];
}
export function log(expr_or_tree) {
  return ["apply", "log", get_tree(expr_or_tree)];
}
export function log10(expr_or_tree) {
  return ["apply", "log10", get_tree(expr_or_tree)];
}
export function sign(expr_or_tree) {
  return ["apply", "sign", get_tree(expr_or_tree)];
}
export function sqrt(expr_or_tree) {
  return ["apply", "sqrt", get_tree(expr_or_tree)];
}
export function conj(expr_or_tree) {
  return ["apply", "conj", get_tree(expr_or_tree)];
}
export function im(expr_or_tree) {
  return ["apply", "im", get_tree(expr_or_tree)];
}
export function re(expr_or_tree) {
  return ["apply", "re", get_tree(expr_or_tree)];
}
export function factorial(expr_or_tree) {
  return ["apply", "factorial", get_tree(expr_or_tree)];
}
export function gamma(expr_or_tree) {
  return ["apply", "gamma", get_tree(expr_or_tree)];
}
export function erf(expr_or_tree) {
  return ["apply", "erf", get_tree(expr_or_tree)];
}
export function acos(expr_or_tree) {
  return ["apply", "acos", get_tree(expr_or_tree)];
}
export function acosh(expr_or_tree) {
  return ["apply", "acosh", get_tree(expr_or_tree)];
}
export function acot(expr_or_tree) {
  return ["apply", "acot", get_tree(expr_or_tree)];
}
export function acoth(expr_or_tree) {
  return ["apply", "acoth", get_tree(expr_or_tree)];
}
export function acsc(expr_or_tree) {
  return ["apply", "acsc", get_tree(expr_or_tree)];
}
export function acsch(expr_or_tree) {
  return ["apply", "acsch", get_tree(expr_or_tree)];
}
export function asec(expr_or_tree) {
  return ["apply", "asec", get_tree(expr_or_tree)];
}
export function asech(expr_or_tree) {
  return ["apply", "asech", get_tree(expr_or_tree)];
}
export function asin(expr_or_tree) {
  return ["apply", "asin", get_tree(expr_or_tree)];
}
export function asinh(expr_or_tree) {
  return ["apply", "asinh", get_tree(expr_or_tree)];
}
export function atan(expr_or_tree) {
  return ["apply", "atan", get_tree(expr_or_tree)];
}
export function atanh(expr_or_tree) {
  return ["apply", "atanh", get_tree(expr_or_tree)];
}
export function cos(expr_or_tree) {
  return ["apply", "cos", get_tree(expr_or_tree)];
}
export function cosh(expr_or_tree) {
  return ["apply", "cosh", get_tree(expr_or_tree)];
}
export function cot(expr_or_tree) {
  return ["apply", "cot", get_tree(expr_or_tree)];
}
export function coth(expr_or_tree) {
  return ["apply", "coth", get_tree(expr_or_tree)];
}
export function csc(expr_or_tree) {
  return ["apply", "csc", get_tree(expr_or_tree)];
}
export function csch(expr_or_tree) {
  return ["apply", "csch", get_tree(expr_or_tree)];
}
export function sec(expr_or_tree) {
  return ["apply", "sec", get_tree(expr_or_tree)];
}
export function sech(expr_or_tree) {
  return ["apply", "sech", get_tree(expr_or_tree)];
}
export function sin(expr_or_tree) {
  return ["apply", "sin", get_tree(expr_or_tree)];
}
export function sinh(expr_or_tree) {
  return ["apply", "sinh", get_tree(expr_or_tree)];
}
export function tan(expr_or_tree) {
  return ["apply", "tan", get_tree(expr_or_tree)];
}
export function tanh(expr_or_tree) {
  return ["apply", "tanh", get_tree(expr_or_tree)];
}

// function of two variables
export function atan2(expr_or_tree1, expr_or_tree2) {
  return [
    "apply",
    "atan2",
    ["tuple", get_tree(expr_or_tree1), get_tree(expr_or_tree2)],
  ];
}
