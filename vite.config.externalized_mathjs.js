import { defineConfig } from "vite";

export default defineConfig({
  build: {
    outDir: "build",
    emptyOutDir: false,
    lib: {
      entry: "lib/math-expressions.js",
      name: "MathExpression",
      formats: ["es"],
      fileName: () => "math-expressions-externalized_mathjs.js",
    },
    rollupOptions: {
      external: ["mathjs"],
      output: {
        exports: "named",
        minify: true,
      },
    },
  },
});
