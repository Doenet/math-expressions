

// import textToLatexObj from './converters/text-to-latex';
// var converter = new textToLatexObj();
// console.log(converter.convert('sin(x)'));

// import textToGuppyObj from './converters/text-to-guppy';
// var converter = new textToGuppyObj();
// console.log(converter.convert('sin(x)'));
import textToMathjsObj from './converters/text-to-mathjs';
var converter = new textToMathjsObj();
console.log(converter.convert('sin(x)'));

// import math from './mathjs';

// import mathjsToGuppyObj from './converters/mathjs-to-guppy';
// var converter = new mathjsToGuppyObj();
// console.log(converter.convert(math.parse('1+x+3')));

// import mathjsToLatexObj from './converters/mathjs-to-latex';
// var converter = new mathjsToLatexObj();
// console.log(converter.convert(math.parse('1+x+3')));

// import mathjsToTextObj from './converters/mathjs-to-text';
// var converter = new mathjsToTextObj();
// console.log(converter.convert(math.parse('1+x+3')));

// import latexToGuppyObj from './converters/latex-to-guppy';
// var converter = new latexToGuppyObj();
// console.log(converter.convert('\\frac{1}{2} x'));

// import latexToMathjsObj from './converters/latex-to-mathjs';
// var converter = new latexToMathjsObj();
// console.log(converter.convert('\\frac{1}{2} x'));

// import latexToTextObj from './converters/latex-to-text';
// var converter = new latexToTextObj();
// console.log(converter.convert('\\frac{1}{2} x'));

// import mmlToGuppyObj from './converters/mml-to-guppy';
// var converter = new mmlToGuppyObj();
// console.log(converter.convert('<mrow><msup><mi>x</mi><mn>2</mn></msup></mrow>'));

// import mmlToMathjsObj from './converters/mml-to-mathjs';
// var converter = new mmlToMathjsObj();
// console.log(converter.convert('<mrow><msup><mi>x</mi><mn>2</mn></msup></mrow>'));

// import mmlToTextObj from './converters/mml-to-text';
// var converter = new mmlToTextObj();
// console.log(converter.convert('<mrow><msup><mi>x</mi><mn>2</mn></msup></mrow>'));

// import mmlToAstObj from './converters/mml-to-ast';
// var converter = new mmlToAstObj();
// console.log(converter.convert('<mrow><msup><mi>x</mi><mn>2</mn></msup></mrow>'));

// import me from './math-expressions';
// console.log(me);

// console.log(me.fromText('x').add(me.fromText('y')).toString());
// var mathstuff = me.fromText('x+y');
// console.log(mathstuff);
// console.log(me.fromText('x'));
// me.fromText('x');

// import * as normalization from './expression/normalization';
//
// console.log(normalization);
