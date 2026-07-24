import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { viteStaticCopy } from "vite-plugin-static-copy";
import fs from "node:fs";
import zlib from "node:zlib";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

// The Rust implementation is built (by `npm run build:wasm`) into the crate's
// own wasm-bindgen output directory. Resolve that location and static-copy its
// browser assets into the served tree under /wasm/, instead of vendoring them
// into the playground source. The glue is loaded at runtime by URL (see
// src/engines.ts), so Vite never bundles the wasm — this copy is its sole
// delivery. (Pattern from Doenet/DoenetML's doenetml-prototype vite.config.ts.)
const wasmPkgDir = path.resolve(here, "../math-expressions-rs-wasm/pkg");

/* --------- build-time provenance for the footer (via `define`) --------- */

const byteLen = (p: string): number => {
  try {
    return fs.statSync(p).size;
  } catch {
    return 0;
  }
};
const gzipLen = (p: string): number => {
  try {
    return zlib.gzipSync(fs.readFileSync(p)).length;
  } catch {
    return 0;
  }
};
const fmtBytes = (n: number): string => {
  if (n <= 0) return "?";
  if (n >= 1024 * 1024) return `${(n / 1024 / 1024).toFixed(2)} MB`;
  if (n >= 1024) return `${Math.round(n / 1024)} KB`;
  return `${n} B`;
};
const jsonVersion = (p: string): string => {
  try {
    return JSON.parse(fs.readFileSync(p, "utf8")).version ?? "?";
  } catch {
    return "?";
  }
};
const cargoVersion = (p: string): string => {
  try {
    const m = fs.readFileSync(p, "utf8").match(/^\s*version\s*=\s*"([^"]+)"/m);
    return m ? m[1] : "?";
  } catch {
    return "?";
  }
};

// The canonical JS package (resolved wherever the workspace hoisted it). Its
// `exports` doesn't expose ./package.json, so walk up from the entry to find it.
const jsBundle = require.resolve("math-expressions-canonical"); // build/math-expressions.js
const findPackageJson = (from: string): string => {
  let dir = path.dirname(from);
  for (let i = 0; i < 6; i++) {
    const p = path.join(dir, "package.json");
    if (fs.existsSync(p)) return p;
    dir = path.dirname(dir);
  }
  return "";
};
const jsPkgJson = findPackageJson(jsBundle);

// The Rust → wasm payload the playground actually serves (glue + binary).
const wasmBin = path.join(wasmPkgDir, "math_expressions_wasm_bg.wasm");
const wasmGlue = path.join(wasmPkgDir, "math_expressions_wasm.js");

const buildInfo = {
  jsVersion: jsonVersion(jsPkgJson),
  jsBundle: fmtBytes(byteLen(jsBundle)),
  jsBundleGz: fmtBytes(gzipLen(jsBundle)),
  rsVersion: cargoVersion(path.resolve(here, "../math-expressions-rs/Cargo.toml")),
  // The wasm-backed drop-in npm package (`math-expressions-js-compat`).
  compatVersion: jsonVersion(
    path.resolve(here, "../math-expressions-js-compat/package.json"),
  ),
  compatBundle: fmtBytes(byteLen(wasmBin) + byteLen(wasmGlue)),
  compatBundleGz: fmtBytes(gzipLen(wasmBin) + gzipLen(wasmGlue)),
};

export default defineConfig({
  plugins: [
    react(),
    viteStaticCopy({
      targets: [
        { src: path.join(wasmPkgDir, "math_expressions_wasm.js"), dest: "wasm" },
        {
          src: path.join(wasmPkgDir, "math_expressions_wasm_bg.wasm"),
          dest: "wasm",
        },
        // Served (not bundled) so the palette can reflect the live API surface
        // at runtime — the "Other" category is generated from this. See wasmApi.ts.
        {
          src: path.join(wasmPkgDir, "math_expressions_wasm.d.ts"),
          dest: "wasm",
        },
      ],
    }),
  ],
  define: {
    __BUILD_INFO__: JSON.stringify(buildInfo),
  },
  server: {
    host: "0.0.0.0",
  },
});
