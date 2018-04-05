import latexToAstObj from './latex-to-ast';
import astToGuppyObj from './ast-to-guppy';

class latexToGuppy{
  constructor(){
   this.latexToAst = new latexToAstObj();
   this.astToGuppy = new astToGuppyObj();
  }

  convert(latex){
    return this.astToGuppy.convert(this.latexToAst.convert(latex));
  }
}

export default latexToGuppy;
