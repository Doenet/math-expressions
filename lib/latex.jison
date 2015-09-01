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

(\s+|"\\,")                   /* skip whitespace */
[0-9]+("."[0-9]+)?  return 'NUMBER'
[,.][0-9]+		return 'NUMBER'
"*"                     return '*'
"/"                     return '/'
"-"                     return '-'
"+"                     return '+'
"^"                     return '^'
"("                     return '('
"\\left("               return '('
"\\right)"              return ')'
"\\left["               return '['
"\\right]"              return ']'
"["                     return '['
"]"                     return ']'
"\\left|"               return '|'
"\\right|"              return '|'
"|"			return '|'
")"                     return ')'
"{"                     return '{'
"}"                     return '}'
"\\cdot"                return '*'
"\\pi"                  return 'PI'
"\\frac"                return 'FRAC'
"\pi"                   return 'PI'
"\\sin"                 return 'SIN'
"\\cos"                 return 'COS'
"\\tan"                 return 'TAN'
"\\csc"                 return 'CSC'
"\\sec"                 return 'SEC'
"\\cot"                 return 'COT'
"\\sin"                 return 'SIN'
"\\cos"                 return 'COS'
"\\tan"                 return 'TAN'
"\\csc"                 return 'CSC'
"\\sec"                 return 'SEC'
"\\cot"                 return 'COT'

"\\pi"                  return 'PI'
"\\theta"               return 'THETA'

"\\arcsin"              return 'ARCSIN'
"\\arccos"              return 'ARCCOS'
"\\arctan"              return 'ARCTAN'
"\\arcsec"              return 'ARCSEC'
"\\arccsc"              return 'ARCCSC'
"\\arccot"              return 'ARCCOT'
"\\asin"                return 'ARCSIN'
"\\acos"                return 'ARCCOS'
"\\atan"                return 'ARCTAN'
"\\log"                 return 'LOG'
"\\ln"                  return 'LOG'
"\\exp"                 return 'EXP'
"\\sqrt"                return 'SQRT'
[A-Za-z]                return 'VAR'

<<EOF>>                 return 'EOF'
.                       return 'INVALID'

/lex

%start empty

%% /* language grammar */

empty
    : EOF
    ;
