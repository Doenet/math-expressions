// import math.js, only with customized constants

var core = require('mathjs/core');

function create (config) {
  // create a new math.js instance
  var math = core.create(config);
  math.create = create;

  // import data types, functions, constants, expression parser, etc.
  math['import'](require('./lib'));

  // import numeric
  math['import'](require('numeric'), {wrap: true, silent: true});  

  // strict power function that returns NaN for 0^0, NaN^0, and Infinty^0
  var pow_original = math.pow;
  function pow_strict_f(base, pow) {
    if(pow==0 && (typeof base === 'number') &&
      (base==0 || !Number.isFinite(base))) {
        return NaN;
    }
    else
       return pow_original(base,pow);
  }

  function set_pow_strict(bool) {
    if(bool)
      math['import']({pow: pow_strict_f}, {override: true});
    else
      math['import']({pow: pow_original}, {override: true});
  }

  var is_pow_strict = () => math.pow === pow_strict_f;
 
  Object.defineProperty(math, 'pow_strict', {
    get: is_pow_strict,
    set: set_pow_strict,
  });
    
  math.pow_strict = true;


  function set_define_e(bool) {
    if(bool)
      math.config({define_e: true});
    else
      math.config({define_e: false});
  }
  var get_define_e = () => math.config().define_e!==false;
  function set_define_i(bool) {
    if(bool)
      math.config({define_i: true});
    else
      math.config({define_i: false});
  }
  var get_define_i = () => math.config().define_i!==false;
  function set_define_pi(bool) {
    if(bool)
      math.config({define_pi: true});
    else
      math.config({define_pi: false});
  }
  var get_define_pi = () => math.config().define_pi!==false;

  Object.defineProperty(math, 'define_e', {
    get: get_define_e,
    set: set_define_e,
  });
  Object.defineProperty(math, 'define_i', {
    get: get_define_i,
    set: set_define_i,
  });
  Object.defineProperty(math, 'define_pi', {
    get: get_define_pi,
    set: set_define_pi,
  });
    
  return math;
}

// return a new instance of math.js
module.exports = create();
