//! Recursive descent LaTeX parser — a faithful port of
//! lib/converters/latex-to-ast.js. Shares the pure helpers in
//! `super::common` and the lexer engine (LaTeX flavour) with the text parser.
//!
//! Structurally close to `text.rs`; the LaTeX-specific parts are: the lexer
//! rule table, `\begin{matrix}` environments, `\sqrt`, `\frac`/`\binom`/etc.
//! operator symbols, `\lfloor`/`\lceil`, `LATEXCOMMAND` validation, the
//! brace-based Leibniz notation, and the `\circ` exponent unit.

use super::common::{
    atom_string, is_positive_number, negate_number, other_op, parse_js_float, sign_string, P,
};
use super::error::ParseError;
use super::lexer::{Lexer, LexerState, Tok, Token};
use crate::expr::{flatten, Expr, MathConst, RelOp, SeqKind};
use crate::num::Number;
use std::collections::HashSet;

type R<T> = Result<T, ParseError>;

#[derive(Debug, Clone)]
pub struct LatexToAstOptions {
    pub allow_simplified_function_application: bool,
    pub allowed_latex_symbols: Vec<String>,
    pub applied_function_symbols: Vec<String>,
    pub function_symbols: Vec<String>,
    pub parse_leibniz_notation: bool,
    pub parse_scientific_notation: bool,
}

impl Default for LatexToAstOptions {
    fn default() -> Self {
        fn v(names: &[&str]) -> Vec<String> {
            names.iter().map(|s| s.to_string()).collect()
        }
        LatexToAstOptions {
            allow_simplified_function_application: true,
            allowed_latex_symbols: v(&[
                "alpha", "beta", "gamma", "Gamma", "delta", "Delta", "epsilon", "zeta", "eta",
                "theta", "Theta", "iota", "kappa", "lambda", "Lambda", "mu", "nu", "xi", "Xi",
                "pi", "Pi", "rho", "sigma", "Sigma", "tau", "Tau", "upsilon", "Upsilon", "phi",
                "Phi", "chi", "psi", "Psi", "omega", "Omega", "partial", "angle", "circ", "%", "$",
                "emptyset",
            ]),
            applied_function_symbols: v(&[
                "abs", "exp", "log", "ln", "log10", "sign", "sqrt", "erf", "cos", "cosh", "acos",
                "acosh", "arccos", "arccosh", "cot", "coth", "acot", "acoth", "arccot", "arccoth",
                "csc", "csch", "acsc", "acsch", "arccsc", "arccsch", "sec", "sech", "asec",
                "asech", "arcsec", "arcsech", "sin", "sinh", "asin", "asinh", "arcsin", "arcsinh",
                "tan", "tanh", "atan", "atan2", "atanh", "arctan", "arctanh", "arg", "conj", "Re",
                "Im", "det", "trace", "nPr", "nCr", "floor", "ceil", "round",
            ]),
            function_symbols: v(&["f", "g"]),
            parse_leibniz_notation: true,
            parse_scientific_notation: true,
        }
    }
}

/// A `\frac`/`\binom`/`\vec`/`\overline` operator symbol.
struct OpSym {
    nargs: usize,
    substitute: Option<&'static str>,
    remove_products: bool,
}

fn operator_symbol(name: &str) -> Option<OpSym> {
    match name {
        "frac" => Some(OpSym {
            nargs: 2,
            substitute: Some("/"),
            remove_products: false,
        }),
        "binom" => Some(OpSym {
            nargs: 2,
            substitute: None,
            remove_products: false,
        }),
        "vec" => Some(OpSym {
            nargs: 1,
            substitute: None,
            remove_products: false,
        }),
        "overline" => Some(OpSym {
            nargs: 1,
            substitute: Some("linesegment"),
            remove_products: true,
        }),
        _ => None,
    }
}

/// Parse-time unit shape (prefix vs postfix, exponent units, substitution).
struct Unit {
    prefix: bool,
    substitute: Option<&'static str>,
    is_exponent: bool,
}

fn unit_of(name: &str) -> Option<Unit> {
    match name {
        "%" => Some(Unit {
            prefix: false,
            substitute: None,
            is_exponent: false,
        }),
        "$" => Some(Unit {
            prefix: true,
            substitute: None,
            is_exponent: false,
        }),
        // \circ acts as a postfix exponent unit substituting for degrees.
        "circ" => Some(Unit {
            prefix: false,
            substitute: Some("deg"),
            is_exponent: true,
        }),
        _ => None,
    }
}

/// JS `this.token.token_type[0] === "|"` — the token is `|` or `|L`.
fn is_pipe(tt: Tok) -> bool {
    matches!(tt, Tok::Pipe | Tok::PipeL)
}

pub struct LatexToAst {
    opts: LatexToAstOptions,
    lexer: Lexer,
    token: Token,
    // The option symbol lists, as sets — looked up per identifier token.
    applied_functions: HashSet<String>,
    functions: HashSet<String>,
    allowed_symbols: HashSet<String>,
}

impl LatexToAst {
    pub fn new(opts: LatexToAstOptions) -> Self {
        let lexer = Lexer::new_latex(opts.parse_scientific_notation);
        LatexToAst {
            applied_functions: opts.applied_function_symbols.iter().cloned().collect(),
            functions: opts.function_symbols.iter().cloned().collect(),
            allowed_symbols: opts.allowed_latex_symbols.iter().cloned().collect(),
            opts,
            lexer,
            token: Token {
                ttype: Tok::Eof,
                text: String::new(),
                original: String::new(),
            },
        }
    }

    fn advance(&mut self) -> R<()> {
        self.token = self.lexer.advance(true);
        if self.token.ttype == Tok::Invalid {
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

    fn is_applied(&self, name: &str) -> bool {
        self.applied_functions.contains(name)
    }
    fn is_function(&self, name: &str) -> bool {
        self.functions.contains(name)
    }
    fn is_allowed_symbol(&self, name: &str) -> bool {
        self.allowed_symbols.contains(name)
    }

    pub fn convert(&mut self, input: &str) -> R<Expr> {
        self.lexer.set_input(input);
        self.advance()?;
        let result = self.statement_list()?;
        if self.token.ttype != Tok::Eof {
            return Err(self.err(format!("Invalid location of '{}'", self.token.original)));
        }
        Ok(flatten(result))
    }

    fn statement_list(&mut self) -> R<Expr> {
        let mut list = vec![self.statement(P::default())?];
        while self.token.ttype == Tok::Comma {
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
        if self.token.ttype == Tok::Ldots {
            self.advance()?;
            return Ok(Expr::Ldots);
        }

        let original_state = self.state();

        match self.statement_main(p) {
            Ok(r) => Ok(r),
            Err(e) => {
                // retry: ignore absolute value, treat bar as a binary operator
                self.set_state(original_state);
                match self.statement_bar_fallback() {
                    Ok(r) => Ok(r),
                    Err(_) => Err(e),
                }
            }
        }
    }

    fn statement_main(&mut self, p: P) -> R<Expr> {
        let lhs = self.statement_a(P {
            inside_absolute_value: p.inside_absolute_value,
            ..P::default()
        })?;

        if self.token.ttype != Tok::Colon && self.token.ttype != Tok::Mid {
            return Ok(lhs);
        }
        let operator = if self.token.ttype == Tok::Colon {
            ":"
        } else {
            "|"
        };
        self.advance()?;
        let rhs = self.statement_a(P::default())?;
        Ok(other_op(operator, vec![lhs, rhs]))
    }

    fn statement_bar_fallback(&mut self) -> R<Expr> {
        let lhs = self.statement_a(P {
            parse_absolute_value: false,
            ..P::default()
        })?;
        if !is_pipe(self.token.ttype) {
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
            Tok::Implies
                | Tok::ImpliedBy
                | Tok::Iff
                | Tok::LeftArrow
                | Tok::RightArrow
                | Tok::LeftRightArrow
        ) {
            let operation = self.token.ttype.op_name();
            self.advance()?;
            let rhs = self.statement_b(fwd)?;
            lhs = other_op(operation, vec![lhs, rhs]);
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
        while self.token.ttype == Tok::Or {
            self.advance()?;
            let rhs = self.statement_c(fwd)?;
            lhs = Expr::Or(vec![lhs, rhs]);
        }
        Ok(lhs)
    }

    fn statement_c(&mut self, p: P) -> R<Expr> {
        let mut lhs = self.relation(p)?;
        while self.token.ttype == Tok::And {
            self.advance()?;
            let rhs = self.relation(p)?;
            lhs = Expr::And(vec![lhs, rhs]);
        }
        Ok(lhs)
    }

    fn relation(&mut self, p: P) -> R<Expr> {
        if self.token.ttype == Tok::Not || self.token.ttype == Tok::Bang {
            self.advance()?;
            return Ok(Expr::Not(Box::new(self.relation(p)?)));
        }
        if self.token.ttype == Tok::Forall || self.token.ttype == Tok::Exists {
            let operator = self.token.ttype.op_name();
            self.advance()?;
            return Ok(other_op(operator, vec![self.relation(p)?]));
        }

        let mut lhs = self.expression(p)?;

        loop {
            let op = match self.token.ttype {
                Tok::Eq => Some(RelOp::Eq),
                Tok::Ne => Some(RelOp::Ne),
                Tok::Lt => Some(RelOp::Lt),
                Tok::Gt => Some(RelOp::Gt),
                Tok::Le => Some(RelOp::Le),
                Tok::Ge => Some(RelOp::Ge),
                Tok::In => Some(RelOp::In),
                Tok::NotIn => Some(RelOp::NotIn),
                Tok::Ni => Some(RelOp::Ni),
                Tok::NotNi => Some(RelOp::NotNi),
                Tok::Subset => Some(RelOp::Subset),
                Tok::NotSubset => Some(RelOp::NotSubset),
                Tok::SubsetEq => Some(RelOp::SubsetEq),
                Tok::NotSubsetEq => Some(RelOp::NotSubsetEq),
                Tok::Superset => Some(RelOp::Superset),
                Tok::NotSuperset => Some(RelOp::NotSuperset),
                Tok::SupersetEq => Some(RelOp::SupersetEq),
                Tok::NotSupersetEq => Some(RelOp::NotSupersetEq),
                _ => None,
            };
            let Some(op) = op else { break };
            self.advance()?;
            let rhs = self.expression(p)?;

            match op {
                RelOp::Lt | RelOp::Le
                    if self.token.ttype == Tok::Lt || self.token.ttype == Tok::Le =>
                {
                    let mut ops = vec![op];
                    let mut operands = vec![lhs, rhs];
                    while self.token.ttype == Tok::Lt || self.token.ttype == Tok::Le {
                        ops.push(if self.token.ttype == Tok::Lt {
                            RelOp::Lt
                        } else {
                            RelOp::Le
                        });
                        self.advance()?;
                        operands.push(self.expression(p)?);
                    }
                    lhs = Expr::Relation { operands, ops };
                }
                RelOp::Gt | RelOp::Ge
                    if self.token.ttype == Tok::Gt || self.token.ttype == Tok::Ge =>
                {
                    let mut ops = vec![op];
                    let mut operands = vec![lhs, rhs];
                    while self.token.ttype == Tok::Gt || self.token.ttype == Tok::Ge {
                        ops.push(if self.token.ttype == Tok::Gt {
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
                    while self.token.ttype == Tok::Eq {
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
        if self.token.ttype == Tok::Not || self.token.ttype == Tok::Bang {
            self.advance()?;
            return Ok(Expr::Not(Box::new(self.expression(p)?)));
        }

        let mut plus_begin = false;
        if self.token.ttype == Tok::Plus {
            plus_begin = true;
            self.advance()?;
        }
        let mut negative_begin = false;
        if self.token.ttype == Tok::Minus {
            negative_begin = true;
            self.advance()?;
        }
        let mut pm_begin = false;
        if self.token.ttype == Tok::Pm {
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
            Tok::Plus
                | Tok::Minus
                | Tok::Pm
                | Tok::Union
                | Tok::Intersect
                | Tok::Perp
                | Tok::Parallel
        ) {
            let op_token = self.token.ttype;
            // Minus and plus-minus contribute a sign to an addition.
            let is_add = matches!(op_token, Tok::Plus | Tok::Minus | Tok::Pm);
            let mut negative = false;
            let mut pm_sign = false;
            let mut positive_then_negative = false;

            if op_token == Tok::Minus {
                negative = true;
                self.advance()?;
            } else if op_token == Tok::Pm {
                pm_sign = true;
                self.advance()?;
            } else {
                self.advance()?;
                if op_token == Tok::Plus {
                    if self.token.ttype == Tok::Minus {
                        negative = true;
                        positive_then_negative = true;
                        self.advance()?;
                    } else if self.token.ttype == Tok::Pm {
                        pm_sign = true;
                        self.advance()?;
                    }
                }
            }

            let rhs_opt = self.term(p)?;

            if is_add {
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

            lhs = match op_token {
                Tok::Plus | Tok::Minus | Tok::Pm => Expr::Add(vec![lhs, rhs]),
                Tok::Union => Expr::Union(vec![lhs, rhs]),
                Tok::Intersect => Expr::Intersect(vec![lhs, rhs]),
                // perp, parallel
                _ => other_op(op_token.op_name(), vec![lhs, rhs]),
            };
        }

        Ok(lhs)
    }

    fn term(&mut self, p: P) -> R<Option<Expr>> {
        let mut lhs = self.factor(p)?;

        loop {
            if self.token.ttype == Tok::Times {
                self.advance()?;
                let l = lhs.take().unwrap_or(Expr::Blank);
                let rhs = self.factor(p)?.unwrap_or(Expr::Blank);
                lhs = Some(Expr::Mul(vec![l, rhs]));
            } else if self.token.ttype == Tok::Slash {
                self.advance()?;
                let l = lhs.take().unwrap_or(Expr::Blank);
                let rhs = self.factor(p)?.unwrap_or(Expr::Blank);
                lhs = Some(Expr::Div(Box::new(l), Box::new(rhs)));
            } else {
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
                    let Some(unit) = unit_of(&name) else { continue };
                    let unit_name = unit.substitute.unwrap_or(&name);
                    if unit.prefix && ind < n - 1 {
                        let post = if ind == n - 2 {
                            ops[n - 1].clone()
                        } else {
                            self.convert_units_in_term(Expr::Mul(ops[ind + 1..].to_vec()))
                        };
                        let unit_tree = other_op("unit", vec![Expr::sym(unit_name), post]);
                        return if ind == 0 {
                            unit_tree
                        } else {
                            let mut rest: Vec<Expr> = ops[..ind]
                                .iter()
                                .map(|o| self.convert_units_in_term(o.clone()))
                                .collect();
                            rest.push(unit_tree);
                            Expr::Mul(rest)
                        };
                    } else if !unit.prefix && ind > 0 {
                        let pre = if ind == 1 {
                            ops[0].clone()
                        } else {
                            Expr::Mul(
                                ops[..ind]
                                    .iter()
                                    .map(|o| self.convert_units_in_term(o.clone()))
                                    .collect(),
                            )
                        };
                        let unit_tree = other_op("unit", vec![pre, Expr::sym(unit_name)]);
                        return if ind == n - 1 {
                            unit_tree
                        } else {
                            let mut rest = vec![unit_tree];
                            rest.extend(ops[ind + 1..].iter().cloned());
                            self.convert_units_in_term(Expr::Mul(rest))
                        };
                    }
                }
                Expr::Mul(
                    ops.into_iter()
                        .map(|o| self.convert_units_in_term(o))
                        .collect(),
                )
            }
            Expr::Div(a, b) => Expr::Div(
                Box::new(self.convert_units_in_term(*a)),
                Box::new(self.convert_units_in_term(*b)),
            ),
            Expr::Pow(base, exp) => {
                if let Expr::Sym(s) = exp.as_ref() {
                    let name = s.name();
                    if let Some(u) = unit_of(&name) {
                        if u.is_exponent {
                            let unit_name = u.substitute.unwrap_or(&name);
                            return other_op("unit", vec![*base, Expr::sym(unit_name)]);
                        }
                    }
                }
                Expr::Pow(
                    Box::new(self.convert_units_in_term(*base)),
                    Box::new(self.convert_units_in_term(*exp)),
                )
            }
            other => other,
        }
    }

    fn factor(&mut self, p: P) -> R<Option<Expr>> {
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

        if self.token.ttype == Tok::Minus {
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
        if result.is_none() && self.token.ttype == Tok::Perp {
            result = Some(Expr::sym("perp"));
            self.advance()?;
        }
        Ok(result)
    }

    fn non_minus_factor(&mut self, p: P) -> R<Option<Expr>> {
        let mut result = self.base_factor(p)?;

        while matches!(self.token.ttype, Tok::Caret | Tok::Bang | Tok::Prime) {
            let r = result.take().unwrap_or(Expr::Blank);
            result = Some(match self.token.ttype {
                Tok::Caret => {
                    self.advance()?;
                    let superscript = self.get_subsuperscript(p)?;
                    Expr::Pow(Box::new(r), Box::new(superscript))
                }
                Tok::Bang => {
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

    /// A single leading digit of a NUMBER token, pushing the rest back on the
    /// lexer. Used by `\frac12`, `x^23`, and operator-symbol arguments.
    fn get_single_digit_as_number(&mut self) -> R<Option<Expr>> {
        if self.token.ttype == Tok::Number && !self.token.text.starts_with('.') {
            let first = self.token.text.as_bytes()[0] as char;
            let num = (first as u8 - b'0') as i64;
            if self.token.text.len() > 1 {
                let rest = self.token.text[1..].to_string();
                self.lexer.unput(&rest);
            }
            self.advance()?;
            return Ok(Some(Expr::int(num)));
        }
        Ok(None)
    }

    fn get_subsuperscript(&mut self, p: P) -> R<Expr> {
        if let Some(num) = self.get_single_digit_as_number()? {
            return Ok(num);
        }
        if matches!(self.token.ttype, Tok::Plus | Tok::Minus | Tok::Perp) {
            let subresult = self.token.ttype.op_name();
            self.advance()?;
            return Ok(Expr::sym(subresult));
        }
        let subresult = self.base_factor(P {
            parse_absolute_value: p.parse_absolute_value,
            in_subsuperscript: true,
            ..P::default()
        })?;
        Ok(subresult.unwrap_or(Expr::Blank))
    }

    fn base_factor(&mut self, p: P) -> R<Option<Expr>> {
        if self.token.ttype == Tok::BeginEnvironment {
            return self.matrix_environment(p).map(Some);
        }

        let mut result: Option<Expr> = None;

        if self.token.ttype == Tok::Number {
            let v = parse_js_float(&self.token.text);
            // A literal can overflow f64 ("1E999"); parseFloat gives Infinity.
            result = Some(if v.is_infinite() {
                Expr::Const(MathConst::Inf)
            } else {
                Expr::Num(Number::from_f64(v))
            });
            self.advance()?;
        } else if self.token.ttype == Tok::Infinity {
            result = Some(Expr::Const(MathConst::Inf));
            self.advance()?;
        } else if self.token.ttype == Tok::Sqrt {
            result = Some(self.sqrt_factor(p)?);
        } else if matches!(
            self.token.ttype,
            Tok::Var | Tok::LatexCommand | Tok::VarMultiChar
        ) {
            match self.symbol_factor(p)? {
                SymbolResult::Return(e) => return Ok(Some(e)),
                SymbolResult::Continue(e) => result = e,
            }
        } else if matches!(
            self.token.ttype,
            Tok::LParen | Tok::LBracket | Tok::LBrace | Tok::SetLBrace | Tok::LAngle
        ) {
            result = Some(self.bracketed(p)?);
        } else if is_pipe(self.token.ttype)
            && p.parse_absolute_value
            && (p.inside_absolute_value == 0
                || !p.allow_absolute_value_closing
                || self.token.ttype == Tok::PipeL)
        {
            let inside = p.inside_absolute_value + 1;
            self.advance()?;
            let st = self.statement(P {
                inside_absolute_value: inside,
                ..P::default()
            })?;
            if self.token.ttype != Tok::Pipe {
                return Err(self.err("Expecting |"));
            }
            self.advance()?;
            result = Some(Expr::Apply(Box::new(Expr::sym("abs")), vec![st]));
        } else if matches!(self.token.ttype, Tok::LFloor | Tok::LCeil) {
            result = Some(self.floor_ceil()?);
        } else if self.token.ttype == Tok::Angle {
            result = self.angle_factor(p)?;
        } else if self.token.ttype == Tok::Int {
            return self.integral_factor(p).map(Some);
        }

        if self.token.ttype == Tok::Underscore {
            let r = result.unwrap_or(Expr::Blank);
            self.advance()?;
            let subscript = self.get_subsuperscript(p)?;
            result = Some(Expr::Index(Box::new(r), Box::new(subscript)));
        }

        Ok(result)
    }

    fn matrix_environment(&mut self, _p: P) -> R<Expr> {
        // token text is "\begin{...}"; extract the environment name.
        let environment = brace_content(&self.token.text);
        if !matches!(environment.as_str(), "matrix" | "pmatrix" | "bmatrix") {
            return Err(self.err(format!("Unrecognized environment {}", environment)));
        }

        let mut all_rows: Vec<Vec<Expr>> = vec![];
        let mut row: Vec<Expr> = vec![];
        let mut n_cols = 0usize;
        // last_token tracks &/LINEBREAK to detect blank entries.
        let mut last_token = self.token.ttype;

        self.advance()?;

        while self.token.ttype != Tok::EndEnvironment {
            if self.token.ttype == Tok::Amp {
                if last_token == Tok::Amp || last_token == Tok::Linebreak {
                    row.push(Expr::int(0));
                }
                last_token = self.token.ttype;
                self.advance()?;
            } else if self.token.ttype == Tok::Linebreak {
                if last_token == Tok::Amp || last_token == Tok::Linebreak {
                    row.push(Expr::int(0));
                }
                n_cols = n_cols.max(row.len());
                all_rows.push(std::mem::take(&mut row));
                last_token = self.token.ttype;
                self.advance()?;
            } else {
                // JS condition is always truthy here, so always parse an entry.
                row.push(self.statement(P::default())?);
                // Marks "just parsed an entry"; a space token never reaches
                // here (advance strips them), so it is free as a sentinel.
                last_token = Tok::Space;
            }
        }

        let environment2 = brace_content(&self.token.text);
        if environment2 != environment {
            return Err(self.err(format!("Expecting \\end{{{}}}", environment)));
        }

        if last_token == Tok::Amp {
            row.push(Expr::int(0));
        }
        n_cols = n_cols.max(row.len());
        all_rows.push(row);

        self.advance()?;

        let n_rows = all_rows.len();
        let mut entries = Vec::with_capacity(n_rows * n_cols);
        for mut r in all_rows {
            let have = r.len();
            entries.append(&mut r);
            for _ in have..n_cols {
                entries.push(Expr::int(0));
            }
        }

        Ok(Expr::Matrix {
            rows: n_rows as u32,
            cols: n_cols as u32,
            entries,
        })
    }

    fn sqrt_factor(&mut self, p: P) -> R<Expr> {
        self.advance()?;

        let mut root = Expr::int(2);
        if self.token.ttype == Tok::LBracket {
            self.advance()?;
            let parameter = self.statement(P {
                parse_absolute_value: p.parse_absolute_value,
                ..P::default()
            })?;
            if self.token.ttype != Tok::RBracket {
                return Err(self.err("Expecting ]"));
            }
            self.advance()?;
            root = parameter;
        }

        if self.token.ttype != Tok::LBrace {
            return Err(self.err("Expecting {"));
        }
        self.advance()?;
        let parameter = self.statement(P {
            parse_absolute_value: p.parse_absolute_value,
            ..P::default()
        })?;
        if self.token.ttype != Tok::RBrace {
            return Err(self.err("Expecting }"));
        }
        self.advance()?;

        Ok(if root == Expr::int(2) {
            Expr::Apply(Box::new(Expr::sym("sqrt")), vec![parameter])
        } else if root == Expr::int(3) {
            Expr::Apply(Box::new(Expr::sym("cbrt")), vec![parameter])
        } else {
            Expr::Apply(Box::new(Expr::sym("nthroot")), vec![parameter, root])
        })
    }

    fn floor_ceil(&mut self) -> R<Expr> {
        let (expected_right, function_name) = if self.token.ttype == Tok::LFloor {
            (Tok::RFloor, "floor")
        } else {
            (Tok::RCeil, "ceil")
        };
        self.advance()?;
        let st = self.statement(P::default())?;
        let result = Expr::Apply(Box::new(Expr::sym(function_name)), vec![st]);
        if self.token.ttype != expected_right {
            return Err(self.err(format!("Expecting {}", expected_right)));
        }
        self.advance()?;
        Ok(result)
    }

    /// The VAR / LATEXCOMMAND / VARMULTICHAR branch of baseFactor.
    fn symbol_factor(&mut self, p: P) -> R<SymbolResult> {
        let mut result = self.token.text.clone();

        if self.token.ttype == Tok::LatexCommand {
            result = result[1..].to_string(); // strip leading backslash
            if !(self.is_applied(&result)
                || self.is_function(&result)
                || self.is_allowed_symbol(&result)
                || operator_symbol(&result).is_some())
            {
                return Err(self.err(format!(
                    "Unrecognized latex command {}",
                    self.token.original
                )));
            }
        } else if self.token.ttype == Tok::VarMultiChar {
            result = brace_content(&result); // \operatorname{...}
        }

        if self.is_applied(&result) || self.is_function(&result) {
            return Ok(SymbolResult::Return(self.function_var(p, result)?));
        }

        if let Some(op) = operator_symbol(&result) {
            return Ok(SymbolResult::Continue(Some(
                self.operator_symbol_factor(p, &result, op)?,
            )));
        }

        // plain symbol
        self.advance()?;
        Ok(SymbolResult::Continue(Some(Expr::sym(&result))))
    }

    fn function_var(&mut self, p: P, name: String) -> R<Expr> {
        let must_apply = self.is_applied(&name);
        let mut name = name;
        if name == "Re" || name == "Im" {
            name = name.to_lowercase();
        }
        let mut result = Expr::sym(&name);
        let is_log = name == "log";
        self.advance()?;

        if self.token.ttype == Tok::Underscore {
            self.advance()?;
            let subscript = self.get_subsuperscript(P {
                parse_absolute_value: p.parse_absolute_value,
                ..P::default()
            })?;
            if is_log && subscript == Expr::int(10) {
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
            while self.token.ttype == Tok::Prime {
                result = Expr::Prime(Box::new(result));
                self.advance()?;
            }
            while self.token.ttype == Tok::Caret {
                self.advance()?;
                let superscript = self.get_subsuperscript(P {
                    parse_absolute_value: p.parse_absolute_value,
                    ..P::default()
                })?;
                result = Expr::Pow(Box::new(result), Box::new(superscript));
            }

            if self.token.ttype == Tok::LBrace || self.token.ttype == Tok::LParen {
                let expected_right = if self.token.ttype == Tok::LBrace {
                    Tok::RBrace
                } else {
                    Tok::RParen
                };
                self.advance()?;
                let parameters = self.statement_list()?;
                if self.token.ttype != expected_right {
                    return Err(self.err(format!("Expecting {}", expected_right)));
                }
                self.advance()?;
                let args = match parameters {
                    Expr::Seq(SeqKind::List, xs) => xs,
                    other => vec![other],
                };
                result = Expr::Apply(Box::new(result), args);
            } else if must_apply {
                if !self.opts.allow_simplified_function_application {
                    return Err(self.err("Expecting ( after function"));
                }
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

    fn operator_symbol_factor(&mut self, p: P, name: &str, op: OpSym) -> R<Expr> {
        self.advance()?;

        if name == "frac" && self.opts.parse_leibniz_notation {
            let original_state = self.state();
            match self.leibniz_notation()? {
                Some(r) => return Ok(r),
                None => self.set_state(original_state),
            }
        }

        let mut args: Vec<Expr> = vec![];
        for _ in 0..op.nargs {
            if self.token.ttype == Tok::LBrace {
                self.advance()?;
                let new_arg = self.statement(P {
                    parse_absolute_value: p.parse_absolute_value,
                    ..P::default()
                })?;
                if op.remove_products {
                    if let Expr::Mul(factors) = new_arg {
                        args.extend(factors);
                    } else {
                        args.push(new_arg);
                    }
                } else {
                    args.push(new_arg);
                }
                if self.token.ttype != Tok::RBrace {
                    return Err(self.err("Expecting }"));
                }
                self.advance()?;
            } else {
                let new_arg = match self.get_single_digit_as_number()? {
                    Some(n) => n,
                    None => {
                        if self.token.ttype == Tok::Var {
                            let v = Expr::sym(&self.token.text);
                            self.advance()?;
                            v
                        } else {
                            return Err(self.err("Expecting {"));
                        }
                    }
                };
                args.push(new_arg);
            }
        }

        Ok(match op.substitute {
            Some("/") => {
                let mut it = args.into_iter();
                let a = it.next().unwrap();
                let b = it.next().unwrap();
                Expr::Div(Box::new(a), Box::new(b))
            }
            Some(sub) => other_op(sub, args),
            None => other_op(name, args),
        })
    }

    fn bracketed(&mut self, _p: P) -> R<Expr> {
        let token_left = self.token.ttype;
        let (expected_right, other_right) = match token_left {
            Tok::LParen => (Tok::RParen, Some(Tok::RBracket)),
            Tok::LBracket => (Tok::RBracket, Some(Tok::RParen)),
            Tok::LBrace => (Tok::RBrace, None),
            Tok::SetLBrace => (Tok::SetRBrace, None),
            _ => (Tok::RAngle, None),
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
            let Expr::Seq(SeqKind::List, xs) = result else {
                unreachable!("n_elements == 2 implies a list");
            };
            let mut it = xs.into_iter();
            let a = it.next().unwrap();
            let b = it.next().unwrap();
            let closed = if token_left == Tok::LParen {
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
                Tok::LParen | Tok::LBrace => SeqKind::Tuple,
                Tok::LBracket => SeqKind::Array,
                Tok::SetLBrace => SeqKind::Set,
                _ => SeqKind::AltVector,
            };
            if let Expr::Seq(_, xs) = result {
                result = Expr::Seq(kind, xs);
            }
        } else if token_left == Tok::SetLBrace {
            result = Expr::Seq(SeqKind::Set, vec![result]);
        }

        self.advance()?;
        Ok(result)
    }

    fn angle_factor(&mut self, p: P) -> R<Option<Expr>> {
        self.advance()?;

        if self.token.ttype == Tok::LBrace || self.token.ttype == Tok::LParen {
            let expected_right = if self.token.ttype == Tok::LBrace {
                Tok::RBrace
            } else {
                Tok::RParen
            };
            self.advance()?;
            let parameters = self.statement_list()?;
            if self.token.ttype != expected_right {
                return Err(self.err(format!("Expecting {}", expected_right)));
            }
            self.advance()?;

            Ok(match parameters {
                Expr::Seq(SeqKind::List, xs) => Some(other_op("angle", xs)),
                m @ Expr::Mul(_) => Some(other_op("angle", vec![m])),
                _ => None,
            })
        } else {
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
        if self.token.ttype == Tok::Underscore {
            self.advance()?;
            let subscript = self.get_subsuperscript(p)?;
            head = Expr::Index(Box::new(head), Box::new(subscript));
        }
        if self.token.ttype == Tok::Caret {
            self.advance()?;
            let superscript = self.get_subsuperscript(p)?;
            head = Expr::Pow(Box::new(head), Box::new(superscript));
        }

        let integrand = self.term(P {
            parse_absolute_value: p.parse_absolute_value,
            ..P::default()
        })?;
        let mut integrand = integrand.map(flatten).unwrap_or(Expr::Blank);

        if let Expr::Mul(ops) = &mut integrand {
            let mut ds = vec![];
            let mut i = 0;
            while i + 1 < ops.len() {
                if ops[i] == Expr::sym("d") {
                    let factor2 = ops.remove(i + 1);
                    ops.remove(i);
                    ds.push(other_op("d", vec![factor2]));
                } else {
                    i += 1;
                }
            }
            ops.extend(ds);
        }

        Ok(Expr::Apply(Box::new(head), vec![integrand]))
    }

    /// `\frac{d^n f}{d x^n}` derivative in Leibniz notation. Assumes the
    /// `\frac` token has already been consumed; returns None (caller restores
    /// state) if the shape does not match.
    fn leibniz_notation(&mut self) -> R<Option<Expr>> {
        if self.token.ttype != Tok::LBrace {
            return Ok(None);
        }
        self.advance()?;

        // Numerator: d or \partial, optional ^ n, then the differentiated var.
        let deriv_symbol =
            if self.token.ttype == Tok::LatexCommand && &self.token.text[1..] == "partial" {
                '∂'
            } else if self.token.ttype == Tok::Var && self.token.text == "d" {
                'd'
            } else {
                return Ok(None);
            };

        let mut n_deriv: f64 = 1.0;
        self.advance()?;

        if self.token.ttype == Tok::Caret {
            self.advance()?;
            let in_braces = self.token.ttype == Tok::LBrace;
            if in_braces {
                self.advance()?;
            }
            if self.token.ttype != Tok::Number {
                return Ok(None);
            }
            n_deriv = parse_js_float(&self.token.text);
            if n_deriv.fract() != 0.0 {
                return Ok(None);
            }
            if in_braces {
                self.advance()?;
                if self.token.ttype != Tok::RBrace {
                    return Ok(None);
                }
            }
            self.advance()?;
        }

        let Some(var1) = self.leibniz_var()? else {
            return Ok(None);
        };

        // } then {
        self.advance()?;
        if self.token.ttype != Tok::RBrace {
            return Ok(None);
        }
        self.advance()?;
        if self.token.ttype != Tok::LBrace {
            return Ok(None);
        }
        self.advance()?;

        // Denominator: repeated (deriv_symbol var ^n?) until exponents sum to n.
        let mut var2s: Vec<String> = vec![];
        let mut var2_exponents: Vec<f64> = vec![];
        let mut exponent_sum = 0.0;

        loop {
            let matches_symbol =
                (deriv_symbol == 'd' && self.token.ttype == Tok::Var && self.token.text == "d")
                    || (deriv_symbol == '∂'
                        && self.token.ttype == Tok::LatexCommand
                        && &self.token.text[1..] == "partial");
            if !matches_symbol {
                return Ok(None);
            }

            self.advance()?;
            let Some(var2) = self.leibniz_var()? else {
                return Ok(None);
            };
            var2s.push(var2);

            let mut this_exponent: f64 = 1.0;
            self.advance()?;

            if self.token.ttype == Tok::Caret {
                self.advance()?;
                let in_braces = self.token.ttype == Tok::LBrace;
                if in_braces {
                    self.advance()?;
                }
                if self.token.ttype != Tok::Number {
                    return Ok(None);
                }
                this_exponent = parse_js_float(&self.token.text);
                if this_exponent.fract() != 0.0 {
                    return Ok(None);
                }
                if in_braces {
                    self.advance()?;
                    if self.token.ttype != Tok::RBrace {
                        return Ok(None);
                    }
                }
                self.advance()?;
            }

            var2_exponents.push(this_exponent);
            exponent_sum += this_exponent;

            if exponent_sum > n_deriv {
                return Ok(None);
            }
            if exponent_sum == n_deriv {
                if self.token.ttype != Tok::RBrace {
                    return Ok(None);
                }
                self.advance()?;

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

    /// A single differentiation variable in Leibniz notation: a VAR, an
    /// `\operatorname{...}`, or a LATEXCOMMAND in allowed symbols. Does not
    /// advance past it (caller advances). Returns None on mismatch.
    fn leibniz_var(&mut self) -> R<Option<String>> {
        Ok(match self.token.ttype {
            Tok::Var => Some(self.token.text.clone()),
            Tok::VarMultiChar => Some(brace_content(&self.token.text)),
            Tok::LatexCommand => {
                let name = self.token.text[1..].to_string();
                if self.is_allowed_symbol(&name) {
                    Some(name)
                } else {
                    None
                }
            }
            _ => None,
        })
    }
}

/// Result of the symbol branch: either a fully-formed factor to return
/// immediately (functions handle their own trailing subscripts), or a value
/// to continue baseFactor with (for the trailing `_` subscript handling).
enum SymbolResult {
    Return(Expr),
    Continue(Option<Expr>),
}

/// Extract the identifier inside a braced LaTeX token like `\begin{matrix}`,
/// `\end{matrix}`, or `\operatorname{name}`.
fn brace_content(s: &str) -> String {
    let start = s.find('{').map(|i| i + 1).unwrap_or(0);
    let end = s.rfind('}').unwrap_or(s.len());
    s[start..end].trim().to_string()
}
