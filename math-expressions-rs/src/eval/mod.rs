//! Numerical evaluation (PORTING_PLAN.md §9) — the minimal complex slice the
//! equality tester needs. `eval_complex` evaluates an expression at a complex
//! assignment of its free symbols; unsupported constructs return `None` so the
//! caller can fall back rather than guess.

use crate::expr::{Expr, MathConst};
use crate::num::Number;
use num_complex::Complex64;
use std::collections::HashMap;

/// Environment mapping free-symbol names to complex sample points.
pub type Env = HashMap<String, Complex64>;

/// Evaluate `e` at `env`. Returns `None` for anything not numerically
/// meaningful here (relations, sequences, blanks, unknown functions).
pub fn eval_complex(e: &Expr, env: &Env) -> Option<Complex64> {
    Some(match e {
        Expr::Num(n) => number_to_complex(n),
        Expr::Const(c) => match c {
            MathConst::Pi => Complex64::new(std::f64::consts::PI, 0.0),
            MathConst::E => Complex64::new(std::f64::consts::E, 0.0),
            MathConst::I => Complex64::I,
            MathConst::Inf | MathConst::NegInf | MathConst::NaN => return None,
        },
        Expr::Sym(s) => *env.get(&s.name())?,

        Expr::Add(xs) => xs
            .iter()
            .try_fold(Complex64::ZERO, |acc, x| Some(acc + eval_complex(x, env)?))?,
        Expr::Mul(xs) => xs
            .iter()
            .try_fold(Complex64::ONE, |acc, x| Some(acc * eval_complex(x, env)?))?,
        Expr::Div(a, b) => eval_complex(a, env)? / eval_complex(b, env)?,
        Expr::Neg(x) => -eval_complex(x, env)?,
        Expr::Pow(b, e) => eval_complex(b, env)?.powc(eval_complex(e, env)?),

        Expr::Apply(head, args) => eval_apply(head, args, env)?,

        // Not numerically meaningful in this slice.
        _ => return None,
    })
}

fn number_to_complex(n: &Number) -> Complex64 {
    Complex64::new(n.to_f64(), 0.0)
}

fn eval_apply(head: &Expr, args: &[Expr], env: &Env) -> Option<Complex64> {
    // A "modified" head like `sin^2` means `sin(arg)^2` — apply the inner
    // function, then raise. (`f'` and other heads are not evaluable here.)
    if let Expr::Pow(inner, exp) = head {
        let base = eval_apply(inner, args, env)?;
        return Some(base.powc(eval_complex(exp, env)?));
    }
    let Expr::Sym(s) = head else { return None };
    let name = s.name();

    // Unary functions cover the common case.
    if let [arg] = args {
        let z = eval_complex(arg, env)?;
        let r = match name.as_str() {
            "sin" => z.sin(),
            "cos" => z.cos(),
            "tan" => z.tan(),
            "sinh" => z.sinh(),
            "cosh" => z.cosh(),
            "tanh" => z.tanh(),
            "asin" => z.asin(),
            "acos" => z.acos(),
            "atan" => z.atan(),
            "asinh" => z.asinh(),
            "acosh" => z.acosh(),
            "atanh" => z.atanh(),
            "sec" => z.cos().inv(),
            "csc" => z.sin().inv(),
            "cot" => z.tan().inv(),
            "exp" => z.exp(),
            "log" => z.ln(), // natural log (ln normalizes to log)
            "log10" => z.log10(),
            "sqrt" => z.sqrt(),
            "cbrt" => z.powf(1.0 / 3.0),
            "abs" => Complex64::new(z.norm(), 0.0),
            "sign" => {
                if z.norm() == 0.0 {
                    Complex64::ZERO
                } else {
                    z / z.norm()
                }
            }
            "conj" => z.conj(),
            "re" => Complex64::new(z.re, 0.0),
            "im" => Complex64::new(z.im, 0.0),
            _ => return None,
        };
        return Some(r);
    }

    // A couple of binary functions.
    if let [a, b] = args {
        let (za, zb) = (eval_complex(a, env)?, eval_complex(b, env)?);
        let r = match name.as_str() {
            "atan2" => Complex64::new(za.re.atan2(zb.re), 0.0),
            "nthroot" => za.powc(zb.inv()),
            _ => return None,
        };
        return Some(r);
    }

    None
}

/// Collect the free-symbol names of an expression (for choosing sample points).
pub fn free_symbols(e: &Expr, out: &mut std::collections::BTreeSet<String>) {
    match e {
        Expr::Sym(s) => {
            out.insert(s.name());
        }
        Expr::Num(_) | Expr::Const(_) | Expr::Blank | Expr::Ldots => {}
        Expr::Neg(x) | Expr::Not(x) | Expr::Prime(x) => free_symbols(x, out),
        Expr::Pow(a, b) | Expr::Div(a, b) | Expr::Index(a, b) => {
            free_symbols(a, out);
            free_symbols(b, out);
        }
        Expr::Add(xs)
        | Expr::Mul(xs)
        | Expr::And(xs)
        | Expr::Or(xs)
        | Expr::Union(xs)
        | Expr::Intersect(xs)
        | Expr::Seq(_, xs) => xs.iter().for_each(|x| free_symbols(x, out)),
        Expr::Apply(h, xs) => {
            free_symbols(h, out);
            xs.iter().for_each(|x| free_symbols(x, out));
        }
        Expr::Interval { endpoints, .. } => {
            free_symbols(&endpoints.0, out);
            free_symbols(&endpoints.1, out);
        }
        Expr::Relation { operands, .. } => operands.iter().for_each(|x| free_symbols(x, out)),
        Expr::Matrix { entries, .. } => entries.iter().for_each(|x| free_symbols(x, out)),
        Expr::OtherOp(_, xs) => xs.iter().for_each(|x| free_symbols(x, out)),
    }
}
