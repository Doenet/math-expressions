/*
 * a lexer for LaTeX expressions written with Jison
 *
 * Copyright 2014-2015 by Jim Fowler <kisonecat@gmail.com>
 *
 * This file is part of a math-expressions library
 * 
 * math-expressions is free software: you can redistribute
 * it and/or modify it under the terms of the GNU General Public
 * License as published by the Free Software Foundation, either
 * version 3 of the License, or at your option any later version.
 * 
 * math-expressions is distributed in the hope that it
 * will be useful, but WITHOUT ANY WARRANTY; without even the implied
 * warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details.
 * 
 */


/* lexical grammar */
%lex
%%

(\s+|"\\,")             /* skip whitespace */
[0-9]+(\.[0-9]+)?(E[+\-]?[0-9]+)?  return 'NUMBER'
\.[0-9]+(E[+\-]?[0-9]+)?	   return 'NUMBER'
"*"                     return '*'
"/"                     return '/'
"-"                     return '-'
"-"                     return '-'
"+"                     return '+'
"^"                     return '^'
"("                     return '('
"\\left"\s*"("          return '('
"\\bigl"\s*"("          return '('
"\\Bigl"\s*"("          return '('
"\\biggl"\s*"("         return '('
"\\Biggl"\s*"("         return '('
")"                     return ')'
"\\right"\s*")"         return ')'
"\\bigr"\s*")"          return ')'
"\\Bigr"\s*")"          return ')'
"\\biggr"\s*")"         return ')'
"\\Biggr"\s*")"         return ')'
"["                     return '['
"\\left"\s*"["          return '['
"\\bigl"\s*"["          return '['
"\\Bigl"\s*"["          return '['
"\\biggl"\s*"["         return '['
"\\Biggl"\s*"["         return '['
"]"                     return ']'
"\\right"\s*"]"         return ']'
"\\bigr"\s*"]"         	return ']'
"\\Bigr"\s*"]"         	return ']'
"\\biggr"\s*"]"         return ']'
"\\Biggr"\s*"]"         return ']'
"|"			return '|'
"\\left"\s*"|"          return '|'
"\\bigl"\s*"|"          return '|'
"\\Bigl"\s*"|"          return '|'
"\\biggl"\s*"|"         return '|'
"\\Biggl"\s*"|"         return '|'
"\\right"\s*"|"         return '|'
"\\bigr"\s*"|"  	return '|'
"\\Bigr"\s*"|"         	return '|'
"\\biggr"\s*"|"         return '|'
"\\Biggr"\s*"|"         return '|'
"\\big"\s*"|"  		return '|'
"\\Big"\s*"|"         	return '|'
"\\bigg"\s*"|"         	return '|'
"\\Bigg"\s*"|"         	return '|'
"{"                     return '{'
"}"                     return '}'
"\\{"               	return 'LBRACE'
"\\left"\s*"\\{"        return 'LBRACE'
"\\bigl"\s*"\\{"        return 'LBRACE'
"\\Bigl"\s*"\\{"        return 'LBRACE'
"\\biggl"\s*"\\{"       return 'LBRACE'
"\\Biggl"\s*"\\{"       return 'LBRACE'
"\\}"                   return 'RBRACE'
"\\right"\s*"\\}"       return 'RBRACE'
"\\bigr"\s*"\\}"        return 'RBRACE'
"\\Bigr"\s*"\\}"        return 'RBRACE'
"\\biggr"\s*"\\}"       return 'RBRACE'
"\\Biggr"\s*"\\}"       return 'RBRACE'
"\\cdot"                return '*'
"\\times"               return '*'
"\\frac"                return 'FRAC'
","			return ","

"\\vartheta"            {yytext='\\theta'; return 'LATEXCOMMAND';}
"\\varepsilon"          {yytext='\\epsilon'; return 'LATEXCOMMAND';}
"\\varrho"        	{yytext='\\rho'; return 'LATEXCOMMAND';}
"\\varphi"            	{yytext='\\phi'; return 'LATEXCOMMAND';}

"\\infty"		return 'INFINITY'

"\\asin"                {yytext='\\arcsin'; return 'LATEXCOMMAND';}
"\\acos"                {yytext='\\arccos'; return 'LATEXCOMMAND';}
"\\atan"                {yytext='\\arctan'; return 'LATEXCOMMAND';}
"\\sqrt"                return 'SQRT'

"\\land"		return 'AND'
"\\wedge"		return 'AND'

"\\lor"			return 'OR'
"\\vee"			return 'OR'

"\lnot"			return 'NOT'

"="			return '='
"\\neq"			return 'NE'
"\\ne"			return 'NE'
"\\not"\s*"="		return 'NE'
"\\leq"			return 'LE'
"\\le"			return 'LE'
"\\geq"			return 'GE'
"\\ge"			return 'GE'
"<"			return '<'
"\\lt"			return '<'
">"			return '>'
"\\gt"			return '>'

"\\in"			return "IN"

"\\notin"		return "NOTIN"
"\\not"\s*"\\in"	return "NOTIN"

"\\ni"			return "NI"

"\\not"\s*"\\ni"	return "NOTNI"

"\\subset"		return 'SUBSET'

"\\not"\s*"\\subset"	return 'NOTSUBSET'

"\\supset"		return 'SUPERSET'

"\\not"\s*"\\supset"	return 'NOTSUPERSET'

"\\cup"			return 'UNION'

"\\cap"			return 'INTERSECT'

"!"			return '!'
"'"			return "'"
[_]			return "_"   // use [_] so don't include word boundary

\\[a-zA-Z][a-zA-Z0-9]*	return 'LATEXCOMMAND'
[a-zA-Z]		return 'VAR'
<<EOF>>                 return 'EOF'
EOF			return 'EOF'
.                       return 'INVALID'

/lex

%start empty

%% /* language grammar */

empty
    : EOF
    ;
