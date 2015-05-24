/*
 * a lexer for plain text expressions written with Jison
 *
 * Copyright 2014-2015 by Jim Fowler <kisonecat@gmail.com>
 *
 * This file is part of a math-expressions library
 * 
 * Some open source application is free software: you can redistribute
 * it and/or modify it under the terms of the GNU General Public
 * License as published by the Free Software Foundation, either
 * version 3 of the License, or at your option any later version.
 * 
 * Some open source application is distributed in the hope that it
 * will be useful, but WITHOUT ANY WARRANTY; without even the implied
 * warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details.
 * 
 */

/* lexical grammar */
%lex
%%

\s+                   /* skip whitespace */
[0-9]+([,.][0-9]+)?     return 'NUMBER'
[,.][0-9]+		return 'NUMBER'
"**"                    return '^'
"*"                     return '*' // there is some variety in multiplication symbols
"\xB7"                  return '*'
"\u00B7"                return '*'
"\u2022"                return '*'
"\u22C5"                return '*'
"\u00D7"                return '*'
"/"                     return '/'
"-"                     return '-'
"\u002D"                return '-' // there is quite some variety with unicode hyphens
"\u007E"                return '-'
"\u00AD"                return '-'
"\u058A"                return '-'
"\u05BE"                return '-'
"\u1400"                return '-'
"\u1806"                return '-'
"\u2010"                return '-'
"\u2011"                return '-'
"\u2012"                return '-'
"\u2013"                return '-'
"\u2014"                return '-'
"\u2015"                return '-'
"\u207B"                return '-'
"\u208B"                return '-'
"\u2212"                return '-'
"\u2E17"                return '-'
"\u2E3A"                return '-'
"\u2E3B"                return '-'
"\u301C"                return '-'
"\u3030"                return '-'
"\u30A0"                return '-'
"\uFE31"                return '-'
"\uFE32"                return '-'
"\uFE58"                return '-'
"\uFE63"                return '-'
"\uFF0D"                return '-'
"\u002D"                return '-'
"\u007E"                return '-'
"\u00AD"                return '-'
"\u058A"                return '-'
"\u1806"                return '-'
"\u2010"                return '-'
"\u2011"                return '-'
"\u2012"                return '-'
"\u2013"                return '-'
"\u2014"                return '-'
"\u2015"                return '-'
"\u2053"                return '-'
"\u207B"                return '-'
"\u208B"                return '-'
"\u2212"                return '-'
"\u301C"                return '-'
"\u3030"                return '-'
"+"                     return '+'
"^"                     return '^' // lots of ways to denote exponentiation
"\u2038"                return '^'
"\u2041"                return '^'
"\u028C"                return '^'
"\u2227"                return '^'
"\u02C7"                return '^'
"|"                     return '|'
"("                     return '('
")"                     return ')'
"["                     return '('
"]"                     return ')'
"{"                     return '('
"}"                     return ')'
"pi"                    return 'PI'
"sin"                   return 'SIN'
"cos"                   return 'COS'
"tan"                   return 'TAN'
"csc"                   return 'CSC'
"sec"                   return 'SEC'
"cot"                   return 'COT'
"arcsin"                return 'ARCSIN'
"arccos"                return 'ARCCOS'
"arctan"                return 'ARCTAN'
"arccsc"                return 'ARCCSC'
"arcsec"                return 'ARCSEC'
"arccot"                return 'ARCCOT'
"asin"                  return 'ARCSIN'
"acos"                  return 'ARCCOS'
"atan"                  return 'ARCTAN'
"acsc"                  return 'ARCCSC'
"asec"                  return 'ARCSEC'
"acot"                  return 'ARCCOT'
"log"                   return 'LOG'
"lg"                    return 'LOG'
"ln"                    return 'LN'
"exp"                   return 'EXP'
"sqrt"                  return 'SQRT'
"abs"                   return 'ABS'
[A-Za-z]                return 'VAR'
<<EOF>>                 return 'EOF'
EOF			return 'EOF'
.                       return 'INVALID'

/lex

%start empty

%% /* language grammar */

empty
    : EOF
    ;
