// Assumptions corpus (PORTING_PLAN.md §11): differential oracle for the eight
// three-valued queries in lib/assumptions/element_of_sets.js. Deterministic
// enumeration (no RNG): every (assumption, expression) pair from the pools
// below, with each query's JS verdict recorded as "T"/"F"/"U".
//   node scripts/generate-assumptions-corpus.mjs

import { writeFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import me from "../../lib/math-expressions.js";
import * as els from "../../lib/assumptions/element_of_sets.js";

const here = dirname(fileURLToPath(import.meta.url));
const outPath = join(here, "../tests/fixtures/assumptions-corpus.json");

const QUERIES = {
  integer: els.is_integer,
  real: els.is_real,
  complex: els.is_complex,
  nonzero: els.is_nonzero,
  nonnegative: els.is_nonnegative,
  positive: els.is_positive,
  negative: els.is_negative,
  nonpositive: els.is_nonpositive,
};

// null = no assumption. Only single-variable assumptions on x / n / y.
const ASSUMPTIONS = [
  null,
  "x > 0",
  "x < 0",
  "x >= 0",
  "x <= 0",
  "x != 0",
  "x = 3",
  "x = -2",
  "x > 2",
  "x < -1",
  "n elementof Z",
  "x elementof R",
  "x > 0 and y < 0",
  "n elementof Z and n > 0",
];

const EXPRS = [
  "5", "-3", "1/2", "0", "pi", "e", "i",
  "x", "-x", "x^2", "x^3", "x+1", "x-1", "2*x", "x/2", "1/x",
  "abs(x)", "exp(x)", "sqrt(x)", "sin(x)", "log(x)",
  "x^2+1", "x*y", "x+y", "y", "-y", "x*x",
  "n", "n+1", "2*n", "n^2", "n/2", "n*n", "-n",
  "x + pi", "pi*x", "exp(x)+1", "abs(x)+1", "x^2*y",
];

const enc = (v) => (v === undefined ? "U" : v ? "T" : "F");

const cases = [];
for (const assume of ASSUMPTIONS) {
  me.clear_assumptions();
  if (assume) me.add_assumption(me.from(assume));
  const a = me.assumptions;
  for (const expr of EXPRS) {
    const verdicts = {};
    for (const [name, f] of Object.entries(QUERIES)) {
      let v;
      try {
        v = f(me.fromText(expr), a);
      } catch {
        v = undefined;
      }
      verdicts[name] = enc(v);
    }
    cases.push({ assume, expr, verdicts });
  }
}
me.clear_assumptions();

writeFileSync(outPath, JSON.stringify(cases, null, 1) + "\n");
console.log(
  `assumptions-corpus.json: ${cases.length} cases ` +
    `(${ASSUMPTIONS.length} assumption contexts × ${EXPRS.length} expressions × 8 queries)`,
);
