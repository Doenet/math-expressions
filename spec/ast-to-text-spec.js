var astToText = require('../lib/converters/parser').ast.to.text;

describe("ast to text", function() {
    it("sum of two numbers", function() {
	expect(astToText(['+',3,4]).replace(/ /g,'')).toEqual('3+4');
    });

    it("sum of three terms", function() {     
        expect(astToText(['+',3,4,'x']).replace(/ /g,'')).toEqual('3+4+x');
    });

    it("nested sum", function() {     
        expect(astToText(['+',3,['+',4,'x']]).replace(/ /g,'')).toEqual('3+(4+x)');
    });

    it("factorial", function() {     
        expect(astToText(['apply', 'factorial',3]).replace(/ /g,'')).toEqual('3!');
    });

    it("factorial", function() {     
        expect(astToText(['apply', 'factorial',['+','x','1']]).replace(/ /g,'')).toEqual('(x+1)!');
    });                

    it("sum of positive and negative number", function() {     
        expect(astToText(['+',3,-4]).replace(/ /g,'')).toEqual('3-4');
    });

    it("product of positive and negative number", function() {     
        expect(astToText(['*',3,-4]).replace(/ /g,'')).toEqual('3(-4)');
    });

    it("product of positive numbers", function() {     
        expect(astToText(['*',3,4]).replace(/ /g,'')).toEqual('3*4');
    });    

    it("sin^2 (3x)", function() {     
        expect(astToText(['apply', ['^','sin',2],['*',3,'x']]).replace(/ /g,'')).toEqual('sin^2(3x)');
    });

    it("arcsec(3x)", function() {     
        expect(astToText(['apply','arcsec',['*',3,'x']]).replace(/ /g,'')).toEqual('arcsec(3x)');
    });

    it("theta", function() {     
        expect(astToText(['+', 1, 'theta']).replace(/ /g,'')).toEqual('1+θ');
    });

    it("factorial", function() {     
        expect(astToText(['apply', 'factorial', 17]).replace(/ /g,'')).toEqual('17!');
    });                

    it("vector", function() {
        expect(astToText(['vector', 1, 'x']).replace(/ /g,'')).toEqual('(1,x)');
    });

    it("throws error apply", function() {
	expect(function () {astToText(['sin', 'x'])}).toThrowError();
    });

    it("throws error lts", function() {
	expect(function () {astToText(['lts', 'x', 'y', 'z'])}).toThrowError();
    });

    it("throws error gts", function() {
	expect(function () {astToText(['gts', 'x', 'y', 'z'])}).toThrowError();
    });
    
    it("throws error interval", function() {
	expect(function () {astToText(['interval', 'x', 'y'])}).toThrowError();
    });


    
});
