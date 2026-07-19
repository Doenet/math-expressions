// Ops corpus (PORTING_PLAN.md §15): differential checks for `variables` and
// `substitute` against the JS reference. For each random input we record JS's
// `.variables()` (exact array) and the tree of `.substitute({v: repl})` for a
// chosen variable `v`. Rust must match variables exactly and substitute via
// `equals`.
//   node scripts/generate-ops-corpus.mjs [count] [seed]

import { writeFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import me from "../../lib/math-expressions.js";

const here = dirname(fileURLToPath(import.meta.url));
const outPath = join(here, "../tests/fixtures/ops-corpus.json");
const COUNT = Number(process.argv[2]) || 200;
const SEED = Number(process.argv[3]) || 0x0b5e1234;

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

const VARS = ["x", "y", "z", "a", "b", "n", "pi", "e"];
const FUNCS = ["sin", "cos", "exp", "log", "sqrt", "f", "g"];
const atom = () => (rng() < 0.7 ? pick(VARS) : String(Math.floor(rng() * 9) + 1));
function gen(d) {
  if (d <= 0 || chance(0.4)) return atom();
  const r = rng();
  if (r < 0.3) return `(${gen(d - 1)} ${pick(["+", "-"])} ${gen(d - 1)})`;
  if (r < 0.55) return `(${gen(d - 1)} ${pick(["*", "/"])} ${gen(d - 1)})`;
  if (r < 0.68) return `(${gen(d - 1)})^${pick(["2", "3"])}`;
  if (r < 0.8) return `-${gen(d - 1)}`;
  return `${pick(FUNCS)}(${gen(d - 1)})`;
}
const randExpr = () => gen(2 + Math.floor(rng() * 2));

// A small replacement expression for substitute.
const REPLS = ["y+1", "2", "a*b", "z", "3*x", "cos(w)"];

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
  if (seen.has(input)) continue;
  seen.add(input);
  try {
    const expr = me.fromText(input);
    const vars = expr.variables();
    // Substitute the first variable (if any) with a random replacement.
    let sub = null;
    if (vars.length > 0) {
      const v = vars[0];
      const repl = pick(REPLS);
      const map = {};
      map[v] = me.fromText(repl);
      sub = { var: v, repl, tree: encodeSpecials(expr.substitute(map).tree) };
    }
    cases.push({ input, vars, sub });
  } catch {
    // skip inputs JS rejects
  }
}

writeFileSync(outPath, JSON.stringify(cases, null, 1) + "\n");
console.log(`ops-corpus.json: ${cases.length} cases (seed ${SEED}, ${attempts} attempts)`);
