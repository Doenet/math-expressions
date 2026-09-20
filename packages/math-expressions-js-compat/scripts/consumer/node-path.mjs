/**
 * Consumer smoke test #1: Node, no injection.
 *
 * Run from a scratch project that has installed the packed tarball, so every
 * specifier here resolves the way a real consumer's would — through
 * `exports`, out of `node_modules`. See `../verify-package.mjs`.
 *
 * This is the path that needs no host cooperation at all: the vendored
 * *nodejs-target* wasm under `vendor/wasm` loads itself on first use.
 */
import assert from "node:assert/strict";
import me, { dopri, isTree, setWasmModule } from "math-expressions";

assert.equal(me.fromText("x^2 + 2x + 1").toString(), "x^2 + 2 x + 1");
assert.equal(me.fromLatex("\\frac{x+1}{2}").toString(), "(x + 1)/2");
assert.equal(me.fromText("x^2").derivative("x").toString(), "2 x");
assert.equal(me.fromText("sin^2 x + cos^2 x").equals(me.fromText("1")), true);

// `fromAst` is how DoenetML builds every expression, and the round trip through
// it has to preserve the non-finite scalars the text parser cannot produce.
assert.ok(Number.isNaN(me.fromAst(["/", 0, 0]).simplify().tree));

// `f()` compiles through math.js, which is the one runtime dependency the
// bundle leaves external. If `dependencies` ever stops carrying it, this is
// where a consumer finds out.
assert.equal(me.fromText("x^2").f()({ x: 3 }), 9);

// The ODE integrator, exported by name because it replaces `me.math.dopri`.
const solution = dopri(0, 1, 1, (_x, y) => y);
assert.ok(Math.abs(solution.at(1) - Math.E) < 1e-6);
solution.free();

assert.equal(isTree(["+", 1, "x"]), true);
assert.equal(typeof setWasmModule, "function");

console.log("node path: ok");
