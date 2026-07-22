//! Recursive descent text parser — a faithful port of
//! lib/converters/text-to-ast.js (grammar documented there).
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
    /// Decimal / argument-separator notation.
    pub notation: crate::notation::NumberNotation,
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
            applied_function_symbols: crate::functions::applied_text_names(),
            function_symbols: v(&["f", "g"]),
            operator_symbols: v(&["binom", "vec", "linesegment"]),
            parse_leibniz_notation: true,
            parse_scientific_notation: true,
            notation: crate::notation::NumberNotation::default(),
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
    /// Recursion depth through the self-recursive parse functions.
    depth: usize,
}

impl TextToAst {
    super::shared_grammar::shared_grammar_methods!();

    pub fn new(opts: TextToAstOptions) -> Self {
        let lexer = Lexer::new(opts.parse_scientific_notation, opts.notation.clone());
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
            result = Some(Expr::Num(Number::from_decimal_str(self.opts.notation.normalize_number(&self.token.text).as_ref())));
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
                n_deriv = parse_js_float(self.opts.notation.normalize_number(&self.token.text).as_ref());
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
                this_exponent = parse_js_float(self.opts.notation.normalize_number(&self.token.text).as_ref());
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
