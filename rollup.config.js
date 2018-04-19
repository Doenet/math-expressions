// rollup.config.js
import resolve from 'rollup-plugin-node-resolve';
import commonjs from 'rollup-plugin-commonjs';
import builtins from 'rollup-plugin-node-builtins';

export default {
  input: 'lib/math-expressions.js',
  output: [{
    file: 'build/math-expressions_umd.js',
    format: 'umd',
    name: 'MathExpression'
  },{
    file: 'build/math-expressions.js',
    format: 'es'
  }],
  plugins: [
    resolve(),
    commonjs(),
    builtins()
  ]
};
