// Generate the JS-oracle corpus for the f64 numeric module (src/numeric.rs)
// and the Doenet-interop utilities (js_match, combined rounding): scalar
// mod/gcd/lcm, statistics, lusolve, eigs (values only — eigenvectors are
// checked by residual on the Rust side), me.utils.match bindings, and
// round_numbers_to_precision_plus_decimals output trees.
//
//   node scripts/generate-numeric-corpus.mjs [seed]
//
// Deterministic (seeded mulberry32). Writes tests/fixtures/numeric-corpus.json.

import me from "../../lib/math-expressions.js";
import fs from "node:fs";

const SEED = Number(process.argv[2] ?? 20260719);
function mulberry32(a) {
  return function () {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const rand = mulberry32(SEED);
const randInt = (lo, hi) => lo + Math.floor(rand() * (hi - lo + 1));
const randNum = (scale = 10) => Math.round((rand() * 2 - 1) * scale * 100) / 100;

const m = me.math;
const out = { seed: SEED, scalar: [], stats: [], lusolve: [], eigs: [], match: [], round: [] };

// ---- scalar: mod / gcd / lcm ----
for (let i = 0; i < 40; i++) {
  const x = randNum(20);
  const y = randNum(10);
  out.scalar.push({ op: "mod", x, y, expected: m.mod(x, y) });
}
for (let i = 0; i < 20; i++) {
  const x = randInt(-100, 100);
  const y = randInt(-100, 100);
  out.scalar.push({ op: "gcd", x, y, expected: m.gcd(x, y) });
  out.scalar.push({ op: "lcm", x, y, expected: m.lcm(x, y) });
}

// ---- statistics ----
for (let i = 0; i < 25; i++) {
  const data = Array.from({ length: randInt(2, 12) }, () => randNum(50));
  const prob = Math.round(rand() * 100) / 100;
  out.stats.push({
    data,
    mean: m.mean(data),
    median: m.median(data),
    variance: m.variance(data),
    std: m.std(data),
    prob,
    quantile: m.quantileSeq(data, prob),
  });
}

// ---- lusolve ----
for (let i = 0; i < 20; i++) {
  const n = randInt(2, 5);
  const a = Array.from({ length: n * n }, () => randNum(5));
  // Diagonal boost for conditioning.
  for (let d = 0; d < n; d++) a[d * n + d] += (a[d * n + d] >= 0 ? 1 : -1) * (n + 2);
  const b = Array.from({ length: n }, () => randNum(10));
  try {
    const rows = Array.from({ length: n }, (_, r) => a.slice(r * n, r * n + n));
    const x = m.lusolve(rows, b).map((row) => (Array.isArray(row) ? row[0] : row));
    out.lusolve.push({ n, a, b, x });
  } catch {
    /* skip singular */
  }
}

// ---- eigs (values only; JS may return complex objects) ----
function pushEigs(a, n) {
  const rows = Array.from({ length: n }, (_, r) => a.slice(r * n, r * n + n));
  try {
    const result = m.eigs(rows);
    const values = result.values.toArray ? result.values.toArray() : result.values;
    const vals = values.map((v) =>
      typeof v === "number" ? { re: v, im: 0 } : { re: v.re, im: v.im },
    );
    out.eigs.push({ n, a, values: vals });
  } catch {
    /* mathjs eigs can fail; skip */
  }
}
for (let i = 0; i < 12; i++) {
  // symmetric
  const n = randInt(2, 4);
  const a = new Array(n * n).fill(0);
  for (let r = 0; r < n; r++)
    for (let c = r; c < n; c++) {
      const v = randNum(4);
      a[r * n + c] = v;
      a[c * n + r] = v;
    }
  pushEigs(a, n);
}
for (let i = 0; i < 12; i++) {
  // general
  const n = randInt(2, 4);
  const a = Array.from({ length: n * n }, () => randNum(4));
  pushEigs(a, n);
}

// ---- me.utils.match (default mode) ----
const matchCases = [
  ["2x + 3", "a x + b"],
  ["2x + 3", "a x + b + c"],
  ["x^2 + 2x + 1", "a x^2 + b x + c"],
  ["sin(2x)", "sin(a x)"],
  ["sin(2x)", "cos(a x)"],
  ["3 (x + 1)^2", "a (x + b)^2"],
  ["x + y + z", "a + b"],
  ["x y z w", "a b"],
  ["-(x y)", "a b"],
  ["(x+1)/(x+2)", "a/b"],
  ["x^2", "a^b"],
  ["x + x", "a + a"],
  ["x + y", "a + a"],
  ["2/3 x", "a x"],
  ["f(x) + 1", "f(a) + b"],
];
for (const [exprText, patternText] of matchCases) {
  const tree = me.fromText(exprText).tree;
  const pattern = me.fromText(patternText).tree;
  const result = me.utils.match(tree, pattern);
  out.match.push({
    tree,
    pattern,
    bindings: result === false ? null : result,
  });
}

// ---- round_numbers_to_precision_plus_decimals ----
const roundInputs = [
  "3.14159 x + 2.71828",
  "0.000123456 + 123456.789 y",
  "1.5 + 2.25 x^2",
  "9.999 x",
  "1234.5678",
  "0.5",
];
const roundParams = [
  [4, 2],
  [3, 0],
  [2, 5],
  [-Infinity, 1],
  [5, -Infinity],
  [20, 2],
];
for (const text of roundInputs) {
  for (const [digits, decimals] of roundParams) {
    const tree = me.fromText(text).tree;
    const rounded = me.round_numbers_to_precision_plus_decimals(tree, digits, decimals);
    out.round.push({
      tree,
      digits: digits === -Infinity ? "-Infinity" : digits,
      decimals: decimals === -Infinity ? "-Infinity" : decimals,
      expected: rounded.tree,
    });
  }
}

const path = new URL("../tests/fixtures/numeric-corpus.json", import.meta.url).pathname;
fs.writeFileSync(path, JSON.stringify(out, null, 1));
console.log(
  `numeric-corpus.json: ${out.scalar.length} scalar, ${out.stats.length} stats, ` +
    `${out.lusolve.length} lusolve, ${out.eigs.length} eigs, ${out.match.length} match, ` +
    `${out.round.length} round (seed ${SEED})`,
);
