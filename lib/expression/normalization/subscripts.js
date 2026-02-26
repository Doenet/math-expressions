import { get_tree } from "../../trees/util.js";
import astToTextObj from "../../converters/ast-to-text.js";

var astToText = new astToTextObj();

export function subscripts_to_strings(expr_or_tree, force = false) {
  // convert ['_', a,b] to string
  // if force is set, perform conversions for any values of a or b
  // otherwise (the default), perform conversion only
  // when both a and b are strings or numbers

  var tree = get_tree(expr_or_tree);

  if (!Array.isArray(tree)) {
    return tree;
  }

  let operator = tree[0];
  let operands = tree.slice(1);

  if (operator === "_") {
    if (
      force ||
      operands.every((x) => ["number", "string"].includes(typeof x))
    ) {
      return astToText.convert(tree);
    }
  }

  return [operator].concat(
    operands.map((x) => subscripts_to_strings(x, force)),
  );
}

export function strings_to_subscripts(expr_or_tree) {
  // convert string 'a_b' to ['_', 'a','b'] and string 'a_1' to ['_', 'a', 1]

  var tree = get_tree(expr_or_tree);

  if (typeof tree === "string") {
    let res = tree.match(/^([0-9a-zA-Z]+)_([a-zA-Z]+|[0-9]+)$/);
    if (res) {
      let base = Number(res[1]);
      if (isNaN(base)) {
        base = res[1];
      }
      let sub = Number(res[2]);
      if (isNaN(sub)) {
        sub = res[2];
      }
      return ["_", base, sub];
    } else {
      return tree;
    }
  }

  if (!Array.isArray(tree)) {
    return tree;
  }

  let operator = tree[0];
  let operands = tree.slice(1);

  return [operator].concat(operands.map(strings_to_subscripts));
}
