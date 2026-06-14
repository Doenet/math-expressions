import { defineConfig } from "vite";

export default defineConfig({
  test: {
    globals: true,
    environment: "node",
    projects: [
      {
        test: {
          name: "quick",
          globals: true,
          environment: "node",
          include: ["spec/quick_*.spec.js", "spec/build_*.spec.js"],
        },
      },
      {
        test: {
          name: "slow",
          globals: true,
          environment: "node",
          include: ["spec/slow_*.spec.js"],
          testTimeout: 60000,
        },
      },
    ],
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
