//! Hand-written lexer, a faithful port of lib/converters/lexer.js plus the
//! token-rule table from text-to-ast.js.
//!
//! The JS lexer is a first-match-wins ordered regex table. Several rules use
//! lookaheads ((?![a-zA-Z0-9]) keyword boundaries, the scientific-notation
//! followed-by-delimiter constraint) that Rust's regex crate does not
//! support, so the rules are hand-coded — in the SAME order as the JS table,
//! since first-match ordering is semantic (e.g. "**" before "*", "!=" is a
//! rule but "!" is too).
//!
//! Token types are the JS token_type strings verbatim; the parser is a
//! line-by-line port and string tokens keep it mechanical. An enum can come
//! later once fixtures pass.

use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct Token {
    pub ttype: &'static str,
    /// Token text after rule replacement (e.g. "α" lexes with text "alpha").
    pub text: String,
    /// The exact input text matched (used in error messages).
    pub original: String,
}

impl Token {
    fn simple(ttype: &'static str, s: &str) -> Token {
        Token {
            ttype,
            text: s.to_string(),
            original: s.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LexerState {
    pub input: String,
    pub location: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Flavor {
    Text,
    Latex,
}

pub struct Lexer {
    /// Remaining (unconsumed) input.
    pub input: String,
    /// Byte offset into the original input at the end of the last match.
    pub location: usize,
    sci_notation: bool,
    flavor: Flavor,
    rules: &'static [Rule],
}

enum Pat {
    /// Literal string match.
    Lit(&'static str),
    /// Literal not followed by an ASCII alphanumeric — the JS
    /// `(?![a-zA-Z0-9])` keyword boundary (text parser).
    Kw(&'static str),
    /// Literal not followed by an ASCII letter — the JS `(?![a-zA-Z])`
    /// keyword boundary (LaTeX parser; digits are allowed to follow).
    KwL(&'static str),
    /// `&&` or `&` (JS rule "\&\&?").
    Amp,
    /// Chunks joined by optional `\s*`, with an optional trailing
    /// not-followed-by-letter boundary. Models `\left\s*(`, `\not\s*=`,
    /// `\left\s*\lfloor(?![a-zA-Z])`, etc.
    Seq(Vec<&'static str>, bool),
    /// `\begin\s*{\s*[a-zA-Z0-9]+\s*}`.
    Begin,
    /// `\end\s*{\s*[a-zA-Z0-9]+\s*}`.
    End,
    /// `\operatorname\s*{\s*[a-zA-Z0-9+-]+\s*}`.
    OpName,
    /// `\[a-zA-Z]+(?![a-zA-Z])` — a LaTeX command word.
    LatexCmd,
}

struct Rule {
    pat: Pat,
    ttype: &'static str,
    replace: Option<&'static str>,
}

/// Optional-`\s*` run (JS regex `\s*` = real whitespace only, not the LaTeX
/// spacing commands).
fn ws_run(s: &str) -> usize {
    s.chars()
        .take_while(|c| c.is_whitespace())
        .map(|c| c.len_utf8())
        .sum()
}

/// Match `\keyword \s* { \s* [class]+ \s* }`; returns total match length.
fn match_braced(s: &str, keyword: &str, plus_minus: bool) -> Option<usize> {
    let mut pos = 0;
    if !s.starts_with(keyword) {
        return None;
    }
    pos += keyword.len();
    pos += ws_run(&s[pos..]);
    if !s[pos..].starts_with('{') {
        return None;
    }
    pos += 1;
    pos += ws_run(&s[pos..]);
    let content_start = pos;
    while let Some(c) = s[pos..].chars().next() {
        if c.is_ascii_alphanumeric() || (plus_minus && (c == '+' || c == '-')) {
            pos += c.len_utf8();
        } else {
            break;
        }
    }
    if pos == content_start {
        return None; // need at least one content char
    }
    pos += ws_run(&s[pos..]);
    if !s[pos..].starts_with('}') {
        return None;
    }
    pos += 1;
    Some(pos)
}

impl Rule {
    fn matches(&self, s: &str) -> Option<usize> {
        match self.pat {
            Pat::Lit(p) => s.starts_with(p).then_some(p.len()),
            Pat::Kw(p) => {
                if !s.starts_with(p) {
                    return None;
                }
                match s.as_bytes().get(p.len()) {
                    Some(c) if c.is_ascii_alphanumeric() => None,
                    _ => Some(p.len()),
                }
            }
            Pat::KwL(p) => {
                if !s.starts_with(p) {
                    return None;
                }
                match s.as_bytes().get(p.len()) {
                    Some(c) if c.is_ascii_alphabetic() => None,
                    _ => Some(p.len()),
                }
            }
            Pat::Amp => {
                if s.starts_with("&&") {
                    Some(2)
                } else if s.starts_with('&') {
                    Some(1)
                } else {
                    None
                }
            }
            Pat::Seq(ref chunks, letter_boundary) => {
                let mut pos = 0;
                for (i, chunk) in chunks.iter().enumerate() {
                    if i > 0 {
                        pos += ws_run(&s[pos..]);
                    }
                    if !s[pos..].starts_with(chunk) {
                        return None;
                    }
                    pos += chunk.len();
                }
                if letter_boundary {
                    if let Some(c) = s[pos..].chars().next() {
                        if c.is_ascii_alphabetic() {
                            return None;
                        }
                    }
                }
                Some(pos)
            }
            Pat::Begin => match_braced(s, "\\begin", false),
            Pat::End => match_braced(s, "\\end", false),
            Pat::OpName => match_braced(s, "\\operatorname", true),
            Pat::LatexCmd => {
                if !s.starts_with('\\') {
                    return None;
                }
                let mut pos = 1;
                while let Some(c) = s[pos..].chars().next() {
                    if c.is_ascii_alphabetic() {
                        pos += 1;
                    } else {
                        break;
                    }
                }
                (pos > 1).then_some(pos)
            }
        }
    }
}

fn l(pat: &'static str, ttype: &'static str) -> Rule {
    Rule {
        pat: Pat::Lit(pat),
        ttype,
        replace: None,
    }
}
fn lr(pat: &'static str, ttype: &'static str, replace: &'static str) -> Rule {
    Rule {
        pat: Pat::Lit(pat),
        ttype,
        replace: Some(replace),
    }
}
fn kw(pat: &'static str, ttype: &'static str) -> Rule {
    Rule {
        pat: Pat::Kw(pat),
        ttype,
        replace: None,
    }
}
fn kwl(pat: &'static str, ttype: &'static str) -> Rule {
    Rule {
        pat: Pat::KwL(pat),
        ttype,
        replace: None,
    }
}
/// KwL with a replacement string (e.g. `\varepsilon` → `\epsilon`).
fn kwlr(pat: &'static str, ttype: &'static str, replace: &'static str) -> Rule {
    Rule {
        pat: Pat::KwL(pat),
        ttype,
        replace: Some(replace),
    }
}
/// A single `Seq` rule (chunks joined by optional `\s*`).
fn seq(chunks: &[&'static str], ttype: &'static str, boundary: bool) -> Rule {
    Rule {
        pat: Pat::Seq(chunks.to_vec(), boundary),
        ttype,
        replace: None,
    }
}

/// LaTeX size-prefixes for opening / closing delimiters (`\left(`, `\bigl(`,
/// ...). The bare delimiter is emitted separately by the caller.
const OPEN_PREFIXES: [&str; 5] = ["\\left", "\\bigl", "\\Bigl", "\\biggl", "\\Biggl"];
const CLOSE_PREFIXES: [&str; 5] = ["\\right", "\\bigr", "\\Bigr", "\\biggr", "\\Biggr"];

/// One rule per prefix in `prefixes`, each matching `prefix \s* delim`, all
/// mapped to `ttype`. `boundary` adds the not-followed-by-letter constraint.
fn family(
    prefixes: &[&'static str],
    delim: &'static str,
    ttype: &'static str,
    boundary: bool,
) -> Vec<Rule> {
    prefixes
        .iter()
        .map(|p| seq(&[p, delim], ttype, boundary))
        .collect()
}

/// The base_text_rules table from text-to-ast.js, in source order.
fn rules() -> &'static [Rule] {
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    RULES.get_or_init(|| {
        vec![
            l("**", "^"),
            l("*", "*"),
            l("\u{B7}", "*"),   // ·
            l("\u{2022}", "*"), // •
            l("\u{22C5}", "*"), // ⋅
            l("\u{D7}", "*"),   // ×
            l("/", "/"),
            l("-", "-"),
            l("\u{58A}", "-"),
            l("\u{5BE}", "-"),
            l("\u{1806}", "-"),
            l("\u{2010}", "-"),
            l("\u{2011}", "-"),
            l("\u{2012}", "-"),
            l("\u{2013}", "-"),
            l("\u{2014}", "-"),
            l("\u{2015}", "-"),
            l("\u{207B}", "-"),
            l("\u{208B}", "-"),
            l("\u{2212}", "-"),
            l("\u{2E3A}", "-"),
            l("\u{2E3B}", "-"),
            l("\u{FE58}", "-"),
            l("\u{FE63}", "-"),
            l("\u{FF0D}", "-"),
            l("+", "+"),
            kw("plusminus", "PM"),
            l("±", "PM"),
            l("^", "^"),
            l("\u{2038}", "^"), // ‸
            l("\u{28C}", "^"),  // ʌ
            l("\u{2032}", "'"), // prime ′
            l("|", "|"),
            l("(", "("),
            l(")", ")"),
            l("[", "["),
            l("]", "]"),
            l("{", "{"),
            l("}", "}"),
            l("\u{27E8}", "LANGLE"),
            l("\u{27E9}", "RANGLE"),
            l("\u{3008}", "LANGLE"),
            l("\u{3009}", "RANGLE"),
            l(",", ","),
            l(":", ":"),
            // Greek letters and named glyphs → VARMULTICHAR with replacement
            lr("\u{3B1}", "VARMULTICHAR", "alpha"),
            lr("\u{3B2}", "VARMULTICHAR", "beta"),
            lr("\u{3D0}", "VARMULTICHAR", "beta"),
            lr("\u{393}", "VARMULTICHAR", "Gamma"),
            lr("\u{3B3}", "VARMULTICHAR", "gamma"),
            lr("\u{394}", "VARMULTICHAR", "Delta"),
            lr("\u{3B4}", "VARMULTICHAR", "delta"),
            lr("\u{3B5}", "VARMULTICHAR", "epsilon"),
            lr("\u{3F5}", "VARMULTICHAR", "epsilon"),
            lr("\u{3B6}", "VARMULTICHAR", "zeta"),
            lr("\u{3B7}", "VARMULTICHAR", "eta"),
            lr("\u{398}", "VARMULTICHAR", "Theta"),
            lr("\u{3F4}", "VARMULTICHAR", "Theta"),
            lr("\u{3B8}", "VARMULTICHAR", "theta"),
            lr("\u{1DBF}", "VARMULTICHAR", "theta"),
            lr("\u{3D1}", "VARMULTICHAR", "theta"),
            lr("\u{3B9}", "VARMULTICHAR", "iota"),
            lr("\u{3BA}", "VARMULTICHAR", "kappa"),
            lr("\u{39B}", "VARMULTICHAR", "Lambda"),
            lr("\u{3BB}", "VARMULTICHAR", "lambda"),
            lr("\u{3BC}", "VARMULTICHAR", "mu"),
            lr("\u{B5}", "VARMULTICHAR", "mu"),
            lr("\u{3BD}", "VARMULTICHAR", "nu"),
            lr("\u{39E}", "VARMULTICHAR", "Xi"),
            lr("\u{3BE}", "VARMULTICHAR", "xi"),
            lr("\u{3A0}", "VARMULTICHAR", "Pi"),
            lr("\u{3C0}", "VARMULTICHAR", "pi"),
            lr("\u{3D6}", "VARMULTICHAR", "pi"),
            lr("\u{3C1}", "VARMULTICHAR", "rho"),
            lr("\u{3F1}", "VARMULTICHAR", "rho"),
            lr("\u{3A3}", "VARMULTICHAR", "Sigma"),
            lr("\u{3C3}", "VARMULTICHAR", "sigma"),
            lr("\u{3C2}", "VARMULTICHAR", "sigma"),
            lr("\u{3C4}", "VARMULTICHAR", "tau"),
            lr("\u{3A5}", "VARMULTICHAR", "Upsilon"),
            lr("\u{3C5}", "VARMULTICHAR", "upsilon"),
            lr("\u{3A6}", "VARMULTICHAR", "Phi"),
            lr("\u{3C6}", "VARMULTICHAR", "phi"),
            lr("\u{3D5}", "VARMULTICHAR", "phi"),
            lr("\u{3A8}", "VARMULTICHAR", "Psi"),
            lr("\u{3C8}", "VARMULTICHAR", "psi"),
            lr("\u{3A9}", "VARMULTICHAR", "Omega"),
            lr("\u{3C9}", "VARMULTICHAR", "omega"),
            lr("\u{2205}", "VARMULTICHAR", "emptyset"),
            kw("oo", "INFINITY"),
            kw("OO", "INFINITY"),
            kw("infty", "INFINITY"),
            kw("infinity", "INFINITY"),
            kw("Infinity", "INFINITY"),
            l("\u{221E}", "INFINITY"),  // ∞
            lr("\u{212F}", "VAR", "e"), // ℯ
            lr("\u{2660}", "VARMULTICHAR", "spade"),
            lr("\u{2661}", "VARMULTICHAR", "heart"),
            lr("\u{2662}", "VARMULTICHAR", "diamond"),
            lr("\u{2663}", "VARMULTICHAR", "club"),
            lr("\u{2605}", "VARMULTICHAR", "bigstar"),
            lr("\u{25EF}", "VARMULTICHAR", "bigcirc"),
            lr("\u{25CA}", "VARMULTICHAR", "lozenge"),
            lr("\u{25B3}", "VARMULTICHAR", "bigtriangleup"),
            lr("\u{25BD}", "VARMULTICHAR", "bigtriangledown"),
            lr("\u{29EB}", "VARMULTICHAR", "blacklozenge"),
            lr("\u{25A0}", "VARMULTICHAR", "blacksquare"),
            lr("\u{25B2}", "VARMULTICHAR", "blacktriangle"),
            lr("\u{25BC}", "VARMULTICHAR", "blacktriangledown"),
            lr("\u{25C0}", "VARMULTICHAR", "blacktriangleleft"),
            lr("\u{25B6}", "VARMULTICHAR", "blacktriangleright"),
            lr("\u{25A1}", "VARMULTICHAR", "Box"),
            lr("\u{2218}", "VARMULTICHAR", "circ"),
            lr("\u{22C6}", "VARMULTICHAR", "star"),
            kw("and", "AND"),
            Rule {
                pat: Pat::Amp,
                ttype: "AND",
                replace: None,
            },
            l("\u{2227}", "AND"), // ∧
            kw("or", "OR"),
            l("\u{2228}", "OR"), // ∨
            kw("not", "NOT"),
            l("\u{AC}", "NOT"), // ¬
            l("=", "="),
            l("\u{1400}", "="),
            l("\u{30A0}", "="),
            l("!=", "NE"),
            l("\u{2260}", "NE"), // ≠
            l("<=", "LE"),
            l("\u{2264}", "LE"), // ≤
            l(">=", "GE"),
            l("\u{2265}", "GE"), // ≥
            l("<", "<"),
            l(">", ">"),
            kw("forall", "FORALL"),
            l("\u{2200}", "FORALL"),
            kw("exists", "EXISTS"),
            l("\u{2203}", "EXISTS"),
            kw("elementof", "IN"),
            l("\u{2208}", "IN"),
            kw("notelementof", "NOTIN"),
            l("\u{2209}", "NOTIN"),
            kw("containselement", "NI"),
            l("\u{220B}", "NI"),
            kw("notcontainselement", "NOTNI"),
            l("\u{220C}", "NOTNI"),
            kw("subset", "SUBSET"),
            l("\u{2282}", "SUBSET"),
            kw("subseteq", "SUBSETEQ"),
            l("\u{2286}", "SUBSETEQ"),
            kw("notsubset", "NOTSUBSET"),
            l("\u{2284}", "NOTSUBSET"),
            kw("notsubseteq", "NOTSUBSETEQ"),
            l("\u{2288}", "NOTSUBSETEQ"),
            kw("superset", "SUPERSET"),
            l("\u{2283}", "SUPERSET"),
            kw("superseteq", "SUPERSETEQ"),
            l("\u{2287}", "SUPERSETEQ"),
            kw("notsuperset", "NOTSUPERSET"),
            l("\u{2285}", "NOTSUPERSET"),
            kw("notsuperseteq", "NOTSUPERSETEQ"),
            l("\u{2289}", "NOTSUPERSETEQ"),
            kw("union", "UNION"),
            l("\u{222A}", "UNION"),
            kw("intersect", "INTERSECT"),
            l("\u{2229}", "INTERSECT"),
            kw("rightarrow", "RIGHTARROW"),
            l("\u{2192}", "RIGHTARROW"),
            l("\u{27F6}", "RIGHTARROW"),
            kw("leftarrow", "LEFTARROW"),
            l("\u{2190}", "LEFTARROW"),
            l("\u{27F5}", "LEFTARROW"),
            kw("leftrightarrow", "LEFTRIGHTARROW"),
            l("\u{2194}", "LEFTRIGHTARROW"),
            l("\u{27F7}", "LEFTRIGHTARROW"),
            kw("implies", "IMPLIES"),
            l("\u{21D2}", "IMPLIES"),
            l("\u{27F9}", "IMPLIES"),
            kw("impliedby", "IMPLIEDBY"),
            l("\u{21D0}", "IMPLIEDBY"),
            l("\u{27F8}", "IMPLIEDBY"),
            kw("iff", "IFF"),
            l("\u{21D4}", "IFF"),
            l("\u{27FA}", "IFF"),
            kw("perp", "PERP"),
            l("\u{27C2}", "PERP"),
            kw("parallel", "PARALLEL"),
            l("\u{2225}", "PARALLEL"),
            kw("angle", "ANGLE"),
            l("\u{2220}", "ANGLE"),
            kw("int", "INT"),
            l("\u{222B}", "INT"),
            l("!", "!"),
            l("'", "'"),
            l("_", "_"),
            l("...", "LDOTS"),
        ]
    })
}

/// The base_latex_rules table from latex-to-ast.js, in source order.
/// Rust string literals hold the actual characters (`"\\("` is backslash-`(`),
/// where the JS table used doubly-escaped regex source.
// Built by interleaving single-rule `push` and delimiter-family `extend`, so a
// single vec! literal isn't applicable.
#[allow(clippy::vec_init_then_push)]
fn latex_rules() -> &'static [Rule] {
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    RULES.get_or_init(|| {
        let mut r: Vec<Rule> = Vec::new();

        r.push(l("*", "*"));
        r.push(l("/", "/"));
        r.push(l("-", "-"));
        r.push(l("+", "+"));
        r.push(kwl("\\pm", "PM"));
        r.push(l("^", "^"));

        // Bracket delimiters: bare form then the \left/\big... size family.
        r.push(l("(", "("));
        r.extend(family(&OPEN_PREFIXES, "(", "(", false));
        r.push(l(")", ")"));
        r.extend(family(&CLOSE_PREFIXES, ")", ")", false));
        r.push(l("[", "["));
        r.extend(family(&OPEN_PREFIXES, "[", "[", false));
        r.push(l("]", "]"));
        r.extend(family(&CLOSE_PREFIXES, "]", "]", false));

        // Pipe: bare | ; \left| ... open forms marked |L; \right| ... and the
        // non-sided \big| ... forms are plain |.
        r.push(l("|", "|"));
        r.extend(family(&OPEN_PREFIXES, "|", "|L", false));
        r.extend(family(&CLOSE_PREFIXES, "|", "|", false));
        r.extend(family(
            &["\\big", "\\Big", "\\bigg", "\\Bigg"],
            "|",
            "|",
            false,
        ));

        r.push(l("{", "{"));
        r.push(l("}", "}"));
        r.push(l("\\{", "LBRACE"));
        r.extend(family(&OPEN_PREFIXES, "\\{", "LBRACE", false));
        r.push(l("\\}", "RBRACE"));
        r.extend(family(&CLOSE_PREFIXES, "\\}", "RBRACE", false));

        // Floor / ceil / angle: word delimiters, so a letter boundary applies.
        r.push(kwl("\\lfloor", "LFLOOR"));
        r.extend(family(&OPEN_PREFIXES, "\\lfloor", "LFLOOR", true));
        r.push(kwl("\\rfloor", "RFLOOR"));
        r.extend(family(&CLOSE_PREFIXES, "\\rfloor", "RFLOOR", true));
        r.push(kwl("\\lceil", "LCEIL"));
        r.extend(family(&OPEN_PREFIXES, "\\lceil", "LCEIL", true));
        r.push(kwl("\\rceil", "RCEIL"));
        r.extend(family(&CLOSE_PREFIXES, "\\rceil", "RCEIL", true));
        r.push(kwl("\\langle", "LANGLE"));
        r.extend(family(&OPEN_PREFIXES, "\\langle", "LANGLE", true));
        r.push(kwl("\\rangle", "RANGLE"));
        r.extend(family(&CLOSE_PREFIXES, "\\rangle", "RANGLE", true));

        r.push(kwl("\\cdot", "*"));
        r.push(kwl("\\div", "/"));
        r.push(kwl("\\times", "*"));
        r.push(l(",", ","));
        r.push(l(":", ":"));
        r.push(kwl("\\mid", "MID"));

        r.push(kwlr("\\varnothing", "LATEXCOMMAND", "\\emptyset"));
        r.push(kwlr("\\vartheta", "LATEXCOMMAND", "\\theta"));
        r.push(kwlr("\\varepsilon", "LATEXCOMMAND", "\\epsilon"));
        r.push(kwlr("\\varrho", "LATEXCOMMAND", "\\rho"));
        r.push(kwlr("\\varphi", "LATEXCOMMAND", "\\phi"));

        r.push(kwl("\\infty", "INFINITY"));

        r.push(kwlr("\\asin", "LATEXCOMMAND", "\\arcsin"));
        r.push(kwlr("\\acos", "LATEXCOMMAND", "\\arccos"));
        r.push(kwlr("\\atan", "LATEXCOMMAND", "\\arctan"));
        r.push(kwl("\\sqrt", "SQRT"));

        r.push(kwl("\\land", "AND"));
        r.push(kwl("\\wedge", "AND"));
        r.push(kwl("\\lor", "OR"));
        r.push(kwl("\\vee", "OR"));
        r.push(kwl("\\lnot", "NOT"));
        r.push(kwl("\\neg", "NOT"));

        r.push(l("=", "="));
        r.push(kwl("\\neq", "NE"));
        r.push(kwl("\\ne", "NE"));
        r.push(seq(&["\\not", "="], "NE", false));
        r.push(kwl("\\leq", "LE"));
        r.push(kwl("\\le", "LE"));
        r.push(kwl("\\geq", "GE"));
        r.push(kwl("\\ge", "GE"));
        r.push(l("<", "<"));
        r.push(kwl("\\lt", "<"));
        r.push(l(">", ">"));
        r.push(kwl("\\gt", ">"));

        r.push(kwl("\\forall", "FORALL"));
        r.push(kwl("\\exists", "EXISTS"));
        r.push(kwl("\\in", "IN"));
        r.push(kwl("\\notin", "NOTIN"));
        r.push(seq(&["\\not", "\\in"], "NOTIN", true));
        r.push(kwl("\\ni", "NI"));
        r.push(seq(&["\\not", "\\ni"], "NOTNI", true));
        r.push(kwl("\\subset", "SUBSET"));
        r.push(kwl("\\subseteq", "SUBSETEQ"));
        r.push(seq(&["\\not", "\\subset"], "NOTSUBSET", true));
        r.push(seq(&["\\not", "\\subseteq"], "NOTSUBSETEQ", true));
        r.push(kwl("\\supset", "SUPERSET"));
        r.push(kwl("\\supseteq", "SUPERSETEQ"));
        r.push(seq(&["\\not", "\\supset"], "NOTSUPERSET", true));
        r.push(seq(&["\\not", "\\supseteq"], "NOTSUPERSETEQ", true));
        r.push(kwl("\\cup", "UNION"));
        r.push(kwl("\\cap", "INTERSECT"));

        r.push(kwl("\\to", "RIGHTARROW"));
        r.push(kwl("\\rightarrow", "RIGHTARROW"));
        r.push(kwl("\\longrightarrow", "RIGHTARROW"));
        r.push(kwl("\\gets", "LEFTARROW"));
        r.push(kwl("\\leftarrow", "LEFTARROW"));
        r.push(kwl("\\longleftarrow", "LEFTARROW"));
        r.push(kwl("\\leftrightarrow", "LEFTRIGHTARROW"));
        r.push(kwl("\\longleftrightarrow", "LEFTRIGHTARROW"));
        r.push(kwl("\\implies", "IMPLIES"));
        r.push(kwl("\\Longrightarrow", "IMPLIES"));
        r.push(kwl("\\Rightarrow", "IMPLIES"));
        r.push(kwl("\\impliedby", "IMPLIEDBY"));
        r.push(kwl("\\Longleftarrow", "IMPLIEDBY"));
        r.push(kwl("\\Leftarrow", "IMPLIEDBY"));
        r.push(kwl("\\iff", "IFF"));
        r.push(kwl("\\Longleftrightarrow", "IFF"));
        r.push(kwl("\\Leftrightarrow", "IFF"));

        r.push(kwl("\\perp", "PERP"));
        r.push(kwl("\\bot", "PERP"));
        r.push(kwl("\\parallel", "PARALLEL"));
        r.push(l("\\|", "PARALLEL"));
        r.push(kwl("\\angle", "ANGLE"));
        r.push(kwl("\\int", "INT"));

        r.push(l("!", "!"));
        r.push(l("'", "'"));
        r.push(l("_", "_"));
        r.push(l("&", "&"));
        r.push(kwl("\\ldots", "LDOTS"));
        r.push(l("\\\\", "LINEBREAK"));

        r.push(rule(Pat::Begin, "BEGINENVIRONMENT"));
        r.push(rule(Pat::End, "ENDENVIRONMENT"));
        r.push(rule(Pat::OpName, "VARMULTICHAR"));
        r.push(rule(Pat::LatexCmd, "LATEXCOMMAND"));
        r.push(l("\\$", "LATEXCOMMAND"));
        r.push(l("\\%", "LATEXCOMMAND"));

        r
    })
}

fn rule(pat: Pat, ttype: &'static str) -> Rule {
    Rule {
        pat,
        ttype,
        replace: None,
    }
}

/// Does `rest` begin with a delimiter that may follow a scientific-notation
/// exponent? Text: end, `,` `|` `)` `}` `]`. LaTeX additionally allows `&`,
/// `\|`, `\}`, `\\` (linebreak), and `\end`.
fn sci_delim_ok(rest: &str, flavor: Flavor) -> bool {
    if rest.is_empty() {
        return true;
    }
    if matches!(rest.as_bytes()[0], b',' | b'|' | b')' | b'}' | b']') {
        return true;
    }
    if flavor == Flavor::Latex {
        return rest.starts_with('&')
            || rest.starts_with("\\|")
            || rest.starts_with("\\}")
            || rest.starts_with("\\\\")
            || rest.starts_with("\\end");
    }
    false
}

/// Scan a NUMBER at the start of `s`; returns the matched length.
/// Mantissa: [0-9]+(\.[0-9]*)? or \.[0-9]+.
/// With sci notation, an optional exponent E[+-]?[0-9]+ matches only when
/// followed (after whitespace, which is included in the match) by
/// end-of-input or a flavor-specific delimiter — the JS lookahead constraint.
fn scan_number(s: &str, sci: bool, flavor: Flavor) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && b[i].is_ascii_digit() {
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i < b.len() && b[i] == b'.' {
            i += 1;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
        }
    } else if b.len() >= 2 && b[0] == b'.' && b[1].is_ascii_digit() {
        i = 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
    } else {
        return None;
    }
    if !sci {
        return Some(i);
    }
    // Optional exponent with the delimiter lookahead.
    if i < b.len() && b[i] == b'E' {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let digits_start = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > digits_start {
            let ws: usize = ws_run(&s[j..]);
            let k = j + ws;
            if sci_delim_ok(&s[k..], flavor) {
                return Some(k); // trailing whitespace is part of the match
            }
        }
    }
    Some(i)
}

/// Text VAR: [a-zA-Z∂][a-zA-Z∂0-9]* or a single char from [＿$%].
fn scan_var(s: &str) -> Option<usize> {
    let mut chars = s.char_indices();
    let (_, c0) = chars.next()?;
    if c0.is_ascii_alphabetic() || c0 == '∂' {
        let mut end = c0.len_utf8();
        for (i, c) in chars {
            if c.is_ascii_alphanumeric() || c == '∂' {
                end = i + c.len_utf8();
            } else {
                break;
            }
        }
        Some(end)
    } else if c0 == '\u{ff3f}' || c0 == '$' || c0 == '%' {
        Some(c0.len_utf8())
    } else {
        None
    }
}

/// LaTeX VAR: a single char from [a-zA-Z＿$%]. Multi-letter identifiers lex
/// as separate single-char VARs (giving implicit multiplication).
fn scan_var_latex(s: &str) -> Option<usize> {
    let c0 = s.chars().next()?;
    if c0.is_ascii_alphabetic() || c0 == '\u{ff3f}' || c0 == '$' || c0 == '%' {
        Some(c0.len_utf8())
    } else {
        None
    }
}

/// Leading-whitespace length. Text: a run of Unicode whitespace. LaTeX also
/// consumes the spacing commands \, \! \(space) \> \; \: \quad \qquad.
fn leading_ws(s: &str, flavor: Flavor) -> usize {
    match flavor {
        Flavor::Text => ws_run(s),
        Flavor::Latex => {
            let mut pos = 0;
            loop {
                let rest = &s[pos..];
                if let Some(c) = rest.chars().next() {
                    if c.is_whitespace() {
                        pos += c.len_utf8();
                        continue;
                    }
                }
                // \qquad and \quad require a word boundary (JS \b).
                if word_command(rest, "\\qquad") {
                    pos += 6;
                    continue;
                }
                if word_command(rest, "\\quad") {
                    pos += 5;
                    continue;
                }
                if rest.len() >= 2 && rest.starts_with('\\') {
                    let c1 = rest.as_bytes()[1];
                    if matches!(c1, b',' | b'!' | b' ' | b'>' | b';' | b':') {
                        pos += 2;
                        continue;
                    }
                }
                break;
            }
            pos
        }
    }
}

/// `cmd` at the start of `s`, followed by a `\b` word boundary (not an
/// alphanumeric — matching JS `\b` after a word char).
fn word_command(s: &str, cmd: &str) -> bool {
    s.starts_with(cmd)
        && !s[cmd.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
}

impl Lexer {
    /// Text-flavor lexer.
    pub fn new(sci_notation: bool) -> Lexer {
        Lexer {
            input: String::new(),
            location: 0,
            sci_notation,
            flavor: Flavor::Text,
            rules: rules(),
        }
    }

    /// LaTeX-flavor lexer.
    pub fn new_latex(sci_notation: bool) -> Lexer {
        Lexer {
            input: String::new(),
            location: 0,
            sci_notation,
            flavor: Flavor::Latex,
            rules: latex_rules(),
        }
    }

    pub fn set_input(&mut self, input: &str) {
        self.input = input.to_string();
        self.location = 0;
    }

    pub fn state(&self) -> LexerState {
        LexerState {
            input: self.input.clone(),
            location: self.location,
        }
    }

    pub fn set_state(&mut self, s: LexerState) {
        self.input = s.input;
        self.location = s.location;
    }

    /// Prepend text to the remaining input (used by symbol splitting and
    /// Leibniz-notation backtracking).
    pub fn unput(&mut self, s: &str) {
        self.location = self.location.saturating_sub(s.len());
        self.input.insert_str(0, s);
    }

    fn consume(&mut self, n: usize) -> String {
        let matched: String = self.input[..n].to_string();
        self.input.drain(..n);
        self.location += n;
        matched
    }

    pub fn advance(&mut self, remove_initial_space: bool) -> Token {
        // Leading whitespace (flavor-specific: LaTeX also skips \, \quad, ...)
        let ws_len = leading_ws(&self.input, self.flavor);
        if ws_len > 0 {
            let ws = self.consume(ws_len);
            if !remove_initial_space {
                return Token::simple("SPACE", &ws);
            }
        }

        if self.input.is_empty() {
            return Token::simple("EOF", "");
        }

        // Number rules come first (they are prepended to the table in JS).
        if let Some(len) = scan_number(&self.input, self.sci_notation, self.flavor) {
            let text = self.consume(len);
            return Token::simple("NUMBER", &text);
        }

        for rule in self.rules {
            if let Some(len) = rule.matches(&self.input) {
                let original = self.consume(len);
                let text = rule
                    .replace
                    .map(str::to_string)
                    .unwrap_or_else(|| original.clone());
                return Token {
                    ttype: rule.ttype,
                    text,
                    original,
                };
            }
        }

        // VAR is the last rule in the JS table.
        let var = match self.flavor {
            Flavor::Text => scan_var(&self.input),
            Flavor::Latex => scan_var_latex(&self.input),
        };
        if let Some(len) = var {
            let text = self.consume(len);
            return Token::simple("VAR", &text);
        }

        // No match: INVALID, and (like the JS) do NOT consume — the parser
        // throws immediately on INVALID.
        let first: String = self.input.chars().take(1).collect();
        Token::simple("INVALID", &first)
    }
}
