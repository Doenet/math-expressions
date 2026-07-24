import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";

export default defineConfig({
  build: {
    outDir: "dist",
    lib: {
      entry: fileURLToPath(new URL("./lib/math-expressions.js", import.meta.url)),
      name: "MathExpression",
      formats: ["es", "umd"],
      fileName: (format) =>
        format === "umd" ? "math-expressions_umd.js" : "math-expressions.js",
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
