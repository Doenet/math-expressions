/**
 * Prove this package is publishable, by publishing it — as far as a tarball —
 * and consuming it from outside the workspace.
 *
 * `npm test` and `npm run build` both run inside the monorepo, where every
 * specifier resolves through workspace symlinks and every build output is
 * simply present. Neither can see the three ways a package breaks only once
 * installed, all of which this repo has had at once:
 *
 *   - a `dependencies` entry that is not on the registry (`npm install` fails
 *     outright with a 404 — it never gets as far as importing anything);
 *   - `main`/`exports` pointing into a git-ignored `dist/` that nothing built,
 *     so the tarball's entry point does not exist;
 *   - a runtime asset that is not in `files`, or not reachable through
 *     `exports` — the `--target web` wasm a browser host has to inject.
 *
 * So: pack, install the tarball into a throwaway project in the OS temp
 * directory, and run `scripts/consumer/*.mjs` there against the bare
 * `math-expressions` specifier.
 *
 * Requires the same toolchain as a real publish, because `prepack` runs: cargo,
 * the wasm32 target, and a matching wasm-bindgen-cli.
 */
import { execFileSync } from "node:child_process";
import {
  cpSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const PKG = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const scratch = mkdtempSync(resolve(tmpdir(), "math-expressions-verify-"));

const run = (cmd, args, cwd) =>
  execFileSync(cmd, args, { cwd, stdio: "inherit" });

try {
  console.log(`packing ${PKG} -> ${scratch}`);
  // The tarball name is derived from the manifest rather than parsed out of
  // `npm pack --json`, so a rename or a version bump reads obviously here.
  const { name, version } = JSON.parse(
    readFileSync(resolve(PKG, "package.json"), "utf8"),
  );
  run("npm", ["pack", "--pack-destination", scratch], PKG);
  const tarball = resolve(scratch, `${name}-${version}.tgz`);

  const consumer = resolve(scratch, "consumer");
  cpSync(resolve(PKG, "scripts/consumer"), consumer, { recursive: true });
  writeFileSync(
    resolve(consumer, "package.json"),
    // No `dependencies` block: the install below adds one. `private` keeps
    // a stray `npm publish` in this directory from doing anything.
    `${JSON.stringify({ name: "consumer", private: true, type: "module", version: "0.0.0" }, null, 2)}\n`,
  );

  console.log(`installing ${tarball}`);
  run("npm", ["install", "--no-audit", "--no-fund", tarball], consumer);

  for (const script of ["node-path.mjs", "web-path.mjs"]) {
    console.log(`running ${script}`);
    run("node", [script], consumer);
  }
  console.log("package verification: ok");
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
