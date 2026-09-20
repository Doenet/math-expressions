// The compat polynomial / Gröbner API's JS side: marshalling, and nothing else.
//
// The engine is `polynomials::compat` in the Rust core. What crosses the
// boundary is the AST spelling the legacy API is written in —
// `["polynomial", v, [[deg, coeff], …]]` and `["monomial", c, [[v, deg], …]]` —
// because callers hand those in as literals and compare what comes back
// structurally. The wire is that same JSON, so nothing here has to know what a
// polynomial is; each export just names an operation and passes its arguments
// through.
//
// `undefined` comes back for an operation with no answer (`mono_div` on a
// non-divisor, `polynomial_pow` on a non-integer exponent), which is what the
// legacy engine returned too.
import wasm from "../_wasm";
import { get_tree } from "../trees/util";
import { astToJson, jsonToAst } from "../converters/ast-json";

/** Invoke one core polynomial operation on the given arguments. */
function op(name: string, ...args: any[]): any {
  const out = (wasm as any).poly_op(name, astToJson(args));
  return out === undefined ? undefined : jsonToAst(out);
}

/**
 * An expression, or a raw tree, read as a polynomial — `false` when it is not
 * one. `pi`, `e` and `i` count as numbers, so `9x^(2/3) - pi*x` has the
 * coefficient `["-", "pi"]`; anything the core cannot take apart (`sin(x)`,
 * `x^(1/2)`, `x/y`) becomes an opaque polynomial variable.
 */
export function expression_to_polynomial(expr_or_tree: any): any {
  return op("expression_to_polynomial", get_tree(expr_or_tree));
}

export function polynomial_to_expression(p: any): any {
  return op("polynomial_to_expression", p);
}

export function polynomial_add(p: any, q: any): any {
  return op("polynomial_add", p, q);
}

export function polynomial_sub(p: any, q: any): any {
  return op("polynomial_sub", p, q);
}

export function polynomial_mul(p: any, q: any): any {
  return op("polynomial_mul", p, q);
}

export function polynomial_neg(p: any): any {
  return op("polynomial_neg", p);
}

export function polynomial_pow(p: any, e: any): any {
  return op("polynomial_pow", p, e);
}

/** The leading term under the lexicographic order, as a monomial. */
export function initial_term(p: any): any {
  return op("initial_term", p);
}

export function mono_less_than(left: any, right: any): boolean {
  return op("mono_less_than", left, right);
}

export function mono_gcd(left: any, right: any): any {
  return op("mono_gcd", left, right);
}

export function mono_div(top: any, bottom: any): any {
  return op("mono_div", top, bottom);
}

export function mono_is_div(top: any, bottom: any): boolean {
  return op("mono_is_div", top, bottom);
}

export function mono_to_poly(mono: any): any {
  return op("mono_to_poly", mono);
}

/**
 * The largest term of `f` divisible by one of `monos`, with the index of the
 * divisor, as `[monomial, index]` — or `0` when no term is.
 */
export function max_div_init(f: any, monos: any): any {
  return op("max_div_init", f, monos);
}

/**
 * Division by a list: `[[[s1, m1], …], f']` with `f = m1·g_s1 + … + f'`.
 */
export function poly_div(f: any, divs: any): any {
  return op("poly_div", f, divs);
}

/** `polys[i]` reduced against every other polynomial in the list. */
export function reduce_ith(i: any, polys: any): any {
  return op("reduce_ith", i, polys);
}

/** The list reduced against itself to a fixpoint, each made monic. */
export function reduce(polys: any): any {
  return op("reduce", polys);
}

export function reduced_grobner(polys: any): any {
  return op("reduced_grobner", polys);
}

export function poly_gcd(f: any, g: any): any {
  return op("poly_gcd", f, g);
}

export function poly_lcm(f: any, g: any): any {
  return op("poly_lcm", f, g);
}

/**
 * A rational expression's numerator and denominator with their common factor
 * cancelled, and both scaled so the denominator's leading coefficient is 1.
 */
export function reduce_rational_expression(top: any, bottom: any): any {
  return op("reduce_rational_expression", top, bottom);
}

/**
 * The legacy library carried a second, flat encoding of the same polynomials
 * (`["polynomial_terms", ["monomial", …], …]`) and a duplicate of every
 * algorithm written against it, as a faster path for single-variable inputs.
 * The core's engine is sparse in both encodings' sense, so the fast path has
 * nothing left to be faster than, and the name is kept only because callers
 * use it.
 */
export const pt_reduce_rational_expression = reduce_rational_expression;
