// import customized constants file

export default [
  require('./type'),        // data types (Matrix, Complex, Unit, ...)
  require('./constants'),   // constants
  require('mathjs/lib/expression'),  // expression parsing
  require('mathjs/lib/function'),    // functions
  require('mathjs/lib/json'),        // serialization utility (math.json.reviver)
  require('mathjs/lib/error')        // errors
];
