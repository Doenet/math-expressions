import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { viteStaticCopy } from "vite-plugin-static-copy";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));

// The Rust implementation is built (by `npm run build:wasm`) into the crate's
// own wasm-bindgen output directory. Resolve that location and static-copy its
// browser assets into the served tree under /wasm/, instead of vendoring them
// into the playground source. The glue is loaded at runtime by URL (see
// src/engines.ts), so Vite never bundles the wasm — this copy is its sole
// delivery. (Pattern from Doenet/DoenetML's doenetml-prototype vite.config.ts.)
const wasmPkgDir = path.resolve(here, "../math-expressions-rs-wasm/pkg");

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
      ],
    }),
  ],
  server: {
    host: "0.0.0.0",
  },
});
