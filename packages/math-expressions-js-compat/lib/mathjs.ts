// The original library bundled a configured math.js instance and re-exported it
// as `me.math` / `../lib/mathjs`. We re-export the npm `mathjs` default so specs
// that reach for it keep working.
import * as mathjs from "mathjs";

const math = mathjs.create ? mathjs.create(mathjs.all) : mathjs;

export default math;
