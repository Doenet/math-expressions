import mmlToLatexObj from './mml-to-latex';
import latexToAstObj from './latex-to-ast';


class mmlToAst{
  constructor(){
   this.mmlToLatex = new mmlToLatexObj();
   this.latexToAst = new latexToAstObj();
  }

  convert(mml){
    return this.latexToAst.convert(this.mmlToLatex.convert(mml));
  }
}

export default mmlToAst;
