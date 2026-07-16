//! Recursive descent text parser — a faithful port of
//! lib/converters/text-to-ast.js (grammar documented there and in
//! PORTING_PLAN.md §6b).
//!
//! Porting conventions:
//! - JS `false` return values ("no factor here") become `Option<Expr>::None`.
//! - JS assigns the blank "＿" where an operand is missing → `Expr::Blank`.
//! - JS builds sign-symbol strings like "2++++" via string concatenation on
//!   numbers/symbols; ported via `atom_string`/`sign_string`.
//! - Parameter objects are re-destructured with defaults at every JS call
//!   site; the `P` struct mirrors this — each call site constructs exactly
//!   the fields the JS passes, everything else reset to defaults. This
//!   includes bug-faithful quirks (e.g. symbol splitting drops the
//!   in_subsuperscript flag — that is what makes "x^ab" parse as
//!   (x^a)*b).

use super::error::ParseError;
use super::lexer::{Lexer, LexerState, Token};
use crate::expr::{flatten, Expr, MathConst, RelOp, SeqKind};
use crate::num::Number;
use crate::sym::Sym;
use std::collections::HashMap;

type R<T> = Result<T, ParseError>;

/// Parse-time parameters, mirroring the JS options objects.
#[derive(Debug, Clone, Copy)]
struct P {
    inside_absolute_value: u32,
    parse_absolute_value: bool,
    allow_absolute_value_closing: bool,
    in_subsuperscript: bool,
}

impl Default for P {
    fn default() -> P {
        P {
            inside_absolute_value: 0,
            parse_absolute_value: true,
            allow_absolute_value_closing: false,
            in_subsuperscript: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextToAstOptions {
    pub allow_simplified_function_application: bool,
    pub split_symbols: bool,
    pub unsplit_symbols: Vec<String>,
    pub applied_function_symbols: Vec<String>,
    pub function_symbols: Vec<String>,
    pub operator_symbols: Vec<String>,
    pub parse_leibniz_notation: bool,
    pub parse_scientific_notation: bool,
}

impl Default for TextToAstOptions {
    fn default() -> Self {
        fn v(names: &[&str]) -> Vec<String> {
            names.iter().map(|s| s.to_string()).collect()
        }
        TextToAstOptions {
            allow_simplified_function_application: true,
            split_symbols: true,
            unsplit_symbols: v(&[
                "alpha", "beta", "gamma", "Gamma", "delta", "Delta", "epsilon", "zeta", "eta",
                "theta", "Theta", "iota", "kappa", "lambda", "Lambda", "mu", "nu", "xi", "Xi",
                "pi", "Pi", "rho", "sigma", "Sigma", "tau", "Tau", "upsilon", "Upsilon", "phi",
                "Phi", "chi", "psi", "Psi", "omega", "Omega", "angle", "deg", "emptyset",
            ]),
            applied_function_symbols: v(&[
                "abs", "exp", "log", "ln", "log10", "sign", "sqrt", "cbrt", "nthroot", "mod",
                "erf", "cos", "cosh", "acos", "acosh", "arccos", "arccosh", "cot", "coth", "acot",
                "acoth", "arccot", "arccoth", "csc", "csch", "acsc", "acsch", "arccsc", "arccsch",
                "sec", "sech", "asec", "asech", "arcsec", "arcsech", "sin", "sinh", "asin",
                "asinh", "arcsin", "arcsinh", "tan", "tanh", "atan", "atan2", "atanh", "arctan",
                "arctanh", "arg", "conj", "re", "im", "det", "trace", "nPr", "nCr", "floor",
                "ceil", "round",
            ]),
            function_symbols: v(&["f", "g"]),
            operator_symbols: v(&["binom", "vec", "linesegment"]),
            parse_leibniz_notation: true,
            parse_scientific_notation: true,
        }
    }
}

/// The units table from lib/expression/units.js — only the shape matters at
/// parse time (prefix vs postfix); the scale closures belong to evaluation.
fn units() -> HashMap<&'static str, bool /* prefix */> {
    HashMap::from([("%", false), ("$", true), ("deg", false)])
}

pub struct TextToAst {
    opts: TextToAstOptions,
    lexer: Lexer,
    token: Token,
    units: HashMap<&'static str, bool>,
}

// ---- JS-semantics helpers ----------------------------------------------

/// JS: `typeof e === "string" && [...e].every(c => "+-".includes(c))`.
/// Sign-symbols are Syms whose name is entirely +/- characters.
fn sign_string(e: &Expr) -> Option<String> {
    if let Expr::Sym(s) = e {
        let name = s.name();
        if !name.is_empty() && name.chars().all(|c| c == '+' || c == '-') {
            return Some(name);
        }
    }
    None
}

/// JS: `typeof e === "number" || typeof e === "string"` with JS
/// stringification. The blank ("＿") and Infinity are a string/number in
/// JS, so they participate.
fn atom_string(e: &Expr) -> Option<String> {
    match e {
        Expr::Num(n) => Some(n.js_string()),
        Expr::Sym(s) => Some(s.name()),
        Expr::Blank => Some("\u{ff3f}".to_string()),
        Expr::Const(MathConst::Inf) => Some("Infinity".to_string()),
        Expr::Const(MathConst::NegInf) => Some("-Infinity".to_string()),
        _ => None,
    }
}

/// JS: `e > 0` (numbers and Infinity only; everything else coerces false).
fn is_positive_number(e: &Expr) -> bool {
    match e {
        Expr::Num(n) => n.is_positive(),
        Expr::Const(MathConst::Inf) => true,
        _ => false,
    }
}

/// JS unary numeric negation `-e` for the `e > 0` case.
fn negate_number(e: Expr) -> Expr {
    match e {
        Expr::Num(n) => Expr::Num(n.neg()),
        Expr::Const(MathConst::Inf) => Expr::Const(MathConst::NegInf),
        _ => unreachable!("negate_number only called when is_positive_number"),
    }
}

/// parseFloat for a NUMBER token (Rust's f64 parser rejects "1.E3" which
/// parseFloat accepts).
fn parse_js_float(text: &str) -> f64 {
    let t = text.trim();
    if let Ok(v) = t.parse::<f64>() {
        return v;
    }
    let fixed = t.replace(".E", "E").replace(".e", "e");
    if let Ok(v) = fixed.parse::<f64>() {
        return v;
    }
    t.trim_end_matches('.').parse::<f64>().unwrap_or(f64::NAN)
}

fn other_op(name: &str, args: Vec<Expr>) -> Expr {
    Expr::OtherOp(Sym::new(name), args)
}

impl TextToAst {
    pub fn new(opts: TextToAstOptions) -> Self {
        let lexer = Lexer::new(opts.parse_scientific_notation);
        TextToAst {
            opts,
            lexer,
            token: Token {
                ttype: "EOF",
                text: String::new(),
                original: String::new(),
            },
            units: units(),
        }
    }

    fn advance(&mut self) -> R<()> {
        self.advance_opts(true)
    }

    fn advance_opts(&mut self, remove_initial_space: bool) -> R<()> {
        self.token = self.lexer.advance(remove_initial_space);
        if self.token.ttype == "INVALID" {
            return Err(ParseError::new(
                format!("Invalid symbol '{}'", self.token.original),
                self.lexer.location,
            ));
        }
        Ok(())
    }

    fn state(&self) -> (LexerState, Token) {
        (self.lexer.state(), self.token.clone())
    }

    fn set_state(&mut self, s: (LexerState, Token)) {
        self.lexer.set_state(s.0);
        self.token = s.1;
    }

    fn err(&self, msg: impl Into<String>) -> ParseError {
        ParseError::new(msg, self.lexer.location)
    }

    pub fn convert(&mut self, input: &str) -> R<Expr> {
        self.lexer.set_input(input);
        self.advance()?;
        let result = self.statement_list()?;
        if self.token.ttype != "EOF" {
            return Err(self.err(format!("Invalid location of '{}'", self.token.original)));
        }
        Ok(flatten(result))
    }

    fn statement_list(&mut self) -> R<Expr> {
        let mut list = vec![self.statement(P::default())?];
        while self.token.ttype == "," {
            self.advance()?;
            list.push(self.statement(P::default())?);
        }
        Ok(if list.len() > 1 {
            Expr::Seq(SeqKind::List, list)
        } else {
            list.pop().unwrap()
        })
    }

    fn statement(&mut self, p: P) -> R<Expr> {
        // three periods ... can be a statement by itself
        if self.token.ttype == "LDOTS" {
            self.advance()?;
            return Ok(Expr::Ldots);
        }

        let original_state = self.state();

        match self.statement_main(p) {
            Ok(r) => Ok(r),
            Err(e) => {
                // If parsing the statement failed, try again ignoring
                // absolute value and interpreting | as a binary operator.
                self.set_state(original_state);
                match self.statement_bar_fallback() {
                    Ok(r) => Ok(r),
                    Err(_) => Err(e), // rethrow the original error
                }
            }
        }
    }

    fn statement_main(&mut self, p: P) -> R<Expr> {
        let lhs = self.statement_a(P {
            inside_absolute_value: p.inside_absolute_value,
            ..P::default()
        })?;

        if self.token.ttype != ":" {
            return Ok(lhs);
        }
        self.advance()?;
        let rhs = self.statement_a(P::default())?;
        Ok(other_op(":", vec![lhs, rhs]))
    }

    fn statement_bar_fallback(&mut self) -> R<Expr> {
        let lhs = self.statement_a(P {
            parse_absolute_value: false,
            ..P::default()
        })?;

        if self.token.ttype != "|" {
            return Err(self.err("statement fallback: no bar"));
        }
        self.advance()?;
        let rhs = self.statement_a(P {
            parse_absolute_value: false,
            ..P::default()
        })?;
        Ok(other_op("|", vec![lhs, rhs]))
    }

    fn statement_a(&mut self, p: P) -> R<Expr> {
        let fwd = P {
            inside_absolute_value: p.inside_absolute_value,
            parse_absolute_value: p.parse_absolute_value,
            ..P::default()
        };
        let mut lhs = self.statement_b(fwd)?;

        while matches!(
            self.token.ttype,
            "IMPLIES" | "IMPLIEDBY" | "IFF" | "LEFTARROW" | "RIGHTARROW" | "LEFTRIGHTARROW"
        ) {
            let operation = self.token.ttype.to_lowercase();
            self.advance()?;
            let rhs = self.statement_b(fwd)?;
            lhs = other_op(&operation, vec![lhs, rhs]);
        }

        Ok(lhs)
    }

    fn statement_b(&mut self, p: P) -> R<Expr> {
        let fwd = P {
            inside_absolute_value: p.inside_absolute_value,
            parse_absolute_value: p.parse_absolute_value,
            ..P::default()
        };
        let mut lhs = self.statement_c(fwd)?;

        while self.token.ttype == "OR" {
            self.advance()?;
            let rhs = self.statement_c(fwd)?;
            lhs = Expr::Or(vec![lhs, rhs]);
        }

        Ok(lhs)
    }

    fn statement_c(&mut self, p: P) -> R<Expr> {
        // AND binds tighter than OR
        let mut lhs = self.relation(p)?;

        while self.token.ttype == "AND" {
            self.advance()?;
            let rhs = self.relation(p)?;
            lhs = Expr::And(vec![lhs, rhs]);
        }

        Ok(lhs)
    }

    fn relation(&mut self, p: P) -> R<Expr> {
        if self.token.ttype == "NOT" || self.token.ttype == "!" {
            self.advance()?;
            return Ok(Expr::Not(Box::new(self.relation(p)?)));
        }

        if self.token.ttype == "FORALL" || self.token.ttype == "EXISTS" {
            let operator = self.token.ttype.to_lowercase();
            self.advance()?;
            return Ok(other_op(&operator, vec![self.relation(p)?]));
        }

        let mut lhs = self.expression(p)?;

        loop {
            let op = match self.token.ttype {
                "=" => Some(RelOp::Eq),
                "NE" => Some(RelOp::Ne),
                "<" => Some(RelOp::Lt),
                ">" => Some(RelOp::Gt),
                "LE" => Some(RelOp::Le),
                "GE" => Some(RelOp::Ge),
                "IN" => Some(RelOp::In),
                "NOTIN" => Some(RelOp::NotIn),
                "NI" => Some(RelOp::Ni),
                "NOTNI" => Some(RelOp::NotNi),
                "SUBSET" => Some(RelOp::Subset),
                "NOTSUBSET" => Some(RelOp::NotSubset),
                "SUBSETEQ" => Some(RelOp::SubsetEq),
                "NOTSUBSETEQ" => Some(RelOp::NotSubsetEq),
                "SUPERSET" => Some(RelOp::Superset),
                "NOTSUPERSET" => Some(RelOp::NotSuperset),
                "SUPERSETEQ" => Some(RelOp::SupersetEq),
                "NOTSUPERSETEQ" => Some(RelOp::NotSupersetEq),
                _ => None,
            };
            let Some(op) = op else { break };

            self.advance()?;
            let rhs = self.expression(p)?;

            match op {
                RelOp::Lt | RelOp::Le if self.token.ttype == "<" || self.token.ttype == "LE" => {
                    // sequence of multiple < or <=
                    let mut ops = vec![op];
                    let mut operands = vec![lhs, rhs];
                    while self.token.ttype == "<" || self.token.ttype == "LE" {
                        ops.push(if self.token.ttype == "<" {
                            RelOp::Lt
                        } else {
                            RelOp::Le
                        });
                        self.advance()?;
                        operands.push(self.expression(p)?);
                    }
                    lhs = Expr::Relation { operands, ops };
                }
                RelOp::Gt | RelOp::Ge if self.token.ttype == ">" || self.token.ttype == "GE" => {
                    let mut ops = vec![op];
                    let mut operands = vec![lhs, rhs];
                    while self.token.ttype == ">" || self.token.ttype == "GE" {
                        ops.push(if self.token.ttype == ">" {
                            RelOp::Gt
                        } else {
                            RelOp::Ge
                        });
                        self.advance()?;
                        operands.push(self.expression(p)?);
                    }
                    lhs = Expr::Relation { operands, ops };
                }
                RelOp::Eq => {
                    let mut operands = vec![lhs, rhs];
                    let mut ops = vec![RelOp::Eq];
                    // check for sequence of multiple =
                    while self.token.ttype == "=" {
                        self.advance()?;
                        operands.push(self.expression(p)?);
                        ops.push(RelOp::Eq);
                    }
                    lhs = Expr::Relation { operands, ops };
                }
                _ => {
                    lhs = Expr::Relation {
                        operands: vec![lhs, rhs],
                        ops: vec![op],
                    };
                }
            }
        }

        Ok(lhs)
    }

    fn expression(&mut self, p: P) -> R<Expr> {
        if self.token.ttype == "NOT" || self.token.ttype == "!" {
            self.advance()?;
            return Ok(Expr::Not(Box::new(self.expression(p)?)));
        }

        let mut plus_begin = false;
        if self.token.ttype == "+" {
            plus_begin = true;
            self.advance()?;
        }

        let mut negative_begin = false;
        if self.token.ttype == "-" {
            negative_begin = true;
            self.advance()?;
        }

        let mut pm_begin = false;
        if self.token.ttype == "PM" {
            pm_begin = true;
            self.advance()?;
        }

        let lhs_opt = self.term(p)?;

        if negative_begin || plus_begin {
            let prefix = format!(
                "{}{}",
                if plus_begin { "+" } else { "" },
                if negative_begin { "-" } else { "" }
            );
            match &lhs_opt {
                None => return Ok(Expr::sym(&prefix)),
                Some(l) => {
                    if let Some(s) = sign_string(l) {
                        return Ok(Expr::sym(&format!("{}{}", prefix, s)));
                    }
                }
            }
        }

        let mut lhs = lhs_opt.unwrap_or(Expr::Blank);

        if negative_begin {
            lhs = if is_positive_number(&lhs) {
                negate_number(lhs)
            } else {
                Expr::Neg(Box::new(lhs))
            };
        }

        if pm_begin {
            lhs = other_op("pm", vec![lhs]);
        }

        if plus_begin {
            lhs = Expr::Add(vec![lhs]);
        }

        while matches!(
            self.token.ttype,
            "+" | "-" | "PM" | "UNION" | "INTERSECT" | "PERP" | "PARALLEL"
        ) {
            let mut operation = self.token.ttype.to_lowercase();
            let mut negative = false;
            let mut pm_sign = false;
            let mut positive_then_negative = false;

            if self.token.ttype == "-" {
                operation = "+".to_string();
                negative = true;
                self.advance()?;
            } else if self.token.ttype == "PM" {
                operation = "+".to_string();
                pm_sign = true;
                self.advance()?;
            } else {
                self.advance()?;
                if operation == "+" {
                    if self.token.ttype == "-" {
                        negative = true;
                        positive_then_negative = true;
                        self.advance()?;
                    } else if self.token.ttype == "PM" {
                        pm_sign = true;
                        self.advance()?;
                    }
                }
            }

            let rhs_opt = self.term(p)?;

            if operation == "+" {
                if rhs_opt.is_none() {
                    if let Some(l) = atom_string(&lhs) {
                        if positive_then_negative {
                            return Ok(Expr::sym(&format!("{}+-", l)));
                        } else if negative {
                            return Ok(Expr::sym(&format!("{}-", l)));
                        } else if !pm_sign {
                            return Ok(Expr::sym(&format!("{}+", l)));
                        }
                    }
                } else if let Some(rs) = rhs_opt.as_ref().and_then(sign_string) {
                    if let Some(l) = atom_string(&lhs) {
                        if positive_then_negative {
                            return Ok(Expr::sym(&format!("{}+-{}", l, rs)));
                        } else if negative {
                            return Ok(Expr::sym(&format!("{}-{}", l, rs)));
                        } else if !pm_sign {
                            return Ok(Expr::sym(&format!("{}+{}", l, rs)));
                        }
                    }
                }
            }

            let mut rhs = rhs_opt.unwrap_or(Expr::Blank);

            if negative {
                rhs = if is_positive_number(&rhs) {
                    negate_number(rhs)
                } else {
                    Expr::Neg(Box::new(rhs))
                };
            }

            if pm_sign {
                rhs = other_op("pm", vec![rhs]);
            }

            lhs = match operation.as_str() {
                "+" => Expr::Add(vec![lhs, rhs]),
                "union" => Expr::Union(vec![lhs, rhs]),
                "intersect" => Expr::Intersect(vec![lhs, rhs]),
                other => other_op(other, vec![lhs, rhs]), // perp, parallel
            };
        }

        Ok(lhs)
    }

    fn term(&mut self, p: P) -> R<Option<Expr>> {
        let mut lhs = self.factor(p)?;

        loop {
            if self.token.ttype == "*" {
                self.advance()?;
                let l = lhs.take().unwrap_or(Expr::Blank);
                let rhs = self.factor(p)?.unwrap_or(Expr::Blank);
                lhs = Some(Expr::Mul(vec![l, rhs]));
            } else if self.token.ttype == "/" {
                self.advance()?;
                let l = lhs.take().unwrap_or(Expr::Blank);
                let rhs = self.factor(p)?.unwrap_or(Expr::Blank);
                lhs = Some(Expr::Div(Box::new(l), Box::new(rhs)));
            } else {
                // the one case where | could close an absolute value
                let p2 = P {
                    allow_absolute_value_closing: true,
                    ..p
                };
                match self.non_minus_factor(p2)? {
                    Some(rhs) => {
                        let l = lhs.take().unwrap_or(Expr::Blank);
                        lhs = Some(Expr::Mul(vec![l, rhs]));
                    }
                    None => break,
                }
            }
        }

        Ok(lhs.map(|e| self.convert_units_in_term(flatten(e))))
    }

    fn convert_units_in_term(&self, tree: Expr) -> Expr {
        match tree {
            Expr::Mul(ops) => {
                let n = ops.len();
                for (ind, op) in ops.iter().enumerate() {
                    let Expr::Sym(s) = op else { continue };
                    let name = s.name();
                    let Some(&prefix) = self.units.get(name.as_str()) else {
                        continue;
                    };
                    if prefix && ind < n - 1 {
                        let post = if ind == n - 2 {
                            ops[n - 1].clone()
                        } else {
                            self.convert_units_in_term(Expr::Mul(ops[ind + 1..].to_vec()))
                        };
                        let unit_tree = other_op("unit", vec![op.clone(), post]);
                        return if ind == 0 {
                            unit_tree
                        } else {
                            let mut rest = ops[..ind].to_vec();
                            rest.push(unit_tree);
                            Expr::Mul(rest)
                        };
                    } else if !prefix && ind > 0 {
                        let pre = if ind == 1 {
                            ops[0].clone()
                        } else {
                            Expr::Mul(ops[..ind].to_vec())
                        };
                        let unit_tree = other_op("unit", vec![pre, op.clone()]);
                        return if ind == n - 1 {
                            unit_tree
                        } else {
                            let mut rest = vec![unit_tree];
                            rest.extend(ops[ind + 1..].iter().cloned());
                            self.convert_units_in_term(Expr::Mul(rest))
                        };
                    }
                }
                Expr::Mul(ops)
            }
            Expr::Div(a, b) => Expr::Div(
                Box::new(self.convert_units_in_term(*a)),
                Box::new(self.convert_units_in_term(*b)),
            ),
            other => other,
        }
    }

    fn factor(&mut self, p: P) -> R<Option<Expr>> {
        // NOTE: JS checks token_text (not token_type) for "+" here.
        if self.token.text == "+" {
            self.advance()?;
            let f = self.factor(p)?;
            return Ok(Some(match f {
                None => Expr::sym("+"),
                Some(e) => match sign_string(&e) {
                    Some(s) => Expr::sym(&format!("+{}", s)),
                    None => Expr::Add(vec![e]),
                },
            }));
        }

        if self.token.ttype == "-" {
            self.advance()?;
            let f = self.factor(p)?;
            return Ok(Some(match f {
                Some(e) if is_positive_number(&e) => negate_number(e),
                None => Expr::sym("-"),
                Some(e) => match sign_string(&e) {
                    Some(s) => Expr::sym(&format!("-{}", s)),
                    None => Expr::Neg(Box::new(e)),
                },
            }));
        }

        let mut result = self.non_minus_factor(p)?;

        if result.is_none() && self.token.ttype == "PERP" {
            result = Some(Expr::sym("perp"));
            self.advance()?;
        }

        Ok(result)
    }

    fn non_minus_factor(&mut self, p: P) -> R<Option<Expr>> {
        let mut result = self.base_factor(p)?;

        // allow arbitrary sequence of exponents, factorials, primes
        while matches!(self.token.ttype, "^" | "!" | "'") {
            let r = result.take().unwrap_or(Expr::Blank);
            result = Some(match self.token.ttype {
                "^" => {
                    self.advance()?;
                    let superscript = self.get_subsuperscript(p)?;
                    Expr::Pow(Box::new(r), Box::new(superscript))
                }
                "!" => {
                    self.advance()?;
                    Expr::Apply(Box::new(Expr::sym("factorial")), vec![r])
                }
                _ => {
                    self.advance()?;
                    Expr::Prime(Box::new(r))
                }
            });
        }

        Ok(result)
    }

    fn get_subsuperscript(&mut self, p: P) -> R<Expr> {
        if matches!(self.token.ttype, "+" | "-" | "PERP") {
            let subresult = self.token.ttype.to_lowercase();
            self.advance()?;
            return Ok(Expr::sym(&subresult));
        }
        let subresult = self.base_factor(P {
            parse_absolute_value: p.parse_absolute_value,
            in_subsuperscript: true,
            ..P::default()
        })?;
        Ok(subresult.unwrap_or(Expr::Blank))
    }

    fn base_factor(&mut self, p: P) -> R<Option<Expr>> {
        let mut result: Option<Expr> = None;

        if self.token.ttype == "NUMBER" {
            let v = parse_js_float(&self.token.text);
            result = Some(Expr::Num(Number::from_f64(v)));
            self.advance()?;
        } else if self.token.ttype == "INFINITY" {
            result = Some(Expr::Const(MathConst::Inf));
            self.advance()?;
        } else if self.token.ttype == "VAR" || self.token.ttype == "VARMULTICHAR" {
            let name = self.token.text.clone();

            if self.opts.applied_function_symbols.contains(&name)
                || self.opts.function_symbols.contains(&name)
            {
                return self.function_var(p, &name).map(Some);
            } else if self.opts.operator_symbols.contains(&name) {
                self.advance()?;

                if self.token.ttype == "(" {
                    self.advance()?;
                    let args = self.statement_list()?;
                    if self.token.ttype != ")" {
                        return Err(self.err("Expecting )"));
                    }
                    self.advance()?;
                    result = Some(match args {
                        Expr::Seq(SeqKind::List, xs) => other_op(&name, xs),
                        a => other_op(&name, vec![a]),
                    });
                } else {
                    let arg = self
                        .factor(P {
                            parse_absolute_value: p.parse_absolute_value,
                            ..P::default()
                        })?
                        .unwrap_or(Expr::Blank);
                    result = Some(other_op(&name, vec![arg]));
                }
            } else {
                // possibly a derivative in Leibniz notation
                if self.opts.parse_leibniz_notation {
                    let original_state = self.state();
                    match self.leibniz_notation()? {
                        Some(r) => return Ok(Some(r)),
                        None => self.set_state(original_state),
                    }
                }

                // determine if should split text into single letter factors
                let mut split = self.opts.split_symbols;
                if split
                    && (self.token.ttype == "VARMULTICHAR"
                        || self.opts.unsplit_symbols.contains(&name)
                        || name.chars().count() == 1
                        || name.chars().any(|c| c.is_ascii_digit()))
                {
                    split = false;
                }

                if split {
                    // put characters back on the input separated by spaces,
                    // then process again
                    for ch in name.chars().rev() {
                        self.lexer.unput(" ");
                        let mut buf = [0u8; 4];
                        self.lexer.unput(ch.encode_utf8(&mut buf));
                    }
                    self.advance()?;
                    // NOTE: in_subsuperscript is deliberately dropped here,
                    // matching the JS (this makes "x^ab" parse as (x^a)*b).
                    return self.base_factor(P {
                        inside_absolute_value: p.inside_absolute_value,
                        parse_absolute_value: p.parse_absolute_value,
                        allow_absolute_value_closing: p.allow_absolute_value_closing,
                        ..P::default()
                    });
                } else {
                    result = Some(Expr::sym(&name));
                    self.advance()?;
                }
            }
        } else if matches!(self.token.ttype, "(" | "[" | "{" | "LANGLE") {
            result = Some(self.bracketed(p)?);
        } else if self.token.ttype == "|"
            && p.parse_absolute_value
            && (p.inside_absolute_value == 0 || !p.allow_absolute_value_closing)
        {
            let inside = p.inside_absolute_value + 1;
            self.advance()?;
            let st = self.statement(P {
                inside_absolute_value: inside,
                ..P::default()
            })?;
            if self.token.ttype != "|" {
                return Err(self.err("Expecting |"));
            }
            self.advance()?;
            result = Some(Expr::Apply(Box::new(Expr::sym("abs")), vec![st]));
        } else if self.token.ttype == "ANGLE" {
            result = self.angle_factor(p)?;
        } else if self.token.ttype == "INT" {
            return self.integral_factor(p).map(Some);
        }

        if self.token.ttype == "_" {
            let r = result.unwrap_or(Expr::Blank);
            self.advance()?;
            let subscript = self.get_subsuperscript(p)?;
            result = Some(Expr::Index(Box::new(r), Box::new(subscript)));
        }

        Ok(result)
    }

    /// The VAR branch for function symbols (applied or unapplied).
    fn function_var(&mut self, p: P, name: &str) -> R<Expr> {
        let must_apply = self
            .opts
            .applied_function_symbols
            .contains(&name.to_string());
        let mut result = Expr::sym(name);
        self.advance()?;

        if self.token.ttype == "_" {
            self.advance()?;
            let subscript = self.get_subsuperscript(P {
                parse_absolute_value: p.parse_absolute_value,
                ..P::default()
            })?;
            if name == "log" && subscript == Expr::int(10) {
                result = Expr::sym("log10");
            } else {
                result = Expr::Index(Box::new(result), Box::new(subscript));
            }
        }

        if p.in_subsuperscript {
            if must_apply {
                result = Expr::Apply(Box::new(result), vec![Expr::Blank]);
            }
        } else {
            while self.token.ttype == "'" {
                result = Expr::Prime(Box::new(result));
                self.advance()?;
            }

            while self.token.ttype == "^" {
                self.advance()?;
                let superscript = self.get_subsuperscript(P {
                    parse_absolute_value: p.parse_absolute_value,
                    ..P::default()
                })?;
                result = Expr::Pow(Box::new(result), Box::new(superscript));
            }

            if self.token.ttype == "(" {
                self.advance()?;
                let parameters = self.statement_list()?;
                if self.token.ttype != ")" {
                    return Err(self.err("Expecting )"));
                }
                self.advance()?;

                let args = match parameters {
                    // rename from list to tuple → native multi-arg apply
                    Expr::Seq(SeqKind::List, xs) => xs,
                    other => vec![other],
                };
                result = Expr::Apply(Box::new(result), args);
            } else if must_apply {
                // an applied function symbol cannot omit its argument
                if !self.opts.allow_simplified_function_application {
                    return Err(self.err("Expecting ( after function"));
                }
                // simplified application: argument is the next factor
                let arg = self
                    .factor(P {
                        parse_absolute_value: p.parse_absolute_value,
                        ..P::default()
                    })?
                    .unwrap_or(Expr::Blank);
                result = Expr::Apply(Box::new(result), vec![arg]);
            }
        }

        Ok(result)
    }

    /// ( [ { ⟨ … grouping, tuples/arrays/sets/altvectors, half-open intervals.
    fn bracketed(&mut self, _p: P) -> R<Expr> {
        let token_left = self.token.ttype;
        let (expected_right, other_right) = match token_left {
            "(" => (")", Some("]")),
            "[" => ("]", Some(")")),
            "{" => ("}", None),
            _ => ("RANGLE", None),
        };

        self.advance()?;
        let mut result = self.statement_list()?;

        let n_elements = match &result {
            Expr::Seq(SeqKind::List, xs) => xs.len(),
            _ => 1,
        };

        if self.token.ttype != expected_right {
            let other_right = match other_right {
                Some(r) if n_elements == 2 => r,
                _ => return Err(self.err(format!("Expecting {}", expected_right))),
            };
            if self.token.ttype != other_right {
                return Err(self.err("Expecting ) or ]"));
            }
            // half-open interval
            let Expr::Seq(SeqKind::List, xs) = result else {
                unreachable!("n_elements == 2 implies a list");
            };
            let mut it = xs.into_iter();
            let a = it.next().unwrap();
            let b = it.next().unwrap();
            let closed = if token_left == "(" {
                (false, true)
            } else {
                (true, false)
            };
            result = Expr::Interval {
                endpoints: Box::new((a, b)),
                closed,
            };
        } else if n_elements >= 2 {
            let kind = match token_left {
                "(" => SeqKind::Tuple,
                "[" => SeqKind::Array,
                "{" => SeqKind::Set,
                _ => SeqKind::AltVector,
            };
            if let Expr::Seq(_, xs) = result {
                result = Expr::Seq(kind, xs);
            }
        } else if token_left == "{" {
            // singleton set (also covers set-builder | and :)
            result = Expr::Seq(SeqKind::Set, vec![result]);
        }
        // single element in ( [ ⟨: plain grouping — result unchanged

        self.advance()?;
        Ok(result)
    }

    fn angle_factor(&mut self, p: P) -> R<Option<Expr>> {
        self.advance()?;

        if self.token.ttype == "(" {
            self.advance()?;
            let parameters = self.statement_list()?;
            if self.token.ttype != ")" {
                return Err(self.err("Expecting )"));
            }
            self.advance()?;

            // JS: only a list or a product is recognised here; anything else
            // leaves result false.
            Ok(match parameters {
                Expr::Seq(SeqKind::List, xs) => Some(other_op("angle", xs)),
                m @ Expr::Mul(_) => Some(other_op("angle", vec![m])),
                _ => None,
            })
        } else {
            // angle not followed by ( — collect non-minus factors
            let mut args = vec![];
            while let Some(sub) = self.non_minus_factor(P {
                parse_absolute_value: p.parse_absolute_value,
                ..P::default()
            })? {
                args.push(sub);
            }
            Ok(Some(if args.is_empty() {
                Expr::sym("angle")
            } else {
                other_op("angle", args)
            }))
        }
    }

    fn integral_factor(&mut self, p: P) -> R<Expr> {
        self.advance()?;

        let mut head = Expr::sym("int");

        if self.token.ttype == "_" {
            self.advance()?;
            let subscript = self.get_subsuperscript(p)?;
            head = Expr::Index(Box::new(head), Box::new(subscript));
        }
        if self.token.ttype == "^" {
            self.advance()?;
            let superscript = self.get_subsuperscript(p)?;
            head = Expr::Pow(Box::new(head), Box::new(superscript));
        }

        let integrand = self.term(P {
            parse_absolute_value: p.parse_absolute_value,
            ..P::default()
        })?;
        // (JS can produce `false` here for a bare "int"; blank is the closest
        // representable tree)
        let mut integrand = integrand.map(flatten).unwrap_or(Expr::Blank);

        if let Expr::Mul(ops) = &mut integrand {
            // extract consecutive factors "d", x as differentials ["d", x]
            let mut ds = vec![];
            let mut i = 0;
            while i + 1 < ops.len() {
                if ops[i] == Expr::sym("d") {
                    let factor2 = ops.remove(i + 1);
                    ops.remove(i);
                    ds.push(other_op("d", vec![factor2]));
                    // do not advance i: re-check the same position (JS i--)
                } else {
                    i += 1;
                }
            }
            ops.extend(ds);
        }

        Ok(Expr::Apply(Box::new(head), vec![integrand]))
    }

    /// Attempt to parse a derivative in Leibniz notation (dy/dx, ∂²f/∂x∂y…).
    /// Returns None (with the caller restoring lexer state) on failure.
    fn leibniz_notation(&mut self) -> R<Option<Expr>> {
        let chars: Vec<char> = self.token.text.chars().collect();

        let valid_start = self.token.ttype == "VAR"
            && !chars.is_empty()
            && (chars[0] == 'd' || chars[0] == '∂')
            && (chars.len() == 1 || (chars.len() == 2 && chars[1].is_ascii_alphabetic()));
        if !valid_start {
            return Ok(None);
        }

        let deriv_symbol = chars[0];
        let mut n_deriv: f64 = 1.0;
        let var1: String;
        let mut var2s: Vec<String> = vec![];
        let mut var2_exponents: Vec<f64> = vec![];

        if chars.len() == 2 {
            var1 = chars[1].to_string();
        } else {
            // just a d or ∂: must be followed by ^ or a variable without ∂
            self.advance()?;
            if self.token.ttype == "VARMULTICHAR"
                || (self.token.ttype == "VAR" && !self.token.text.contains('∂'))
            {
                var1 = self.token.text.clone();
            } else {
                if self.token.ttype != "^" {
                    return Ok(None);
                }
                self.advance()?;
                if self.token.ttype != "NUMBER" {
                    return Ok(None);
                }
                n_deriv = parse_js_float(&self.token.text);
                if n_deriv.fract() != 0.0 {
                    return Ok(None);
                }
                self.advance()?;
                if (self.token.ttype == "VAR" && !self.token.text.contains('∂'))
                    || self.token.ttype == "VARMULTICHAR"
                {
                    var1 = self.token.text.clone();
                } else {
                    return Ok(None);
                }
            }
        }

        self.advance()?;
        if self.token.ttype != "/" {
            return Ok(None);
        }

        let mut exponent_sum = 0.0;
        self.advance()?;

        loop {
            // next must be a VAR starting with the derivative symbol
            if self.token.ttype != "VAR" || !self.token.text.starts_with(deriv_symbol) {
                return Ok(None);
            }

            let tchars: Vec<char> = self.token.text.chars().collect();
            if tchars.len() > 2 {
                // put extra characters back on the lexer, keep two
                let rest: String = tchars[2..].iter().collect();
                self.lexer.unput(&rest);
                self.token.text = tchars[..2].iter().collect();
            }

            let tchars: Vec<char> = self.token.text.chars().collect();
            if tchars.len() == 2 {
                if tchars[1].is_ascii_alphabetic() {
                    var2s.push(tchars[1].to_string());
                } else {
                    return Ok(None);
                }
            } else {
                // token was just the derivative symbol
                self.advance()?;
                if (self.token.ttype == "VAR" && !self.token.text.contains('∂'))
                    || self.token.ttype == "VARMULTICHAR"
                {
                    var2s.push(self.token.text.clone());
                } else {
                    return Ok(None);
                }
            }

            // optional ^ integer (no spaces before ^)
            let mut this_exponent: f64 = 1.0;
            let mut last_was_space = false;

            self.advance_opts(false)?;
            if self.token.ttype == "SPACE" {
                last_was_space = true;
                self.advance()?;
            }

            if self.token.ttype == "^" {
                self.advance()?;
                if self.token.ttype != "NUMBER" {
                    return Ok(None);
                }
                this_exponent = parse_js_float(&self.token.text);
                if this_exponent.fract() != 0.0 {
                    return Ok(None);
                }
                last_was_space = false;
                self.advance_opts(false)?;
                if self.token.ttype == "SPACE" {
                    last_was_space = true;
                    self.advance()?;
                }
            }

            var2_exponents.push(this_exponent);
            exponent_sum += this_exponent;

            if exponent_sum > n_deriv {
                return Ok(None);
            }

            if exponent_sum == n_deriv {
                // the derivative must be separated from what follows
                if !last_was_space
                    && (self.token.ttype == "VAR" || self.token.ttype == "VARMULTICHAR")
                {
                    return Ok(None);
                }
                if self.token.ttype == "SPACE" {
                    self.advance()?;
                }

                let result_name = if deriv_symbol == '∂' {
                    "partial_derivative_leibniz"
                } else {
                    "derivative_leibniz"
                };

                let arg1 = if n_deriv == 1.0 {
                    Expr::sym(&var1)
                } else {
                    Expr::Seq(
                        SeqKind::Tuple,
                        vec![Expr::sym(&var1), Expr::Num(Number::from_f64(n_deriv))],
                    )
                };

                let r2: Vec<Expr> = var2s
                    .iter()
                    .zip(&var2_exponents)
                    .map(|(v, &e)| {
                        if e == 1.0 {
                            Expr::sym(v)
                        } else {
                            Expr::Seq(
                                SeqKind::Tuple,
                                vec![Expr::sym(v), Expr::Num(Number::from_f64(e))],
                            )
                        }
                    })
                    .collect();

                return Ok(Some(other_op(
                    result_name,
                    vec![arg1, Expr::Seq(SeqKind::Tuple, r2)],
                )));
            }
        }
    }
}
