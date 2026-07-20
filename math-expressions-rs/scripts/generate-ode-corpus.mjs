// ODE corpus (ODE_PLAN.md §3.2): sample `numeric.dopri` (the exact solver
// Doenet's ODESystem uses today, smuggled through me.math) on a fixed set of
// systems, at fixed abscissae. The Rust solver must agree to a mutual
// tolerance (both sides are approximations of the same trajectory).
//   node scripts/generate-ode-corpus.mjs

import { writeFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import me from "../../lib/math-expressions.js";

const here = dirname(fileURLToPath(import.meta.url));
const outPath = join(here, "../tests/fixtures/ode-corpus.json");
const { dopri } = me.math;

// Each system: rhs expressed BOTH as JS (for numeric.dopri) and as
// math-expressions text (for the Rust `solve_ode_exprs` side).
const SYSTEMS = [
  {
    name: "exp growth",
    rhs_text: ["y"],
    vars: ["y"],
    f: (t, y) => [y[0]],
    y0: [1],
    t1: 2,
  },
  {
    name: "decay",
    rhs_text: ["-2y"],
    vars: ["y"],
    f: (t, y) => [-2 * y[0]],
    y0: [3],
    t1: 3,
  },
  {
    name: "logistic",
    rhs_text: ["y(1-y)"],
    vars: ["y"],
    f: (t, y) => [y[0] * (1 - y[0])],
    y0: [0.1],
    t1: 8,
  },
  {
    name: "forced decay",
    rhs_text: ["-y/2 + sin(2t)"],
    vars: ["y"],
    f: (t, y) => [-y[0] / 2 + Math.sin(2 * t)],
    y0: [1],
    t1: 6,
  },
  {
    name: "harmonic",
    rhs_text: ["v", "-x"],
    vars: ["x", "v"],
    f: (t, y) => [y[1], -y[0]],
    y0: [1, 0],
    t1: 10,
  },
  {
    name: "damped oscillator",
    rhs_text: ["v", "-x - v/5"],
    vars: ["x", "v"],
    f: (t, y) => [y[1], -y[0] - y[1] / 5],
    y0: [0, 2],
    t1: 12,
  },
  {
    name: "pendulum",
    rhs_text: ["v", "-sin(x)"],
    vars: ["x", "v"],
    f: (t, y) => [y[1], -Math.sin(y[0])],
    y0: [2.5, 0],
    t1: 10,
  },
  {
    name: "van der pol (mild)",
    rhs_text: ["v", "(1 - x^2) v - x"],
    vars: ["x", "v"],
    f: (t, y) => [y[1], (1 - y[0] * y[0]) * y[1] - y[0]],
    y0: [0.5, 0],
    t1: 10,
  },
  {
    name: "lotka-volterra",
    rhs_text: ["x(1 - y)", "y(x - 1)/2"],
    vars: ["x", "y"],
    f: (t, y) => [y[0] * (1 - y[1]), (y[1] * (y[0] - 1)) / 2],
    y0: [1.5, 0.7],
    t1: 12,
  },
  {
    name: "3d rotation",
    rhs_text: ["y - z", "z - x", "x - y"],
    vars: ["x", "y", "z"],
    f: (t, y) => [y[1] - y[2], y[2] - y[0], y[0] - y[1]],
    y0: [1, 0, 0],
    t1: 6,
  },
];

const TOL = 1e-8;
const SAMPLES = 17;
const rows = [];
for (const sys of SYSTEMS) {
  const sol = dopri(0, sys.t1, sys.y0, sys.f, TOL, 4000);
  const samples = [];
  for (let i = 0; i <= SAMPLES; i++) {
    const t = (sys.t1 * i) / SAMPLES;
    samples.push({ t, y: sol.at(t) });
  }
  rows.push({
    name: sys.name,
    rhs: sys.rhs_text,
    vars: sys.vars,
    y0: sys.y0,
    t1: sys.t1,
    samples,
  });
}

writeFileSync(outPath, JSON.stringify(rows, null, 1));
console.log(`wrote ${rows.length} systems to ${outPath}`);
