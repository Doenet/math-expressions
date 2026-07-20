// Expand corpus (PORTING_PLAN.md §15). A seeded grammar biased toward products
// and integer powers of sums; JS `me.fromText(input).expand()` (mathjs-backed)
// is the oracle. Rust's `expand` must be mathematically equal (via `equals`).
//   node scripts/generate-expand-corpus.mjs [count] [seed]
//
// Watchdog: mathjs `expand` HANGS on some inputs (e.g. a product of cubes over a
// constant sum). Each expansion runs in a killed-on-timeout subprocess; a
// hanging input is dropped from the corpus and reported.

import { writeFileSync, readFileSync, writeSync } from "fs";
import { spawnSync } from "child_process";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import me from "../../../lib/math-expressions.js";

const here = dirname(fileURLToPath(import.meta.url));

function encodeSpecials(tree) {
  if (Array.isArray(tree)) return tree.map(encodeSpecials);
  if (typeof tree === "number") {
    if (tree === Infinity) return { $: "Inf" };
    if (tree === -Infinity) return { $: "-Inf" };
    if (Number.isNaN(tree)) return { $: "NaN" };
  }
  return tree;
}

// Worker mode: read inputs on stdin, stream {input, expanded} per line (flushed
// synchronously so a subsequent hang can't swallow completed results).
if (process.argv[2] === "--worker") {
  const inputs = JSON.parse(readFileSync(0, "utf8"));
  for (const input of inputs) {
    let expanded = null;
    try {
      expanded = encodeSpecials(me.fromText(input).expand().tree);
    } catch {
      expanded = undefined; // JS rejects → skip sentinel
    }
    writeSync(1, JSON.stringify({ input, expanded: expanded ?? null }) + "\n");
  }
  process.exit(0);
}

const outPath = join(here, "../tests/fixtures/expand-corpus.json");
const COUNT = Number(process.argv[2]) || 250;
const SEED = Number(process.argv[3]) || 0xe1a2b3c4;
const TIMEOUT_MS = 4000;

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

const VARS = ["x", "y", "z", "a", "b"];
const atom = () => (rng() < 0.55 ? pick(VARS) : String(Math.floor(rng() * 6) + 1));
// A binomial — the raw material for distribution.
const binom = () => `(${atom()} ${pick(["+", "-"])} ${atom()})`;

// A single factor whose expansion is small. NO powers of products / deep
// nesting — those make mathjs `expand` blow up combinatorially (and hang).
function factor() {
  const r = rng();
  if (r < 0.4) return binom();
  if (r < 0.56) return `${binom()}^${pick(["2", "3"])}`;
  if (r < 0.68) return atom();
  if (r < 0.82) return `-${binom()}`;
  return `${atom()}*${binom()}`;
}

// A product of 1–3 factors, optionally wrapped in a function or divided by a
// binomial (exercises the recurse-into-args and division-distribution paths).
function randExpr() {
  const k = 1 + Math.floor(rng() * 3);
  let e = Array.from({ length: k }, factor).join(" * ");
  if (chance(0.15)) e = `${pick(["sin", "exp", "sqrt"])}(${e})`;
  else if (chance(0.15)) e = `(${e}) / ${binom()}`;
  return e;
}

// Deterministic candidate inputs.
const seen = new Set();
const inputs = [];
let attempts = 0;
while (inputs.length < COUNT && attempts < COUNT * 30) {
  attempts++;
  const input = randExpr();
  if (seen.has(input)) continue;
  seen.add(input);
  inputs.push(input);
}

// Expand each under the watchdog; drop (and report) inputs where mathjs hangs.
const self = fileURLToPath(import.meta.url);
const cases = [];
const jsHangs = [];
let idx = 0;
while (idx < inputs.length) {
  const remaining = inputs.slice(idx);
  const r = spawnSync(process.execPath, [self, "--worker"], {
    input: JSON.stringify(remaining),
    timeout: TIMEOUT_MS,
    maxBuffer: 128 * 1024 * 1024,
  });
  const lines = (r.stdout || "").toString().split("\n").filter(Boolean);
  for (const line of lines) {
    const rec = JSON.parse(line);
    if (rec.expanded !== null) cases.push(rec);
  }
  const done = lines.length;
  if (r.signal === "SIGTERM" || r.error) {
    jsHangs.push(inputs[idx + done]);
    idx += done + 1;
  } else {
    idx += done;
    if (done >= remaining.length) break;
  }
}

writeFileSync(outPath, JSON.stringify({ cases, jsHangs }, null, 1) + "\n");
console.log(
  `expand-corpus.json: ${cases.length} cases (seed ${SEED}), ` +
    `${jsHangs.length} inputs dropped as mathjs-expand hangs`,
);
for (const h of jsHangs) console.log(`  EXPAND HANG: ${h}`);
