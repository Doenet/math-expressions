function tuple(entries){
    var expression=[]
    expression.push('tuple');
    var len = entries.length;
    for (let i = 0; i < len; i++){
        expression.push(entries[i]);
    }
    return expression;
}

function matrix(entries){       //entries is an array of arrays of math expressions
    var expression=[];
    expression.push('matrix')
    var r = entries.length;
    var c = entries[0].length;
    for (let i = 1; i < r; i++){
        if (entries[i].length != c){      //check if columns are equal size
            throw new Error("Matrix dimensions mismatch");
        }
    }
    expression.push(tuple([r,c]));
    let theMatrix = [];
    for (let j = 0; j < r; j++){
        theMatrix.push(tuple(entries[j].map(function(v) {return v.tree;})));
    }
    expression.push(tuple(theMatrix))
    return expression;
}

export { matrix };
