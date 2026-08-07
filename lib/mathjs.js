import { create, all } from "mathjs";
import numeric from "numeric";

// numeric.js builds most of its helpers at load time with the `Function`
// constructor, and the generated bodies reference a bare `numeric` (e.g.
// `_s = numeric.dim(x)`). Those functions are evaluated in global scope, so the
// reference resolves only if `numeric` is a property of the global object.
// numeric.js puts it there itself — but only through `global`, which exists in
// Node and nowhere else, so in a browser or a web worker every generated helper
// throws `ReferenceError: numeric is not defined` the first time it is called.
// Publish it ourselves so the generated code resolves wherever this runs.
//
// Assigned unconditionally rather than only when `globalThis.numeric` is unset:
// on a page with an element whose id is `numeric`, the named-element global
// makes it look occupied while still being useless to the generated code.
globalThis.numeric = numeric;

export function createInstance({
  define_e = true,
  define_pi = true,
  define_i = true,
  pow_strict = true,
} = {}) {
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

  math.import(numeric, { wrap: true, silent: true });

  // strict power function that returns NaN for 0^0, NaN^0, and Infinity^0
  var pow_original = math.pow;
  function pow_strict_f(base, pow) {
    if (
      pow === 0 &&
      typeof base === "number" &&
      (base === 0 || !Number.isFinite(base))
    ) {
      return NaN;
    } else return pow_original(base, pow);
  }

  if (pow_strict) math["import"]({ pow: pow_strict_f }, { override: true });
  else math["import"]({ pow: pow_original }, { override: true });

  math.define_e = define_e;
  math.define_pi = define_pi;
  math.define_i = define_i;
  math.pow_strict = pow_strict;

  return math;
}

export default createInstance();
