import { create, all } from 'mathjs';
import numeric from 'numeric';

export function createInstance({ define_e = true, define_pi = true, define_i = true, pow_strict = true } = {}) {
  let options = { ...all };

  delete options.createTau;
  delete options.createPhi;

  if (!define_e) {
    delete options.createE;
  }

  if (!define_pi) {
    delete options.createPi;
  }

  if (!define_i) {
    delete options.createI;
  }

  let math = create(options);

  math.import(numeric, { wrap: true, silent: true })

  // strict power function that returns NaN for 0^0, NaN^0, and Infinity^0
  var pow_original = math.pow;
  function pow_strict_f(base, pow) {
    if (pow === 0 && (typeof base === 'number') &&
      (base === 0 || !Number.isFinite(base))) {
      return NaN;
    }
    else
      return pow_original(base, pow);
  }

  if (pow_strict)
    math['import']({ pow: pow_strict_f }, { override: true });
  else
    math['import']({ pow: pow_original }, { override: true });


  math.define_e = define_e;
  math.define_pi = define_pi;
  math.define_i = define_i;
  math.pow_strict = pow_strict;

  return math;

}

export default createInstance();