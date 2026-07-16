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

use super::common::{
    atom_string, is_positive_number, negate_number, other_op, parse_js_float, sign_string,
    MAX_PARSE_DEPTH, P,
};
use super::error::ParseError;
use super::lexer::{Lexer, LexerState, Tok, Token};
use crate::expr::{flatten, Expr, MathConst, RelOp, SeqKind};
use crate::num::Number;
use std::collections::HashSet;

type R<T> = Result<T, ParseError>;

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
/// parse time; `Some(prefix)` says whether the unit precedes its value.
fn unit_prefix(name: &str) -> Option<bool> {
    match name {
        "%" | "deg" => Some(false),
        "$" => Some(true),
        _ => None,
    }
}

pub struct TextToAst {
    opts: TextToAstOptions,
    lexer: Lexer,
    token: Token,
    // The option symbol lists, as sets — looked up per identifier token.
    applied_functions: HashSet<String>,
    functions: HashSet<String>,
    operators: HashSet<String>,
    unsplit: HashSet<String>,
    /// Recursion depth through the self-recursive parse functions (§6e).
    depth: usize,
}

impl TextToAst {
    pub fn new(opts: TextToAstOptions) -> Self {
        let lexer = Lexer::new(opts.parse_scientific_notation);
        TextToAst {
            applied_functions: opts.applied_function_symbols.iter().cloned().collect(),
            functions: opts.function_symbols.iter().cloned().collect(),
            operators: opts.operator_symbols.iter().cloned().collect(),
            unsplit: opts.unsplit_symbols.iter().cloned().collect(),
            opts,
            lexer,
            token: Token {
                ttype: Tok::Eof,
                text: String::new(),
                original: String::new(),
            },
            depth: 0,
        }
    }

    /// Enter a recursive parse function; errors if the depth budget (§6e) is
    /// exhausted. Increments only on success, so each `enter` that returns
    /// `Ok` is balanced by exactly one `leave` (even on the error-unwind path).
    fn enter(&mut self) -> R<()> {
        if self.depth >= MAX_PARSE_DEPTH {
            return Err(self.err("Expression too deeply nested"));
        }
        self.depth += 1;
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    fn advance(&mut self) -> R<()> {
        self.advance_opts(true)
    }

    fn advance_opts(&mut self, remove_initial_space: bool) -> R<()> {
        self.token = self.lexer.advance(remove_initial_space);
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

    pub fn convert(&mut self, input: &str) -> R<Expr> {
        self.lexer.set_input(input);
        self.depth = 0;
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
        // Capped: `statement`'s increment is held across the main attempt AND
        // the bar-fallback retry, so deeply nested `|…|` (which rewinds and
        // re-descends here) is bounded, not just a single descent.
        self.enter()?;
        let r = self.statement_inner(p);
        self.leave();
        r
    }

    fn statement_inner(&mut self, p: P) -> R<Expr> {
        // three periods ... can be a statement by itself
        if self.token.ttype == Tok::Ldots {
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

        if self.token.ttype != Tok::Colon {
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

        if self.token.ttype != Tok::Pipe {
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
        // AND binds tighter than OR
        let mut lhs = self.relation(p)?;

        while self.token.ttype == Tok::And {
            self.advance()?;
            let rhs = self.relation(p)?;
            lhs = Expr::And(vec![lhs, rhs]);
        }

        Ok(lhs)
    }

    fn relation(&mut self, p: P) -> R<Expr> {
        self.enter()?;
        let r = self.relation_inner(p);
        self.leave();
        r
    }

    fn relation_inner(&mut self, p: P) -> R<Expr> {
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
                    // sequence of multiple < or <=
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
                    // check for sequence of multiple =
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
        self.enter()?;
        let r = self.expression_inner(p);
        self.leave();
        r
    }

    fn expression_inner(&mut self, p: P) -> R<Expr> {
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
                    let Some(prefix) = unit_prefix(&name) else {
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
        self.enter()?;
        let r = self.factor_inner(p);
        self.leave();
        r
    }

    fn factor_inner(&mut self, p: P) -> R<Option<Expr>> {
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

        // allow arbitrary sequence of exponents, factorials, primes
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

    fn get_subsuperscript(&mut self, p: P) -> R<Expr> {
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
        self.enter()?;
        let r = self.base_factor_inner(p);
        self.leave();
        r
    }

    fn base_factor_inner(&mut self, p: P) -> R<Option<Expr>> {
        let mut result: Option<Expr> = None;

        if self.token.ttype == Tok::Number {
            // Decimals parse to exact rationals, never floats (§3a).
            result = Some(Expr::Num(Number::from_decimal_str(&self.token.text)));
            self.advance()?;
        } else if self.token.ttype == Tok::Infinity {
            result = Some(Expr::Const(MathConst::Inf));
            self.advance()?;
        } else if self.token.ttype == Tok::Var || self.token.ttype == Tok::VarMultiChar {
            let name = self.token.text.clone();

            if self.applied_functions.contains(&name) || self.functions.contains(&name) {
                return self.function_var(p, &name).map(Some);
            } else if self.operators.contains(&name) {
                self.advance()?;

                if self.token.ttype == Tok::LParen {
                    self.advance()?;
                    let args = self.statement_list()?;
                    if self.token.ttype != Tok::RParen {
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
                    && (self.token.ttype == Tok::VarMultiChar
                        || self.unsplit.contains(&name)
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
        } else if matches!(
            self.token.ttype,
            Tok::LParen | Tok::LBracket | Tok::LBrace | Tok::LAngle
        ) {
            result = Some(self.bracketed(p)?);
        } else if self.token.ttype == Tok::Pipe
            && p.parse_absolute_value
            && (p.inside_absolute_value == 0 || !p.allow_absolute_value_closing)
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

    /// The VAR branch for function symbols (applied or unapplied).
    fn function_var(&mut self, p: P, name: &str) -> R<Expr> {
        let must_apply = self.applied_functions.contains(name);
        let mut result = Expr::sym(name);
        self.advance()?;

        if self.token.ttype == Tok::Underscore {
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

            if self.token.ttype == Tok::LParen {
                self.advance()?;
                let parameters = self.statement_list()?;
                if self.token.ttype != Tok::RParen {
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
            Tok::LParen => (Tok::RParen, Some(Tok::RBracket)),
            Tok::LBracket => (Tok::RBracket, Some(Tok::RParen)),
            Tok::LBrace => (Tok::RBrace, None),
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
            // half-open interval
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
                Tok::LParen => SeqKind::Tuple,
                Tok::LBracket => SeqKind::Array,
                Tok::LBrace => SeqKind::Set,
                _ => SeqKind::AltVector,
            };
            if let Expr::Seq(_, xs) = result {
                result = Expr::Seq(kind, xs);
            }
        } else if token_left == Tok::LBrace {
            // singleton set (also covers set-builder | and :)
            result = Expr::Seq(SeqKind::Set, vec![result]);
        }
        // single element in ( [ ⟨: plain grouping — result unchanged

        self.advance()?;
        Ok(result)
    }

    fn angle_factor(&mut self, p: P) -> R<Option<Expr>> {
        self.advance()?;

        if self.token.ttype == Tok::LParen {
            self.advance()?;
            let parameters = self.statement_list()?;
            if self.token.ttype != Tok::RParen {
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

        let valid_start = self.token.ttype == Tok::Var
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
            if self.token.ttype == Tok::VarMultiChar
                || (self.token.ttype == Tok::Var && !self.token.text.contains('∂'))
            {
                var1 = self.token.text.clone();
            } else {
                if self.token.ttype != Tok::Caret {
                    return Ok(None);
                }
                self.advance()?;
                if self.token.ttype != Tok::Number {
                    return Ok(None);
                }
                n_deriv = parse_js_float(&self.token.text);
                if n_deriv.fract() != 0.0 {
                    return Ok(None);
                }
                self.advance()?;
                if (self.token.ttype == Tok::Var && !self.token.text.contains('∂'))
                    || self.token.ttype == Tok::VarMultiChar
                {
                    var1 = self.token.text.clone();
                } else {
                    return Ok(None);
                }
            }
        }

        self.advance()?;
        if self.token.ttype != Tok::Slash {
            return Ok(None);
        }

        let mut exponent_sum = 0.0;
        self.advance()?;

        loop {
            // next must be a VAR starting with the derivative symbol
            if self.token.ttype != Tok::Var || !self.token.text.starts_with(deriv_symbol) {
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
                if (self.token.ttype == Tok::Var && !self.token.text.contains('∂'))
                    || self.token.ttype == Tok::VarMultiChar
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
            if self.token.ttype == Tok::Space {
                last_was_space = true;
                self.advance()?;
            }

            if self.token.ttype == Tok::Caret {
                self.advance()?;
                if self.token.ttype != Tok::Number {
                    return Ok(None);
                }
                this_exponent = parse_js_float(&self.token.text);
                if this_exponent.fract() != 0.0 {
                    return Ok(None);
                }
                last_was_space = false;
                self.advance_opts(false)?;
                if self.token.ttype == Tok::Space {
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
                    && (self.token.ttype == Tok::Var || self.token.ttype == Tok::VarMultiChar)
                {
                    return Ok(None);
                }
                if self.token.ttype == Tok::Space {
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
