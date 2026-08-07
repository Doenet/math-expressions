// The original library bundled a configured math.js instance and re-exported it
// as `me.math` / `../lib/mathjs`. We re-export the npm `mathjs` default so specs
// that reach for it keep working.
import * as mathjs from "mathjs";
import numeric from "numeric";

const math = mathjs.create ? mathjs.create(mathjs.all) : mathjs;

// The original `me.math` carried numeric.js's functions alongside math.js's, and
// consumers reach for the ones math.js has no equivalent of: DoenetML's
// `<odeSystem>` integrates with `me.math.dopri`. Keep importing them, or this
// drop-in silently drops those names. `silent` skips the ones math.js already
// defines, leaving math.js's own implementations in place.
(math as mathjs.MathJsInstance).import(numeric, { wrap: true, silent: true });

// numeric.js builds most of its helpers at load time with the `Function`
// constructor, and the generated bodies reference a bare `numeric` (e.g.
// `_s = numeric.dim(x)`). Functions made that way are evaluated in global scope,
// so the reference resolves only if `numeric` is a property of the global
// object. numeric.js puts it there itself, but only through `global`, which
// exists in Node and nowhere else — so in a browser or a web worker every
// generated helper throws `ReferenceError: numeric is not defined` the first
// time it is called, `dopri` included. Publish it ourselves to cover every
// runtime.
//
// Assigned unconditionally rather than guarded on `globalThis.numeric ===
// undefined`: on a page holding an element whose id is `numeric`, the
// named-element global makes the slot look occupied while still being useless
// to the generated code.
(globalThis as Record<string, unknown>).numeric = numeric;

export default math;
