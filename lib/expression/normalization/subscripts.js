import { get_tree } from '../../trees/util';
import astToTextObj from '../../converters/ast-to-text';

var astToText = new astToTextObj();

export function subscripts_to_strings(expr_or_tree, force=false) {
  // convert ['_', a,b] to string
  // if force is set, perform conversions for any values of a or b
  // otherwise (the default), perform conversion only
  // when both a and b are strings or numbers

  var tree = get_tree(expr_or_tree);

  if(!Array.isArray(tree)) {
    return tree;
  }
  
  let operator = tree[0];
  let operands = tree.slice(1);
  
  if(operator === '_') {
    if(force || operands.every(x => ['number', 'string'].includes(typeof x))) {
      return astToText.convert(tree);
    }
  }

  return [operator].concat(operands.map(subscripts_to_strings));
}
