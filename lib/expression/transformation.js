import * as flatten from '../trees/flatten.js';
import * as trans from '../trees/basic.js';
import { normalize_negatives } from '../trees/default_order';
import * as tuples from './normalization/tuples';
import { get_tree } from '../trees/util';
import textToAstObj from '../converters/text-to-ast';
import {collect_like_terms_factors, evaluate_numbers} from './simplify';
var textToAst = new textToAstObj();


function expand(expr_or_tree, no_division) {
  // Initial implementation of expand
  // Expands polynomials only up to degree 4

  var tree = get_tree(expr_or_tree);

  var transformations = [];
  transformations.push([textToAst.convert("a*(b+c)"), textToAst.convert("a*b+a*c")]);
  transformations.push([textToAst.convert("(a+b)*c"), textToAst.convert("a*c+b*c")]);
  if(!no_division)
    transformations.push([textToAst.convert("(a+b)/c"), textToAst.convert("a/c+b/c")]);
  transformations.push([textToAst.convert("-(a+b)"), textToAst.convert("-a-b")]);
  transformations.push([textToAst.convert("a(-b)"), textToAst.convert("-ab")]);
  transformations.push([textToAst.convert("(a+b)^2"), textToAst.convert("a^2+2ab+b^2")]);
  transformations.push([textToAst.convert("(a+b)^3"), textToAst.convert("a^3+3a^2b+3ab^2+b^3")]);
  transformations.push([textToAst.convert("(a+b)^4"), textToAst.convert("a^4+4a^3b+6a^2b^2+4ab^3+b^4")]);
  transformations.push([textToAst.convert("(-a)^2"), textToAst.convert("a^2")]);
  transformations.push([textToAst.convert("(-a)^3"), textToAst.convert("-a^3")]);
  transformations.push([textToAst.convert("(-a)^4"), textToAst.convert("a^4")]);

  tree = trans.applyAllTransformations(tree, transformations, 20);

  tree = flatten.flatten(tree);

  tree = evaluate_numbers(tree);

  tree = collect_like_terms_factors(tree);

  tree = normalize_negatives(tree);

  return tree;
}

function expand_relations(expr_or_tree) {
  var tree = get_tree(expr_or_tree);
  return trans.transform(tree, expand_relations_transform);
}

function expand_relations_transform (ast) {
  if(!Array.isArray(ast)) {
    return ast;
  }

  var operator = ast[0];
  var operands = ast.slice(1);
  // since transforms in bottom up fashion,
  // operands have already been expanded

  if(operator === '=') {
    if(operands.length <= 2)
      return ast;
    let result = ['and'];
    for(let i=0; i < operands.length-1; i++) {
      result.push(['=', operands[i], operands[i+1]]);
    }
    return result;
  }
  if(operator === 'gts' || operator === 'lts') {
    let args = operands[0]
    let strict = operands[1];

    if(args[0] !== 'tuple' || strict[0] !== 'tuple')
      // something wrong if args or strict are not tuples
      throw new Error("Badly formed ast");


    let comparisons = []
    for(let i=1; i< args.length-1; i++) {
      let new_operator;
      if(strict[i]) {
	if(operator === 'lts')
	  new_operator = '<';
	else
	  new_operator = '>';
      }
      else {
	if(operator === 'lts')
	  new_operator = 'le';
	else
	  new_operator = 'ge';
      }
      comparisons.push([new_operator, args[i], args[i+1]]);
    }

    let result = ['and', comparisons[0], comparisons[1]];
    for(let i=2; i<comparisons.length; i++)
      result = ['and', result, comparisons[i]];
    return result;
  }

  // convert interval containment to inequalities
  if(operator === 'in' || operator === 'notin' ||
     operator === 'ni' || operator === 'notni') {

    let negate=false;
    if(operator === 'notin' || operator === 'notni')
      negate=true;

    let x, interval;
    if(operator === 'in' || operator === 'notin') {
      x = operands[0];
      interval = operands[1];
    }
    else {
      x = operands[1];
      interval = operands[0];
    }

    // convert any tuples/arrays of length two to intervals
    interval = tuples.to_intervals(interval);

    // if not interval, don't transform
    if(interval[0] !== 'interval')
      return ast;

    let args = interval[1];
    let closed = interval[2];
    if(args[0] !== 'tuple' || closed[0] !== 'tuple')
      throw new Error("Badly formed ast");

    let a = args[1];
    let b = args[2];

    let comparisons = [];
    if(closed[1]) {
      if(negate)
	comparisons.push(['<', x, a]);
      else
	comparisons.push(['ge', x, a]);
    }
    else {
      if(negate)
	comparisons.push(['le', x, a]);
      else
	comparisons.push(['>', x, a]);
    }
    if(closed[2]) {
      if(negate)
	comparisons.push(['>', x, b]);
      else
	comparisons.push(['le', x, b]);
    }
    else {
      if(negate)
	comparisons.push(['ge', x, b]);
      else
	comparisons.push(['<', x, b]);
    }

    let result;
    if(negate)
      result =  ['or'].concat(comparisons);
    else
      result =  ['and'].concat(comparisons);

    return result;
  }

  // convert interval containment to inequalities
  if(operator === 'subset' || operator === 'notsubset' ||
     operator === 'superset' || operator === 'notsuperset') {

    let negate=false;
    if(operator === 'notsubset' || operator === 'notsuperset')
      negate=true;

    let small, big;
    if(operator === 'subset' || operator === 'notsubset') {
      small = operands[0];
      big = operands[1];
    }
    else {
      small = operands[1];
      big = operands[0];
    }

    // convert any tuples/arrays of length two to intervals
    small = tuples.to_intervals(small);
    big = tuples.to_intervals(big);

    // if not interval, don't transform
    if(small[0] !== 'interval' || big[0] !== 'interval')
      return ast;

    let small_args = small[1];
    let small_closed = small[2];
    let big_args = big[1];
    let big_closed = big[2];
    if(small_args[0] !== 'tuple' || small_closed[0] !== 'tuple' ||
       big_args[0] !== 'tuple' || big_closed[0] !== 'tuple')
      throw new Error("Badly formed ast");

    let small_a = small_args[1];
    let small_b = small_args[2];
    let big_a = big_args[1];
    let big_b = big_args[2];

    let comparisons = [];
    if(small_closed[1] && !big_closed[1]) {
      if(negate)
	comparisons.push(['le', small_a,big_a]);
      else
	comparisons.push(['>', small_a,big_a]);
    }
    else {
      if(negate)
	comparisons.push(['<', small_a,big_a]);
      else
	comparisons.push(['ge', small_a,big_a]);
    }
    if(small_closed[2] && !big_closed[2]) {
      if(negate)
	comparisons.push(['ge', small_b,big_b]);
      else
	comparisons.push(['<', small_b,big_b]);
    }
    else {
      if(negate)
	comparisons.push(['>',small_b,big_b]);
      else
	comparisons.push(['le',small_b,big_b]);
    }
    let result;
    if(negate)
      result =  ['or'].concat(comparisons);
    else
      result =  ['and'].concat(comparisons);

    return result;

  }

  return ast;
}

function substitute(pattern, bindings) {
  var pattern_tree = get_tree(pattern);

  var bindings_tree = {}
  for(let b in bindings) {
    bindings_tree[b] = get_tree(bindings[b]);
  }

  return trans.substitute(pattern_tree, bindings_tree);

}

function substitute_component(pattern, component, value) {
  let pattern_tree = get_tree(pattern);
  let value_tree = get_tree(value);

  if(typeof component === "number") {
    component = [component];
  }
  else if(!Array.isArray(component)) {
    throw Error("Invalid substitute_component: " + component);
  }


  let container_operators = ["list", "tuple", "vector", "array"];

  return substitute_component_sub(pattern_tree, component, value_tree);


  function substitute_component_sub(tree, component, value_tree) {

    if(component.length === 0) {
      return value;
    }
    if(!Array.isArray(tree)) {
      throw Error("Invalid substitute_component: expected list, tuple, vector, or array");
    }

    let operator = tree[0];
    let operands = tree.slice(1);

    if(!container_operators.includes(operator)) {
      throw Error("Invalid substitute_component: expected list, tuple, vector, or array");
    }

    let ind = component[0];
    if(ind < 0 || ind > operands.length-1) {
      throw Error("Invalid substitute_component: component out of range");
    }
    let new_components = component.slice(1);
    let result = substitute_component_sub(operands[ind], new_components, value_tree);

    return [operator, ...operands.slice(0,ind), result, ...operands.slice(ind+1)];
  }

}

function get_component(pattern, component) {
  let pattern_tree = get_tree(pattern);

  if(typeof component === "number") {
    component = [component];
  }
  else if(!Array.isArray(component)) {
    throw Error("Invalid get_component: " + component);
  }

  let container_operators = ["list", "tuple", "vector", "array"];

  return get_component_sub(pattern_tree, component);


  function get_component_sub(tree, component) {

    if(component.length === 0) {
      return tree;
    }

    if(!Array.isArray(tree)) {
      throw Error("Invalid get_component: expected list, tuple, vector, or array");
    }

    let operator = tree[0];
    let operands = tree.slice(1);

    if(!container_operators.includes(operator)) {
      throw Error("Invalid get_component: expected list, tuple, vector, or array");
    }

    let ind = component[0];
    if(ind < 0 || ind > operands.length-1) {
      throw Error("Invalid get_component: component out of range");
    }
    let new_components = component.slice(1);
    return get_component_sub(operands[ind], new_components);

  }

}


export { expand, expand_relations, substitute, substitute_component, get_component };
