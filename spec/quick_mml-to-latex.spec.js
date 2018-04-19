import mmlToLatex from '../lib/converters/mml-to-latex';

const converter = new mmlToLatex();

const objectsToTest = [
  {
    'mml': '<mrow><mrow><msup><mi>x</mi><mn>2</mn></msup><mo>+</mo><mrow><mn>4</mn><mo>&invisibletimes;</mo><mi>x</mi></mrow><mo>+</mo><mn>4</mn></mrow><mo>=</mo><mn>0</mn></mrow>',
    'latex': '((x^{2} + (4 &invisibletimes; x) + 4) = 0)'

  },
]


for (let objectToTest of objectsToTest) {
  test("parses " + objectToTest.mml + ' to ' + objectToTest.latex, () => {
    expect(converter.convert(objectToTest.mml)).toEqual(objectToTest.latex);
  });

}
