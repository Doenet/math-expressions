import { defineConfig } from "vite";

export default defineConfig({
  build: {
    outDir: "build",
    lib: {
      entry: "lib/math-expressions.js",
      name: "MathExpression",
      formats: ["es", "umd"],
      fileName: (format) =>
        format === "umd" ? "math-expressions_umd.js" : "math-expressions.js",
    },
    rollupOptions: {
      output: {
        exports: "named",
      },
    },
  },
});
