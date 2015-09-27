
var MathExpression = (function() {

// lib/complex-number
var ___LIB_COMPLEX_NUMBER___ = (function(module) {
  



function ComplexNumber(real,imaginary) {
    this.real = real;
    this.imaginary = imaginary;
}


ComplexNumber.prototype = {
    
    real: 0,
    
    
    imaginary: 0,
    
    
    add: function() {
	if(arguments.length == 1)
	    return new ComplexNumber(this.real + arguments[0].real, this.imaginary + arguments[0].imaginary);
	else
	    return new ComplexNumber(this.real + arguments[0], this.imaginary + arguments[1]);
    },

    
    sum: function() {
	return new ComplexNumber(this.real + arguments[0].real, this.imaginary + arguments[0].imaginary);
    },

    
    subtract: function() { 
	if(arguments.length == 1)
	    return new ComplexNumber(this.real - arguments[0].real, this.imaginary - arguments[0].imaginary);
	else
	    return new ComplexNumber(this.real - arguments[0], this.imaginary - arguments[1]);
    },

    
    multiply: function() {
	var multiplier = arguments[0];

	if (arguments.length != 1)
	    multiplier = new ComplexNumber(arguments[0], arguments[1]);

	return new ComplexNumber(this.real * multiplier.real - this.imaginary * multiplier.imaginary, 
				 this.real * multiplier.imaginary + this.imaginary * multiplier.real);
    },

    
    modulus: function() {
	return Math.sqrt(this.real * this.real + this.imaginary * this.imaginary);
    },

    
    argument: function() {
	return Math.atan2( this.imaginary, this.real ) + Math.PI;
    },

    
    toString: function() {
	return this.real + " + " + this.imaginary + "i";
    },

    real_part: function() {
	return this.real;
    },

    imaginary_part: function() {
	return this.imaginary;
    },

    negate: function() {
	return new ComplexNumber( -this.real, -this.imaginary );
    },

    conjugate: function() {
	return new ComplexNumber( this.real, -this.imaginary );
    },

    exp: function() {
	var this_exp = Math.exp( this.real );

	return new ComplexNumber( (this_exp * Math.cos( this.imaginary )),
				  (this_exp * Math.sin( this.imaginary )) );
    },

    log: function() {
	var this_modulus = Math.log(Math.sqrt( this.real * this.real + this.imaginary * this.imaginary ));
	var this_argument = Math.atan2( this.imaginary, this.real );

	return new ComplexNumber( this_modulus, this_argument );
    },

    cos: function() {
	var this_exp_i = Math.exp( + this.imaginary );
	var this_exp_minus_i = Math.exp(-( this.imaginary ));

	return new ComplexNumber( (Math.cos(  this.real )*( this_exp_minus_i + this_exp_i )/2.0),
				  (Math.sin(  this.real )*( this_exp_minus_i - this_exp_i )/2.0) );
    },

    sin: function() {
	var this_exp_i = Math.exp( this.imaginary );
	var this_exp_minus_i = Math.exp(-( this.imaginary ));

	return new ComplexNumber( (Math.sin( this.real )*( this_exp_i + this_exp_minus_i )/2.0),
				  (Math.cos( this.real )*( this_exp_i - this_exp_minus_i )/2.0) );
    },

    power: function(other) {
	var this_log_modulus = Math.log(Math.sqrt( this.real * this.real + this.imaginary * this.imaginary ));
	var this_argument = Math.atan2( this.imaginary , this.real );
	var this_new_log_modulus =other.real * this_log_modulus - other.imaginary * this_argument;
	var this_new_argument =other.real * this_argument + other.imaginary * this_log_modulus;

	return new ComplexNumber( (Math.exp( this_new_log_modulus ) * Math.cos( this_new_argument )),
				  (Math.exp( this_new_log_modulus ) * Math.sin( this_new_argument )) );
    },

    sqrt: function() {
	return this.power( new ComplexNumber(0.5,0) );
    },

    divide: function(other) {
	var denominator = other.real * other.real + other.imaginary * other.imaginary;

	return new ComplexNumber( (( this.real * other.real + this.imaginary * other.imaginary ) / ( denominator )),
				  (( this.imaginary *other.real - this.real * other.imaginary ) / ( denominator )) );
    },

    reciprocal: function() {
	return (new ComplexNumber(1,0)).divide( this );
    },

    tan: function() {
	return this.sin().divide( this.cos() );
    },

    sec: function() {
	var one = new ComplexNumber(1,0);
	return one.divide( this.cos() );
    },

    csc: function() {
	var one = new ComplexNumber(1,0);
	return one.divide( this.sin() );
    },

    cot: function() {
	var one = new ComplexNumber(1,0);
	return one.divide( this.tan() );
    },
    
    arcsin: function() {
	var minus_i = new ComplexNumber(0,-1);
	var i = new ComplexNumber(0,1);
	var one = new ComplexNumber(1,0);

	return ((i.multiply( this )).sum(  one.subtract( this.multiply( this ) ).sqrt() )).log().multiply( minus_i );
    },

    arccos: function() {
	var half_pi = new ComplexNumber( Math.PI / 2, 0 );

	return half_pi.subtract( this.arcsin() );
    },

    arctan: function() {
	var minus_i = new ComplexNumber(0,-1);
	var i = new ComplexNumber(0,1);
	var half_i = new ComplexNumber(0,0.5);
	var one = new ComplexNumber(1,0);

	return (one.subtract( i.multiply( this ) ).log()).subtract( 
	    one.sum( i.multiply( this ) ).log()
	).multiply( half_i );
    },

    jasmineToString: function() {
	return this.toString();
    },

    // http://en.wikipedia.org/wiki/Lanczos_approximation
    gamma: function() {
	// Coefficients
	var p = [ 676.5203681218851, -1259.1392167224028, 771.32342877765313, -176.61502916214059,12.507343278686905, -0.13857109526572012, 9.9843695780195716e-6, 1.5056327351493116e-7];

	var result;
	var z = this;
	
	var one = new ComplexNumber( 1, 0 );
	
	// Reflection formula
	if (this.real < 0.5) {
	    var pi = new ComplexNumber( Math.PI, 0 );
            result = pi.divide(z.multiply(pi).sin().multiply(one.subtract(z).gamma()));
	} else {
	    z = z.subtract(one);
	    x = new ComplexNumber(0.99999999999980993,0);

	    p.forEach( function(pval,i) {
		x = x.add( (new ComplexNumber(pval,0)).divide(z.add( new ComplexNumber(i+1,0) )) );
	    });
 
            var t = z.add( new ComplexNumber( p.length - 0.5, 0 ) );
	    var sqrt2pi = new ComplexNumber( Math.sqrt(2*Math.PI), 0 );
	    
	    result = sqrt2pi.multiply( t.power(z.add(new ComplexNumber(0.5,0))) ).multiply( t.negate().exp() ).multiply( x );
	}

	return result;
    },

    factorial: function() {
	return this.add( new ComplexNumber(1,0) ).gamma();
    },
    
			 
};

exports.ComplexNumber = ComplexNumber;

  return module.exports;
})({exports: {}});
// lib/lexers/latex
var ___LIB_LEXERS_LATEX___ = (function(module) {
  

var parser = (function(){
var o=function(k,v,o,l){for(o=o||{},l=k.length;l--;o[k[l]]=v);return o};
var parser = {trace: function trace() { },
yy: {},
symbols_: {"error":2,"empty":3,"EOF":4,"$accept":0,"$end":1},
terminals_: {2:"error",4:"EOF"},
productions_: [0,[3,1]],
performAction: function anonymous(yytext, yyleng, yylineno, yy, yystate , $$ , _$ ) {


var $0 = $$.length - 1;
switch (yystate) {
}
},
table: [{3:1,4:[1,2]},{1:[3]},{1:[2,1]}],
defaultActions: {2:[2,1]},
parseError: function parseError(str, hash) {
    if (hash.recoverable) {
        this.trace(str);
    } else {
        throw new Error(str);
    }
},
parse: function parse(input) {
    var self = this, stack = [0], tstack = [], vstack = [null], lstack = [], table = this.table, yytext = '', yylineno = 0, yyleng = 0, recovering = 0, TERROR = 2, EOF = 1;
    var args = lstack.slice.call(arguments, 1);
    var lexer = Object.create(this.lexer);
    var sharedState = { yy: {} };
    for (var k in this.yy) {
        if (Object.prototype.hasOwnProperty.call(this.yy, k)) {
            sharedState.yy[k] = this.yy[k];
        }
    }
    lexer.setInput(input, sharedState.yy);
    sharedState.yy.lexer = lexer;
    sharedState.yy.parser = this;
    if (typeof lexer.yylloc == 'undefined') {
        lexer.yylloc = {};
    }
    var yyloc = lexer.yylloc;
    lstack.push(yyloc);
    var ranges = lexer.options && lexer.options.ranges;
    if (typeof sharedState.yy.parseError === 'function') {
        this.parseError = sharedState.yy.parseError;
    } else {
        this.parseError = Object.getPrototypeOf(this).parseError;
    }
    function popStack(n) {
        stack.length = stack.length - 2 * n;
        vstack.length = vstack.length - n;
        lstack.length = lstack.length - n;
    }
    _token_stack:
        function lex() {
            var token;
            token = lexer.lex() || EOF;
            if (typeof token !== 'number') {
                token = self.symbols_[token] || token;
            }
            return token;
        }
    var symbol, preErrorSymbol, state, action, a, r, yyval = {}, p, len, newState, expected;
    while (true) {
        state = stack[stack.length - 1];
        if (this.defaultActions[state]) {
            action = this.defaultActions[state];
        } else {
            if (symbol === null || typeof symbol == 'undefined') {
                symbol = lex();
            }
            action = table[state] && table[state][symbol];
        }
                    if (typeof action === 'undefined' || !action.length || !action[0]) {
                var errStr = '';
                expected = [];
                for (p in table[state]) {
                    if (this.terminals_[p] && p > TERROR) {
                        expected.push('\'' + this.terminals_[p] + '\'');
                    }
                }
                if (lexer.showPosition) {
                    errStr = 'Parse error on line ' + (yylineno + 1) + ':\n' + lexer.showPosition() + '\nExpecting ' + expected.join(', ') + ', got \'' + (this.terminals_[symbol] || symbol) + '\'';
                } else {
                    errStr = 'Parse error on line ' + (yylineno + 1) + ': Unexpected ' + (symbol == EOF ? 'end of input' : '\'' + (this.terminals_[symbol] || symbol) + '\'');
                }
                this.parseError(errStr, {
                    text: lexer.match,
                    token: this.terminals_[symbol] || symbol,
                    line: lexer.yylineno,
                    loc: yyloc,
                    expected: expected
                });
            }
        if (action[0] instanceof Array && action.length > 1) {
            throw new Error('Parse Error: multiple actions possible at state: ' + state + ', token: ' + symbol);
        }
        switch (action[0]) {
        case 1:
            stack.push(symbol);
            vstack.push(lexer.yytext);
            lstack.push(lexer.yylloc);
            stack.push(action[1]);
            symbol = null;
            if (!preErrorSymbol) {
                yyleng = lexer.yyleng;
                yytext = lexer.yytext;
                yylineno = lexer.yylineno;
                yyloc = lexer.yylloc;
                if (recovering > 0) {
                    recovering--;
                }
            } else {
                symbol = preErrorSymbol;
                preErrorSymbol = null;
            }
            break;
        case 2:
            len = this.productions_[action[1]][1];
            yyval.$ = vstack[vstack.length - len];
            yyval._$ = {
                first_line: lstack[lstack.length - (len || 1)].first_line,
                last_line: lstack[lstack.length - 1].last_line,
                first_column: lstack[lstack.length - (len || 1)].first_column,
                last_column: lstack[lstack.length - 1].last_column
            };
            if (ranges) {
                yyval._$.range = [
                    lstack[lstack.length - (len || 1)].range[0],
                    lstack[lstack.length - 1].range[1]
                ];
            }
            r = this.performAction.apply(yyval, [
                yytext,
                yyleng,
                yylineno,
                sharedState.yy,
                action[1],
                vstack,
                lstack
            ].concat(args));
            if (typeof r !== 'undefined') {
                return r;
            }
            if (len) {
                stack = stack.slice(0, -1 * len * 2);
                vstack = vstack.slice(0, -1 * len);
                lstack = lstack.slice(0, -1 * len);
            }
            stack.push(this.productions_[action[1]][0]);
            vstack.push(yyval.$);
            lstack.push(yyval._$);
            newState = table[stack[stack.length - 2]][stack[stack.length - 1]];
            stack.push(newState);
            break;
        case 3:
            return true;
        }
    }
    return true;
}};

var lexer = (function(){
var lexer = ({

EOF:1,

parseError:function parseError(str, hash) {
        if (this.yy.parser) {
            this.yy.parser.parseError(str, hash);
        } else {
            throw new Error(str);
        }
    },


setInput:function (input, yy) {
        this.yy = yy || this.yy || {};
        this._input = input;
        this._more = this._backtrack = this.done = false;
        this.yylineno = this.yyleng = 0;
        this.yytext = this.matched = this.match = '';
        this.conditionStack = ['INITIAL'];
        this.yylloc = {
            first_line: 1,
            first_column: 0,
            last_line: 1,
            last_column: 0
        };
        if (this.options.ranges) {
            this.yylloc.range = [0,0];
        }
        this.offset = 0;
        return this;
    },


input:function () {
        var ch = this._input[0];
        this.yytext += ch;
        this.yyleng++;
        this.offset++;
        this.match += ch;
        this.matched += ch;
        var lines = ch.match(/(?:\r\n?|\n).*/g);
        if (lines) {
            this.yylineno++;
            this.yylloc.last_line++;
        } else {
            this.yylloc.last_column++;
        }
        if (this.options.ranges) {
            this.yylloc.range[1]++;
        }

        this._input = this._input.slice(1);
        return ch;
    },


unput:function (ch) {
        var len = ch.length;
        var lines = ch.split(/(?:\r\n?|\n)/g);

        this._input = ch + this._input;
        this.yytext = this.yytext.substr(0, this.yytext.length - len);
        
        this.offset -= len;
        var oldLines = this.match.split(/(?:\r\n?|\n)/g);
        this.match = this.match.substr(0, this.match.length - 1);
        this.matched = this.matched.substr(0, this.matched.length - 1);

        if (lines.length - 1) {
            this.yylineno -= lines.length - 1;
        }
        var r = this.yylloc.range;

        this.yylloc = {
            first_line: this.yylloc.first_line,
            last_line: this.yylineno + 1,
            first_column: this.yylloc.first_column,
            last_column: lines ?
                (lines.length === oldLines.length ? this.yylloc.first_column : 0)
                 + oldLines[oldLines.length - lines.length].length - lines[0].length :
              this.yylloc.first_column - len
        };

        if (this.options.ranges) {
            this.yylloc.range = [r[0], r[0] + this.yyleng - len];
        }
        this.yyleng = this.yytext.length;
        return this;
    },


more:function () {
        this._more = true;
        return this;
    },


reject:function () {
        if (this.options.backtrack_lexer) {
            this._backtrack = true;
        } else {
            return this.parseError('Lexical error on line ' + (this.yylineno + 1) + '. You can only invoke reject() in the lexer when the lexer is of the backtracking persuasion (options.backtrack_lexer = true).\n' + this.showPosition(), {
                text: "",
                token: null,
                line: this.yylineno
            });

        }
        return this;
    },


less:function (n) {
        this.unput(this.match.slice(n));
    },


pastInput:function () {
        var past = this.matched.substr(0, this.matched.length - this.match.length);
        return (past.length > 20 ? '...':'') + past.substr(-20).replace(/\n/g, "");
    },


upcomingInput:function () {
        var next = this.match;
        if (next.length < 20) {
            next += this._input.substr(0, 20-next.length);
        }
        return (next.substr(0,20) + (next.length > 20 ? '...' : '')).replace(/\n/g, "");
    },


showPosition:function () {
        var pre = this.pastInput();
        var c = new Array(pre.length + 1).join("-");
        return pre + this.upcomingInput() + "\n" + c + "^";
    },


test_match:function (match, indexed_rule) {
        var token,
            lines,
            backup;

        if (this.options.backtrack_lexer) {
            
            backup = {
                yylineno: this.yylineno,
                yylloc: {
                    first_line: this.yylloc.first_line,
                    last_line: this.last_line,
                    first_column: this.yylloc.first_column,
                    last_column: this.yylloc.last_column
                },
                yytext: this.yytext,
                match: this.match,
                matches: this.matches,
                matched: this.matched,
                yyleng: this.yyleng,
                offset: this.offset,
                _more: this._more,
                _input: this._input,
                yy: this.yy,
                conditionStack: this.conditionStack.slice(0),
                done: this.done
            };
            if (this.options.ranges) {
                backup.yylloc.range = this.yylloc.range.slice(0);
            }
        }

        lines = match[0].match(/(?:\r\n?|\n).*/g);
        if (lines) {
            this.yylineno += lines.length;
        }
        this.yylloc = {
            first_line: this.yylloc.last_line,
            last_line: this.yylineno + 1,
            first_column: this.yylloc.last_column,
            last_column: lines ?
                         lines[lines.length - 1].length - lines[lines.length - 1].match(/\r?\n?/)[0].length :
                         this.yylloc.last_column + match[0].length
        };
        this.yytext += match[0];
        this.match += match[0];
        this.matches = match;
        this.yyleng = this.yytext.length;
        if (this.options.ranges) {
            this.yylloc.range = [this.offset, this.offset += this.yyleng];
        }
        this._more = false;
        this._backtrack = false;
        this._input = this._input.slice(match[0].length);
        this.matched += match[0];
        token = this.performAction.call(this, this.yy, this, indexed_rule, this.conditionStack[this.conditionStack.length - 1]);
        if (this.done && this._input) {
            this.done = false;
        }
        if (token) {
            return token;
        } else if (this._backtrack) {
            
            for (var k in backup) {
                this[k] = backup[k];
            }
            return false; 
        }
        return false;
    },


next:function () {
        if (this.done) {
            return this.EOF;
        }
        if (!this._input) {
            this.done = true;
        }

        var token,
            match,
            tempMatch,
            index;
        if (!this._more) {
            this.yytext = '';
            this.match = '';
        }
        var rules = this._currentRules();
        for (var i = 0; i < rules.length; i++) {
            tempMatch = this._input.match(this.rules[rules[i]]);
            if (tempMatch && (!match || tempMatch[0].length > match[0].length)) {
                match = tempMatch;
                index = i;
                if (this.options.backtrack_lexer) {
                    token = this.test_match(tempMatch, rules[i]);
                    if (token !== false) {
                        return token;
                    } else if (this._backtrack) {
                        match = false;
                        continue; 
                    } else {
                        
                        return false;
                    }
                } else if (!this.options.flex) {
                    break;
                }
            }
        }
        if (match) {
            token = this.test_match(match, rules[index]);
            if (token !== false) {
                return token;
            }
            
            return false;
        }
        if (this._input === "") {
            return this.EOF;
        } else {
            return this.parseError('Lexical error on line ' + (this.yylineno + 1) + '. Unrecognized text.\n' + this.showPosition(), {
                text: "",
                token: null,
                line: this.yylineno
            });
        }
    },


lex:function lex() {
        var r = this.next();
        if (r) {
            return r;
        } else {
            return this.lex();
        }
    },


begin:function begin(condition) {
        this.conditionStack.push(condition);
    },


popState:function popState() {
        var n = this.conditionStack.length - 1;
        if (n > 0) {
            return this.conditionStack.pop();
        } else {
            return this.conditionStack[0];
        }
    },


_currentRules:function _currentRules() {
        if (this.conditionStack.length && this.conditionStack[this.conditionStack.length - 1]) {
            return this.conditions[this.conditionStack[this.conditionStack.length - 1]].rules;
        } else {
            return this.conditions["INITIAL"].rules;
        }
    },


topState:function topState(n) {
        n = this.conditionStack.length - 1 - Math.abs(n || 0);
        if (n >= 0) {
            return this.conditionStack[n];
        } else {
            return "INITIAL";
        }
    },


pushState:function pushState(condition) {
        this.begin(condition);
    },


stateStackSize:function stateStackSize() {
        return this.conditionStack.length;
    },
options: {},
performAction: function anonymous(yy,yy_,$avoiding_name_collisions,YY_START) {
var YYSTATE=YY_START;
switch($avoiding_name_collisions) {
case 0:
break;
case 1:return 'NUMBER'
break;
case 2:return 'NUMBER'
break;
case 3:return '*'
break;
case 4:return '/'
break;
case 5:return '-'
break;
case 6:return '+'
break;
case 7:return '^'
break;
case 8:return '('
break;
case 9:return '('
break;
case 10:return ')'
break;
case 11:return '['
break;
case 12:return ']'
break;
case 13:return '['
break;
case 14:return ']'
break;
case 15:return '|'
break;
case 16:return '|'
break;
case 17:return '|'
break;
case 18:return ')'
break;
case 19:return '{'
break;
case 20:return '}'
break;
case 21:return '*'
break;
case 22:return 'PI'
break;
case 23:return 'FRAC'
break;
case 24:return 'PI'
break;
case 25:return 'SIN'
break;
case 26:return 'COS'
break;
case 27:return 'TAN'
break;
case 28:return 'CSC'
break;
case 29:return 'SEC'
break;
case 30:return 'COT'
break;
case 31:return 'SIN'
break;
case 32:return 'COS'
break;
case 33:return 'TAN'
break;
case 34:return 'CSC'
break;
case 35:return 'SEC'
break;
case 36:return 'COT'
break;
case 37:return 'PI'
break;
case 38:return 'THETA'
break;
case 39:return 'ARCSIN'
break;
case 40:return 'ARCCOS'
break;
case 41:return 'ARCTAN'
break;
case 42:return 'ARCSEC'
break;
case 43:return 'ARCCSC'
break;
case 44:return 'ARCCOT'
break;
case 45:return 'ARCSIN'
break;
case 46:return 'ARCCOS'
break;
case 47:return 'ARCTAN'
break;
case 48:return 'LOG'
break;
case 49:return 'LOG'
break;
case 50:return 'EXP'
break;
case 51:return 'SQRT'
break;
case 52:return 'VAR'
break;
case 53:return 4
break;
case 54:return 'INVALID'
break;
}
},
rules: [/^(?:(\s+|\\,))/,/^(?:[0-9]+(\.[0-9]+)?)/,/^(?:[,.][0-9]+)/,/^(?:\*)/,/^(?:\/)/,/^(?:-)/,/^(?:\+)/,/^(?:\^)/,/^(?:\()/,/^(?:\\left\()/,/^(?:\\right\))/,/^(?:\\left\[)/,/^(?:\\right\])/,/^(?:\[)/,/^(?:\])/,/^(?:\\left\|)/,/^(?:\\right\|)/,/^(?:\|)/,/^(?:\))/,/^(?:\{)/,/^(?:\})/,/^(?:\\cdot\b)/,/^(?:\\pi\b)/,/^(?:\\frac\b)/,/^(?:\\pi\b)/,/^(?:\\sin\b)/,/^(?:\\cos\b)/,/^(?:\\tan\b)/,/^(?:\\csc\b)/,/^(?:\\sec\b)/,/^(?:\\cot\b)/,/^(?:\\sin\b)/,/^(?:\\cos\b)/,/^(?:\\tan\b)/,/^(?:\\csc\b)/,/^(?:\\sec\b)/,/^(?:\\cot\b)/,/^(?:\\pi\b)/,/^(?:\\theta\b)/,/^(?:\\arcsin\b)/,/^(?:\\arccos\b)/,/^(?:\\arctan\b)/,/^(?:\\arcsec\b)/,/^(?:\\arccsc\b)/,/^(?:\\arccot\b)/,/^(?:\\asin\b)/,/^(?:\\acos\b)/,/^(?:\\atan\b)/,/^(?:\\log\b)/,/^(?:\\ln\b)/,/^(?:\\exp\b)/,/^(?:\\sqrt\b)/,/^(?:[A-Za-z])/,/^(?:$)/,/^(?:.)/],
conditions: {"INITIAL":{"rules":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54],"inclusive":true}}
});
return lexer;
})();
parser.lexer = lexer;
function Parser () {
  this.yy = {};
}
Parser.prototype = parser;parser.Parser = Parser;
return new Parser;
})();


if (typeof require !== 'undefined' && typeof exports !== 'undefined') {
exports.parser = parser;
exports.Parser = parser.Parser;
exports.parse = function () { return parser.parse.apply(parser, arguments); };
exports.main = function commonjsMain(args) {
    if (!args[1]) {
        console.log('Usage: '+args[0]+' FILE');
        process.exit(1);
    }
    return exports.parser.parse(source);
};
if (typeof module !== 'undefined' && require.main === module) {
  exports.main(process.argv.slice(1));
}
}

module.exports.parser = parser;
module.exports.Parser = parser.Parser;
module.exports.parse = function () { return parser.parse.apply(parser, arguments); };
    
  return module.exports;
})({exports: {}});
// lib/latex-to-ast
var ___LIB_LATEX_TO_AST___ = (function(module) {
  





    var Parser = ___LIB_LEXERS_LATEX___.Parser;
var lexer = new Parser();




lexer.parse('');
lexer = lexer.lexer;

var symbol = '';
    
function advance() {
    symbol = lexer.lex();
    
    if (symbol == 4)
	symbol = 'EOF';
    
    return symbol;
}
    
function yytext() {
    return lexer.yytext;
}

function parse(input) {
    lexer.setInput(input);
    advance();
    
    return expression();
}

    


    
function expression() {
    var lhs = term();
    
    while ((symbol == '+') || (symbol == '-')) {
	var operation = false;
	
	if (symbol == '+')
	    operation = '+';
	
	if (symbol == '-')
	    operation = '-';
	
	advance();
	
	var rhs = term();
	
	lhs = [operation, lhs, rhs];
    }
    
    return lhs;
}

function isFunctionSymbol( symbol )
{
    var functionSymbols = ['SIN', 'COS', 'TAN', 'CSC', 'SEC', 'COT', 'ARCSIN', 'ARCCOS', 'ARCTAN', 'ARCCOT', 'ARCCSC', 'ARCSEC', 'LOG', 'LOG', 'EXP', 'SQRT', 'ABS'];
    return (functionSymbols.indexOf(symbol) != -1);
}    

function term() {
    if (symbol == 'FRAC') {
	advance();
	
	if (symbol != '{') {
	    throw 'Expected {';
	}
	advance();	    
	
	var numerator = expression();
	
	if (symbol != '}') {
	    throw 'Expected }';
	}
	advance();
	
	if (symbol != '{') {
	    throw 'Expected {';
	}
	advance();	    
	
	var denominator = expression();
	
	if (symbol != '}') {
	    throw 'Expected }';
	}
	advance();
	
	return ['/', numerator, denominator];
    }
    
    var lhs = factor();
    
    var keepGoing = false;
    
    do {
	keepGoing = false;
	
	if (symbol == '*') {
	    advance();
	    lhs = ['*', lhs, factor()];
	    keepGoing = true;
	} else if (symbol == '/') {
	    advance();
	    lhs = ['/', lhs, factor()];
	    keepGoing = true;
	} else {
	    rhs = nonMinusFactor();
	    if (rhs != false) {
		lhs = ['*', lhs, rhs];
		keepGoing = true;
	    }
	}
    } while( keepGoing );
    
    return lhs;
}
    
function factor() {
    if (symbol == '-') {
	advance();
	return ['~', factor()];
    }

    if (symbol == '|') {
	advance();
	
	var result = expression();
	result = ['abs', result];
	    
	if (symbol != '|'){
	    throw 'Expected |';
	}
	advance();	    
	return result;
    }    
    
    return nonMinusFactor();
}

function nonMinusFactor() {
    var result = false;
    
    if (symbol == 'NUMBER') {
	result = parseFloat( yytext() );
	advance();
    } else if (symbol == 'VAR') {
	result = yytext();
	advance();
    } else if (symbol == 'PI') {
	result = "pi"
	advance();
    } else if (symbol == 'THETA') {
	result = "theta"
	advance();	
    } else if (isFunctionSymbol(symbol)) {
	var functionName = symbol.toLowerCase();
	advance();

	if (symbol == '{') {
	    advance();
	    var parameter = expression();
	    if (symbol != '}') {
		throw 'Expected }';
	    }
	    advance();
	    
	    result = [functionName, parameter];
	} else if (symbol == '(') {
	    advance();
	    var parameter = expression();
	    if (symbol != ')') {
		throw 'Expected )';	    
	    }
	    advance();
	    
	    result = [functionName, parameter];
	} else if (symbol == '^') {
	    advance();
	    var power = factor();
	    var parameter = factor();
	    result = ['^', [functionName, parameter], power];
	} else {
	    result = [functionName, factor()];
	}
    } else if (symbol == '(') {
	advance();
	var result = expression();
	if (symbol != ')') {
	    throw 'Expected )';	    
	}
	advance();
    } else if (symbol == '{') {
	advance();
	var result = expression();
	if (symbol != '}') {
	    throw 'Expected )';	    
	}
	advance();
    }
	
    if (symbol == '^') {
	advance();
	return ['^', result, factor()];
    }
    
    return result;
}


function parse(input) {
    lexer.setInput(input);
    advance();
    return expression();
}

exports.latexToAst = parse;

  return module.exports;
})({exports: {}});
// lib/lexers/text
var ___LIB_LEXERS_TEXT___ = (function(module) {
  

var parser = (function(){
var o=function(k,v,o,l){for(o=o||{},l=k.length;l--;o[k[l]]=v);return o};
var parser = {trace: function trace() { },
yy: {},
symbols_: {"error":2,"empty":3,"EOF":4,"$accept":0,"$end":1},
terminals_: {2:"error",4:"EOF"},
productions_: [0,[3,1]],
performAction: function anonymous(yytext, yyleng, yylineno, yy, yystate , $$ , _$ ) {


var $0 = $$.length - 1;
switch (yystate) {
}
},
table: [{3:1,4:[1,2]},{1:[3]},{1:[2,1]}],
defaultActions: {2:[2,1]},
parseError: function parseError(str, hash) {
    if (hash.recoverable) {
        this.trace(str);
    } else {
        throw new Error(str);
    }
},
parse: function parse(input) {
    var self = this, stack = [0], tstack = [], vstack = [null], lstack = [], table = this.table, yytext = '', yylineno = 0, yyleng = 0, recovering = 0, TERROR = 2, EOF = 1;
    var args = lstack.slice.call(arguments, 1);
    var lexer = Object.create(this.lexer);
    var sharedState = { yy: {} };
    for (var k in this.yy) {
        if (Object.prototype.hasOwnProperty.call(this.yy, k)) {
            sharedState.yy[k] = this.yy[k];
        }
    }
    lexer.setInput(input, sharedState.yy);
    sharedState.yy.lexer = lexer;
    sharedState.yy.parser = this;
    if (typeof lexer.yylloc == 'undefined') {
        lexer.yylloc = {};
    }
    var yyloc = lexer.yylloc;
    lstack.push(yyloc);
    var ranges = lexer.options && lexer.options.ranges;
    if (typeof sharedState.yy.parseError === 'function') {
        this.parseError = sharedState.yy.parseError;
    } else {
        this.parseError = Object.getPrototypeOf(this).parseError;
    }
    function popStack(n) {
        stack.length = stack.length - 2 * n;
        vstack.length = vstack.length - n;
        lstack.length = lstack.length - n;
    }
    _token_stack:
        function lex() {
            var token;
            token = lexer.lex() || EOF;
            if (typeof token !== 'number') {
                token = self.symbols_[token] || token;
            }
            return token;
        }
    var symbol, preErrorSymbol, state, action, a, r, yyval = {}, p, len, newState, expected;
    while (true) {
        state = stack[stack.length - 1];
        if (this.defaultActions[state]) {
            action = this.defaultActions[state];
        } else {
            if (symbol === null || typeof symbol == 'undefined') {
                symbol = lex();
            }
            action = table[state] && table[state][symbol];
        }
                    if (typeof action === 'undefined' || !action.length || !action[0]) {
                var errStr = '';
                expected = [];
                for (p in table[state]) {
                    if (this.terminals_[p] && p > TERROR) {
                        expected.push('\'' + this.terminals_[p] + '\'');
                    }
                }
                if (lexer.showPosition) {
                    errStr = 'Parse error on line ' + (yylineno + 1) + ':\n' + lexer.showPosition() + '\nExpecting ' + expected.join(', ') + ', got \'' + (this.terminals_[symbol] || symbol) + '\'';
                } else {
                    errStr = 'Parse error on line ' + (yylineno + 1) + ': Unexpected ' + (symbol == EOF ? 'end of input' : '\'' + (this.terminals_[symbol] || symbol) + '\'');
                }
                this.parseError(errStr, {
                    text: lexer.match,
                    token: this.terminals_[symbol] || symbol,
                    line: lexer.yylineno,
                    loc: yyloc,
                    expected: expected
                });
            }
        if (action[0] instanceof Array && action.length > 1) {
            throw new Error('Parse Error: multiple actions possible at state: ' + state + ', token: ' + symbol);
        }
        switch (action[0]) {
        case 1:
            stack.push(symbol);
            vstack.push(lexer.yytext);
            lstack.push(lexer.yylloc);
            stack.push(action[1]);
            symbol = null;
            if (!preErrorSymbol) {
                yyleng = lexer.yyleng;
                yytext = lexer.yytext;
                yylineno = lexer.yylineno;
                yyloc = lexer.yylloc;
                if (recovering > 0) {
                    recovering--;
                }
            } else {
                symbol = preErrorSymbol;
                preErrorSymbol = null;
            }
            break;
        case 2:
            len = this.productions_[action[1]][1];
            yyval.$ = vstack[vstack.length - len];
            yyval._$ = {
                first_line: lstack[lstack.length - (len || 1)].first_line,
                last_line: lstack[lstack.length - 1].last_line,
                first_column: lstack[lstack.length - (len || 1)].first_column,
                last_column: lstack[lstack.length - 1].last_column
            };
            if (ranges) {
                yyval._$.range = [
                    lstack[lstack.length - (len || 1)].range[0],
                    lstack[lstack.length - 1].range[1]
                ];
            }
            r = this.performAction.apply(yyval, [
                yytext,
                yyleng,
                yylineno,
                sharedState.yy,
                action[1],
                vstack,
                lstack
            ].concat(args));
            if (typeof r !== 'undefined') {
                return r;
            }
            if (len) {
                stack = stack.slice(0, -1 * len * 2);
                vstack = vstack.slice(0, -1 * len);
                lstack = lstack.slice(0, -1 * len);
            }
            stack.push(this.productions_[action[1]][0]);
            vstack.push(yyval.$);
            lstack.push(yyval._$);
            newState = table[stack[stack.length - 2]][stack[stack.length - 1]];
            stack.push(newState);
            break;
        case 3:
            return true;
        }
    }
    return true;
}};

var lexer = (function(){
var lexer = ({

EOF:1,

parseError:function parseError(str, hash) {
        if (this.yy.parser) {
            this.yy.parser.parseError(str, hash);
        } else {
            throw new Error(str);
        }
    },


setInput:function (input, yy) {
        this.yy = yy || this.yy || {};
        this._input = input;
        this._more = this._backtrack = this.done = false;
        this.yylineno = this.yyleng = 0;
        this.yytext = this.matched = this.match = '';
        this.conditionStack = ['INITIAL'];
        this.yylloc = {
            first_line: 1,
            first_column: 0,
            last_line: 1,
            last_column: 0
        };
        if (this.options.ranges) {
            this.yylloc.range = [0,0];
        }
        this.offset = 0;
        return this;
    },


input:function () {
        var ch = this._input[0];
        this.yytext += ch;
        this.yyleng++;
        this.offset++;
        this.match += ch;
        this.matched += ch;
        var lines = ch.match(/(?:\r\n?|\n).*/g);
        if (lines) {
            this.yylineno++;
            this.yylloc.last_line++;
        } else {
            this.yylloc.last_column++;
        }
        if (this.options.ranges) {
            this.yylloc.range[1]++;
        }

        this._input = this._input.slice(1);
        return ch;
    },


unput:function (ch) {
        var len = ch.length;
        var lines = ch.split(/(?:\r\n?|\n)/g);

        this._input = ch + this._input;
        this.yytext = this.yytext.substr(0, this.yytext.length - len);
        
        this.offset -= len;
        var oldLines = this.match.split(/(?:\r\n?|\n)/g);
        this.match = this.match.substr(0, this.match.length - 1);
        this.matched = this.matched.substr(0, this.matched.length - 1);

        if (lines.length - 1) {
            this.yylineno -= lines.length - 1;
        }
        var r = this.yylloc.range;

        this.yylloc = {
            first_line: this.yylloc.first_line,
            last_line: this.yylineno + 1,
            first_column: this.yylloc.first_column,
            last_column: lines ?
                (lines.length === oldLines.length ? this.yylloc.first_column : 0)
                 + oldLines[oldLines.length - lines.length].length - lines[0].length :
              this.yylloc.first_column - len
        };

        if (this.options.ranges) {
            this.yylloc.range = [r[0], r[0] + this.yyleng - len];
        }
        this.yyleng = this.yytext.length;
        return this;
    },


more:function () {
        this._more = true;
        return this;
    },


reject:function () {
        if (this.options.backtrack_lexer) {
            this._backtrack = true;
        } else {
            return this.parseError('Lexical error on line ' + (this.yylineno + 1) + '. You can only invoke reject() in the lexer when the lexer is of the backtracking persuasion (options.backtrack_lexer = true).\n' + this.showPosition(), {
                text: "",
                token: null,
                line: this.yylineno
            });

        }
        return this;
    },


less:function (n) {
        this.unput(this.match.slice(n));
    },


pastInput:function () {
        var past = this.matched.substr(0, this.matched.length - this.match.length);
        return (past.length > 20 ? '...':'') + past.substr(-20).replace(/\n/g, "");
    },


upcomingInput:function () {
        var next = this.match;
        if (next.length < 20) {
            next += this._input.substr(0, 20-next.length);
        }
        return (next.substr(0,20) + (next.length > 20 ? '...' : '')).replace(/\n/g, "");
    },


showPosition:function () {
        var pre = this.pastInput();
        var c = new Array(pre.length + 1).join("-");
        return pre + this.upcomingInput() + "\n" + c + "^";
    },


test_match:function (match, indexed_rule) {
        var token,
            lines,
            backup;

        if (this.options.backtrack_lexer) {
            
            backup = {
                yylineno: this.yylineno,
                yylloc: {
                    first_line: this.yylloc.first_line,
                    last_line: this.last_line,
                    first_column: this.yylloc.first_column,
                    last_column: this.yylloc.last_column
                },
                yytext: this.yytext,
                match: this.match,
                matches: this.matches,
                matched: this.matched,
                yyleng: this.yyleng,
                offset: this.offset,
                _more: this._more,
                _input: this._input,
                yy: this.yy,
                conditionStack: this.conditionStack.slice(0),
                done: this.done
            };
            if (this.options.ranges) {
                backup.yylloc.range = this.yylloc.range.slice(0);
            }
        }

        lines = match[0].match(/(?:\r\n?|\n).*/g);
        if (lines) {
            this.yylineno += lines.length;
        }
        this.yylloc = {
            first_line: this.yylloc.last_line,
            last_line: this.yylineno + 1,
            first_column: this.yylloc.last_column,
            last_column: lines ?
                         lines[lines.length - 1].length - lines[lines.length - 1].match(/\r?\n?/)[0].length :
                         this.yylloc.last_column + match[0].length
        };
        this.yytext += match[0];
        this.match += match[0];
        this.matches = match;
        this.yyleng = this.yytext.length;
        if (this.options.ranges) {
            this.yylloc.range = [this.offset, this.offset += this.yyleng];
        }
        this._more = false;
        this._backtrack = false;
        this._input = this._input.slice(match[0].length);
        this.matched += match[0];
        token = this.performAction.call(this, this.yy, this, indexed_rule, this.conditionStack[this.conditionStack.length - 1]);
        if (this.done && this._input) {
            this.done = false;
        }
        if (token) {
            return token;
        } else if (this._backtrack) {
            
            for (var k in backup) {
                this[k] = backup[k];
            }
            return false; 
        }
        return false;
    },


next:function () {
        if (this.done) {
            return this.EOF;
        }
        if (!this._input) {
            this.done = true;
        }

        var token,
            match,
            tempMatch,
            index;
        if (!this._more) {
            this.yytext = '';
            this.match = '';
        }
        var rules = this._currentRules();
        for (var i = 0; i < rules.length; i++) {
            tempMatch = this._input.match(this.rules[rules[i]]);
            if (tempMatch && (!match || tempMatch[0].length > match[0].length)) {
                match = tempMatch;
                index = i;
                if (this.options.backtrack_lexer) {
                    token = this.test_match(tempMatch, rules[i]);
                    if (token !== false) {
                        return token;
                    } else if (this._backtrack) {
                        match = false;
                        continue; 
                    } else {
                        
                        return false;
                    }
                } else if (!this.options.flex) {
                    break;
                }
            }
        }
        if (match) {
            token = this.test_match(match, rules[index]);
            if (token !== false) {
                return token;
            }
            
            return false;
        }
        if (this._input === "") {
            return this.EOF;
        } else {
            return this.parseError('Lexical error on line ' + (this.yylineno + 1) + '. Unrecognized text.\n' + this.showPosition(), {
                text: "",
                token: null,
                line: this.yylineno
            });
        }
    },


lex:function lex() {
        var r = this.next();
        if (r) {
            return r;
        } else {
            return this.lex();
        }
    },


begin:function begin(condition) {
        this.conditionStack.push(condition);
    },


popState:function popState() {
        var n = this.conditionStack.length - 1;
        if (n > 0) {
            return this.conditionStack.pop();
        } else {
            return this.conditionStack[0];
        }
    },


_currentRules:function _currentRules() {
        if (this.conditionStack.length && this.conditionStack[this.conditionStack.length - 1]) {
            return this.conditions[this.conditionStack[this.conditionStack.length - 1]].rules;
        } else {
            return this.conditions["INITIAL"].rules;
        }
    },


topState:function topState(n) {
        n = this.conditionStack.length - 1 - Math.abs(n || 0);
        if (n >= 0) {
            return this.conditionStack[n];
        } else {
            return "INITIAL";
        }
    },


pushState:function pushState(condition) {
        this.begin(condition);
    },


stateStackSize:function stateStackSize() {
        return this.conditionStack.length;
    },
options: {},
performAction: function anonymous(yy,yy_,$avoiding_name_collisions,YY_START) {
var YYSTATE=YY_START;
switch($avoiding_name_collisions) {
case 0:
break;
case 1:return 'NUMBER'
break;
case 2:return 'NUMBER'
break;
case 3:return '^'
break;
case 4:return '*' 
break;
case 5:return '*'
break;
case 6:return '*'
break;
case 7:return '*'
break;
case 8:return '*'
break;
case 9:return '*'
break;
case 10:return '/'
break;
case 11:return '-'
break;
case 12:return '-' 
break;
case 13:return '-'
break;
case 14:return '-'
break;
case 15:return '-'
break;
case 16:return '-'
break;
case 17:return '-'
break;
case 18:return '-'
break;
case 19:return '-'
break;
case 20:return '-'
break;
case 21:return '-'
break;
case 22:return '-'
break;
case 23:return '-'
break;
case 24:return '-'
break;
case 25:return '-'
break;
case 26:return '-'
break;
case 27:return '-'
break;
case 28:return '-'
break;
case 29:return '-'
break;
case 30:return '-'
break;
case 31:return '-'
break;
case 32:return '-'
break;
case 33:return '-'
break;
case 34:return '-'
break;
case 35:return '-'
break;
case 36:return '-'
break;
case 37:return '-'
break;
case 38:return '-'
break;
case 39:return '-'
break;
case 40:return '-'
break;
case 41:return '-'
break;
case 42:return '-'
break;
case 43:return '-'
break;
case 44:return '-'
break;
case 45:return '-'
break;
case 46:return '-'
break;
case 47:return '-'
break;
case 48:return '-'
break;
case 49:return '-'
break;
case 50:return '-'
break;
case 51:return '-'
break;
case 52:return '-'
break;
case 53:return '-'
break;
case 54:return '-'
break;
case 55:return '-'
break;
case 56:return '+'
break;
case 57:return '^' 
break;
case 58:return '^'
break;
case 59:return '^'
break;
case 60:return '^'
break;
case 61:return '^'
break;
case 62:return '^'
break;
case 63:return '|'
break;
case 64:return '('
break;
case 65:return ')'
break;
case 66:return '('
break;
case 67:return ')'
break;
case 68:return '('
break;
case 69:return ')'
break;
case 70:return 'PI'
break;
case 71:return 'SIN'
break;
case 72:return 'COS'
break;
case 73:return 'TAN'
break;
case 74:return 'CSC'
break;
case 75:return 'CSC'
break;
case 76:return 'SEC'
break;
case 77:return 'COT'
break;
case 78:return 'COT'
break;
case 79:return 'ARCSIN'
break;
case 80:return 'ARCCOS'
break;
case 81:return 'ARCTAN'
break;
case 82:return 'ARCCSC'
break;
case 83:return 'ARCSEC'
break;
case 84:return 'ARCCOT'
break;
case 85:return 'ARCSIN'
break;
case 86:return 'ARCCOS'
break;
case 87:return 'ARCTAN'
break;
case 88:return 'GAMMA'
break;
case 89:return 'ARCCSC'
break;
case 90:return 'ARCSEC'
break;
case 91:return 'ARCCOT'
break;
case 92:return 'LOG'
break;
case 93:return 'LOG'
break;
case 94:return 'LOG'
break;
case 95:return 'EXP'
break;
case 96:return 'SQRT'
break;
case 97:return 'ABS'
break;
case 98:return 'THETA'
break;
case 99:return 'THETA'
break;
case 100:return '!'
break;
case 101:return 'VAR'
break;
case 102:return 4
break;
case 103:return 4
break;
case 104:return 'INVALID'
break;
}
},
rules: [/^(?:\s+)/,/^(?:[0-9]+([,.][0-9]+)?)/,/^(?:[,.][0-9]+)/,/^(?:\*\*)/,/^(?:\*)/,/^(?:\\xB7)/,/^(?:\u00B7)/,/^(?:\u2022)/,/^(?:\u22C5)/,/^(?:\u00D7)/,/^(?:\/)/,/^(?:-)/,/^(?:\u002D)/,/^(?:\u007E)/,/^(?:\u00AD)/,/^(?:\u058A)/,/^(?:\u05BE)/,/^(?:\u1400)/,/^(?:\u1806)/,/^(?:\u2010)/,/^(?:\u2011)/,/^(?:\u2012)/,/^(?:\u2013)/,/^(?:\u2014)/,/^(?:\u2015)/,/^(?:\u207B)/,/^(?:\u208B)/,/^(?:\u2212)/,/^(?:\u2E17)/,/^(?:\u2E3A)/,/^(?:\u2E3B)/,/^(?:\u301C)/,/^(?:\u3030)/,/^(?:\u30A0)/,/^(?:\uFE31)/,/^(?:\uFE32)/,/^(?:\uFE58)/,/^(?:\uFE63)/,/^(?:\uFF0D)/,/^(?:\u002D)/,/^(?:\u007E)/,/^(?:\u00AD)/,/^(?:\u058A)/,/^(?:\u1806)/,/^(?:\u2010)/,/^(?:\u2011)/,/^(?:\u2012)/,/^(?:\u2013)/,/^(?:\u2014)/,/^(?:\u2015)/,/^(?:\u2053)/,/^(?:\u207B)/,/^(?:\u208B)/,/^(?:\u2212)/,/^(?:\u301C)/,/^(?:\u3030)/,/^(?:\+)/,/^(?:\^)/,/^(?:\u2038)/,/^(?:\u2041)/,/^(?:\u028C)/,/^(?:\u2227)/,/^(?:\u02C7)/,/^(?:\|)/,/^(?:\()/,/^(?:\))/,/^(?:\[)/,/^(?:\])/,/^(?:\{)/,/^(?:\})/,/^(?:pi\b)/,/^(?:sin\b)/,/^(?:cos\b)/,/^(?:tan\b)/,/^(?:csc\b)/,/^(?:cosec\b)/,/^(?:sec\b)/,/^(?:cot\b)/,/^(?:cotan\b)/,/^(?:arcsin\b)/,/^(?:arccos\b)/,/^(?:arctan\b)/,/^(?:arccsc\b)/,/^(?:arcsec\b)/,/^(?:arccot\b)/,/^(?:asin\b)/,/^(?:acos\b)/,/^(?:atan\b)/,/^(?:gamma\b)/,/^(?:acsc\b)/,/^(?:asec\b)/,/^(?:acot\b)/,/^(?:log\b)/,/^(?:lg\b)/,/^(?:ln\b)/,/^(?:exp\b)/,/^(?:sqrt\b)/,/^(?:abs\b)/,/^(?:theta\b)/,/^(?:\u03B8)/,/^(?:!)/,/^(?:[A-Za-z])/,/^(?:$)/,/^(?:EOF\b)/,/^(?:.)/],
conditions: {"INITIAL":{"rules":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,64,65,66,67,68,69,70,71,72,73,74,75,76,77,78,79,80,81,82,83,84,85,86,87,88,89,90,91,92,93,94,95,96,97,98,99,100,101,102,103,104],"inclusive":true}}
});
return lexer;
})();
parser.lexer = lexer;
function Parser () {
  this.yy = {};
}
Parser.prototype = parser;parser.Parser = Parser;
return new Parser;
})();


if (typeof require !== 'undefined' && typeof exports !== 'undefined') {
exports.parser = parser;
exports.Parser = parser.Parser;
exports.parse = function () { return parser.parse.apply(parser, arguments); };
exports.main = function commonjsMain(args) {
    if (!args[1]) {
        console.log('Usage: '+args[0]+' FILE');
        process.exit(1);
    }
    return exports.parser.parse(source);
};
if (typeof module !== 'undefined' && require.main === module) {
  exports.main(process.argv.slice(1));
}
}
    module.exports.parser = parser;
module.exports.Parser = parser.Parser;
module.exports.parse = function () { return parser.parse.apply(parser, arguments); };
    
  return module.exports;
})({exports: {}});
// lib/text-to-ast
var ___LIB_TEXT_TO_AST___ = (function(module) {
  





    var Parser = ___LIB_LEXERS_TEXT___.Parser;
    console.log( Parser );
var lexer = new Parser();




lexer.parse('');
lexer = lexer.lexer;

var symbol = '';

function advance() {
    symbol = lexer.lex();
    
    if (symbol == 4)
	symbol = 'EOF';
    
    return symbol;
}

function yytext() {
    return lexer.yytext;
}




function expression() {
    var lhs = term();
    
    while ((symbol == '+') || (symbol == '-')) {
	var operation = false;
	
	if (symbol == '+')
	    operation = '+';
	
	if (symbol == '-')
	    operation = '-';
	
	advance();
	
	var rhs = term();
	
	lhs = [operation, lhs, rhs];
    }
    
    return lhs;
}

function isFunctionSymbol( symbol )
{
    var functionSymbols = ['SIN', 'COS', 'TAN', 'CSC', 'SEC', 'COT', 'ARCSIN', 'ARCCOS', 'ARCTAN', 'ARCCSC', 'ARCSEC', 'ARCCOT', 'LOG', 'LN', 'EXP', 'SQRT', 'ABS', 'GAMMA'];
    return (functionSymbols.indexOf(symbol) != -1);
}    

function term() {
    var lhs = factor();

    var keepGoing = false;
    
    do {
	keepGoing = false;
	
	if (symbol == '*') {
	    advance();
	    lhs = ['*', lhs, factor()];
	    keepGoing = true;
	} else if (symbol == '/') {
	    advance();
	    lhs = ['/', lhs, factor()];
	    keepGoing = true;
	} else {
	    rhs = nonMinusFactor();
	    if (rhs !== false) {
		lhs = ['*', lhs, rhs];
		keepGoing = true;
	    }
	}
    } while( keepGoing );
    
    return lhs;
}

function factor() {
    if (symbol == '-') {
	advance();
	return ['~', factor()];
    }

    if (symbol == '|') {
	advance();
	
	var result = expression();
	result = ['abs', result];
	    
	if (symbol != '|') {
	    throw 'Expected |';
	}
	advance();	    
	return result;
    }
    
    return nonMinusFactor();
}

function nonMinusFactor() {
    var result = false;
    
    if (symbol == 'NUMBER') {
	result = parseFloat( yytext() );
	advance();
    } else if (symbol == 'VAR') {
	result = yytext();
	advance();
    } else if (symbol == 'PI') {
	result = "pi";
	advance();
    } else if (symbol == 'THETA') {
	result = "theta";
	advance();	
    } else if (isFunctionSymbol(symbol)) {
	var functionName = symbol.toLowerCase();
	advance();

	if (symbol == '(') {
	    advance();
	    var parameter = expression();
	    if (symbol != ')') {
		throw 'Expected )';
	    }
	    advance();

	    result = [functionName, parameter];
	} else if (symbol == '^') {
	    advance();
	    var power = factor();
	    var parameter = factor();
	    result = ['^', [functionName, parameter], power];
	} else {
	    result = [functionName, factor()];
	}
    } else if (symbol == '(') {
	advance();
	result = expression();
	if (symbol != ')') {
	    throw 'Expected )';	    
	}
	advance();
    }
    
    if (symbol == '^') {
	advance();
	return ['^', result, factor()];
    }

    if (symbol == '!') {
	advance();
	return ['factorial', result];
    }
    
    return result;
}


function associate_ast( tree, op ) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	return tree;
    }    
    
    var operator = tree[0];
    var operands = tree.slice(1);
    operands = operands.map( function(v,i) { 
	return associate_ast(v, op); } );
    
    if (operator == op) {
	var result = [];
	
	for( var i=0; i<operands.length; i++ ) {
	    if ((typeof operands[i] !== 'number') && (typeof operands[i] !== 'string') && (operands[i][0] === op)) {
		result = result.concat( operands[i].slice(1) );
	    } else {
		result.push( operands[i] );
	    }
	}
	
	operands = result;
    }
    
    return [operator].concat( operands );
}

function clean_ast( tree ) {
    tree = associate_ast( tree, '+' );
    tree = associate_ast( tree, '-' );
    tree = associate_ast( tree, '*' );
    return tree;
}

function parse(input) {
    lexer.setInput(input);
    advance();
    return clean_ast(expression());
}

exports.textToAst = parse;

  return module.exports;
})({exports: {}});
// lib/ast-to-text
var ___LIB_AST_TO_TEXT___ = (function(module) {
  



var operators = {
    "+": function(operands) { return operands.join( ' + ' ); },
    "-": function(operands) { return operands.join( ' - ' ); },
    "~": function(operands) { return "-" + operands.join( ' - ' ); },
    "*": function(operands) { return operands.join( " " ); },
    "/": function(operands) { return "" + operands[0] + "/" + operands[1]; },
    "^": function(operands) { return operands[0]  + "^" + operands[1] + ""; },
    "sin": function(operands) { return "sin " + operands[0]; },
    "cos": function(operands) { return "cos " + operands[0]; },
    "tan": function(operands) { return "tan " + operands[0]; },
    "arcsin": function(operands) { return "arcsin " + operands[0]; },
    "arccos": function(operands) { return "arccos " + operands[0]; },
    "arctan": function(operands) { return "arctan " + operands[0]; },
    "arccsc": function(operands) { return "arccsc " + operands[0]; },
    "arcsec": function(operands) { return "arcsec " + operands[0]; },
    "arccot": function(operands) { return "arccot " + operands[0]; },
    "csc": function(operands) { return "csc " + operands[0]; },
    "sec": function(operands) { return "sec " + operands[0]; },
    "cot": function(operands) { return "cot " + operands[0]; },
    "log": function(operands) { return "ln " + operands[0]; },
    "exp": function(operands) { return "exp " + operands[0]; },    
    "ln": function(operands) { return "ln " + operands[0]; },
    "sqrt": function(operands) { return "sqrt " + operands[0] + ""; },
    "abs": function(operands) { return "|" + operands[0] + "|"; },
    "apply": function(operands) { return operands[0] + "(" + operands[1] + ")"; },
    "factorial": function(operands) { return operands[0] + "!"; },
};



function expression(tree) {
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
	return term(tree);	
    }
    
    var operator = tree[0];
    var operands = tree.slice(1);
    
    if ((operator == '+') || (operator == '-')) {
	return operators[operator]( operands.map( function(v,i) { return factorWithParenthesesIfNegated(v); } ));
    }
    
    return term(tree);
}



function term(tree) {
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
	return factor(tree);	
    }
    
    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator == '*') {
	return operators[operator]( operands.map( function(v,i) {
	    var result = factorWithParenthesesIfNegated(v);
	    
	    if (result.toString().match( /^[0-9]/ ) && (i > 0))
		return ' * ' + result;
	    else
		return result;
	}));
    }
    
    if (operator == '/') {
	return operators[operator]( operands.map( function(v,i) { return factor(v); } ) );
    }
    
    return factor(tree);	
}



function isFunctionSymbol( symbol )
{
    var functionSymbols = ['sin', 'cos', 'tan', 'csc', 'sec', 'cot', 'arcsin', 'arccos', 'arctan', 'arccsc', 'arcsec', 'arccot', 'log', 'ln', 'exp', 'sqrt', 'abs', 'factorial'];
    return (functionSymbols.indexOf(symbol) != -1);
}

function factor(tree) {
    if (typeof tree === 'string') {
	return tree;
    }    
    
    if (typeof tree === 'number') {
	return tree;
    }
    
    var operator = tree[0];
    var operands = tree.slice(1);	

    
    if (operator === "abs") {
	return operators[operator]( operands.map( function(v,i) { return expression(v); } ));
    } else if (isFunctionSymbol(operator)) {
	return operators[operator]( operands.map( function(v,i) {
	    
	    
	    var result = factor(v);
	    if ((result.toString().length > 1) && (!(result.toString().match( /^\(/))) && (!(result.toString().match( /^\|/))))
		return '(' + result.toString() + ')';
	    else
		return result;
	}));
    }
    
    
    if (operator === "^") {
	if (operands[0][0] === "sin")
	    return "sin^" + factor(operands[1]) + " " + factor(operands[0][1]);
	if (operands[0][0] === "cos")
	    return "cos^" + factor(operands[1]) + " " + factor(operands[0][1]);
	if (operands[0][0] === "tan")
	    return "tan^" + factor(operands[1]) + " " + factor(operands[0][1]);
	if (operands[0][0] === "sec")
	    return "sec^" + factor(operands[1]) + " " + factor(operands[0][1]);
	if (operands[0][0] === "csc")
	    return "csc^" + factor(operands[1]) + " " + factor(operands[0][1]);
	if (operands[0][0] === "cot")
	    return "cot^" + factor(operands[1]) + " " + factor(operands[0][1]);
    }
    
    if (operator === "^") {
	return operators[operator]( operands.map( function(v,i) { return factor(v); } ) );
    }
    
    if (operator == '~') {
	return operators[operator]( operands.map( function(v,i) { return factor(v); } ) );
    }
    
    return '(' + expression(tree) + ')';
}

function factorWithParenthesesIfNegated(tree)
{
    var result = factor(tree);

    if (result.toString().match( /^-/ ))
	return '(' + result.toString() + ')';

    
    return result;
}

function astToText(tree) {
    return expression(tree);
}

exports.astToText = astToText;

  return module.exports;
})({exports: {}});
// lib/ast-to-latex
var ___LIB_AST_TO_LATEX___ = (function(module) {
  



var operators = {
    "+": function(operands) { return operands.join( ' + ' ); },
    "-": function(operands) { return operands.join( ' - ' ); },
    "~": function(operands) { return "-" + operands.join( ' - ' ); },
    "*": function(operands) { return operands.join( " \\, " ); },
    "/": function(operands) { return "\\frac{" + operands[0] + "}{" + operands[1] + "}"; },
    "^": function(operands) { return operands[0]  + "^{" + operands[1] + "}"; },
    "sin": function(operands) { return "\\sin " + operands[0]; },
    "cos": function(operands) { return "\\cos " + operands[0]; },
    "tan": function(operands) { return "\\tan " + operands[0]; },
    "arcsin": function(operands) { return "\\arcsin " + operands[0]; },
    "arccos": function(operands) { return "\\arccos " + operands[0]; },
    "arctan": function(operands) { return "\\arctan " + operands[0]; },
    "arccsc": function(operands) { return "\\arccsc " + operands[0]; },
    "arcsec": function(operands) { return "\\arcsec " + operands[0]; },
    "arccot": function(operands) { return "\\arccot " + operands[0]; },
    "csc": function(operands) { return "\\csc " + operands[0]; },
    "sec": function(operands) { return "\\sec " + operands[0]; },
    "cot": function(operands) { return "\\cot " + operands[0]; },
    "log": function(operands) { return "\\ln " + operands[0]; },
    "exp": function(operands) { return "e^{" + operands[0] + "}"; },
    "ln": function(operands) { return "\\ln " + operands[0]; },
    "sqrt": function(operands) { return "\\sqrt{" + operands[0] + "}"; },
    "factorial": function(operands) { return operands[0] + "!"; },
    "gamma": function(operands) { return "\\Gamma " + operands[0]; },
    "abs": function(operands) { return "\\left|" + operands[0] + "\\right|"; },
    "apply": function(operands) { return operands[0] + "(" + operands[1] + ")"; },
};



function expression(tree) {
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
	return term(tree);	
    }

    var operator = tree[0];
    var operands = tree.slice(1);
    
    if ((operator == '+') || (operator == '-')) {
	return operators[operator]( operands.map( function(v,i) { return term(v); } ) );
    }
    
    return term(tree);
}



function term(tree) {
    if ((typeof tree === 'string') || (typeof tree === 'number')) {
	return factor(tree);	
    }

    var operator = tree[0];
    var operands = tree.slice(1);
    
    if ((operator == '*') || (operator == '/')) {
	return operators[operator]( operands.map( function(v,i) { return factor(v); } ) );
    }
    
    return factor(tree);	
}



function isFunctionSymbol( symbol )
{
    var functionSymbols = ['sin', 'cos', 'tan', 'csc', 'sec', 'cot', 'arcsin', 'arccos', 'arctan', 'arccsc', 'arcsec', 'arccot', 'log', 'ln', 'exp', 'sqrt', 'factorial', 'gamma', 'abs'];
    return (functionSymbols.indexOf(symbol) != -1);
}


function factor(tree) {
    if (typeof tree === 'string') {
	if (tree == "pi") return "\\pi";
	if (tree == "theta") return "\\theta";	
	return tree;
    }    
    
    if (typeof tree === 'number') {
	return tree;
    }
    
    var operator = tree[0];
    var operands = tree.slice(1);	

    if (operator == 'sqrt') {
	return operators[operator]( operands.map( function(v,i) { return expression(v); } ) );
    }

    if (operator == 'gamma') {
	return operators[operator]( operands.map( function(v,i) { return '\\left(' + expression(v) + '\\right)'; } ) );
    }    

    if (isFunctionSymbol(operator)) {
	return operators[operator]( operands.map( function(v,i) { return factor(v); } ) );
    }

    
    if (operator === "^") {
	if (operands[0][0] === "sin")
	    return "\\sin^{" + factor(operands[1]) + "} " + factorWithParensIfNeeded(operands[0][1]);
	if (operands[0][0] === "cos")
	    return "\\cos^{" + factor(operands[1]) + "} " + factorWithParensIfNeeded(operands[0][1]);
	if (operands[0][0] === "tan")
	    return "\\tan^{" + factor(operands[1]) + "} " + factorWithParensIfNeeded(operands[0][1]);
	if (operands[0][0] === "sec")
	    return "\\sec^{" + factor(operands[1]) + "} " + factorWithParensIfNeeded(operands[0][1]);
	if (operands[0][0] === "csc")
	    return "\\csc^{" + factor(operands[1]) + "} " + factorWithParensIfNeeded(operands[0][1]);
	if (operands[0][0] === "cot")
	    return "\\cot^{" + factor(operands[1]) + "} " + factorWithParensIfNeeded(operands[0][1]);

	return operators[operator]( operands.map( function(v,i) { return factor(v); } ) );
    }

    if (operator == '~') {
	return operators[operator]( operands.map( function(v,i) { return factor(v); } ) );
    }

    return '\\left(' + expression(tree) + '\\right)';
}

function factorWithParensIfNeeded(tree) {
    return factor(tree);
}


function astToText(tree) {
    return expression(tree);
}

exports.astToLatex = astToText;

  return module.exports;
})({exports: {}});
// lib/ast-to-glsl
var ___LIB_AST_TO_GLSL___ = (function(module) {
  

var glslOperators = {
    "+": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(rhs) { result = result + "+" + rhs; }); return result; },
    "-": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(rhs) { result = result + "-" + rhs; }); return result; },
    "~": function(operands) { var result = "vec2(0.0,0.0)"; operands.forEach(function(rhs) { result = result + "-" + rhs; }); return result; },
    "*": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(rhs) { result = "cmul(" + result + "," + rhs + ")"; }); return result; },
    "/": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(rhs) { result = "cdiv(" + result + "," + rhs + ")"; }); return result; },

    "sin": function(operands) { return "csin(" + operands[0] + ")"; },
    "cos": function(operands) { return "ccos(" + operands[0] + ")"; },
    "tan": function(operands) { return "ctan(" + operands[0] + ")"; },

    "arcsin": function(operands) { return "carcsin(" + operands[0] + ")"; },
    "arccos": function(operands) { return "carccos(" + operands[0] + ")"; },
    "arctan": function(operands) { return "carctan(" + operands[0] + ")"; },

    "arccsc": function(operands) { return "carcsin(cdiv(vec2(1.0,0)," + operands[0] + "))"; },
    "arcsec": function(operands) { return "carccos(cdiv(vec2(1.0,0)," + operands[0] + "))"; },
    "arccot": function(operands) { return "carctan(cdiv(vec2(1.0,0)," + operands[0] + "))"; },

    "csc": function(operands) { return "ccsc(" + operands[0] + ")"; },
    "sec": function(operands) { return "csec(" + operands[0] + ")"; },
    "cot": function(operands) { return "ccot(" + operands[0] + ")"; },

    "exp": function(operands) { return "cexp(" + operands[0] + ")"; },    
    
    "sqrt": function(operands) { return "cpower(" + operands[0] + ",vec2(0.5,0.0))"; },
    "log": function(operands) { return "clog(" + operands[0] + ")"; },
    "ln": function(operands) { return "clog(" + operands[0] + ")"; },    
    "^": function(operands) { return "cpower(" + operands[0] + "," + operands[1] + ")"; },
    
    "abs": function(operands) { return "cabs(" + operands[0] + ")"; },
    "apply": function(operands) { return "vec2(NaN,NaN)"; },
};

function astToGlsl(tree, bindings) {
    if (typeof tree === 'string') {
	if (tree === "e")
	    return "vec2(2.71828182845905,0.0)";
	
	if (tree === "pi")
	    return "vec2(3.14159265358979,0.0)";
	
	if (tree === "i")
	    return "vec2(0.0,1.0)";

	if (bindings) {
	    if (tree in bindings)
		return "vec2(" + String(bindings[tree][0]) + "," + String(bindings[tree][1]) + ")";
	} else {
	    return "vec2(" + String(tree) + "," + String(0) + ")";
	}
	
	return tree;
    }    
    
    if (typeof tree === 'number') {
	return "vec2(" + String(tree) + ",0.0)";
    }
    
    if (("real" in tree) && ("imaginary" in tree))
	return tree;
    
    var operator = tree[0];
    var operands = tree.slice(1);
    
    if (operator in glslOperators) {
	return glslOperators[operator]( operands.map( function(v,i) { return astToGlsl(v,bindings); } ) );
    }
    
    return "vec2(NaN,NaN)";
}

exports.astToGlsl = astToGlsl;

  return module.exports;
})({exports: {}});
// lib/ast-to-function
var ___LIB_AST_TO_FUNCTION___ = (function(module) {
  




var math_functions = {
    "+": function(operands) { var result = 0; operands.forEach(function(x) { result += x; }); return result; },
    "-": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(x) { result -= x; }); return result; },
    "*": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(x) { result *= x; }); return result; },
    "/": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(x) { result /= x; }); return result; },
    "~": function(operands) { var result = 0; operands.forEach(function(x) { result -= x; }); return result; },
    "sin": function(operands) { return Math.sin(operands[0]); },
    "cos": function(operands) { return Math.cos(operands[0]); },
    "tan": function(operands) { return Math.tan(operands[0]); },
    "arcsin": function(operands) { return Math.asin(operands[0]); },
    "arccos": function(operands) { return Math.acos(operands[0]); },
    "arctan": function(operands) { return Math.atan(operands[0]); },
    "arccsc": function(operands) { return Math.asin(1.0/operands[0]); },
    "arcsec": function(operands) { return Math.acos(1.0/operands[0]); },
    "arccot": function(operands) { return Math.atan(1.0/operands[0]); },
    "csc": function(operands) { return 1.0/Math.sin(operands[0]); },
    "sec": function(operands) { return 1.0/Math.cos(operands[0]); },
    "cot": function(operands) { return 1.0/Math.tan(operands[0]); },
    "sqrt": function(operands) { return Math.sqrt(operands[0]); },
    "log": function(operands) { return Math.log(operands[0]); },
    "exp": function(operands) { return Math.exp(operands[0]); },    
    "^": function(operands) { return Math.pow(operands[0], operands[1]); },
    "abs": function(operands) { return Math.abs(operands[0]); },
    
    "factorial": function(operands) { return (new ComplexNumber(operands[0],0)).factorial().real_part(); },
    "gamma": function(operands) { return (new ComplexNumber(operands[0],0)).gamma().real_part(); },
    
    "apply": function(operands) { return NaN; },
};

function evaluate_ast(tree, bindings) {
    if (typeof tree === 'number') {
	return tree;
    }

    if (typeof tree === 'string') {
	if (tree === "e")
	    return Math.E;

	if (tree === "pi")
	    return Math.PI;

	if (tree in bindings)
	    return bindings[tree];
	
	return tree;
    }    
    
    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator in math_functions) {
	return math_functions[operator]( operands.map( function(v,i) { return evaluate_ast(v,bindings); } ) );
    }
    
    return NaN;
}

function astToFunction(tree) {
    return function(bindings) { return evaluate_ast( tree, bindings ); };
}

exports.astToFunction = astToFunction;

  return module.exports;
})({exports: {}});
// lib/ast-to-complex-function
var ___LIB_AST_TO_COMPLEX_FUNCTION___ = (function(module) {
  


var ComplexNumber = ___LIB_COMPLEX_NUMBER___.ComplexNumber;

var complex_math_functions = {
    "+": function(operands) { var result = new ComplexNumber(0,0); operands.forEach(function(v,i) { result = result.sum( v ); }); return result; },
    "-": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(v,i) { result = result.subtract( v ); }); return result; },
    "~": function(operands) { var result = new ComplexNumber(0,0); operands.forEach(function(v,i) { result = result.subtract( v ); }); return result; },
    "*": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(v,i) { result = result.multiply( v ); }); return result; },
    "/": function(operands) { var result = operands[0]; operands.slice(1).forEach(function(v,i) { result = result.divide( v ); }); return result; },

    "sin": function(operands) { return operands[0].sin(); },
    "cos": function(operands) { return operands[0].cos(); },
    "tan": function(operands) { return operands[0].tan(); },
    "arcsin": function(operands) { return operands[0].arcsin(); },
    "arccos": function(operands) { return operands[0].arccos(); },
    "arctan": function(operands) { return operands[0].arctan(); },
    "arccsc": function(operands) { return operands[0].reciprocal().arcsin(); },
    "arcsec": function(operands) { return operands[0].reciprocal().arccos(); },
    "arccot": function(operands) { return operands[0].reciprocal().arctan(); },

    "csc": function(operands) { return operands[0].csc(); },
    "sec": function(operands) { return operands[0].sec(); },
    "cot": function(operands) { return operands[0].cot(); },

    "sqrt": function(operands) { return operands[0].power( new ComplexNumber(0.5,0) ); },
    "log": function(operands) { return operands[0].log(); },
    "exp": function(operands) { return operands[0].exp(); },
    
    "factorial": function(operands) { return operands[0].factorial(); },
    "gamma": function(operands) { return operands[0].gamma(); },
    
    "^": function(operands) { return operands[0].power(operands[1]); },
    "abs": function(operands) {return operands[0].power(new ComplexNumber(2,0)).log().multiply(new ComplexNumber(.5,0)).exp();},
    "apply": function(operands) { return NaN; },
};

function complex_evaluate_ast(tree, bindings) {
    if (typeof tree === 'string') {
	if (tree === "e")
	    return new ComplexNumber( Math.E, 0 );

	if (tree === "pi")
	    return new ComplexNumber( Math.PI, 0 );

	if (tree === "i")
	    return new ComplexNumber( 0, 1 );

	if (tree in bindings)
	    return bindings[tree];
	
	return tree;
    }    

    if (typeof tree === 'number') {
	return new ComplexNumber( tree, 0 );
    }

    if (("real" in tree) && ("imaginary" in tree))
	return tree;
    
    var operator = tree[0];
    var operands = tree.slice(1);

    if (operator in complex_math_functions) {
	return complex_math_functions[operator]( operands.map( function(v,i) { return complex_evaluate_ast(v,bindings); } ) );
    }
    
    return new ComplexNumber( NaN,NaN );
}

function astToComplexFunction(tree) {
    return function(bindings) { return complex_evaluate_ast( tree, bindings ); };
}

exports.astToComplexFunction = astToComplexFunction;

  return module.exports;
})({exports: {}});
// lib/parser
var ___LIB_PARSER___ = (function(module) {
  

kinds = ['text', 'latex', 'ast', 'glsl', 'function', 'complexFunction'];


converters = {
    latex: {
	to: { 
	    ast: ___LIB_LATEX_TO_AST___.latexToAst,
	}
    },
    text: {
	to: { 
	    ast: ___LIB_TEXT_TO_AST___.textToAst,
	}
    },
    ast: {
	to: {
	    text: ___LIB_AST_TO_TEXT___.astToText,
	    latex: ___LIB_AST_TO_LATEX___.astToLatex,
	    glsl:  ___LIB_AST_TO_GLSL___.astToGlsl,
	    function:  ___LIB_AST_TO_FUNCTION___.astToFunction,
	    complexFunction: ___LIB_AST_TO_COMPLEX_FUNCTION___.astToComplexFunction,
	}
    }
};


var foundNew = true;

while( foundNew ) {
    foundNew = false;
    
    kinds.forEach( function(a) {
	if (a in converters) {
	    kinds.forEach( function(b) {
		if ((b in converters) && (b in converters[a].to)) {
		    kinds.forEach( function(c) {
			if ((c in converters[b].to) && (!(c in converters[a].to))) {
			    foundNew = true;
			    converters[a].to[c] = function(x) { return (converters[b].to[c])( (converters[a].to[b])(x) ); };
			}
		    });
		}
	    });
	}
    });
}


kinds.forEach( function(a) {
    exports[a] = converters[a];
});

  return module.exports;
})({exports: {}});
// lib/math-expressions
var ___LIB_MATH_EXPRESSIONS___ = (function(module) {
  var ComplexNumber = ___LIB_COMPLEX_NUMBER___.ComplexNumber;
var parser = ___LIB_PARSER___;
var textToAst = parser.text.to.ast;
var latexToAst = parser.latex.to.ast;
var astToLatex = ___LIB_PARSER___.ast.to.latex;
var astToFunction = ___LIB_PARSER___.ast.to.function;
var astToComplexFunction = ___LIB_PARSER___.ast.to.complexFunction;



function substitute_ast(tree, bindings) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	if (tree in bindings)
	    return bindings[tree];
	
	return tree;
    }    
    
    var operator = tree[0];
    var operands = tree.slice(1);
    
    var result = [operator].concat( operands.map( function(v,i) { return substitute_ast(v,bindings); } ) );
    return result;
};

function tree_match( haystack, needle ) {
    var match = {};

    if (typeof needle === 'string') {
	match[needle] = haystack;
	return match;
    }

    if (typeof haystack === 'number') {
	if (typeof needle === 'number') {
	    if (needle === haystack) {
		return {};
	    }
	}

	return null;
    }

    if (typeof haystack === 'string') {
	if (typeof needle === 'string') {
	    match[needle] = haystack;
	    return match;
	}

	return null;
    }

    var haystack_operator = haystack[0];
    var haystack_operands = haystack.slice(1);

    var needle_operator = needle[0];
    var needle_operands = needle.slice(1);

    if (haystack_operator === needle_operator) {
	if (haystack_operands.length >= needle_operands.length) {
	    var matches = {}

	    needle_operands.forEach( function(i) {
		var new_matches = tree_match( haystack_operands[i], needle_operands[i] );
		
		if (new_matches === null) {
		    matches = null;
		}

		if (matches != null) {
		    matches = $.extend( matches, new_matches );
		}
	    } );

	    if (matches != null) {
		matches = $.extend( matches, { remainder: haystack_operands.slice( needle_operands.length ) } );
	    }

	    return matches;
	}

	return null;
    }

    return null;
};

function subtree_matches(haystack, needle) {
    if (typeof haystack === 'number') {
	return (typeof needle === 'string');
    }    
    
    if (typeof haystack === 'string') {
	return (typeof needle === 'string');
    }    

    var match = tree_match( haystack, needle );
    if (match != null) {
	return true;
    }

    var operator = haystack[0];
    var operands = haystack.slice(1);

    var any_matches = false;

    $.each( operands, function(i) {
	if (subtree_matches(operands[i], needle))
	    any_matches = true;
    } );

    return any_matches;
};

function replace_subtree(haystack, needle, replacement) {
    if (typeof haystack === 'number') {
	return haystack;
    }    
    
    if (typeof haystack === 'string') {
	if (typeof needle === 'string')
	    if (needle === haystack)
		return replacement;
	
	return haystack;
    }    

    var match = tree_match( haystack, needle );
    if (match != null) {
	return substitute_ast( replacement, match ).concat( match.remainder );
    }

    var operator = haystack[0];
    var operands = haystack.slice(1);

    return [operator].concat( operands.map( function(v,i) { return replace_subtree(v, needle, replacement); } ) );
};

function associate_ast( tree, op ) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	return tree;
    }    

    var operator = tree[0];
    var operands = tree.slice(1);
    operands = operands.map( function(v,i) { 
	return associate_ast(v, op); } );

    if (operator == op) {
	var result = [];
	
	for( var i=0; i<operands.length; i++ ) {
	    if ((typeof operands[i] !== 'number') && (typeof operands[i] !== 'string') && (operands[i][0] === op)) {
		result = result.concat( operands[i].slice(1) );
	    } else {
		result.push( operands[i] );
	    }
	}

	operands = result;
    }

    return [operator].concat( operands );
}

function remove_identity( tree, op, identity ) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	return tree;
    }    

    var operator = tree[0];
    var operands = tree.slice(1);
    operands = operands.map( function(v,i) { return remove_identity(v, op, identity); } );

    if (operator == op) {
	operands = operands.filter( function (a) { return a != identity; });
	if (operands.length == 0)
	    operands = [identity];

	if (operands.length == 1)
	    return operands[0];
    }

    return [operator].concat( operands );
}

function remove_zeroes( tree ) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	return tree;
    }    

    var operator = tree[0];
    var operands = tree.slice(1);
    operands = operands.map( function(v,i) { return remove_zeroes(v); } );

    if (operator === "*") {
	for( var i=0; i<operands.length; i++ ) {
	    if (operands[i] === 0)
		return 0;
	}
    }

    return [operator].concat( operands );
}

function collapse_unary_minus( tree ) {
    if (typeof tree === 'number') {
	return tree;
    }    
    
    if (typeof tree === 'string') {
	return tree;
    }    

    var operator = tree[0];
    var operands = tree.slice(1);
    operands = operands.map( function(v,i) { return collapse_unary_minus(v); } );

    if (operator == "~") {
	if (typeof operands[0] === 'number')
	    return -operands[0];
    }

    return [operator].concat( operands );
}

function clean_ast( tree ) {
    tree = associate_ast( tree, '+' );
    tree = associate_ast( tree, '-' );
    tree = associate_ast( tree, '*' );
    tree = remove_identity( tree, '*', 1 );
    tree = collapse_unary_minus( tree );
    tree = remove_zeroes( tree );
    tree = remove_identity( tree, '+', 0 );
    
    return tree;
};





function leaves( tree ) {
    if (typeof tree === 'number') {
	return [tree];
    }

    if (typeof tree === 'string') {
	return [tree];
    }    

    var operator = tree[0];
    var operands = tree.slice(1);

    return operands.map( function(v,i) { return leaves(v); } )
	.reduce( function(a,b) { return a.concat(b); } );
}

function variables_in_ast( tree ) {
    var result = leaves( tree );

    result = result.filter( function(v,i) {
	return (typeof v === 'string') && (v != "e") && (v != "pi");
    });

    result = result.filter(function(itm,i,a){
	return i==result.indexOf(itm);
    });
    
    return result;
}










var derivatives = {
    "sin": textToAst('cos x'),
    "cos": textToAst('-(sin x)'),
    "tan": textToAst('(sec x)^2'),
    "cot": textToAst('-((csc x)^2)'),
    "sec": textToAst('(sec x)*(tan x)'),
    "csc": textToAst('-(csc x)*(cot x)'),
    "sqrt": textToAst('1/(2*sqrt(x))'),
    "log": textToAst('1/x'),
    "arcsin": textToAst('1/sqrt(1 - x^2)'),
    "arccos": textToAst('-1/sqrt(1 - x^2)'),
    "arctan": textToAst('1/(1 + x^2)'),
    "arccsc": textToAst('-1/(sqrt(-1/x^2 + 1)*x^2)'),
    "arcsec": textToAst('1/(sqrt(-1/x^2 + 1)*x^2)'),
    "arccot": textToAst('-1/(1 + x^2)'),
    "abs": textToAst('abs(x)/x'),
};

function derivative_of_ast(tree,x,story) {
    var ddx = '\\frac{d}{d' + x + '} ';

    
    if (typeof tree === 'number') {
	story.push( 'The derivative of a constant is zero, that is, \\(' + ddx + astToLatex(tree) + ' = 0\\).' );
	return 0;
    }

    
    if ((variables_in_ast(tree)).indexOf(x) < 0) {
	story.push( 'The derivative of a constant is zero, that is, \\(' + ddx + astToLatex(tree) + ' = 0\\).' );
	return 0;
    }	

    
    if (typeof tree === 'string') {
	if (x === tree) {
	    story.push( 'We know the derivative of the identity function is one, that is, \\(' + ddx + astToLatex(tree) + ' = 1\\).' );
	    return 1;
	}
	
	story.push( 'As far as \\(' + astToLatex(x) + '\\) is concerned, \\(' + astToLatex(tree) + '\\) is constant, so ' + ddx + astToLatex(tree) + ' = 0\\).' );
	return 0;
    }
    
    var operator = tree[0];
    var operands = tree.slice(1);

    
    if ((operator === '+') || (operator === '-') || (operator === '~')) {
	story.push( 'Using the sum rule, \\(' + ddx + astToLatex( tree ) + ' = ' + (operands.map( function(v,i) { return ddx + astToLatex(v); } )).join( ' + ' ) + '\\).' );
	var result = [operator].concat( operands.map( function(v,i) { return derivative_of_ast(v,x,story); } ) );
	result = clean_ast(result);
	story.push( 'So using the sum rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	return result;
    }
    
    
    if (operator === '*') {
	var non_numeric_operands = [];
	var numeric_operands = [];

	for( var i=0; i<operands.length; i++ ) {
	    if ((typeof operands[i] === 'number') || ((variables_in_ast(operands[i])).indexOf(x) < 0)) {
		any_numbers = true;
		numeric_operands.push( operands[i] );
	    } else {
		non_numeric_operands.push( operands[i] );
	    } 
	}

	if (numeric_operands.length > 0) {
	    if (non_numeric_operands.length == 0) {
		story.push( 'Since the derivative of a constant is zero, \\(' + ddx + astToLatex( tree ) + ' = 0.\\)' );
		var result = 0;
		return result;
	    }

	    var remaining = ['*'].concat( non_numeric_operands );
	    if (non_numeric_operands.length == 1) 
		remaining = non_numeric_operands[0];



	    if (remaining === x) {
		story.push( 'By the constant multiple rule, \\(' + ddx + astToLatex( tree ) + ' = ' + (numeric_operands.map( function(v,i) { return astToLatex(v); } )).join( ' \\cdot ' ) + '\\).' );
		var result = ['*'].concat( numeric_operands );
		result = clean_ast(result);
		return result;
	    }

	    story.push( 'By the constant multiple rule, \\(' + ddx + astToLatex( tree ) + ' = ' + (numeric_operands.map( function(v,i) { return astToLatex(v); } )).join( ' \\cdot ' ) + ' \\cdot ' + ddx + '\\left(' + astToLatex(remaining) + '\\right)\\).' );

	    var d = derivative_of_ast(remaining,x,story);
	    var result = ['*'].concat( numeric_operands.concat( [d] ) );
	    result = clean_ast(result);
	    story.push( 'And so \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;
	}

	story.push( 'Using the product rule, \\(' + ddx + astToLatex( tree ) + ' = ' +
		    (operands.map( function(v,i) {
			return (operands.map( function(w,j) {
			    if (i == j)
				return ddx + '\\left(' + astToLatex(v) + '\\right)';
			    else
				return astToLatex(w);
			})).join( ' \\cdot ' ) })).join( ' + ' ) + '\\).' );

	var inner_operands = operands.slice();

	var result = ['+'].concat( operands.map( function(v,i) {
	    return ['*'].concat( inner_operands.map( function(w,j) {
		if (i == j) {
		    var d = derivative_of_ast(w,x,story);
		    
		    if (d === 1)
			return null;

		    return d;
		} else {
		    return w;
		}
	    } ).filter( function(t) { return t != null; } ) );
	} ) );
	result = clean_ast(result);
	story.push( 'So using the product rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );

	return result;
    }
    
    
    if (operator === '/') {
	var f = operands[0];
	var g = operands[1];

	if ((variables_in_ast(g)).indexOf(x) < 0) {
	    story.push( 'By the constant multiple rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(['/', 1, g]) + ' \\cdot ' + ddx + '\\left(' + astToLatex(f) + '\\right)\\).' );

	    var df = derivative_of_ast(f,x,story);		
	    var quotient_rule = textToAst('(1/g)*d');
	    var result = substitute_ast( quotient_rule, { "d": df, "g": g } );
	    result = clean_ast(result);
	    story.push( 'So \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    
	    return result;		
	}

	if ((variables_in_ast(f)).indexOf(x) < 0) {
	    if (f !== 1) {
		story.push( 'By the constant multiple rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(f) + ' \\cdot ' + ddx + '\\left(' + astToLatex(['/',1,g]) + '\\right)\\).' );
	    }

	    story.push( 'Since \\(\\frac{d}{du} \\frac{1}{u}\\) is \\(\\frac{-1}{u^2}\\), the chain rule gives \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(f) + '\\cdot \\frac{-1}{ ' + astToLatex(g) + '^2' + '} \\cdot ' + ddx + astToLatex( g ) + "\\)." );

	    var a = derivative_of_ast(g,x,story);

	    var quotient_rule = textToAst('f * (-a/(g^2))');
	    var result = substitute_ast( quotient_rule, { "f": f, "a": a, "g": g } );
	    result = clean_ast(result);
	    story.push( 'So since \\(\\frac{d}{du} \\frac{1}{u}\\) is \\(\\frac{-1}{u^2}\\), the chain rule gives \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );

	    return result;
	}

	story.push( 'Using the quotient rule, \\(' + ddx + astToLatex( tree ) + ' = \\frac{' + ddx + '\\left(' + astToLatex(f) + '\\right) \\cdot ' + astToLatex(g) + ' - ' + astToLatex(f) + '\\cdot ' + ddx + '\\left(' + astToLatex(g) + '\\right)}{ \\left( ' + astToLatex(g) + ' \\right)^2} \\).' );

	var a = derivative_of_ast(f,x,story);
	var b = derivative_of_ast(g,x,story);
	var f_prime = a;
	var g_prime = b;

	var quotient_rule = textToAst('(a * g - f * b)/(g^2)');

	var result = substitute_ast( quotient_rule, { "a": a, "b": b, "f": f, "g": g } );
	result = clean_ast(result);
	story.push( 'So using the quotient rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );

	return result;
    }
    
    
    if (operator === '^') {
	var base = operands[0];
	var exponent = operands[1];
	
	if ((variables_in_ast(exponent)).indexOf(x) < 0) {
	    if ((typeof base === 'string') && (base === 'x')) {
		if (typeof exponent === 'number') {
		    var power_rule = textToAst('n * (f^m)');
		    var result = substitute_ast( power_rule, { "n": exponent, "m": exponent - 1, "f": base } );
		    result = clean_ast(result);
		    story.push( 'By the power rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex( exponent ) + ' \\cdot \\left(' + astToLatex( base ) + '\\right)^{' + astToLatex( ['-', exponent, 1] ) + '}\\).' );
		    return result;
		}

		var power_rule = textToAst('n * (f^(n-1))');
		var result = substitute_ast( power_rule, { "n": exponent, "f": base } );
		result = clean_ast(result);
		story.push( 'By the power rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex( exponent ) + ' \\cdot \\left(' + astToLatex( base ) + '\\right)^{' + astToLatex( ['-', exponent, 1] ) + '}\\).' );

		return result;
	    }

	    if (exponent != 1) {
		story.push( 'By the power rule and the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex( exponent ) + ' \\cdot \\left(' + astToLatex( base ) + '\\right)^{' + astToLatex( ['-', exponent, 1] ) + '} \\cdot ' + ddx + astToLatex( base ) + '\\).' );
	    }

	    var a = derivative_of_ast(base,x,story);

	    if (exponent === 1)
		return a;

	    if (typeof exponent === 'number') {
		var power_rule = textToAst('n * (f^m) * a');
		var result = substitute_ast( power_rule, { "n": exponent, "m": exponent - 1, "f": base, "a" : a } );
		result = clean_ast(result);
		story.push( 'So by the power rule and the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
		return result;
	    }

	    var power_rule = textToAst('n * (f^(n-1)) * a');
	    var result = substitute_ast( power_rule, { "n": exponent, "f": base, "a" : a } );
	    result = clean_ast(result);
	    story.push( 'So by the power rule and the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;
	}
	
	if (base === 'e') {
	    if ((typeof exponent === 'string') && (exponent === x)) {
		var power_rule = textToAst('e^(f)');
		var result = substitute_ast( power_rule, { "f": exponent } );
		result = clean_ast(result);
		story.push( 'The derivative of \\(e^' + astToLatex( x ) + '\\) is itself, that is, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex( tree ) + '\\).' );

		return result;
	    }
	    
	    story.push( 'Using the rule for \\(e^x\\) and the chain rule, we know \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex( tree ) + ' \\cdot ' + ddx + astToLatex( exponent ) + '\\).' );

	    var power_rule = textToAst('e^(f)*d');

	    var d = derivative_of_ast(exponent,x,story);
	    var result = substitute_ast( power_rule, { "f": exponent, "d": d } );
	    result = clean_ast(result);
	    story.push( 'So using the rule for \\(e^x\\) and the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;
	}
	
	if (typeof base === 'number') {
	    if ((typeof exponent === 'string') && (exponent === x)) {
		var power_rule = textToAst('a^(f) * log(a)');
		var result = substitute_ast( power_rule, { "a": base, "f": exponent } );
		result = clean_ast(result);
		story.push( 'The derivative of \\(a^' + astToLatex( x ) + '\\) is \\(a^{' + astToLatex( x ) + '} \\, \\log a\\), that is, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex( result ) + '\\).' );

		return result;
	    }

	    var exp_rule = textToAst('a^(f) * log(a)');
	    var partial_result = substitute_ast( exp_rule, { "a": base, "f": exponent } );

	    story.push( 'Using the rule for \\(a^x\\) and the chain rule, we know \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex( partial_result ) + ' \\cdot ' + ddx + astToLatex( exponent ) + '\\).' );

	    var power_rule = textToAst('a^(b)*log(a)*d');
	    var d = derivative_of_ast(exponent,x,story);
	    var result = substitute_ast( power_rule, { "a": base, "b": exponent, "d": d } );
	    result = clean_ast(result);
	    story.push( 'So using the rule for \\(a^x\\) and the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;		
	}
	
	
	var f = base;
	var g = exponent;

	story.push( "Recall the general rule for exponents, namely that \\(\\frac{d}{dx} u(x)^{v(x)} = u(x)^{v(x)} \\cdot \\left( v'(x) \\cdot \\log u(x) + \\frac{v(x) \\cdot u'(x)}{u(x)} \\right)\\).  In this case, \\(u(x) = " +  astToLatex( f ) + "\\) and \\(v(x) = " + astToLatex( g ) + "\\)." );

	var a = derivative_of_ast(f,x,story);
	var b = derivative_of_ast(g,x,story);

	var power_rule = textToAst('(f^g)*(b * log(f) + (g * a)/f)');
	var result = substitute_ast( power_rule, { "a": a, "b": b, "f": f, "g": g } );
	result = clean_ast(result);
	story.push( 'So by the general rule for exponents, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	return result;
    }

    if (operator === "apply") {
	var input = operands[1];

	story.push( 'By the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(substitute_ast( ["apply",operands[0] + "'","x"], { "x": input } )) + " \\cdot " + ddx + astToLatex(input)  + '\\).' );	    

	var result = ['*',
		      substitute_ast( ["apply",operands[0] + "'","x"], { "x": input } ),
		      derivative_of_ast( input, x, story )];
	result = clean_ast(result);		
	story.push( 'So by the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	return result;	    
    }

    
    if (operator in derivatives) {
	var input = operands[0];

	if (typeof input == "number") {
	    var result = 0;
	    story.push( 'The derivative of a constant is zero so \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;		
	} else if ((typeof input == "string") && (input == x)) {
	    var result = ['*',
			  substitute_ast( derivatives[operator], { "x": input } )];
	    result = clean_ast(result);
	    story.push( 'It is the case that \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;
	} else if ((typeof input == "string") && (input != x)) {
	    var result = 0;
	    story.push( 'Since the derivative of a constant is zero, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;
	} else {
	    story.push( 'Recall \\(\\frac{d}{du}' + astToLatex( [operator, 'u'] ) + ' = ' +
			astToLatex( derivative_of_ast( [operator, 'u'], 'u', [] ) ) + '\\).' );

	    story.push( 'By the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(substitute_ast( derivatives[operator], { "x": input } )) + " \\cdot " + ddx + astToLatex(input)  + '\\).' );	    

	    var result = ['*',
			  substitute_ast( derivatives[operator], { "x": input } ),
			  derivative_of_ast( input, x, story )];
	    result = clean_ast(result);		
	    story.push( 'So by the chain rule, \\(' + ddx + astToLatex( tree ) + ' = ' + astToLatex(result) + '\\).' );
	    return result;
	}
    }
    
    return 0;
};








function lowercaseFirstLetter(string)
{
    return string.charAt(0).toLowerCase() + string.slice(1);
}

function simplify_story( story ) {
    
    for (var i = story.length - 1; i >= 1; i--) {
	if (story[i] == story[i-1])
	    story.splice( i, 1 );
    }

    
    for (var i = 0; i < story.length; i++ ) {
	for( var j = i + 1; j < story.length; j++ ) {
	    if (story[i] == story[j]) {
		story[j] = 'Again, ' + lowercaseFirstLetter( story[j] );
	    }
	}
    }

    return story;
};

function randomBindings(variables) {
    var result = {};
    
    variables.forEach( function(v) {
	result[v] = Math.random() * 20.0 - 10.0;
    });

    return result;
};

function randomComplexBindings(variables) {
    var result = {};
    
    variables.forEach( function(v) {
	result[v] = new ComplexNumber( Math.random() * 20.0 - 10.0,  Math.random() * 20.0 - 10.0 );
    });

    return result;
};

function randomComplexBindingsBall(variables,real,imag) {
    var result = {};
    
    variables.forEach( function(v) {
	result[v] = new ComplexNumber( real+Math.random()-.5, imag +Math.random()-.5);
    });

    return result;
};

function randomIntegerBindings(variables) {
    var result = {};
    variables.forEach( function(v) {
        result[v]=new ComplexNumber(Math.floor(Math.random()*30),0);
    });
    return result;
};


function StraightLineProgram(tree)
{
    this.syntax_tree = clean_ast(tree);
}

StraightLineProgram.prototype = {
    f: function(bindings) {
	return astToFunction( this.syntax_tree )( bindings );
    },
    
    evaluate: function(bindings) {
	return astToFunction( this.syntax_tree )( bindings );
    },

    complex_evaluate: function(bindings) {
	return astToComplexFunction( this.syntax_tree )( bindings );
    },

    substitute: function(bindings) {
	var ast_bindings = new Object();

	var alphabet = "abcdefghijklmnopqrstuvwxyz";
	for(var i=0; i<alphabet.length; i++) {
	    var c = alphabet.charAt(i);
	    if (c in bindings)
		ast_bindings[c] = bindings[c].syntax_tree;
	}

	return new StraightLineProgram( substitute_ast( this.syntax_tree, ast_bindings ) );
    },
    
    tex: function() {
	return astToLatex( this.syntax_tree );
    },

    toString: function() {
	return astToText( this.syntax_tree );
    },

    
    integrate: function(x,a,b) {
	var intervals = 100;
	var total = 0.0;
	var bindings = new Object();

        for( var i=0; i < intervals; i++ ) {
	    var sample_point = a + ((b - a) * (i + 0.5) / intervals);
	    bindings[x] = sample_point;
	    total = total + this.evaluate( bindings );
	}

	return total * (b - a) / intervals;
    },

    
    equalsForBinding: function(other,bindings) {
	var epsilon = 0.01;
	var this_evaluated = this.evaluate(bindings);	
	var other_evaluated = other.evaluate(bindings);

	return (Math.abs(this_evaluated/other_evaluated - 1.0) < epsilon) ||
	    (this_evaluated == other_evaluated) ||
	    (isNaN(this_evaluated) && isNaN(other_evaluated));
    },
    
    derivative: function(x) {
	var story = [];
	return new StraightLineProgram(derivative_of_ast( this.syntax_tree, x, story ));
    },

    derivative_story: function(x) {
	var story = [];
	derivative_of_ast( this.syntax_tree, x, story );
	story = simplify_story( story );
	return story;
    },

    variables: function() {
	return variables_in_ast( this.syntax_tree );
    },
    
    equals: function(other) {
	var finite_tries = 0;
	var epsilon = 0.001; 
        var sum_of_differences = 0;
        var sum = 0;

	// Get set of variables mentioned in at least one of the two expressions
	var variables = [ this.variables(), other.variables() ];
	variables = variables.reduce( function(a,b) { return a.concat(b); } )
	variables = variables.reduce(function(p, c) {
            if (p.indexOf(c) < 0) p.push(c);
            return p;
	}, []);

       for (var i=0;i<variables.length;i++)
            { 
            if (variables[i]=='n') 
                {
                 for (var i=1;i<11;i++)
                     {
                     var bindings = randomIntegerBindings(variables); 
	             var this_evaluated = this.complex_evaluate(bindings); 	
	             var other_evaluated = other.complex_evaluate(bindings); 
	             if (isFinite(this_evaluated.real) && isFinite(other_evaluated.real) &&
		     isFinite(this_evaluated.imaginary) && isFinite(other_evaluated.imaginary)) 
                         {
		         finite_tries++;
                         sum_of_differences = sum_of_differences + this_evaluated.subtract(other_evaluated).modulus()
		         sum = sum + other_evaluated.modulus()                       
                  
                         } 
                     }
               if (finite_tries<1)
                   {return false}


	       if (sum_of_differences < epsilon*sum+(epsilon*epsilon))
               {return true;}
               else
               {return false;} 
                } 
             }

//end integer case      

//converts a variable name to a small offset, for use in the complex case above, via ascii code.

	function varToOffset(s){
	    return (s.charCodeAt(0)-100)*0.3;	
	}

//begin complex case 
        var points=[]

        for( var i=-10; i < 11; i=i+2)
            {
             for (var j=-10; j<11; j=j+2)
                 {
                  var bindings = {};   
                     variables.forEach( function(v) {
	         bindings[v] = new ComplexNumber(i + varToOffset(v),j+varToOffset(v));
    });
	          var this_evaluated = this.complex_evaluate(bindings); 	
	          var other_evaluated = other.complex_evaluate(bindings);
	          if (isFinite(this_evaluated.real) && isFinite(other_evaluated.real) &&
		  isFinite(this_evaluated.imaginary) && isFinite(other_evaluated.imaginary)) 
                       {
		       finite_tries++;
                       var difference=this_evaluated.subtract(other_evaluated).modulus();
                       sum_of_differences = sum_of_differences + difference ;
		       sum = sum + other_evaluated.modulus();
                       if (difference<.00001 && points.length<3)
                           {points.push([i,j]);}                       
                        } 
                 }
            
            }
           //console.log('first grid check');
           //console.log(bindings);
           //console.log(sum_of_differences)
           //console.log(points)
          if (finite_tries<1)
              {return false}
	  if (sum_of_differences < epsilon*sum+(epsilon*epsilon))
              {return true;}
          else
              {
               //console.log('bad branch case');
               for (i=0;i<points.length;i++)
                  {
                   var ballsum=0;
                   var sum=0;
                   for (j=0;j<20;j++)
                      {
                       var bindings= randomComplexBindingsBall(variables,points[i][0],points[i][1]);
                       var this_evaluated = this.complex_evaluate(bindings); 	
	               var other_evaluated = other.complex_evaluate(bindings);
                       sum=sum+this_evaluated.subtract(other_evaluated).modulus();
                      }
                   //console.log(sum);
                   if (sum<.0001)
                       {return true}
                   
                  }
              return false;
              }  

    },
};

var parse = function(string) {
    return new StraightLineProgram( textToAst(string) );
};

var parse_tex = function (string) {
    return new StraightLineProgram( latexToAst(string) );
};

exports.fromText = parse;
exports.parse = parse;
exports.fromLaTeX = parse_tex;
exports.fromLatex = parse_tex;
exports.fromTeX = parse_tex;
exports.fromTex = parse_tex;
exports.parse_tex = parse_tex;

  return module.exports;
})({exports: {}});


  return ___LIB_MATH_EXPRESSIONS___;
});

console.log( "hello!" );

MathExpression();
    //.fromText( "5x" );
