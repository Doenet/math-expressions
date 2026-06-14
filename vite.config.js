import { defineConfig } from "vite";

export default defineConfig({
  test: {
    globals: true,
    environment: "node",
    include: ["spec/**/*.spec.js"],
  },
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
