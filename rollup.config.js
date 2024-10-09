// rollup.config.js
import { nodeResolve } from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";
import polyfill from "rollup-plugin-polyfill-node";
import terser from "@rollup/plugin-terser";

export default {
  input: "lib/math-expressions.js",
  output: [
    {
      file: "build/math-expressions_umd.js",
      format: "umd",
      name: "MathExpression",
    },
    {
      file: "build/math-expressions.js",
      format: "es",
    },
  ],
  plugins: [
    commonjs({
      transformMixedEsModules: true,
    }),
    nodeResolve({
      preferBuiltins: true,
      browser: true,
    }),
    polyfill(),
    terser(),
  ],
};
