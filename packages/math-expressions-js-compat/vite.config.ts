import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";

export default defineConfig({
  build: {
    outDir: "dist",
    lib: {
      entry: fileURLToPath(
        new URL("./lib/math-expressions.ts", import.meta.url),
      ),
      name: "MathExpression",
      formats: ["es", "umd"],
      fileName: (format) =>
        format === "umd" ? "math-expressions_umd.js" : "math-expressions.js",
    },
    rollupOptions: {
      // math.js is the one runtime dependency that stays a bare import rather
      // than being inlined. It is 95% of the bundle: with it inlined the ES
      // build is 1,002 kB, without it 48 kB. Every consumer that reaches
      // `Expression#f()` or the AST ↔ math.js converters resolves `mathjs`
      // anyway — it is in `dependencies` — and a private copy would be both
      // dead weight and a second `math.create(math.all)` instance.
      //
      // `math-expressions-rs-wasm` is deliberately *not* external: it is a
      // workspace-internal package that is not published, so the published
      // `dist/` has to carry it. See the note in `package.json`'s
      // devDependencies placement.
      external: ["mathjs"],
      // UMD has no module resolution, so an external has to name a global.
      // `math` is math.js's own UMD global.
      output: { globals: { mathjs: "math" } },
    },
  },
  test: {
    // The specs use bare describe/it/test/expect (Jasmine/Vitest globals) and
    // load the wasm synchronously via a Node require, so run in the node env.
    globals: true,
    environment: "node",
    include: ["spec/**/*.spec.ts"],
    // The nodejs-target wasm in vendor/ is a CommonJS package loaded through
    // createRequire; keep Vite from trying to transform it.
    server: { deps: { external: [/vendor[\\/]wasm/] } },
    testTimeout: 60000,
  },
});
