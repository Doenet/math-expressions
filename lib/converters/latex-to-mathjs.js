import latexToAstObj from './latex-to-ast';
import astToMathjsObj from './ast-to-mathjs';

class latexToMathjs{
  constructor(){
   this.latexToAst = new latexToAstObj();
   this.astToMathjs = new astToMathjsObj();
  }

  convert(latex){
    return this.astToMathjs.convert(this.latexToAst.convert(latex));
  }
}

export default latexToMathjs;
