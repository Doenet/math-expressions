import mathjsToAstObj from "./mathjs-to-ast";
import astToGuppyObj from "./ast-to-guppy";

class mathjsToGuppy {
  constructor() {
    this.mathjsToAst = new mathjsToAstObj();
    this.astToGuppy = new astToGuppyObj();
  }

  convert(mathjs) {
    return this.astToGuppy.convert(this.mathjsToAst.convert(mathjs));
  }
}

export default mathjsToGuppy;
