import textToAstObj from "./text-to-ast.js";
import astToGuppyObj from "./ast-to-guppy.js";

class textToGuppy {
  constructor() {
    this.textToAst = new textToAstObj();
    this.astToGuppy = new astToGuppyObj();
  }

  convert(text) {
    return this.astToGuppy.convert(this.textToAst.convert(text));
  }
}

export default textToGuppy;
