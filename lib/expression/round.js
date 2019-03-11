import { get_tree } from '../trees/util';
import math from '../mathjs';


export function round_numbers_to_precision(expr_or_tree, digits=14) {
  // round any decimals to specified number of significant digits

  var tree = get_tree(expr_or_tree);

  if(digits < 1) {
    throw Error("For round_numbers_to_precision, digits must be positive");
  }

  return round_numbers_to_precision_sub(tree,digits);
}

const round_numbers_to_precision_sub = function(tree, digits=14) {
  if(typeof tree === "number") {
    if(Number.isFinite(tree)) {
      const scaleFactor = math.floor(math.log10(math.abs(tree)));
      const n = digits - scaleFactor - 1;
      if(n > 15) {
        return tree; // can't round to more than 15 digits
      }
      if(n >= 0) {
        return math.round(tree, n);
      }
      const m = math.pow(10, n);
      return math.round(tree * m) / m;
    }
  }
  if(!Array.isArray(tree)) {
    return tree;
  }
  let operator = tree[0];
  let operands = tree.slice(1);
  return [operator, ...operands.map(x => round_numbers_to_precision_sub(x,digits))]
}

