// Extract the `allow_error_in_numbers` grading matrix from the JS spec into a
// JSON fixture consumed by tests/tolerance.rs.
//
//   node scripts/generate-tolerance-corpus.mjs
//
// Only the *numeric* file (slow_check-equality-numerical-errors) is emitted:
// its cases drive Rust `equals(a, b, EqOptions)`. The *symbolic* companion
// (slow_check-symbolic-equality-numerical-errors) exercises `equalsViaSyntax`
// with number tolerance, which Rust's exact `equals_syntactic` does not apply —
// see active-plans/JS_TEST_COVERAGE_AUDIT.md. See scripts/README.md for the
// oracle-path / js-compat migration note.

import { readFileSync, writeFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const here = dirname(fileURLToPath(import.meta.url));
const specFile = join(
  here,
  "../../../tmp/js-legacy/spec/slow_check-equality-numerical-errors.spec.js",
);
const outFile = join(here, "../tests/fixtures/tolerance-corpus.json");

// Slice `const allow_error_in_numbers = [ ... ];` out and eval it as data.
const src = readFileSync(specFile, "utf8");
const marker = "const allow_error_in_numbers = [";
const start = src.indexOf(marker);
if (start === -1) throw new Error("array not found");
const open = start + marker.length - 1;
const end = src.indexOf("\n];", open);
if (end === -1) throw new Error("end of array not found");
const literal = src.slice(open, end + 2);
const objs = eval(`(${literal})`);

writeFileSync(outFile, JSON.stringify(objs, null, 1) + "\n");
console.log(`tolerance-corpus.json: ${objs.length} objects`);
