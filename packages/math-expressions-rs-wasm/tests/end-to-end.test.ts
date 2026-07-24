// End-to-end test for the wasm-bindgen build. Loads the **web-target** wasm the
// same way a browser bundle does — an ESM `import` plus explicit instantiation,
// no CommonJS `require` — and exercises a representative slice of every
// subsystem, so a miscompile or a broken binding is caught immediately. This is
// the post-build verification (replacing the old `wasm-smoke.cjs`); run it with
// `npm test` after `npm run build:wasm`.
import { readFileSync } from "node:fs";
import { describe, expect, test } from "vitest";
import * as me from "../pkg/math_expressions_wasm.js";

// Instantiate synchronously from the sibling `_bg.wasm`. `initSync` is the
// browser's synchronous-instantiation entry (given the compiled bytes); reading
// them off disk is the only Node-specific step.
me.initSync({
  module: readFileSync(new URL("../pkg/math_expressions_wasm_bg.wasm", import.meta.url)),
});

const P = (s: string) => me.parse_text(s);

describe("parse / render / equals", () => {
  test("text round-trip", () => {
    expect(P("sin^2 x + cos^2 x").to_text()).toContain("sin");
  });
  test("equals: trig identity", () => {
    expect(P("sin^2(x)+cos^2(x)").equals(P("1"))).toBe(true);
  });
  test("equals: commutativity", () => {
    expect(P("x+y").equals(P("y+x"))).toBe(true);
  });
  test("not equals", () => {
    expect(P("x").equals(P("y"))).toBe(false);
  });
  test("parse_latex + to_latex", () => {
    expect(me.parse_latex("\\frac{1}{2}").equals(P("1/2"))).toBe(true);
    expect(P("1/2").to_latex().length).toBeGreaterThan(0);
  });
});

describe("algebra", () => {
  test("simplify", () => {
    expect(P("2*x+3*x").simplify().equals(P("5*x"))).toBe(true);
  });
  test("expand", () => {
    expect(P("(x+1)^2").expand().equals(P("x^2+2*x+1"))).toBe(true);
  });
  test("reduce_rational", () => {
    expect(P("(x^2-1)/(x-1)").reduce_rational().equals(P("x+1"))).toBe(true);
  });
  test("evaluate_numbers", () => {
    expect(P("4+x-2").evaluate_numbers().equals(P("x+2"))).toBe(true);
  });
});

describe("calculus", () => {
  test("derivative x^2", () => {
    expect(P("x^2").derivative("x").equals(P("2*x"))).toBe(true);
  });
  test("derivative chain rule", () => {
    expect(P("sin(x^2)").derivative("x").equals(P("2*x*cos(x^2)"))).toBe(true);
  });
  test("integrate table + rational (verify by derivative)", () => {
    expect(P("sin(x)").integrate("x")?.equals(P("-cos(x)"))).toBe(true);
    const F = P("1/(x^2+1)").integrate("x");
    expect(F).toBeDefined();
    expect(F!.derivative("x").equals(P("1/(x^2+1)"))).toBe(true);
  });
  test("integrate honest failure", () => {
    expect(P("exp(x^2)").integrate("x")).toBeUndefined();
  });
  test("integrate_to_precision pi", () => {
    const s = P("4/(1+x^2)").integrate_to_precision("x", P("0"), P("1"), 10);
    expect(typeof s).toBe("string");
    expect((s as string).replace(/[^0-9]/g, "")).toMatch(/^31415926/);
  });
});

describe("builders / substitute / evaluate", () => {
  test("arithmetic builders", () => {
    expect(P("x").add(P("1")).equals(P("x+1"))).toBe(true);
    expect(P("x").subtract(P("1")).equals(P("x-1"))).toBe(true);
    expect(P("x").multiply(P("y")).divide(P("z")).pow(P("2")).equals(P("((x*y)/z)^2"))).toBe(true);
  });
  test("substitute_var", () => {
    expect(P("x^2").substitute_var("x", P("y+1")).equals(P("(y+1)^2"))).toBe(true);
  });
  test("evaluate at bindings", () => {
    expect(P("x^2+y").evaluate(["x", "y"], new Float64Array([3, 1]))).toBe(10);
    expect(P("x^2").evaluate([], new Float64Array([]))).toBeUndefined();
  });
  test("evaluate_to_constant", () => {
    expect(P("2+3").evaluate_to_constant()).toBe(5);
    expect(P("sqrt(2)").evaluate_to_constant()).toBeCloseTo(Math.SQRT2, 9);
    expect(P("x+1").evaluate_to_constant()).toBeUndefined();
  });
  test("variables / functions", () => {
    expect(P("x^2+y*z").variables()).toEqual(["x", "y", "z"]);
    expect(P("sin(x)+f(y)").functions()).toEqual(["sin", "f"]);
  });
});

describe("number normalization", () => {
  test("round to decimals / precision / plus-decimals", () => {
    expect(P("3.14159").round_numbers_to_decimals(2).equals(P("3.14"))).toBe(true);
    expect(P("1234.5").round_numbers_to_precision(3).equals(P("1230"))).toBe(true);
    expect(P("3.14159").round_numbers_to_precision_plus_decimals(4, 2).equals(P("3.142"))).toBe(true);
  });
  test("constants_to_floats", () => {
    expect(P("pi").constants_to_floats().evaluate_to_constant()).toBeCloseTo(Math.PI, 9);
  });
});

describe("Doenet interop surface", () => {
  test("from_ast + rejects garbage", () => {
    expect(me.from_ast(JSON.stringify(["+", "x", 1])).equals(P("x+1"))).toBe(true);
    expect(() => me.from_ast(JSON.stringify([[]]))).toThrow();
  });
  test("serialized round-trip", () => {
    const s = P("x^2+1").to_serialized();
    expect(JSON.parse(s).objectType).toBe("math-expression");
    expect(me.from_serialized(s).equals(P("x^2+1"))).toBe(true);
  });
  test("match_template", () => {
    const m = me.match_template(
      JSON.stringify(["+", ["*", 2, "x"], 3]),
      JSON.stringify(["+", ["*", "a", "x"], "b"]),
    );
    expect(m).toBeDefined();
    const b = JSON.parse(m as string);
    expect(b.a).toBe(2);
    expect(b.b).toBe(3);
  });
  test("unflatten_left", () => {
    const t = JSON.parse(me.unflatten_left(JSON.stringify(["+", 1, 2, 3])) as string);
    expect(t[1][0]).toBe("+");
  });
  test("parse_text_with_options splitSymbols", () => {
    expect(me.parse_text_with_options("xy", "{}").equals(P("x*y"))).toBe(true);
    expect(
      me.parse_text_with_options("xy", JSON.stringify({ splitSymbols: false })).equals(P("x*y")),
    ).toBe(false);
  });
  test("discrete infinite set equals", () => {
    const s1 = me.discrete_infinite_set(P("pi/4"), P("pi"));
    const s2 = me.discrete_infinite_set(P("pi/4, 5*pi/4"), P("2*pi"));
    expect(s1!.equals(s2!)).toBe(true);
  });
});

describe("numeric utilities", () => {
  test("scalar helpers", () => {
    expect(me.math_mod(-7, 3)).toBe(2);
    expect(me.gcd(12, 18)).toBe(6);
    expect(me.lcm(4, 6)).toBe(12);
  });
  test("stats", () => {
    expect(me.mean(new Float64Array([1, 2, 3]))).toBe(2);
    expect(me.median(new Float64Array([3, 1, 2]))).toBe(2);
  });
  test("lusolve", () => {
    const x = me.lusolve(new Float64Array([1, 1, 1, -1]), new Float64Array([3, 1]), 2);
    expect(x).toBeDefined();
    expect(x![0]).toBeCloseTo(2, 12);
    expect(x![1]).toBeCloseTo(1, 12);
  });
  test("eigs", () => {
    const r = JSON.parse(me.eigs(new Float64Array([2, 1, 1, 2]), 2) as string);
    expect(r.values[0]).toBeCloseTo(1, 9);
    expect(r.values[1]).toBeCloseTo(3, 9);
    expect(r.eigenvectors).toHaveLength(2);
    expect(r.eigenvectors[0].vector).toHaveLength(2);
  });
});

describe("arbitrary precision", () => {
  test("real / complex / unknown", () => {
    expect(P("sqrt(2)").evaluate_to_precision(30)).toMatch(/^1\.4142135623730950488016887242/);
    expect(P("sqrt(-2)").evaluate_to_precision(20)).toContain(" i");
    expect(P("x+1").evaluate_to_precision(10)).toBeUndefined();
  });
});

describe("matrix / eigen surface", () => {
  const M = () => me.parse_latex("\\begin{bmatrix}2&1\\\\1&2\\end{bmatrix}");
  test("char_poly", () => {
    expect(M().char_poly("x")?.equals(P("x^2 - 4x + 3"))).toBe(true);
  });
  test("eigenvalues (rational)", () => {
    const vals = JSON.parse(M().eigenvalues() as string);
    expect(vals).toHaveLength(2);
    expect(P(vals[0].value).equals(P("1"))).toBe(true);
    expect(P(vals[1].value).equals(P("3"))).toBe(true);
  });
  test("eigenvalues (rootof)", () => {
    const m = me.parse_latex("\\begin{bmatrix}0&0&1\\\\1&0&1\\\\0&1&0\\end{bmatrix}");
    const vals = JSON.parse(m.eigenvalues() as string);
    expect(vals).toHaveLength(3);
    expect(vals[0].value).toContain("rootof");
  });
  test("eigenvectors (verified)", () => {
    const pairs = JSON.parse(M().eigenvectors() as string);
    expect(pairs).toHaveLength(2);
    expect(pairs[0].basis).toHaveLength(1);
    expect(P(pairs[0].basis[0][0]).equals(P("1"))).toBe(true);
    expect(P(pairs[0].basis[0][1]).equals(P("-1"))).toBe(true);
  });
  test("rootof round trip", () => {
    expect(P("rootof(t^3 - t - 1, 0)^3").equals(P("rootof(t^3 - t - 1, 0) + 1"))).toBe(true);
  });
});

describe("ODE solving", () => {
  test("callback: exponential growth", () => {
    const sol = me.solve_ode((_t: number, y: Float64Array) => [y[0]], 0, 1, new Float64Array([1]), 1e-6, 10000);
    expect(sol.terminated_early()).toBe(false);
    expect(sol.at(1)[0]).toBeCloseTo(Math.E, 4);
  });
  test("expression RHS: harmonic oscillator", () => {
    const sol = me.solve_ode_expressions(P("(v, -x)"), "t", ["x", "v"], 0, Math.PI, new Float64Array([1, 0]), 1e-8, 10000);
    expect(sol).toBeDefined();
    expect(sol!.terminated_early()).toBe(false);
    const y = sol!.at(Math.PI);
    expect(y[0]).toBeCloseTo(-1, 5);
    expect(y[1]).toBeCloseTo(0, 5);
  });
  test("blow-up flag", () => {
    const sol = me.solve_ode((_t: number, y: Float64Array) => [y[0] * y[0]], 0, 2, new Float64Array([1]), 1e-6, 10000);
    expect(sol.terminated_early()).toBe(true);
    expect(sol.last_t()).toBeLessThan(1.01);
    expect(Number.isFinite(sol.last_y()[0])).toBe(true);
  });
  test("chunk chaining", () => {
    const f = (_t: number, y: Float64Array) => [-y[0] / 2];
    const a = me.solve_ode(f, 0, 1, new Float64Array([1]), 1e-8, 10000);
    const b = me.solve_ode(f, a.last_t(), 2, a.last_y(), 1e-8, 10000);
    expect(b.last_y()[0]).toBeCloseTo(Math.exp(-1), 6);
  });
});

describe("integral divergence classification", () => {
  test("divergent", () => {
    const r = JSON.parse(P("1/x^2").integrate_analyzed("x", P("-1"), P("1"), 8));
    expect(r.status).toBe("divergent");
    expect(Math.abs(r.singularities[0].location)).toBeLessThan(1e-9);
  });
  test("improper but convergent value", () => {
    const r = JSON.parse(P("1/sqrt(x)").integrate_analyzed("x", P("0"), P("1"), 8));
    expect(r.status).toBe("value");
    expect(r.value).toBeCloseTo(2, 7);
  });
  test("proper value", () => {
    const r = JSON.parse(P("sin(x)").integrate_analyzed("x", P("0"), P("1"), 10));
    expect(r.status).toBe("value");
    expect(r.value).toBeCloseTo(1 - Math.cos(1), 9);
  });
  test("integrate_to_precision refuses a divergent integral", () => {
    expect(P("tan(x)").integrate_to_precision("x", P("0"), P("2"), 8)).toBeUndefined();
  });
});
