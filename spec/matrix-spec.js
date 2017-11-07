var me=require("../lib/math-expressions");
describe("matrixtest", function(){
         it ("creation",function(){
             var a11=me.from('x1');
             var a12=me.from('x2');
             var a21=me.from('x3');
             var a22=me.from('x4');
             var matrix=me.matrix([[a11,a12],[a21,a22]]);
             expect(matrix.tree).toEqual(["matrix",["tuple",2,2],["tuple",["tuple",'y1','x2'],["tuple",'x3','x4']]]);
             
             
             
             });
         
         
         
         });

