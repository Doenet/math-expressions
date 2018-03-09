/*
 * a lexer for plain text expressions written with Jison
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

\s+                   /* skip whitespace */
[0-9]+(\.[0-9]+)?(E[+\-]?[0-9]+)?  return 'NUMBER'
\.[0-9]+(E[+\-]?[0-9]+)?	   return 'NUMBER'
"**"                    return '^'
"*"                     return '*' // there is some variety in multiplication symbols
"\xB7"                  return '*' // '·'
"\u00B7"                return '*' // '·'
"\u2022"                return '*' // '•'
"\u22C5"                return '*' // '⋅'
"\u00D7"                return '*' // '×'
"/"                     return '/'
"-"                     return '-'// there is quite some variety with unicode hyphens
"\u058A"                return '-' // '֊'
"\u05BE"                return '-' // '־'
"\u1806"                return '-' // '᠆'
"\u2010"                return '-' // '‐'
"\u2011"                return '-' // '‑'
"\u2012"                return '-' // '‒'
"\u2013"                return '-' // '–'
"\u2014"                return '-' // '—'
"\u2015"                return '-' // '―'
"\u207B"                return '-' // '⁻'
"\u208B"                return '-' // '₋'
"\u2212"                return '-' // '−'
"\u2E3A"                return '-' // '⸺'
"\u2E3B"                return '-' // '⸻'
"\uFE58"                return '-' // '﹘'
"\uFE63"                return '-' // '﹣'
"\uFF0D"                return '-' // '－'
"+"                     return '+'
"^"                     return '^' // a few ways to denote exponentiation
"\u2038"                return '^' // '‸'
"\u028C"                return '^' // 'ʌ'
"|"                     return '|'
"("                     return '('
")"                     return ')'
"["			return "["
"]"			return "]"
"{"			return "{"
"}"			return "}"
","			return ","

"\u03B1"		{yytext='alpha'; return 'VARMULTICHAR';} // 'α'
"\u03B2"		{yytext='beta'; return 'VARMULTICHAR';} // 'β'
"\u03D0"		{yytext='beta'; return 'VARMULTICHAR';} // 'ϐ'
"\u0393"		{yytext='Gamma'; return 'VARMULTICHAR';} // 'Γ'
"\u03B3"		{yytext='gamma'; return 'VARMULTICHAR';} // 'γ'
"\u0394"		{yytext='Delta'; return 'VARMULTICHAR';} // 'Δ'
"\u03B4"		{yytext='delta'; return 'VARMULTICHAR';} // 'δ'
"\u03B5"		{yytext='epsilon'; return 'VARMULTICHAR';}  // 'ε' should this be varepsilon?
"\u03F5"		{yytext='epsilon'; return 'VARMULTICHAR';} // 'ϵ'
"\u03B6"		{yytext='zeta'; return 'VARMULTICHAR';} // 'ζ'
"\u03B7"		{yytext='eta'; return 'VARMULTICHAR';} // 'η'
"\u0398"                {yytext='Theta'; return 'VARMULTICHAR';} // 'Θ'
"\u03F4"                {yytext='Theta'; return 'VARMULTICHAR';} // 'ϴ'
"\u03B8"                {yytext='theta'; return 'VARMULTICHAR';} // 'θ'
"\u1DBF"                {yytext='theta'; return 'VARMULTICHAR';} // 'ᶿ'
"\u03D1"                {yytext='theta'; return 'VARMULTICHAR';} // 'ϑ'
"\u03B9"		{yytext='iota'; return 'VARMULTICHAR';} // 'ι'
"\u03BA"		{yytext='kappa'; return 'VARMULTICHAR';} // 'κ'
"\u039B"		{yytext='Lambda'; return 'VARMULTICHAR';} // 'Λ'
"\u03BB"		{yytext='lambda'; return 'VARMULTICHAR';} // 'λ'
"\u03BC"		{yytext='mu'; return 'VARMULTICHAR';} // 'μ'
"\u00B5"		{yytext='mu'; return 'VARMULTICHAR';} // 'µ' should this be micro?
"\u03BD"		{yytext='nu'; return 'VARMULTICHAR';} // 'ν'
"\u039E"		{yytext='Xi'; return 'VARMULTICHAR';} // 'Ξ'
"\u03BE"		{yytext='xi'; return 'VARMULTICHAR';} // 'ξ'
"\u03A0"		{yytext='Pi'; return 'VARMULTICHAR';} // 'Π'
"\u03C0"		{yytext='pi'; return 'VARMULTICHAR';} // 'π'
"\u03D6"		{yytext='pi'; return 'VARMULTICHAR';} // 'ϖ' should this be varpi?
"\u03C1"		{yytext='rho'; return 'VARMULTICHAR';} // 'ρ'
"\u03F1"		{yytext='rho'; return 'VARMULTICHAR';}  // 'ϱ' should this be varrho?
"\u03A3"		{yytext='Sigma'; return 'VARMULTICHAR';} // 'Σ'
"\u03C3"		{yytext='sigma'; return 'VARMULTICHAR';} // 'σ'
"\u03C2"		{yytext='sigma'; return 'VARMULTICHAR';} // 'ς' should this be varsigma?
"\u03C4"		{yytext='tau'; return 'VARMULTICHAR';} // 'τ'
"\u03A5"		{yytext='Upsilon'; return 'VARMULTICHAR';} // 'Υ'
"\u03C5"		{yytext='upsilon'; return 'VARMULTICHAR';} // 'υ'
"\u03A6"		{yytext='Phi'; return 'VARMULTICHAR';} // 'Φ'
"\u03C6"		{yytext='phi'; return 'VARMULTICHAR';} // 'φ' should this be varphi?
"\u03D5"		{yytext='phi'; return 'VARMULTICHAR';} // 'ϕ'
"\u03A8"		{yytext='Psi'; return 'VARMULTICHAR';} // 'Ψ'
"\u03C8"		{yytext='psi'; return 'VARMULTICHAR';} // 'ψ'
"\u03A9"		{yytext='Omega'; return 'VARMULTICHAR';} // 'Ω'
"\u03C9"		{yytext='omega'; return 'VARMULTICHAR';} // 'ω'

"oo"			return 'INFINITY'
"OO"			return 'INFINITY'
"infty"			return 'INFINITY'
"infinity"		return 'INFINITY'
"Infinity"		return 'INFINITY'
"\u221E"		return 'INFINITY' // '∞'

"\u212F"		{yytext='e'; return 'VAR';}  // 'ℯ'

"\u2660"		{yytext='spade'; return 'VARMULTICHAR';} // '♠'
"\u2661"		{yytext='heart'; return 'VARMULTICHAR';} // '♡'
"\u2662"		{yytext='diamond'; return 'VARMULTICHAR';} // '♢'
"\u2663"		{yytext='club'; return 'VARMULTICHAR';} // '♣'
"\u2605"		{yytext='bigstar'; return 'VARMULTICHAR';} // '★'
"\u25EF"		{yytext='bigcirc'; return 'VARMULTICHAR';} // '◯'
"\u25CA"		{yytext='lozenge'; return 'VARMULTICHAR';} // '◊'
"\u25B3"		{yytext='bigtriangleup'; return 'VARMULTICHAR';} // '△'
"\u25BD"		{yytext='bigtriangledown'; return 'VARMULTICHAR';} // '▽'
"\u29EB"		{yytext='blacklozenge'; return 'VARMULTICHAR';} // '⧫'
"\u25A0"		{yytext='blacksquare'; return 'VARMULTICHAR';} // '■'
"\u25B2"		{yytext='blacktriangle'; return 'VARMULTICHAR';} // '▲'
"\u25BC"		{yytext='blacktriangledown'; return 'VARMULTICHAR';} //'▼'
"\u25C0"		{yytext='blacktriangleleft'; return 'VARMULTICHAR';} // '◀'
"\u25B6"		{yytext='blacktriangleright'; return 'VARMULTICHAR';} // '▶'
"\u25A1"		{yytext='Box'; return 'VARMULTICHAR';} // '□'
"\u2218"		{yytext='circ'; return 'VARMULTICHAR';} // '∘'
"\u22C6"		{yytext='star'; return 'VARMULTICHAR';} // '⋆'

"and"			return 'AND'
\&\&?			return 'AND'
"\u2227"		return 'AND' // '∧'

"or"			return 'OR'
"\u2228"		return 'OR' // '∨'

"not"			return 'NOT'
"\u00ac"		return 'NOT' // '¬'

"="			return '='
"\u1400"                return '='  // '᐀'
"\u30A0"                return '='  // '゠'
"!="			return 'NE'
"\u2260"		return 'NE' // '≠'
"<="			return 'LE'
"\u2264"		return 'LE' // '≤'
">="			return 'GE'
"\u2265"		return 'GE' // '≥'
"<"			return '<'
">"			return '>'

"elementof"		return "IN"
'\u2208'		return "IN" // '∈'

"notelementof"		return "NOTIN"
'\u2209'		return "NOTIN" //'∉' 

"containselement"	return "NI"
"\u220B"		return "NI" // '∋'

"notcontainselement"	return "NOTNI"
"\u220C"		return "NOTNI" // '∌'

"subset"		return 'SUBSET'
'\u2282'		return 'SUBSET' // '⊂'

"notsubset"		return 'NOTSUBSET'
'\u2284'		return 'NOTSUBSET' // '⊄'

"superset"		return 'SUPERSET'
'\u2283'		return 'SUPERSET' // '⊃'

"notsuperset"		return 'NOTSUPERSET'
'\u2285'		return 'NOTSUPERSET' //'⊅' 

"union"			return 'UNION'
'\u222A'		return 'UNION' // '∪'

"intersect"		return 'INTERSECT'
'\u2229'		return 'INTERSECT' //'∩' 

"!"			return '!'
"'"			return "'"
[_]			return "_"   // use [_] so don't include word boundary

[a-zA-Z][a-zA-Z0-9]*	return 'VAR'
<<EOF>>                 return 'EOF'
EOF			return 'EOF'
.                       return 'INVALID'

/lex

%start empty

%% /* language grammar */

empty
    : EOF
    ;
