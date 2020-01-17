// rollup.config.js
import resolve from 'rollup-plugin-node-resolve';
import commonjs from 'rollup-plugin-commonjs';
import globals from 'rollup-plugin-node-globals';
import builtins from 'rollup-plugin-node-builtins';

export default {
  input: 'lib/math-expressions.js',
  output: {
    file: 'build/math-expressions.js',
    format: 'umd',
    name: 'MathExpression'
  },
  plugins: [
    resolve(),
    commonjs(),
    globals(),
    builtins()
  ]
};
