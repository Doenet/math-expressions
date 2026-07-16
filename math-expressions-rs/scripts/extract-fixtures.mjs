// Extract test fixtures from the JS spec data maps into JSON files.
// Re-run when the upstream specs change:
//   node scripts/extract-fixtures.mjs
//
// JS trees may contain Infinity/NaN, which JSON cannot represent; those are
// encoded as {"$": "Inf"} / {"$": "-Inf"} / {"$": "NaN"} objects. This is
// unambiguous because a JS Tree never contains plain objects.

import { readFileSync, writeFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const here = dirname(fileURLToPath(import.meta.url));
const specDir = join(here, "../../spec");
const outDir = join(here, "../tests/fixtures");

// Slice `var <name> = {...};` out of a spec file and eval it as an object
// literal. The maps are plain data (strings, numbers, nested arrays).
function extractMap(src, name) {
  const startMarker = `var ${name} = {`;
  const start = src.indexOf(startMarker);
  if (start === -1) throw new Error(`map ${name} not found`);
  const open = start + startMarker.length - 1;
  const end = src.indexOf("\n};", open);
  if (end === -1) throw new Error(`end of map ${name} not found`);
  const literal = src.slice(open, end + 2);
  return eval(`(${literal})`);
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

function writeFixture(outName, specName, mapName, kind) {
  const src = readFileSync(join(specDir, specName), "utf8");
  const map = extractMap(src, mapName);
  const cases = Object.entries(map).map(([input, expected]) =>
    kind === "tree"
      ? { input, tree: encodeSpecials(expected) }
      : { input, error: expected },
  );
  writeFileSync(join(outDir, outName), JSON.stringify(cases, null, 1) + "\n");
  console.log(`${outName}: ${cases.length} cases`);
}

writeFixture("text-to-ast.json", "quick_text-to-ast.spec.js", "trees", "tree");
writeFixture(
  "text-to-ast-errors.json",
  "quick_text-to-ast.spec.js",
  "bad_inputs",
  "error",
);
