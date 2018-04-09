import mathjsToAstObj from './mathjs-to-ast';
import astToTextObj from './ast-to-text';

class mathjsToText{
  constructor(){
   this.mathjsToAst = new mathjsToAstObj();
   this.astToText = new astToTextObj();
  }

  convert(mathjs){
    return this.astToText.convert(this.mathjsToAst.convert(mathjs));
  }
}

export default mathjsToText;
