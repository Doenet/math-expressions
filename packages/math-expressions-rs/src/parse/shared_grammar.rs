//! The grammar skeleton shared VERBATIM by the text and LaTeX parsers.
//!
//! Both parsers are recursive-descent over the same grammar; before this
//! module existed, ~20 methods (the statement/relation/expression/term/factor
//! ladder and the enter/leave/state scaffolding) were maintained as
//! copy-pasted twins in `text.rs` and `latex.rs` — a fix applied to one
//! silently missed the other. They are now stamped into both `impl` blocks
//! by [`shared_grammar_methods!`]; genuinely
//! flavor-specific productions (`statement_main`'s `Tok::Mid`, the pipe
//! fallback, sub/superscript digits, unit tables, `advance`, and the
//! LaTeX-only constructs) stay in their own files.
//!
//! A method here may freely call flavor-specific methods: names resolve at
//! the expansion site, so `self.statement_main(...)` binds to each parser's
//! own implementation.

/// Stamp the shared grammar methods into a parser `impl` block. The bodies
/// are the (formerly duplicated) text-parser versions, unchanged.
macro_rules! shared_grammar_methods {
    () => {
    // The result keeps the raw associative *grouping* (`convert` does not apply
    // the whole-tree `flatten`, per STRUCTURAL_COMPARISON §3's inverted design),
    // so form analysis can see the tree closer to as-typed. `flatten` is instead
    // the leading step of the *consumers* that need a canonical shape —
    // `normalize_syntactic`, the output formatters, `js_tree::to_js`, and
    // `check_structural_comparison` — while the value path (`equals`/`simplify`/…)
    // flattens implicitly via `canonicalize`.
    /// Parse `input` to an expression tree, erroring if any tokens remain. The
    /// tree preserves the associative grouping as typed rather than flattening
    /// it; individual terms are still locally flattened for unit detection.
    pub fn convert(&mut self, input: &str) -> R<Expr> {
        self.lexer.set_input(input);
        self.depth = 0;
        self.advance()?;
        let result = self.statement_list()?;
        if self.token.ttype != Tok::Eof {
            return Err(self.err(format!("Invalid location of '{}'", self.token.original)));
        }
        Ok(result)
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
    /// Enter a recursive parse function; errors if the depth budget is
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

    };
}

pub(crate) use shared_grammar_methods;
