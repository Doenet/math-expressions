import textToAstObj from './text-to-ast';
import astToGuppyObj from './ast-to-guppy';

class textToGuppy{
  constructor(){
   this.textToAst = new textToAstObj();
   this.astToGuppy = new astToGuppyObj();
  }

  convert(text){
    return this.astToGuppy.convert(this.textToAst.convert(text));
  }
}

export default textToGuppy;
