// import customized constants file
import type from './type';
import constants from './constants';
import expression from 'mathjs/lib/expression';
import mathjsFunction from 'mathjs/lib/function';
import json from 'mathjs/lib/json';
import error from 'mathjs/lib/error';

export default [
  type,        // data types (Matrix, Complex, Unit, ...)
  constants,   // constants
  expression,  // expression parsing
  mathjsFunction,    // functions
  json,        // serialization utility (math.json.reviver)
  error        // errors
];
