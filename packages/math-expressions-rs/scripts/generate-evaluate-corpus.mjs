// Evaluate corpus (PORTING_PLAN.md §15): differential checks for `evaluate`
// (with random real bindings) and `evaluate_to_constant` against the JS
// reference. Each result is encoded as {re, im} or null. Rust must match within
// tolerance.  node scripts/generate-evaluate-corpus.mjs [count] [seed]

import { writeFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import me from "../../../lib/math-expressions.js";

const here = dirname(fileURLToPath(import.meta.url));
const outPath = join(here, "../tests/fixtures/evaluate-corpus.json");
const COUNT = Number(process.argv[2]) || 250;
const SEED = Number(process.argv[3]) || 0xea1c0de5;

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

const VARS = ["x", "y", "z"];
const FUNCS = ["sin", "cos", "exp", "log", "sqrt", "abs", "tan", "atan"];
const atom = () => (rng() < 0.55 ? pick(VARS) : String(Math.floor(rng() * 9) + 1));
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

// Encode a JS evaluate result: number → {re}, Complex → {re, im}, else null.
function encodeVal(v) {
  if (typeof v === "number" && Number.isFinite(v)) return { re: v, im: 0 };
  if (v && typeof v === "object" && v.re !== undefined) {
    if (Number.isFinite(v.re) && Number.isFinite(v.im)) return { re: v.re, im: v.im };
  }
  return null;
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
    const vars = expr.variables().filter((v) => !["pi", "e", "i"].includes(v));
    // Random real bindings in [-3, 3].
    const binds = {};
    for (const v of vars) binds[v] = Math.round((rng() * 6 - 3) * 100) / 100;
    let evaluated = null;
    try {
      evaluated = encodeVal(expr.evaluate(binds));
    } catch {
      evaluated = null;
    }
    let constant = null;
    try {
      constant = encodeVal(expr.evaluate_to_constant());
    } catch {
      constant = null;
    }
    cases.push({ input, binds, evaluated, constant });
  } catch {
    // skip inputs JS rejects
  }
}

writeFileSync(outPath, JSON.stringify(cases, null, 1) + "\n");
console.log(`evaluate-corpus.json: ${cases.length} cases (seed ${SEED}, ${attempts} attempts)`);
