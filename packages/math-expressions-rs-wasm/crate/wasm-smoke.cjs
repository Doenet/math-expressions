const me = require("./math_expressions_wasm.js");
const P = (s) => me.parse_text(s);
let pass = 0, fail = 0;
function check(name, cond) { if (cond) { pass++; } else { fail++; console.log("FAIL:", name); } }

// parse + render round-trip
check("to_text sin^2 x", P("sin^2 x + cos^2 x").to_text().includes("sin"));
// equals
check("equals sin^2+cos^2 == 1", P("sin^2(x)+cos^2(x)").equals(P("1")));
check("equals x+y == y+x", P("x+y").equals(P("y+x")));
check("not equals x == y", !P("x").equals(P("y")));
// simplify
check("simplify 2x+3x = 5x", P("2*x+3*x").simplify().equals(P("5*x")));
// expand
check("expand (x+1)^2", P("(x+1)^2").expand().equals(P("x^2+2*x+1")));
// derivative
check("d/dx x^2 = 2x", P("x^2").derivative("x").equals(P("2*x")));
check("d/dx sin(x^2)", P("sin(x^2)").derivative("x").equals(P("2*x*cos(x^2)")));
// variables
check("variables", JSON.stringify(P("x^2+y*z").variables()) === '["x","y","z"]');
// evaluate_to_constant
check("evc 2+3 = 5", P("2+3").evaluate_to_constant() === 5);
check("evc sqrt(2)", Math.abs(P("sqrt(2)").evaluate_to_constant() - Math.SQRT2) < 1e-9);
check("evc x+1 = undefined", P("x+1").evaluate_to_constant() === undefined);
// latex
check("parse_latex frac", me.parse_latex("\\frac{1}{2}").equals(P("1/2")));
check("to_latex", P("1/2").to_latex().length > 0);
// new surface: builders, substitute, evaluate, reduce, sets
check("add builder", P("x").add(P("1")).equals(P("x+1")));
check("subtract builder", P("x").subtract(P("1")).equals(P("x-1")));
check("multiply/divide/pow", P("x").multiply(P("y")).divide(P("z")).pow(P("2")).equals(P("((x*y)/z)^2")));
check("substitute_var", P("x^2").substitute_var("x", P("y+1")).equals(P("(y+1)^2")));
check("evaluate bindings", P("x^2+y").evaluate(["x","y"],[3,1]) === 10);
check("evaluate unbound -> undefined", P("x^2").evaluate([],[]) === undefined);
check("evaluate_numbers", P("4+x-2").evaluate_numbers().equals(P("x+2")));
check("reduce_rational", P("(x^2-1)/(x-1)").reduce_rational().equals(P("x+1")));
check("functions", JSON.stringify(P("sin(x)+f(y)").functions()) === '["sin","f"]');
check("discrete set equals", (() => {
  const s1 = me.discrete_infinite_set(P("pi/4"), P("pi"));
  const s2 = me.discrete_infinite_set(P("pi/4, 5*pi/4"), P("2*pi"));
  return s1.equals(s2);
})());
// number normalization
check("round_to_decimals 3.14159->3.14", P("3.14159").round_numbers_to_decimals(2).equals(P("3.14")));
check("round_to_precision 1234.5->1230", P("1234.5").round_numbers_to_precision(3).equals(P("1230")));
check("constants_to_floats pi", Math.abs(P("pi").constants_to_floats().evaluate_to_constant() - Math.PI) < 1e-9);


// ---- Doenet-interop surface ----
check("from_ast", me.from_ast(JSON.stringify(["+", "x", 1])).equals(P("x+1")));
check("from_ast rejects garbage", (() => {
  try { me.from_ast(JSON.stringify([[]])); return false; } catch { return true; }
})());
check("serialized round-trip", (() => {
  const s = P("x^2+1").to_serialized();
  const parsed = JSON.parse(s);
  return parsed.objectType === "math-expression" && me.from_serialized(s).equals(P("x^2+1"));
})());
check("match_template", (() => {
  const m = me.match_template(JSON.stringify(["+", ["*", 2, "x"], 3]), JSON.stringify(["+", ["*", "a", "x"], "b"]));
  if (!m) return false;
  const b = JSON.parse(m);
  return b.a === 2 && b.b === 3;
})());
check("unflatten_left", JSON.parse(me.unflatten_left(JSON.stringify(["+", 1, 2, 3])))[1][0] === "+");
check("parse_text_with_options splitSymbols", (() => {
  const spl = me.parse_text_with_options("xy", "{}");
  const nospl = me.parse_text_with_options("xy", JSON.stringify({ splitSymbols: false }));
  return spl.equals(P("x*y")) && !nospl.equals(P("x*y"));
})());
check("round plus decimals", P("3.14159").round_numbers_to_precision_plus_decimals(4, 2).equals(P("3.142")));
// ---- numeric utilities ----
check("math_mod", me.math_mod(-7, 3) === 2);
check("gcd/lcm", me.gcd(12, 18) === 6 && me.lcm(4, 6) === 12);
check("stats", me.mean(new Float64Array([1, 2, 3])) === 2 && me.median(new Float64Array([3, 1, 2])) === 2);
check("lusolve", (() => {
  const x = me.lusolve(new Float64Array([1, 1, 1, -1]), new Float64Array([3, 1]), 2);
  return x && Math.abs(x[0] - 2) < 1e-12 && Math.abs(x[1] - 1) < 1e-12;
})());
check("eigs", (() => {
  const r = JSON.parse(me.eigs(new Float64Array([2, 1, 1, 2]), 2));
  return Math.abs(r.values[0] - 1) < 1e-9 && Math.abs(r.values[1] - 3) < 1e-9
    && r.eigenvectors.length === 2 && r.eigenvectors[0].vector.length === 2;
})());
check("evaluate_to_precision sqrt2", (() => {
  const s = P("sqrt(2)").evaluate_to_precision(30);
  return typeof s === "string" && s.startsWith("1.4142135623730950488016887242");
})());
check("evaluate_to_precision complex", (() => {
  const s = P("sqrt(-2)").evaluate_to_precision(20);
  return typeof s === "string" && s.includes(" i");
})());
check("evaluate_to_precision unknown", P("x+1").evaluate_to_precision(10) === undefined);

// ---- matrix eigen surface (MATRIX_PLAN M3-M5) ----
check("char_poly", (() => {
  const p = me.parse_latex("\\begin{bmatrix}2&1\\\\1&2\\end{bmatrix}").char_poly("x");
  return p !== undefined && p.equals(P("x^2 - 4x + 3"));
})());
check("eigenvalues rational", (() => {
  const m = me.parse_latex("\\begin{bmatrix}2&1\\\\1&2\\end{bmatrix}");
  const vals = JSON.parse(m.eigenvalues());
  return vals.length === 2 && P(vals[0].value).equals(P("1")) && P(vals[1].value).equals(P("3"));
})());
check("eigenvalues rootof", (() => {
  const m = me.parse_latex("\\begin{bmatrix}0&0&1\\\\1&0&1\\\\0&1&0\\end{bmatrix}");
  const vals = JSON.parse(m.eigenvalues());
  return vals.length === 3 && vals[0].value.includes("rootof");
})());
check("eigenvectors verify", (() => {
  const m = me.parse_latex("\\begin{bmatrix}2&1\\\\1&2\\end{bmatrix}");
  const pairs = JSON.parse(m.eigenvectors());
  return pairs.length === 2 && pairs[0].basis.length === 1
    && P(pairs[0].basis[0][0]).equals(P("1")) && P(pairs[0].basis[0][1]).equals(P("-1"));
})());
check("rootof round trip", P("rootof(t^3 - t - 1, 0)^3").equals(P("rootof(t^3 - t - 1, 0) + 1")));

// ---- integration (INTEGRATION_PLAN I1+I2 + certified quadrature) ----
check("integrate table", (() => {
  const F = P("sin(x)").integrate("x");
  return F !== undefined && F.equals(P("-cos(x)"));
})());
check("integrate rational", (() => {
  const F = P("1/(x^2+1)").integrate("x");
  if (F === undefined) return false;
  // Antiderivatives differ by constants/spellings: verify by derivative.
  return F.derivative("x").equals(P("1/(x^2+1)"));
})());
check("integrate honest failure", P("exp(x^2)").integrate("x") === undefined);
check("integrate_to_precision pi", (() => {
  const s = P("4/(1+x^2)").integrate_to_precision("x", P("0"), P("1"), 10);
  return typeof s === "string" && s.replace(/[^0-9]/g, "").startsWith("31415926");
})());

// ---- ODE solving (ODE_PLAN O1+O2) ----
check("solve_ode callback", (() => {
  const sol = me.solve_ode((t, y) => [y[0]], 0, 1, new Float64Array([1]), 1e-6, 10000);
  const v = sol.at(1)[0];
  return !sol.terminated_early() && Math.abs(v - Math.E) < 1e-4;
})());
check("solve_ode_expressions harmonic", (() => {
  const rhs = P("(v, -x)");
  const sol = me.solve_ode_expressions(rhs, "t", ["x", "v"], 0, Math.PI, new Float64Array([1, 0]), 1e-8, 10000);
  if (sol === undefined || sol.terminated_early()) return false;
  const y = sol.at(Math.PI);
  return y.length === 2 && Math.abs(y[0] + 1) < 1e-5 && Math.abs(y[1]) < 1e-5;
})());
check("solve_ode blow-up flag", (() => {
  const sol = me.solve_ode((t, y) => [y[0] * y[0]], 0, 2, new Float64Array([1]), 1e-6, 10000);
  return sol.terminated_early() && sol.last_t() < 1.01 && isFinite(sol.last_y()[0]);
})());
check("solve_ode chunk chaining", (() => {
  const f = (t, y) => [-y[0] / 2];
  const a = me.solve_ode(f, 0, 1, new Float64Array([1]), 1e-8, 10000);
  const b = me.solve_ode(f, a.last_t(), 2, a.last_y(), 1e-8, 10000);
  return Math.abs(b.last_y()[0] - Math.exp(-1)) < 1e-6;
})());

// ---- divergence classification (DIVERGENCE_PLAN) ----
check("integrate_analyzed divergent", (() => {
  const r = JSON.parse(P("1/x^2").integrate_analyzed("x", P("-1"), P("1"), 8));
  return r.status === "divergent" && Math.abs(r.singularities[0].location) < 1e-9;
})());
check("integrate_analyzed improper value", (() => {
  const r = JSON.parse(P("1/sqrt(x)").integrate_analyzed("x", P("0"), P("1"), 8));
  return r.status === "value" && Math.abs(r.value - 2) < 1e-7;
})());
check("integrate_analyzed proper value", (() => {
  const r = JSON.parse(P("sin(x)").integrate_analyzed("x", P("0"), P("1"), 10));
  return r.status === "value" && Math.abs(r.value - (1 - Math.cos(1))) < 1e-9;
})());
check("integrate_to_precision divergence reason", (() => {
  const s = P("tan(x)").integrate_to_precision("x", P("0"), P("2"), 8);
  return s === undefined; // refused (divergent), same contract as before
})());

console.log(`\n${pass} passed, ${fail} failed`);
process.exit(fail ? 1 : 0);
