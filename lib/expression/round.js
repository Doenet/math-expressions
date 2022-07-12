import { get_tree } from '../trees/util';
import math from '../mathjs';

function toFixed(value, n) {
  return math.format(value, {notation: "fixed", precision: n})
}

export function round_numbers_to_precision(expr_or_tree, digits=14) {
  // round any decimals to specified number of significant digits

  var tree = get_tree(expr_or_tree);

  if(digits < 1) {
    throw Error("For round_numbers_to_precision, digits must be positive");
  }

  if(!Number.isFinite(digits)) {
    throw Error("For round_numbers_to_precision, digits must be a number");
  }

  if(digits > 15) {
    return tree;
  }

  digits = Math.round(digits);

  return round_numbers_to_precision_sub(tree,digits);
}

const round_numbers_to_precision_sub = function(tree, digits=14) {

  if(typeof tree === "number") {

    if(Number.isFinite(tree) && tree !== 0) {
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


export function round_numbers_to_decimals(expr_or_tree, ndecimals=14) {
  // round any numbers to specified number of decimals

  var tree = get_tree(expr_or_tree);


  if(!Number.isFinite(ndecimals)) {
    throw Error("For round_numbers_to_decimals, ndecimals must be a number");
  }

  ndecimals = Math.round(ndecimals);

  // no need to go much beyond limits of double precision
  ndecimals = Math.max(-330, Math.min(330, ndecimals));

  return round_numbers_to_decimals_sub(tree,ndecimals);
}

const round_numbers_to_decimals_sub = function(tree, ndecimals=0) {

  if(typeof tree === "number") {
    if(ndecimals < 0) {
      // mathjs toFixed truncates zeros when n is negative
      // so add back on when creating float
      return parseFloat(toFixed(tree, ndecimals)+'0'.repeat(math.abs(ndecimals)));
    } else {
      return parseFloat(toFixed(tree, ndecimals))
    }
  }
  if(!Array.isArray(tree)) {
    return tree;
  }
  let operator = tree[0];
  let operands = tree.slice(1);
  return [operator, ...operands.map(x => round_numbers_to_decimals_sub(x,ndecimals))]
}

