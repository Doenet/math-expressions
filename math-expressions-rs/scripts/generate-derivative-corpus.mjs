// Derivative corpus (PORTING_PLAN.md §15 Phase 8). A seeded grammar generates
// random differentiable expressions in `x`; JS `me.fromText(input).derivative('x')`
// (mathjs-backed) is the oracle. The Rust side (tests/derivative_corpus.rs)
// checks its own `derivative` is mathematically equal (via `equals`) to JS's.
//
//   node scripts/generate-derivative-corpus.mjs [count] [seed]
//
// Infinity/NaN are JSON-encoded as {"$":"Inf"} / {"$":"-Inf"} / {"$":"NaN"}.

import { writeFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import me from "../../lib/math-expressions.js";

const here = dirname(fileURLToPath(import.meta.url));
const outPath = join(here, "../tests/fixtures/derivative-corpus.json");

const COUNT = Number(process.argv[2]) || 300;
const SEED = Number(process.argv[3]) || 0x5eed1234;

let state = SEED >>> 0;
function rng() {
  state |= 0;
  state = (state + 0x6d2b79f5) | 0;
  let t = Math.imul(state ^ (state >>> 15), 1 | state);
  t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
}
const pick = (a) => a[Math.floor(rng() * a.length)];
const chance = (p) => rng() < p;

// Everything here is differentiable by mathjs.
const FUNCS = ["sin", "cos", "tan", "exp", "log", "sqrt", "abs", "sinh", "cosh", "atan", "asin"];
function atom() {
  const r = rng();
  if (r < 0.5) return "x"; // bias toward the differentiation variable
  if (r < 0.7) return String(Math.floor(rng() * 9) + 1);
  return pick(["y", "n", "a"]);
}
function gen(depth) {
  if (depth <= 0 || chance(0.35)) return atom();
  const f = rng();
  if (f < 0.26) return `(${gen(depth - 1)} ${pick(["+", "-"])} ${gen(depth - 1)})`;
  if (f < 0.48) return `(${gen(depth - 1)} ${pick(["*", "/"])} ${gen(depth - 1)})`;
  if (f < 0.6) return `(${gen(depth - 1)})^${pick(["2", "3", "n"])}`;
  if (f < 0.72) return `-${gen(depth - 1)}`;
  return `${pick(FUNCS)}(${gen(depth - 1)})`;
}
const randExpr = () => gen(2 + Math.floor(rng() * 3));

function encodeSpecials(tree) {
  if (Array.isArray(tree)) return tree.map(encodeSpecials);
  if (typeof tree === "number") {
    if (tree === Infinity) return { $: "Inf" };
    if (tree === -Infinity) return { $: "-Inf" };
    if (Number.isNaN(tree)) return { $: "NaN" };
  }
  return tree;
}

const seen = new Set();
const cases = [];
let attempts = 0;
while (cases.length < COUNT && attempts < COUNT * 30) {
  attempts++;
  const input = randExpr();
  if (!input.includes("x") || seen.has(input)) continue;
  seen.add(input);
  try {
    const deriv = me.fromText(input).derivative("x").tree;
    cases.push({ input, deriv: encodeSpecials(deriv) });
  } catch {
    // mathjs can't differentiate this one (e.g. an unsupported form) — skip.
  }
}

writeFileSync(outPath, JSON.stringify(cases, null, 1) + "\n");
console.log(`derivative-corpus.json: ${cases.length} cases (seed ${SEED}, ${attempts} attempts)`);
