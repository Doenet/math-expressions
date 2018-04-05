import mmlToLatexObj from './mml-to-latex';
import latexToAstObj from './latex-to-ast';
import astToTextObj from './ast-to-text';

class mmlToText{
  constructor(){
   this.mmlToLatex = new mmlToLatexObj();
   this.latexToAst = new latexToAstObj();
   this.astToText = new astToTextObj();
  }

  convert(mml){
    return this.astToText.convert(this.latexToAst.convert(this.mmlToLatex.convert(mml)));
  }
}

export default mmlToText;
