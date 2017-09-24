// import math.js, only with customized constants

var core = require('mathjs/core');

function create (config) {
  // create a new math.js instance
  var math = core.create(config);
  math.create = create;

  // import data types, functions, constants, expression parser, etc.
  math['import'](require('./lib'));

  return math;
}

// return a new instance of math.js
module.exports = create();
