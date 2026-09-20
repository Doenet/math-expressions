// The assumption store's JS side: the handle, and the marshalling around it.
//
// The wasm `Assumptions` handle answers *predicates* (`is_real(x+y)`) and, in
// its other half, holds the same facts as trees — filed per variable, chained
// into their consequences, and handed back with the queried variable on the
// left. All of that is `assumptions::TreeStore` in the core.
//
// What is left here is the shape of the legacy API, which is a JS shape and not
// a reasoning question: every entry point takes an Expression *or* a raw tree,
// the query takes a params object, and the answer is a tree rather than a
// handle. That is the glue the plan names as an accepted exception.

import { get_tree } from "../trees/util";
import { astToJson, jsonToAst } from "../converters/ast-json";

/** A wasm `Assumptions` handle. */
type Handle = any;

/**
 * The JSON spelling of an assumption, or undefined when there is not one.
 *
 * An empty assumption is a no-op rather than an error: the spec tables drive
 * `me.add_assumption(me.from(input))` over rows whose input is undefined,
 * meaning "no assumptions for this row".
 */
function json(expr_or_tree: any): string | undefined {
  const tree = get_tree(expr_or_tree);
  if (!Array.isArray(tree)) return undefined;
  return astToJson(tree);
}

export function add_assumption(
  handle: Handle,
  expr_or_tree: any,
  exclude_generic?: boolean,
): number {
  const tree = json(expr_or_tree);
  return tree === undefined
    ? 0
    : handle.add_ast(tree, Boolean(exclude_generic));
}

export function add_generic_assumption(
  handle: Handle,
  expr_or_tree: any,
): number {
  const tree = json(expr_or_tree);
  return tree === undefined ? 0 : handle.add_generic_ast(tree);
}

export function remove_assumption(handle: Handle, expr_or_tree: any): number {
  const tree = json(expr_or_tree);
  return tree === undefined ? 0 : handle.remove_ast(tree);
}

export function remove_generic_assumption(
  handle: Handle,
  expr_or_tree: any,
): number {
  const tree = json(expr_or_tree);
  return tree === undefined ? 0 : handle.remove_generic_ast(tree);
}

/**
 * Everything known about a variable, a list of variables (`[["a","b"]]`) or an
 * expression, as a tree stating it — undefined when nothing is known.
 *
 * The query is passed through as written: which of the three shapes it is
 * decides how it is answered, and the core decides that, since the first two
 * are not expressions and would not survive being parsed as one.
 */
export function get_assumptions(
  handle: Handle,
  variables_or_expr: any,
  params: any = {},
): any {
  const query = get_tree(variables_or_expr);
  if (query === undefined) return undefined;

  let exclude_variables = params.exclude_variables;
  if (exclude_variables === undefined) exclude_variables = [];
  else if (!Array.isArray(exclude_variables))
    exclude_variables = [exclude_variables];

  const out = handle.get_ast(
    astToJson(query),
    exclude_variables.map(String),
    Boolean(params.omit_derived),
  );
  return out === undefined ? undefined : jsonToAst(out);
}

/**
 * The inspection surface the legacy store object carried: the facts per
 * variable, the ones derived from combining them, and the generic assumption.
 * A variable recorded with no fact comes back as `null` over the wire and is
 * restored to `undefined` here.
 */
export function byvar(handle: Handle): Record<string, any> {
  return revive(jsonToAst(handle.byvar_ast()) as Record<string, any>);
}

export function derived(handle: Handle): Record<string, any> {
  return revive(jsonToAst(handle.derived_ast()) as Record<string, any>);
}

export function generic(handle: Handle): any {
  const g = jsonToAst(handle.generic_ast());
  return g === null ? undefined : g;
}

function revive(map: Record<string, any>): Record<string, any> {
  for (const k of Object.keys(map)) if (map[k] === null) map[k] = undefined;
  return map;
}
