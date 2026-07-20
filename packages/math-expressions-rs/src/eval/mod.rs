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
/// meaningful here (relations, sequences, blanks).
///
/// Subtrees this slice can't compute symbolically — applications of unknown
/// functions (`f(a)`), subscripts (`y_t`), primes, and `OtherOp` nodes
/// (`vec(x)`) — are treated as *opaque atoms*: sampled as a single variable
/// keyed by their structure, so `f(a)` takes the same value on both sides of a
/// comparison. This lets `(f(a)-f(b))·x` and `(f(b)-f(a))·(-x)` agree.
pub fn eval_complex(e: &Expr, env: &Env) -> Option<Complex64> {
    if is_opaque_atom(e) {
        return env.get(&opaque_key(e)).copied();
    }
    Some(match e {
        Expr::Num(n) => number_to_complex(n),
        // A numeric constant: the k-th root of its polynomial, isolation
        // cached per polynomial (MATRIX_PLAN §2d).
        Expr::RootOf { poly, index } => return crate::rootof::numeric_root(poly, *index),
        Expr::Const(c) => match c {
            MathConst::Pi => Complex64::new(std::f64::consts::PI, 0.0),
            MathConst::E => Complex64::new(std::f64::consts::E, 0.0),
            MathConst::I => Complex64::I,
            MathConst::Inf | MathConst::NegInf | MathConst::NaN => return None,
        },
        // `pi`, `e`, `i` are number-symbols (constants), not free variables —
        // the parser emits them as plain symbols (matching JS convention).
        Expr::Sym(s) => match s.name().as_str() {
            "pi" => Complex64::new(std::f64::consts::PI, 0.0),
            "e" => Complex64::new(std::f64::consts::E, 0.0),
            "i" => Complex64::I,
            name => *env.get(name)?,
        },

        Expr::Add(xs) => xs
            .iter()
            .try_fold(Complex64::ZERO, |acc, x| Some(acc + eval_complex(x, env)?))?,
        Expr::Mul(xs) => xs
            .iter()
            .try_fold(Complex64::ONE, |acc, x| Some(acc * eval_complex(x, env)?))?,
        Expr::Div(a, b) => eval_complex(a, env)? / eval_complex(b, env)?,
        Expr::Neg(x) => -eval_complex(x, env)?,
        Expr::Pow(b, e) => {
            let base = eval_complex(b, env)?;
            let exp = eval_complex(e, env)?;
            // Real base with a small integer exponent: exact real powi —
            // `powc` goes through exp/ln and yields 3² = 9.000000000000002,
            // which mathjs (real pow) does not. Matches mathjs fidelity and
            // removes float noise from the sampler.
            if base.im == 0.0
                && exp.im == 0.0
                && exp.re.fract() == 0.0
                && exp.re.abs() <= i32::MAX as f64
            {
                Complex64::new(base.re.powi(exp.re as i32), 0.0)
            } else {
                base.powc(exp)
            }
        }

        Expr::Apply(head, args) => eval_apply(head, args, env)?,

        // Not numerically meaningful in this slice.
        _ => return None,
    })
}

/// A subtree evaluated as a single opaque sample variable: an application of an
/// unknown function, a subscript, a prime, or an `OtherOp` (`vec`, `angle`, …).
pub(crate) fn is_opaque_atom(e: &Expr) -> bool {
    match e {
        Expr::Apply(head, args) => !head_evaluable(head, args.len()),
        Expr::Index(..) | Expr::Prime(_) | Expr::OtherOp(..) => true,
        _ => false,
    }
}

/// Can `eval_apply` handle this head/arity? (A `Pow` head is `sin^2`-style; an
/// `Index` head is a subscripted log `log_b`.)
fn head_evaluable(head: &Expr, nargs: usize) -> bool {
    match head {
        Expr::Pow(inner, _) => head_evaluable(inner, nargs),
        Expr::Sym(s) => known_function(&s.name(), nargs),
        Expr::Index(inner, _) => {
            nargs == 1 && matches!(inner.as_ref(), Expr::Sym(s) if s.name() == "log")
        }
        _ => false,
    }
}

/// Can the registry evaluate this head at this arity? (`FnDef::eval1`/
/// `eval2` in `crate::functions` — canonical spellings only, matching the
/// historical hardcoded list.)
fn known_function(name: &str, nargs: usize) -> bool {
    match nargs {
        1 => crate::functions::eval1(name).is_some(),
        2 => crate::functions::eval2(name).is_some(),
        _ => false,
    }
}

/// A structural key identifying an opaque subtree (stable within a run).
pub(crate) fn opaque_key(e: &Expr) -> String {
    format!("{e:?}")
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
    // Subscripted logarithm `log_b(x) = ln(x) / ln(b)` (change of base).
    if let Expr::Index(inner, base) = head {
        if let (Expr::Sym(s), [arg]) = (inner.as_ref(), args) {
            if s.name() == "log" {
                let x = eval_complex(arg, env)?;
                let b = eval_complex(base, env)?;
                return Some(x.ln() / b.ln());
            }
        }
    }
    let Expr::Sym(s) = head else { return None };
    let name = s.name();

    // The per-function evaluation rules are `FnDef::eval1`/`eval2` in
    // `crate::functions`; this dispatch only routes by arity.
    if let [arg] = args {
        let f = crate::functions::eval1(&name)?;
        let z = eval_complex(arg, env)?;
        return f(z);
    }
    if let [a, b] = args {
        let f = crate::functions::eval2(&name)?;
        let (za, zb) = (eval_complex(a, env)?, eval_complex(b, env)?);
        return f(za, zb);
    }

    None
}

/// Collect the sample-variable keys of an expression: free symbols plus opaque
/// subtrees (see [`eval_complex`]), which are keyed by structure and not
/// descended into. Must mirror `eval_complex`'s opaque/known-function
/// decisions so every key it reads is populated here.
pub fn free_symbols(e: &Expr, out: &mut std::collections::BTreeSet<String>) {
    if is_opaque_atom(e) {
        out.insert(opaque_key(e));
        return;
    }
    match e {
        Expr::Sym(s) => {
            // Constant symbols (`pi`/`e`/`i`) are not sample variables.
            let name = s.name();
            if !crate::sym::is_constant_symbol(&name) {
                out.insert(name);
            }
        }
        Expr::Num(_) | Expr::Const(_) | Expr::RootOf { .. } | Expr::Blank | Expr::Ldots => {}
        Expr::Neg(x) | Expr::Not(x) => free_symbols(x, out),
        Expr::Pow(a, b) | Expr::Div(a, b) => {
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
        // An evaluable application: descend into arguments only, not the
        // function-name head (`sin` is not a variable) — except a subscripted
        // log `log_b(x)` carries its base `b` as free data in the head.
        Expr::Apply(head, xs) => {
            xs.iter().for_each(|x| free_symbols(x, out));
            if let Expr::Index(_, base) = head.as_ref() {
                free_symbols(base, out);
            }
        }
        Expr::Interval { endpoints, .. } => {
            free_symbols(&endpoints.0, out);
            free_symbols(&endpoints.1, out);
        }
        Expr::Relation { operands, .. } => operands.iter().for_each(|x| free_symbols(x, out)),
        Expr::Matrix { entries, .. } => entries.iter().for_each(|x| free_symbols(x, out)),
        // Opaque nodes (Index, Prime, OtherOp) are handled above.
        Expr::Index(..) | Expr::Prime(_) | Expr::OtherOp(..) => {}
    }
}
