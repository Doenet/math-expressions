// Generate the simplify corpus fixture from spec/slow_simplify.spec.js.
// Re-run when the upstream spec changes:
//   node scripts/generate-simplify-corpus.mjs
//
// The spec is imperative (`me.fromText("...").simplify().tree` toEqual ...), not
// a data map, so we cannot slice it like the parser specs. Instead we harvest
// every *text* input literal fed to me.from(...) / me.fromText(...) and run each
// through the JS library as the oracle, recording {input, tree} where `tree` is
// what JS `.simplify()` produces. JS is the correctness oracle; we never invent
// expected trees (same philosophy as the parser fixtures and equality corpus).
//
// Infinity/NaN — which JSON cannot represent — are encoded as {"$":"Inf"} /
// {"$":"-Inf"} / {"$":"NaN"}, matching extract-fixtures.mjs.

import { readFileSync, writeFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import me from "../../../tmp/js-legacy/lib/math-expressions.js";

const here = dirname(fileURLToPath(import.meta.url));
const specPath = join(here, "../../../tmp/js-legacy/spec/slow_simplify.spec.js");
const outPath = join(here, "../tests/fixtures/simplify-corpus.json");

const src = readFileSync(specPath, "utf8");

// Harvest the string argument of every me.from(...) / me.fromText(...) call.
// Handles the three JS quote styles with escapes. me.fromLatex(...) is excluded
// for now (latex inputs get their own corpus later); the spec has 249 me.from +
// 266 me.fromText text inputs, which is a rich sample of real simplify targets.
function harvestInputs(text) {
  const inputs = [];
  const re = /me\.(?:from|fromText)\(\s*(["'`])((?:\\.|(?!\1)[\s\S])*?)\1/g;
  let m;
  while ((m = re.exec(text)) !== null) {
    // Unescape the captured literal the way JS would.
    const raw = m[2];
    let val;
    try {
      val = JSON.parse('"' + raw.replace(/\\'/g, "'").replace(/\\`/g, "`") + '"');
    } catch {
      val = raw; // fall back to the raw slice if it isn't clean JSON
    }
    inputs.push(val);
  }
  return inputs;
}

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
let skipped = 0;
for (const input of harvestInputs(src)) {
  if (seen.has(input)) continue;
  seen.add(input);
  try {
    const tree = me.fromText(input).simplify().tree;
    cases.push({ input, tree: encodeSpecials(tree) });
  } catch {
    skipped++; // inputs the JS library itself rejects are not simplify targets
  }
}

writeFileSync(outPath, JSON.stringify(cases, null, 1) + "\n");
console.log(
  `simplify-corpus.json: ${cases.length} cases (${skipped} inputs skipped as JS errors)`,
);
