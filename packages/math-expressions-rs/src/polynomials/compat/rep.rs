//! The representation the compat polynomial API is written in, and the
//! coefficient arithmetic underneath it.
//!
//! A polynomial is *recursive and sparse*: `["polynomial", v, [[d, c], …]]` is
//! a polynomial in `v` whose coefficients are themselves polynomials (in a
//! later variable) or plain expressions. Sparsity is not an optimisation here —
//! `t^1000000000` is one of the cases this API is asked about.
//!
//! Two things distinguish it from [`multivariate`](super::super::multivariate):
//!
//! - **Variables are trees, not names.** `sin(x)` and `x^(1/2)` are ordinary
//!   polynomial variables; which of two variables comes first is decided by the
//!   legacy [`cmp_default_order`](crate::cmp_default_order), because that choice
//!   is visible in every AST this API returns.
//! - **Coefficients are expressions, not rationals.** `π`, `e` and `i` count as
//!   numbers, so `9x^(2/3) − πx` has the coefficient `−π`. Coefficient
//!   arithmetic is therefore `simplify` on a two-operand tree — the same
//!   operation the JS engine reached for through wasm.

use crate::expr::Expr;
use crate::num::Number;
use std::cmp::Ordering;

/// A polynomial, or a coefficient standing in for one.
///
/// [`Poly::Coeff`] covers everything the JS engine tested with
/// `tree[0] !== "polynomial"`: a number, a symbol, or any non-polynomial tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Poly {
    Coeff(Expr),
    /// Terms are ordered by strictly increasing degree; the last is the leading
    /// term under the lexicographic order this module uses throughout.
    Rec {
        var: Expr,
        terms: Vec<(i64, Poly)>,
    },
}

/// A single term: a coefficient times a product of variable powers.
///
/// [`Mono::Coeff`] is the degenerate "no variables at all" case that the JS
/// engine spelled as a bare number rather than a `["monomial", c, []]` node —
/// the two are distinguishable, and `mono_div`/`mono_is_div` branch on which
/// one they were handed, so both are kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mono {
    Coeff(Expr),
    Term { coeff: Expr, vars: Vec<(Expr, i64)> },
}

impl Poly {
    pub fn zero() -> Poly {
        Poly::Coeff(Expr::int(0))
    }

    pub fn one() -> Poly {
        Poly::Coeff(Expr::int(1))
    }

    pub fn is_rec(&self) -> bool {
        matches!(self, Poly::Rec { .. })
    }

    /// The leading variable, for a recursive node.
    pub fn var(&self) -> &Expr {
        match self {
            Poly::Rec { var, .. } => var,
            Poly::Coeff(_) => unreachable!("var() on a coefficient"),
        }
    }

    pub fn terms(&self) -> &[(i64, Poly)] {
        match self {
            Poly::Rec { terms, .. } => terms,
            Poly::Coeff(_) => &[],
        }
    }

    /// Wrap `self` as a degree-0 polynomial in `var`.
    pub fn constant_in(self, var: &Expr) -> Poly {
        Poly::Rec {
            var: var.clone(),
            terms: vec![(0, self)],
        }
    }

    /// The JS `if (p)` test: a polynomial node is always truthy, a coefficient
    /// is truthy unless it is zero or NaN. Cancellation is detected with this,
    /// so it decides which terms survive an addition.
    pub fn truthy(&self) -> bool {
        match self {
            Poly::Rec { .. } => true,
            Poly::Coeff(e) => truthy(e),
        }
    }

    /// The JS `p === 0` test — literally the number zero, not "value zero".
    pub fn is_literal_zero(&self) -> bool {
        matches!(self, Poly::Coeff(Expr::Num(n)) if n.is_zero())
    }
}

impl Mono {
    pub fn coeff(&self) -> &Expr {
        match self {
            Mono::Coeff(e) | Mono::Term { coeff: e, .. } => e,
        }
    }

    pub fn vars(&self) -> &[(Expr, i64)] {
        match self {
            Mono::Term { vars, .. } => vars,
            Mono::Coeff(_) => &[],
        }
    }

    pub fn is_term(&self) -> bool {
        matches!(self, Mono::Term { .. })
    }
}

/// The JS truthiness of a coefficient tree: false for `0`, `-0` and `NaN`.
pub fn truthy(e: &Expr) -> bool {
    match e {
        Expr::Num(n) => !n.is_zero() && !n.to_f64().is_nan(),
        Expr::Const(crate::expr::MathConst::NaN) => false,
        _ => true,
    }
}

/// The JS `c === 1` test on a coefficient.
pub fn is_literal_one(e: &Expr) -> bool {
    matches!(e, Expr::Num(Number::Int(1)))
}

/// `simplify(a + b)` — coefficient addition.
pub fn c_add(a: &Expr, b: &Expr) -> Expr {
    crate::simplify(&Expr::Add(vec![a.clone(), b.clone()]))
}

/// `simplify(-a)`.
pub fn c_neg(a: &Expr) -> Expr {
    crate::simplify(&Expr::Neg(Box::new(a.clone())))
}

/// `simplify(a * b)`.
pub fn c_mul(a: &Expr, b: &Expr) -> Expr {
    crate::simplify(&Expr::Mul(vec![a.clone(), b.clone()]))
}

/// `evaluate_numbers(a / b)` — the weaker fold monomial division uses, so that
/// `7/2` stays spelled `["/", 7, 2]` rather than collapsing to a rational.
pub fn c_div(a: &Expr, b: &Expr) -> Expr {
    crate::evaluate_numbers(&Expr::Div(Box::new(a.clone()), Box::new(b.clone())))
}

/// `1 / c`, unevaluated: the multiplier that makes a leading coefficient 1.
pub fn c_recip(c: &Expr) -> Expr {
    Expr::Div(Box::new(Expr::int(1)), Box::new(c.clone()))
}

/// Are these the same polynomial variable?
///
/// Structural equality, where the JS engine used `!==` on the raw tree and so
/// only ever recognised two *string* variables as equal — which is why it
/// rewrote every variable to a JSON string before running a gcd. Holding trees
/// makes that pass unnecessary, and `sin(x)` built twice is now one variable
/// rather than two that compare equal under the default order and confuse the
/// rewrite below.
pub fn same_var(a: &Expr, b: &Expr) -> bool {
    a == b
}

/// Which of two *distinct* variables comes first, under the legacy default
/// order — the one that decides which variable a polynomial is written in, and
/// therefore the shape of every AST this API returns.
pub fn cmp_var(a: &Expr, b: &Expr) -> Ordering {
    crate::cmp_default_order(a, b)
}
