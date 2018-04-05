import textToAstObj from './text-to-ast';
import astToMathjsObj from './ast-to-mathjs';

class textToMathjs{
  constructor(){
   this.textToAst = new textToAstObj();
   this.astToMathjs = new astToMathjsObj();
  }

  convert(text){
    return this.astToMathjs.convert(this.textToAst.convert(text));
  }
}

export default textToMathjs;
