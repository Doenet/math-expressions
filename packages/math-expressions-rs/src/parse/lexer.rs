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
//! Token types are the `Tok` enum; its `Display` impl reproduces the JS
//! token_type strings, which error messages embed.

use std::rc::Rc;
use std::sync::OnceLock;

use crate::notation::NumberNotation;

/// Token types for both lexer flavours. The JS lexer used strings ("NUMBER",
/// "^", ...); an enum makes every comparison compile-checked. `Display`
/// yields the original JS token_type string, which error messages embed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tok {
    // Control
    Eof,
    Invalid,
    Space,
    // Literals / identifiers
    Number,
    Var,
    VarMultiChar,
    LatexCommand,
    Infinity,
    Ldots,
    // Operators and punctuation
    Times,
    Slash,
    Plus,
    Minus,
    Pm,
    Caret,
    Bang,
    Prime,
    Underscore,
    Pipe,
    /// A LaTeX `\left|`-style opening pipe (must open an absolute value).
    PipeL,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    /// LaTeX `\{` / `\}` set braces (plain `{`/`}` are grouping).
    SetLBrace,
    SetRBrace,
    LAngle,
    RAngle,
    LFloor,
    RFloor,
    LCeil,
    RCeil,
    Comma,
    Colon,
    Mid,
    Amp,
    Linebreak,
    BeginEnvironment,
    EndEnvironment,
    Sqrt,
    // Logic and quantifiers
    And,
    Or,
    Not,
    Forall,
    Exists,
    // Relations
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    In,
    NotIn,
    Ni,
    NotNi,
    Subset,
    NotSubset,
    SubsetEq,
    NotSubsetEq,
    Superset,
    NotSuperset,
    SupersetEq,
    NotSupersetEq,
    // Set / statement operators
    Union,
    Intersect,
    RightArrow,
    LeftArrow,
    LeftRightArrow,
    Implies,
    ImpliedBy,
    Iff,
    Perp,
    Parallel,
    Angle,
    Int,
}

impl Tok {
    /// Operator name used in the expression tree for tokens that map straight
    /// to an operator (what the JS derived via `token_type.toLowerCase()`).
    pub fn op_name(self) -> &'static str {
        match self {
            Tok::Implies => "implies",
            Tok::ImpliedBy => "impliedby",
            Tok::Iff => "iff",
            Tok::RightArrow => "rightarrow",
            Tok::LeftArrow => "leftarrow",
            Tok::LeftRightArrow => "leftrightarrow",
            Tok::Forall => "forall",
            Tok::Exists => "exists",
            Tok::Perp => "perp",
            Tok::Parallel => "parallel",
            Tok::Plus => "+",
            Tok::Minus => "-",
            _ => unreachable!("no operator name for {:?}", self),
        }
    }
}

impl std::fmt::Display for Tok {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The JS token_type strings, for error messages ("Expecting )").
        f.write_str(match self {
            Tok::Eof => "EOF",
            Tok::Invalid => "INVALID",
            Tok::Space => "SPACE",
            Tok::Number => "NUMBER",
            Tok::Var => "VAR",
            Tok::VarMultiChar => "VARMULTICHAR",
            Tok::LatexCommand => "LATEXCOMMAND",
            Tok::Infinity => "INFINITY",
            Tok::Ldots => "LDOTS",
            Tok::Times => "*",
            Tok::Slash => "/",
            Tok::Plus => "+",
            Tok::Minus => "-",
            Tok::Pm => "PM",
            Tok::Caret => "^",
            Tok::Bang => "!",
            Tok::Prime => "'",
            Tok::Underscore => "_",
            Tok::Pipe => "|",
            Tok::PipeL => "|L",
            Tok::LParen => "(",
            Tok::RParen => ")",
            Tok::LBracket => "[",
            Tok::RBracket => "]",
            Tok::LBrace => "{",
            Tok::RBrace => "}",
            Tok::SetLBrace => "LBRACE",
            Tok::SetRBrace => "RBRACE",
            Tok::LAngle => "LANGLE",
            Tok::RAngle => "RANGLE",
            Tok::LFloor => "LFLOOR",
            Tok::RFloor => "RFLOOR",
            Tok::LCeil => "LCEIL",
            Tok::RCeil => "RCEIL",
            Tok::Comma => ",",
            Tok::Colon => ":",
            Tok::Mid => "MID",
            Tok::Amp => "&",
            Tok::Linebreak => "LINEBREAK",
            Tok::BeginEnvironment => "BEGINENVIRONMENT",
            Tok::EndEnvironment => "ENDENVIRONMENT",
            Tok::Sqrt => "SQRT",
            Tok::And => "AND",
            Tok::Or => "OR",
            Tok::Not => "NOT",
            Tok::Forall => "FORALL",
            Tok::Exists => "EXISTS",
            Tok::Eq => "=",
            Tok::Ne => "NE",
            Tok::Lt => "<",
            Tok::Le => "LE",
            Tok::Gt => ">",
            Tok::Ge => "GE",
            Tok::In => "IN",
            Tok::NotIn => "NOTIN",
            Tok::Ni => "NI",
            Tok::NotNi => "NOTNI",
            Tok::Subset => "SUBSET",
            Tok::NotSubset => "NOTSUBSET",
            Tok::SubsetEq => "SUBSETEQ",
            Tok::NotSubsetEq => "NOTSUBSETEQ",
            Tok::Superset => "SUPERSET",
            Tok::NotSuperset => "NOTSUPERSET",
            Tok::SupersetEq => "SUPERSETEQ",
            Tok::NotSupersetEq => "NOTSUPERSETEQ",
            Tok::Union => "UNION",
            Tok::Intersect => "INTERSECT",
            Tok::RightArrow => "RIGHTARROW",
            Tok::LeftArrow => "LEFTARROW",
            Tok::LeftRightArrow => "LEFTRIGHTARROW",
            Tok::Implies => "IMPLIES",
            Tok::ImpliedBy => "IMPLIEDBY",
            Tok::Iff => "IFF",
            Tok::Perp => "PERP",
            Tok::Parallel => "PARALLEL",
            Tok::Angle => "ANGLE",
            Tok::Int => "INT",
        })
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub ttype: Tok,
    /// Token text after rule replacement (e.g. "α" lexes with text "alpha").
    pub text: String,
    /// The exact input text matched (used in error messages).
    pub original: String,
}

impl Token {
    fn simple(ttype: Tok, s: &str) -> Token {
        Token {
            ttype,
            text: s.to_string(),
            original: s.to_string(),
        }
    }
}

/// A saved lexer position. The input is behind `Rc`, so saving/restoring —
/// which the parsers do at every backtrack point — is O(1); only `unput`
/// (rare: symbol splitting, Leibniz backtracking) copies the string.
#[derive(Debug, Clone)]
pub struct LexerState {
    input: Rc<String>,
    offset: usize,
    location: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Flavor {
    Text,
    Latex,
}

pub struct Lexer {
    /// The working input (shared with saved states; copy-on-write on unput).
    input: Rc<String>,
    /// Byte offset of the cursor into `input`.
    offset: usize,
    /// Error-reporting location: bytes consumed minus bytes unput (matches
    /// the JS lexer's counter, which error positions are reported against).
    pub location: usize,
    sci_notation: bool,
    flavor: Flavor,
    rules: &'static [Rule],
    /// Decimal / argument-separator notation (I18N_MATH_NOTATION_PLAN). The
    /// argument separator lexes to `Tok::Comma`; the decimal separator is
    /// folded into number scanning.
    notation: NumberNotation,
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
    ttype: Tok,
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

fn l(pat: &'static str, ttype: Tok) -> Rule {
    Rule {
        pat: Pat::Lit(pat),
        ttype,
        replace: None,
    }
}
fn lr(pat: &'static str, ttype: Tok, replace: &'static str) -> Rule {
    Rule {
        pat: Pat::Lit(pat),
        ttype,
        replace: Some(replace),
    }
}
fn kw(pat: &'static str, ttype: Tok) -> Rule {
    Rule {
        pat: Pat::Kw(pat),
        ttype,
        replace: None,
    }
}
fn kwl(pat: &'static str, ttype: Tok) -> Rule {
    Rule {
        pat: Pat::KwL(pat),
        ttype,
        replace: None,
    }
}
/// KwL with a replacement string (e.g. `\varepsilon` → `\epsilon`).
fn kwlr(pat: &'static str, ttype: Tok, replace: &'static str) -> Rule {
    Rule {
        pat: Pat::KwL(pat),
        ttype,
        replace: Some(replace),
    }
}
/// A single `Seq` rule (chunks joined by optional `\s*`).
fn seq(chunks: &[&'static str], ttype: Tok, boundary: bool) -> Rule {
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
fn family(prefixes: &[&'static str], delim: &'static str, ttype: Tok, boundary: bool) -> Vec<Rule> {
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
            l("**", Tok::Caret),
            l("*", Tok::Times),
            l("\u{B7}", Tok::Times),   // ·
            l("\u{2022}", Tok::Times), // •
            l("\u{22C5}", Tok::Times), // ⋅
            l("\u{D7}", Tok::Times),   // ×
            l("/", Tok::Slash),
            l("-", Tok::Minus),
            l("\u{58A}", Tok::Minus),
            l("\u{5BE}", Tok::Minus),
            l("\u{1806}", Tok::Minus),
            l("\u{2010}", Tok::Minus),
            l("\u{2011}", Tok::Minus),
            l("\u{2012}", Tok::Minus),
            l("\u{2013}", Tok::Minus),
            l("\u{2014}", Tok::Minus),
            l("\u{2015}", Tok::Minus),
            l("\u{207B}", Tok::Minus),
            l("\u{208B}", Tok::Minus),
            l("\u{2212}", Tok::Minus),
            l("\u{2E3A}", Tok::Minus),
            l("\u{2E3B}", Tok::Minus),
            l("\u{FE58}", Tok::Minus),
            l("\u{FE63}", Tok::Minus),
            l("\u{FF0D}", Tok::Minus),
            l("+", Tok::Plus),
            kw("plusminus", Tok::Pm),
            l("±", Tok::Pm),
            l("^", Tok::Caret),
            l("\u{2038}", Tok::Caret), // ‸
            l("\u{28C}", Tok::Caret),  // ʌ
            l("\u{2032}", Tok::Prime), // prime ′
            l("|", Tok::Pipe),
            l("(", Tok::LParen),
            l(")", Tok::RParen),
            l("[", Tok::LBracket),
            l("]", Tok::RBracket),
            l("{", Tok::LBrace),
            l("}", Tok::RBrace),
            l("\u{27E8}", Tok::LAngle),
            l("\u{27E9}", Tok::RAngle),
            l("\u{3008}", Tok::LAngle),
            l("\u{3009}", Tok::RAngle),
            // NOTE: the argument separator (`,` by default) is emitted as
            // `Tok::Comma` by a notation-aware intercept in `advance`, not a
            // static rule, so comma-decimal notation can retask `,`.
            l(":", Tok::Colon),
            // Greek letters and named glyphs → VARMULTICHAR with replacement
            lr("\u{3B1}", Tok::VarMultiChar, "alpha"),
            lr("\u{3B2}", Tok::VarMultiChar, "beta"),
            lr("\u{3D0}", Tok::VarMultiChar, "beta"),
            lr("\u{393}", Tok::VarMultiChar, "Gamma"),
            lr("\u{3B3}", Tok::VarMultiChar, "gamma"),
            lr("\u{394}", Tok::VarMultiChar, "Delta"),
            lr("\u{3B4}", Tok::VarMultiChar, "delta"),
            lr("\u{3B5}", Tok::VarMultiChar, "epsilon"),
            lr("\u{3F5}", Tok::VarMultiChar, "epsilon"),
            lr("\u{3B6}", Tok::VarMultiChar, "zeta"),
            lr("\u{3B7}", Tok::VarMultiChar, "eta"),
            lr("\u{398}", Tok::VarMultiChar, "Theta"),
            lr("\u{3F4}", Tok::VarMultiChar, "Theta"),
            lr("\u{3B8}", Tok::VarMultiChar, "theta"),
            lr("\u{1DBF}", Tok::VarMultiChar, "theta"),
            lr("\u{3D1}", Tok::VarMultiChar, "theta"),
            lr("\u{3B9}", Tok::VarMultiChar, "iota"),
            lr("\u{3BA}", Tok::VarMultiChar, "kappa"),
            lr("\u{39B}", Tok::VarMultiChar, "Lambda"),
            lr("\u{3BB}", Tok::VarMultiChar, "lambda"),
            lr("\u{3BC}", Tok::VarMultiChar, "mu"),
            lr("\u{B5}", Tok::VarMultiChar, "mu"),
            lr("\u{3BD}", Tok::VarMultiChar, "nu"),
            lr("\u{39E}", Tok::VarMultiChar, "Xi"),
            lr("\u{3BE}", Tok::VarMultiChar, "xi"),
            lr("\u{3A0}", Tok::VarMultiChar, "Pi"),
            lr("\u{3C0}", Tok::VarMultiChar, "pi"),
            lr("\u{3D6}", Tok::VarMultiChar, "pi"),
            lr("\u{3C1}", Tok::VarMultiChar, "rho"),
            lr("\u{3F1}", Tok::VarMultiChar, "rho"),
            lr("\u{3A3}", Tok::VarMultiChar, "Sigma"),
            lr("\u{3C3}", Tok::VarMultiChar, "sigma"),
            lr("\u{3C2}", Tok::VarMultiChar, "sigma"),
            lr("\u{3C4}", Tok::VarMultiChar, "tau"),
            lr("\u{3A5}", Tok::VarMultiChar, "Upsilon"),
            lr("\u{3C5}", Tok::VarMultiChar, "upsilon"),
            lr("\u{3A6}", Tok::VarMultiChar, "Phi"),
            lr("\u{3C6}", Tok::VarMultiChar, "phi"),
            lr("\u{3D5}", Tok::VarMultiChar, "phi"),
            lr("\u{3A8}", Tok::VarMultiChar, "Psi"),
            lr("\u{3C8}", Tok::VarMultiChar, "psi"),
            lr("\u{3A9}", Tok::VarMultiChar, "Omega"),
            lr("\u{3C9}", Tok::VarMultiChar, "omega"),
            lr("\u{2205}", Tok::VarMultiChar, "emptyset"),
            kw("oo", Tok::Infinity),
            kw("OO", Tok::Infinity),
            kw("infty", Tok::Infinity),
            kw("infinity", Tok::Infinity),
            kw("Infinity", Tok::Infinity),
            l("\u{221E}", Tok::Infinity),  // ∞
            lr("\u{212F}", Tok::Var, "e"), // ℯ
            lr("\u{2660}", Tok::VarMultiChar, "spade"),
            lr("\u{2661}", Tok::VarMultiChar, "heart"),
            lr("\u{2662}", Tok::VarMultiChar, "diamond"),
            lr("\u{2663}", Tok::VarMultiChar, "club"),
            lr("\u{2605}", Tok::VarMultiChar, "bigstar"),
            lr("\u{25EF}", Tok::VarMultiChar, "bigcirc"),
            lr("\u{25CA}", Tok::VarMultiChar, "lozenge"),
            lr("\u{25B3}", Tok::VarMultiChar, "bigtriangleup"),
            lr("\u{25BD}", Tok::VarMultiChar, "bigtriangledown"),
            lr("\u{29EB}", Tok::VarMultiChar, "blacklozenge"),
            lr("\u{25A0}", Tok::VarMultiChar, "blacksquare"),
            lr("\u{25B2}", Tok::VarMultiChar, "blacktriangle"),
            lr("\u{25BC}", Tok::VarMultiChar, "blacktriangledown"),
            lr("\u{25C0}", Tok::VarMultiChar, "blacktriangleleft"),
            lr("\u{25B6}", Tok::VarMultiChar, "blacktriangleright"),
            lr("\u{25A1}", Tok::VarMultiChar, "Box"),
            lr("\u{2218}", Tok::VarMultiChar, "circ"),
            lr("\u{22C6}", Tok::VarMultiChar, "star"),
            kw("and", Tok::And),
            Rule {
                pat: Pat::Amp,
                ttype: Tok::And,
                replace: None,
            },
            l("\u{2227}", Tok::And), // ∧
            kw("or", Tok::Or),
            l("\u{2228}", Tok::Or), // ∨
            kw("not", Tok::Not),
            l("\u{AC}", Tok::Not), // ¬
            l("=", Tok::Eq),
            l("\u{1400}", Tok::Eq),
            l("\u{30A0}", Tok::Eq),
            l("!=", Tok::Ne),
            l("\u{2260}", Tok::Ne), // ≠
            l("<=", Tok::Le),
            l("\u{2264}", Tok::Le), // ≤
            l(">=", Tok::Ge),
            l("\u{2265}", Tok::Ge), // ≥
            l("<", Tok::Lt),
            l(">", Tok::Gt),
            kw("forall", Tok::Forall),
            l("\u{2200}", Tok::Forall),
            kw("exists", Tok::Exists),
            l("\u{2203}", Tok::Exists),
            kw("elementof", Tok::In),
            l("\u{2208}", Tok::In),
            kw("notelementof", Tok::NotIn),
            l("\u{2209}", Tok::NotIn),
            kw("containselement", Tok::Ni),
            l("\u{220B}", Tok::Ni),
            kw("notcontainselement", Tok::NotNi),
            l("\u{220C}", Tok::NotNi),
            kw("subset", Tok::Subset),
            l("\u{2282}", Tok::Subset),
            kw("subseteq", Tok::SubsetEq),
            l("\u{2286}", Tok::SubsetEq),
            kw("notsubset", Tok::NotSubset),
            l("\u{2284}", Tok::NotSubset),
            kw("notsubseteq", Tok::NotSubsetEq),
            l("\u{2288}", Tok::NotSubsetEq),
            kw("superset", Tok::Superset),
            l("\u{2283}", Tok::Superset),
            kw("superseteq", Tok::SupersetEq),
            l("\u{2287}", Tok::SupersetEq),
            kw("notsuperset", Tok::NotSuperset),
            l("\u{2285}", Tok::NotSuperset),
            kw("notsuperseteq", Tok::NotSupersetEq),
            l("\u{2289}", Tok::NotSupersetEq),
            kw("union", Tok::Union),
            l("\u{222A}", Tok::Union),
            kw("intersect", Tok::Intersect),
            l("\u{2229}", Tok::Intersect),
            kw("rightarrow", Tok::RightArrow),
            l("\u{2192}", Tok::RightArrow),
            l("\u{27F6}", Tok::RightArrow),
            kw("leftarrow", Tok::LeftArrow),
            l("\u{2190}", Tok::LeftArrow),
            l("\u{27F5}", Tok::LeftArrow),
            kw("leftrightarrow", Tok::LeftRightArrow),
            l("\u{2194}", Tok::LeftRightArrow),
            l("\u{27F7}", Tok::LeftRightArrow),
            kw("implies", Tok::Implies),
            l("\u{21D2}", Tok::Implies),
            l("\u{27F9}", Tok::Implies),
            kw("impliedby", Tok::ImpliedBy),
            l("\u{21D0}", Tok::ImpliedBy),
            l("\u{27F8}", Tok::ImpliedBy),
            kw("iff", Tok::Iff),
            l("\u{21D4}", Tok::Iff),
            l("\u{27FA}", Tok::Iff),
            kw("perp", Tok::Perp),
            l("\u{27C2}", Tok::Perp),
            kw("parallel", Tok::Parallel),
            l("\u{2225}", Tok::Parallel),
            kw("angle", Tok::Angle),
            l("\u{2220}", Tok::Angle),
            kw("int", Tok::Int),
            l("\u{222B}", Tok::Int),
            l("!", Tok::Bang),
            l("'", Tok::Prime),
            l("_", Tok::Underscore),
            l("...", Tok::Ldots),
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

        r.push(l("*", Tok::Times));
        r.push(l("/", Tok::Slash));
        r.push(l("-", Tok::Minus));
        r.push(l("+", Tok::Plus));
        r.push(kwl("\\pm", Tok::Pm));
        r.push(l("^", Tok::Caret));

        // Bracket delimiters: bare form then the \left/\big... size family.
        r.push(l("(", Tok::LParen));
        r.extend(family(&OPEN_PREFIXES, "(", Tok::LParen, false));
        r.push(l(")", Tok::RParen));
        r.extend(family(&CLOSE_PREFIXES, ")", Tok::RParen, false));
        r.push(l("[", Tok::LBracket));
        r.extend(family(&OPEN_PREFIXES, "[", Tok::LBracket, false));
        r.push(l("]", Tok::RBracket));
        r.extend(family(&CLOSE_PREFIXES, "]", Tok::RBracket, false));

        // Pipe: bare | ; \left| ... open forms marked |L; \right| ... and the
        // non-sided \big| ... forms are plain |.
        r.push(l("|", Tok::Pipe));
        r.extend(family(&OPEN_PREFIXES, "|", Tok::PipeL, false));
        r.extend(family(&CLOSE_PREFIXES, "|", Tok::Pipe, false));
        r.extend(family(
            &["\\big", "\\Big", "\\bigg", "\\Bigg"],
            "|",
            Tok::Pipe,
            false,
        ));

        r.push(l("{", Tok::LBrace));
        r.push(l("}", Tok::RBrace));
        r.push(l("\\{", Tok::SetLBrace));
        r.extend(family(&OPEN_PREFIXES, "\\{", Tok::SetLBrace, false));
        r.push(l("\\}", Tok::SetRBrace));
        r.extend(family(&CLOSE_PREFIXES, "\\}", Tok::SetRBrace, false));

        // Floor / ceil / angle: word delimiters, so a letter boundary applies.
        r.push(kwl("\\lfloor", Tok::LFloor));
        r.extend(family(&OPEN_PREFIXES, "\\lfloor", Tok::LFloor, true));
        r.push(kwl("\\rfloor", Tok::RFloor));
        r.extend(family(&CLOSE_PREFIXES, "\\rfloor", Tok::RFloor, true));
        r.push(kwl("\\lceil", Tok::LCeil));
        r.extend(family(&OPEN_PREFIXES, "\\lceil", Tok::LCeil, true));
        r.push(kwl("\\rceil", Tok::RCeil));
        r.extend(family(&CLOSE_PREFIXES, "\\rceil", Tok::RCeil, true));
        r.push(kwl("\\langle", Tok::LAngle));
        r.extend(family(&OPEN_PREFIXES, "\\langle", Tok::LAngle, true));
        r.push(kwl("\\rangle", Tok::RAngle));
        r.extend(family(&CLOSE_PREFIXES, "\\rangle", Tok::RAngle, true));

        r.push(kwl("\\cdot", Tok::Times));
        r.push(kwl("\\div", Tok::Slash));
        r.push(kwl("\\times", Tok::Times));
        // The argument separator lexes to `Tok::Comma` via the notation-aware
        // intercept in `advance` (see the text-rules note above).
        r.push(l(":", Tok::Colon));
        r.push(kwl("\\mid", Tok::Mid));

        r.push(kwlr("\\varnothing", Tok::LatexCommand, "\\emptyset"));
        r.push(kwlr("\\vartheta", Tok::LatexCommand, "\\theta"));
        r.push(kwlr("\\varepsilon", Tok::LatexCommand, "\\epsilon"));
        r.push(kwlr("\\varrho", Tok::LatexCommand, "\\rho"));
        r.push(kwlr("\\varphi", Tok::LatexCommand, "\\phi"));

        r.push(kwl("\\infty", Tok::Infinity));

        r.push(kwlr("\\asin", Tok::LatexCommand, "\\arcsin"));
        r.push(kwlr("\\acos", Tok::LatexCommand, "\\arccos"));
        r.push(kwlr("\\atan", Tok::LatexCommand, "\\arctan"));
        r.push(kwl("\\sqrt", Tok::Sqrt));

        r.push(kwl("\\land", Tok::And));
        r.push(kwl("\\wedge", Tok::And));
        r.push(kwl("\\lor", Tok::Or));
        r.push(kwl("\\vee", Tok::Or));
        r.push(kwl("\\lnot", Tok::Not));
        r.push(kwl("\\neg", Tok::Not));

        r.push(l("=", Tok::Eq));
        r.push(kwl("\\neq", Tok::Ne));
        r.push(kwl("\\ne", Tok::Ne));
        r.push(seq(&["\\not", "="], Tok::Ne, false));
        r.push(kwl("\\leq", Tok::Le));
        r.push(kwl("\\le", Tok::Le));
        r.push(kwl("\\geq", Tok::Ge));
        r.push(kwl("\\ge", Tok::Ge));
        r.push(l("<", Tok::Lt));
        r.push(kwl("\\lt", Tok::Lt));
        r.push(l(">", Tok::Gt));
        r.push(kwl("\\gt", Tok::Gt));

        r.push(kwl("\\forall", Tok::Forall));
        r.push(kwl("\\exists", Tok::Exists));
        r.push(kwl("\\in", Tok::In));
        r.push(kwl("\\notin", Tok::NotIn));
        r.push(seq(&["\\not", "\\in"], Tok::NotIn, true));
        r.push(kwl("\\ni", Tok::Ni));
        r.push(seq(&["\\not", "\\ni"], Tok::NotNi, true));
        r.push(kwl("\\subset", Tok::Subset));
        r.push(kwl("\\subseteq", Tok::SubsetEq));
        r.push(seq(&["\\not", "\\subset"], Tok::NotSubset, true));
        r.push(seq(&["\\not", "\\subseteq"], Tok::NotSubsetEq, true));
        r.push(kwl("\\supset", Tok::Superset));
        r.push(kwl("\\supseteq", Tok::SupersetEq));
        r.push(seq(&["\\not", "\\supset"], Tok::NotSuperset, true));
        r.push(seq(&["\\not", "\\supseteq"], Tok::NotSupersetEq, true));
        r.push(kwl("\\cup", Tok::Union));
        r.push(kwl("\\cap", Tok::Intersect));

        r.push(kwl("\\to", Tok::RightArrow));
        r.push(kwl("\\rightarrow", Tok::RightArrow));
        r.push(kwl("\\longrightarrow", Tok::RightArrow));
        r.push(kwl("\\gets", Tok::LeftArrow));
        r.push(kwl("\\leftarrow", Tok::LeftArrow));
        r.push(kwl("\\longleftarrow", Tok::LeftArrow));
        r.push(kwl("\\leftrightarrow", Tok::LeftRightArrow));
        r.push(kwl("\\longleftrightarrow", Tok::LeftRightArrow));
        r.push(kwl("\\implies", Tok::Implies));
        r.push(kwl("\\Longrightarrow", Tok::Implies));
        r.push(kwl("\\Rightarrow", Tok::Implies));
        r.push(kwl("\\impliedby", Tok::ImpliedBy));
        r.push(kwl("\\Longleftarrow", Tok::ImpliedBy));
        r.push(kwl("\\Leftarrow", Tok::ImpliedBy));
        r.push(kwl("\\iff", Tok::Iff));
        r.push(kwl("\\Longleftrightarrow", Tok::Iff));
        r.push(kwl("\\Leftrightarrow", Tok::Iff));

        r.push(kwl("\\perp", Tok::Perp));
        r.push(kwl("\\bot", Tok::Perp));
        r.push(kwl("\\parallel", Tok::Parallel));
        r.push(l("\\|", Tok::Parallel));
        r.push(kwl("\\angle", Tok::Angle));
        r.push(kwl("\\int", Tok::Int));

        r.push(l("!", Tok::Bang));
        r.push(l("'", Tok::Prime));
        r.push(l("_", Tok::Underscore));
        r.push(l("&", Tok::Amp));
        r.push(kwl("\\ldots", Tok::Ldots));
        r.push(l("\\\\", Tok::Linebreak));

        r.push(rule(Pat::Begin, Tok::BeginEnvironment));
        r.push(rule(Pat::End, Tok::EndEnvironment));
        r.push(rule(Pat::OpName, Tok::VarMultiChar));
        r.push(rule(Pat::LatexCmd, Tok::LatexCommand));
        r.push(l("\\$", Tok::LatexCommand));
        r.push(l("\\%", Tok::LatexCommand));

        r
    })
}

fn rule(pat: Pat, ttype: Tok) -> Rule {
    Rule {
        pat,
        ttype,
        replace: None,
    }
}

/// Does `rest` begin with a delimiter that may follow a scientific-notation
/// exponent? Text: end, `,` `|` `)` `}` `]`. LaTeX additionally allows `&`,
/// `\|`, `\}`, `\\` (linebreak), and `\end`.
fn sci_delim_ok(rest: &str, flavor: Flavor, arg_sep: char) -> bool {
    if rest.is_empty() {
        return true;
    }
    if rest.starts_with(arg_sep) {
        return true;
    }
    if matches!(rest.as_bytes()[0], b'|' | b')' | b'}' | b']') {
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

/// A decimal separator at the start of `rest`: a bare accepted decimal char,
/// or (LaTeX) a brace-wrapped one `{,}` — the form `render_number` emits for a
/// decimal comma (A5). Returns the byte length consumed. Bare `.` is never
/// brace-matched, so default notation is byte-identical.
fn decimal_at(rest: &str, decimals: &[char], flavor: Flavor) -> Option<usize> {
    for &d in decimals {
        if rest.starts_with(d) {
            return Some(d.len_utf8());
        }
    }
    if flavor == Flavor::Latex && rest.starts_with('{') {
        let inner = &rest[1..];
        for &d in decimals {
            if d != '.' && inner.starts_with(d) && inner[d.len_utf8()..].starts_with('}') {
                return Some(1 + d.len_utf8() + 1);
            }
        }
    }
    None
}

/// Scan a NUMBER at the start of `s`; returns the matched length.
/// Mantissa: [0-9]+(D[0-9]*)? or D[0-9]+, where `D` is the configured decimal
/// separator (`.` by default; also its LaTeX `{,}` form).
/// With sci notation, an optional exponent E[+-]?[0-9]+ matches only when
/// followed (after whitespace, which is included in the match) by
/// end-of-input or a flavor-specific delimiter — the JS lookahead constraint.
fn scan_number(s: &str, sci: bool, flavor: Flavor, notation: &NumberNotation) -> Option<usize> {
    let decimals = notation.accepted_decimals();
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && b[i].is_ascii_digit() {
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if let Some(dl) = decimal_at(&s[i..], &decimals, flavor) {
            i += dl;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
        }
    } else {
        // Leading decimal like `.5`: a decimal separator then ≥1 digit.
        let dl = decimal_at(s, &decimals, flavor)?;
        if !b.get(dl).is_some_and(u8::is_ascii_digit) {
            return None;
        }
        i = dl;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
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
            if sci_delim_ok(&s[k..], flavor, notation.argument_separator) {
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
    pub fn new(sci_notation: bool, notation: NumberNotation) -> Lexer {
        Lexer {
            input: Rc::new(String::new()),
            offset: 0,
            location: 0,
            sci_notation,
            flavor: Flavor::Text,
            rules: rules(),
            notation,
        }
    }

    /// LaTeX-flavor lexer.
    pub fn new_latex(sci_notation: bool, notation: NumberNotation) -> Lexer {
        Lexer {
            input: Rc::new(String::new()),
            offset: 0,
            location: 0,
            sci_notation,
            flavor: Flavor::Latex,
            rules: latex_rules(),
            notation,
        }
    }

    pub fn set_input(&mut self, input: &str) {
        self.input = Rc::new(input.to_string());
        self.offset = 0;
        self.location = 0;
    }

    pub fn state(&self) -> LexerState {
        LexerState {
            input: Rc::clone(&self.input),
            offset: self.offset,
            location: self.location,
        }
    }

    pub fn set_state(&mut self, s: LexerState) {
        self.input = s.input;
        self.offset = s.offset;
        self.location = s.location;
    }

    /// The unconsumed input.
    fn rest(&self) -> &str {
        &self.input[self.offset..]
    }

    /// Prepend text to the remaining input (used by symbol splitting and
    /// Leibniz-notation backtracking). Copies the input if a saved state
    /// still shares it, so restoring that state undoes the unput.
    pub fn unput(&mut self, s: &str) {
        self.location = self.location.saturating_sub(s.len());
        Rc::make_mut(&mut self.input).insert_str(self.offset, s);
    }

    fn consume(&mut self, n: usize) -> String {
        let matched = self.input[self.offset..self.offset + n].to_string();
        self.offset += n;
        self.location += n;
        matched
    }

    pub fn advance(&mut self, remove_initial_space: bool) -> Token {
        // Leading whitespace (flavor-specific: LaTeX also skips \, \quad, ...)
        let ws_len = leading_ws(self.rest(), self.flavor);
        if ws_len > 0 {
            let ws = self.consume(ws_len);
            if !remove_initial_space {
                return Token::simple(Tok::Space, &ws);
            }
        }

        if self.rest().is_empty() {
            return Token::simple(Tok::Eof, "");
        }

        // Number rules come first (they are prepended to the table in JS).
        if let Some(len) = scan_number(self.rest(), self.sci_notation, self.flavor, &self.notation) {
            let text = self.consume(len);
            return Token::simple(Tok::Number, &text);
        }

        // The configured argument separator lexes to `Comma` (the token the
        // grammar reads), whatever its glyph. The decimal separator is never a
        // separator on its own — it only appears inside numbers (handled
        // above), so a bare one falls through to INVALID.
        if self.rest().starts_with(self.notation.argument_separator) {
            let text = self.consume(self.notation.argument_separator.len_utf8());
            return Token::simple(Tok::Comma, &text);
        }

        let rules = self.rules;
        for rule in rules {
            if let Some(len) = rule.matches(self.rest()) {
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
            Flavor::Text => scan_var(self.rest()),
            Flavor::Latex => scan_var_latex(self.rest()),
        };
        if let Some(len) = var {
            let text = self.consume(len);
            return Token::simple(Tok::Var, &text);
        }

        // No match: INVALID, and (like the JS) do NOT consume — the parser
        // throws immediately on INVALID.
        let first: String = self.rest().chars().take(1).collect();
        Token::simple(Tok::Invalid, &first)
    }
}
