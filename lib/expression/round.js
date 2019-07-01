import { get_tree } from '../trees/util';
import math from '../mathjs';
import {toFixed} from 'mathjs/lib/utils/number';

export function round_numbers_to_precision(expr_or_tree, digits=14) {
  // round any decimals to specified number of significant digits

  var tree = get_tree(expr_or_tree);

  if(digits < 1) {
    throw Error("For round_numbers_to_precision, digits must be positive");
  }

  return round_numbers_to_precision_sub(tree,digits);
}

const round_numbers_to_precision_sub = function(tree, digits=14) {
  if(digits > 15) {
    return tree;
  }
  if(typeof tree === "number") {
    if(Number.isFinite(tree)) {
      const scaleFactor = math.floor(math.log10(math.abs(tree)));
      const n = digits - scaleFactor - 1;
      if(n < 0) {
        // mathjs toFixed truncates zeros when n is negative
        // so add back on when creating float
        return parseFloat(toFixed(tree, n)+'0'.repeat(math.abs(n)));
      } else {
        return parseFloat(toFixed(tree, n))
      }
    }
  }
  if(!Array.isArray(tree)) {
    return tree;
  }
  let operator = tree[0];
  let operands = tree.slice(1);
  return [operator, ...operands.map(x => round_numbers_to_precision_sub(x,digits))]
}

