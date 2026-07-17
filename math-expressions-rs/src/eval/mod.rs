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
        Expr::Pow(b, e) => eval_complex(b, env)?.powc(eval_complex(e, env)?),

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

fn known_function(name: &str, nargs: usize) -> bool {
    match nargs {
        1 => matches!(
            name,
            "sin"
                | "cos"
                | "tan"
                | "sinh"
                | "cosh"
                | "tanh"
                | "asin"
                | "acos"
                | "atan"
                | "asinh"
                | "acosh"
                | "atanh"
                | "sec"
                | "csc"
                | "cot"
                | "sech"
                | "csch"
                | "coth"
                | "asec"
                | "acsc"
                | "acot"
                | "asech"
                | "acsch"
                | "acoth"
                | "exp"
                | "log"
                | "log10"
                | "sqrt"
                | "cbrt"
                | "abs"
                | "sign"
                | "conj"
                | "re"
                | "im"
                | "arg"
                | "floor"
                | "ceil"
                | "round"
                | "trace"
                | "factorial"
        ),
        2 => matches!(name, "atan2" | "nthroot" | "nCr" | "nPr" | "mod"),
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
            "sech" => z.cosh().inv(),
            "csch" => z.sinh().inv(),
            "coth" => z.tanh().inv(),
            // Inverse reciprocal-trig via the primary inverses of 1/z.
            "asec" => z.inv().acos(),
            "acsc" => z.inv().asin(),
            "acot" => z.inv().atan(),
            "asech" => z.inv().acosh(),
            "acsch" => z.inv().asinh(),
            "acoth" => z.inv().atanh(),
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
            "arg" => Complex64::new(z.arg(), 0.0),
            // Rounding functions are only defined on (near-)real inputs.
            "floor" => real_only(z, f64::floor)?,
            "ceil" => real_only(z, f64::ceil)?,
            "round" => real_only(z, f64::round)?,
            "trace" => z, // trace of a 1×1 / scalar is itself
            // `n! = Γ(n+1)`, evaluated as a complex function so identities like
            // `(n+1)·n! = (n+1)!` (the Γ recurrence) hold at sampled points.
            "factorial" => gamma(z + 1.0),
            _ => return None,
        };
        return Some(r);
    }

    // Binary functions.
    if let [a, b] = args {
        let (za, zb) = (eval_complex(a, env)?, eval_complex(b, env)?);
        let r = match name.as_str() {
            "atan2" => Complex64::new(za.re.atan2(zb.re), 0.0),
            "nthroot" => za.powc(zb.inv()),
            "nCr" => combinatorial(za, zb, false)?,
            "nPr" => combinatorial(za, zb, true)?,
            "mod" => Complex64::new(za.re.rem_euclid(zb.re), 0.0),
            _ => return None,
        };
        return Some(r);
    }

    None
}

/// Complex gamma function via the Lanczos approximation (g = 7, 9 coefficients),
/// with the reflection formula for the left half-plane. Accurate to ~1e-13, so
/// the recurrence `Γ(z+1) = z·Γ(z)` holds well within the equality tolerance —
/// which is what lets `(n+1)·n! = (n+1)!` and `n/n! = 1/(n-1)!` pass.
fn gamma(z: Complex64) -> Complex64 {
    const G: f64 = 7.0;
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    let pi = std::f64::consts::PI;
    if z.re < 0.5 {
        // Reflection: Γ(z)·Γ(1-z) = π / sin(πz).
        Complex64::new(pi, 0.0)
            / ((Complex64::new(pi, 0.0) * z).sin() * gamma(Complex64::new(1.0, 0.0) - z))
    } else {
        let z = z - 1.0;
        let mut x = Complex64::new(C[0], 0.0);
        for (i, &c) in C.iter().enumerate().skip(1) {
            x += c / (z + i as f64);
        }
        let t = z + (G + 0.5);
        let sqrt_2pi = (2.0 * pi).sqrt();
        Complex64::new(sqrt_2pi, 0.0) * t.powc(z + 0.5) * (-t).exp() * x
    }
}

/// Apply a real function to a (near-)real complex value, else `None`.
fn real_only(z: Complex64, f: fn(f64) -> f64) -> Option<Complex64> {
    if z.im.abs() < 1e-9 {
        Some(Complex64::new(f(z.re), 0.0))
    } else {
        None
    }
}

/// `nCr`/`nPr` on non-negative integer arguments.
fn combinatorial(n: Complex64, r: Complex64, ordered: bool) -> Option<Complex64> {
    let is_int = |z: Complex64| z.im.abs() < 1e-9 && (z.re.round() - z.re).abs() < 1e-9;
    if !is_int(n) || !is_int(r) {
        return None;
    }
    let (n, r) = (n.re.round() as i64, r.re.round() as i64);
    // The r-length product loop must stay bounded on any input; past ~10^4
    // the f64 result is astronomically large/imprecise anyway.
    if n < 0 || r < 0 || r > n || r > 10_000 {
        return None;
    }
    // P(n,r) = n·(n-1)···(n-r+1); C(n,r) = P(n,r)/r!.
    let mut num = 1.0f64;
    for k in 0..r {
        num *= (n - k) as f64;
    }
    if ordered {
        return Some(Complex64::new(num, 0.0));
    }
    let mut den = 1.0f64;
    for k in 1..=r {
        den *= k as f64;
    }
    Some(Complex64::new(num / den, 0.0))
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
            // `pi`/`e`/`i` are constants, not sample variables.
            let name = s.name();
            if !matches!(name.as_str(), "pi" | "e" | "i") {
                out.insert(name);
            }
        }
        Expr::Num(_) | Expr::Const(_) | Expr::Blank | Expr::Ldots => {}
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
