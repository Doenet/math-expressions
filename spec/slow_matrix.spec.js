import me from '../lib/math-expressions';
import * as simplify from '../lib/expression/simplify';
import * as matrix from '../lib/expression/matrix';

describe("matrixtest", function(){
         it ("creation",function(){
             var a11=me.from('x1');
             var a12=me.from('x2');
             var a21=me.from('x3');
             var a22=me.from('x4');
             var matrix=me.matrix([[a11,a12],[a21,a22]]);
             expect(matrix.tree).toEqual(["matrix",["tuple",2,2],["tuple",["tuple",'x1','x2'],["tuple",'x3','x4']]]);
             });
         });

describe("vector addition", function() {
         var vectors = [
            ['(1,2)', '(5,6)', '(6,8)'],
            ['(1,2,3)', '(7,6,5)', '(8,8,8)'],
            ['(1,2,3,4)', '(5,2,1,2)', '(6,4,4,6)']
         ]
         
         vectors.forEach(function(example) {
                             it(example, function() {
                                let first_vector = me.fromText(example[0]);
                                let second_vector = me.fromText(example[1]);
                                let sum_vector = me.fromText(example[2]).tuples_to_vectors();
                                expect(me.vector_add(first_vector, second_vector).tree).toEqual(sum_vector.tree);
                                expect(me.vector_add(first_vector.tuples_to_vectors(), second_vector.tuples_to_vectors()).tree).toEqual(sum_vector.tree);
                                })
                             })
         })

describe("scalar times vector", function() {
         var vectors = [
                        ['(1,2)', '5', '(5,10)'],
                        ['(1,2,3)', '-1', '(-1, -2, -3)'],
                        ['(1,2,3,4)', 'x', '(x,2x,3x,4x)']
                        ]
         
         vectors.forEach(function(example) {
                         it(example, function() {
                            let vector = me.fromText(example[0]);
                            let scalar = me.fromText(example[1]);
                            let prod_vector = me.fromText(example[2]).tuples_to_vectors().simplify();
                            expect(me.scalar_mul(scalar, vector).tree).toEqual(prod_vector.tree);
                            expect(me.scalar_mul(scalar, vector.tuples_to_vectors()).tree).toEqual(prod_vector.tree);

                            })
                         })
         })

describe("vector subtraction", function() {
         var vectors = [
                        ['(1,2)', '(5,6)', '(-4,-4)'],
                        ['(1,2,3)', '(x,y,z)', '(1-x,2-y,3-z)'],
                        ['(1,2,3,4)', '(-1,-2,-1,-2)', '(2,4,4,6)']
                        ]
         
         vectors.forEach(function(example) {
                         it(example, function() {
                            let first_vector = me.fromText(example[0]);
                            let second_vector = me.fromText(example[1]);
                            let sum_vector = me.fromText(example[2]).tuples_to_vectors().simplify();
                            expect(me.vector_sub(first_vector, second_vector).tree).toEqual(sum_vector.tree);
                            expect(me.vector_sub(first_vector.tuples_to_vectors(), second_vector.tuples_to_vectors()).tree).toEqual(sum_vector.tree);
                            })
                         })
         })

describe("dot product", function() {
         var vectors = [
                        ['(1,2)', '(5,6)', '17'],
                        ['(1,2,3)', '(x,y,z)', 'x+2y+3z'],
                        ['(1,2,3,4)', '(5,2,1,2)', '20']
                        ]
         
         vectors.forEach(function(example) {
                         it(example, function() {
                            let first_vector = me.fromText(example[0]);
                            let second_vector = me.fromText(example[1]);
                            let dot = me.fromText(example[2]).simplify();
                            expect(me.dot_prod(first_vector, second_vector).tree).toEqual(dot.tree);
                            expect(first_vector.dot_prod(second_vector).tree).toEqual(dot.tree);
                            expect(me.dot_prod(first_vector.tuples_to_vectors(), second_vector.tuples_to_vectors()).tree).toEqual(dot.tree);
                            })
                         })
         })

describe("cross product", function(){
         var vectors = [
                        ['(1,0)', '(0,1)', '1'],
                        ['(0,1)', '(1,0)', '-1'],
                        ['(1,0,0)', '(0,1,0)', '(0,0,1)'],
                        ['(0,1,0)', '(1,0,0)', '(0,0,-1)'],
                        ['(0,0,1)', '(1,0,0)', '(0,1,0)']
                        ]
         vectors.forEach(function(example) {
                         it(example, function() {
                            let first_vector = me.fromText(example[0]);
                            let second_vector = me.fromText(example[1]);
                            let cross_vector = me.fromText(example[2]).tuples_to_vectors().simplify();
                            expect(me.cross_prod(first_vector, second_vector).tree).toEqual(cross_vector.tree);
                            expect(me.cross_prod(first_vector.tuples_to_vectors(), second_vector.tuples_to_vectors()).tree).toEqual(cross_vector.tree);
                            })
                         })
         })


