//! Tier 0 (ARBITRARY_PERCISION_PLAN §4): the tape over f64 values with
//! certified forward error bounds. This is the fast path — when `err ≤ tol`
//! no bignum work happens — and its recorded per-op values feed the Tier-2
//! planning pass (the magnitude information whose absence sank the §1c
//! prototype's static planning).

use super::kernels::registry;
use super::tape::{CompiledExpr, Op};
use crate::num::Number;

const U: f64 = f64::EPSILON / 2.0; // unit roundoff

#[derive(Clone, Copy, Debug)]
pub struct Approx64 {
    pub val: f64,
    pub err: f64, // |true − val| ≤ err (absolute)
}

pub enum Tier0Outcome {
    /// Finished with a finite value and a finite error bound.
    Ok(Approx64),
    /// Domain edge / overflow / non-finite intermediate: Tier 2 must decide.
    /// Recorded values up to the failure point are still valid.
    Escalate(&'static str),
}

/// Run the tape; `record[i]` receives op i's value (NaN past an escalation).
pub fn run(
    tape: &CompiledExpr,
    bindings: &[f64],
    record: &mut Vec<f64>,
) -> Tier0Outcome {
    record.clear();
    record.reserve(tape.ops.len());
    let mut stack: Vec<Approx64> = Vec::with_capacity(tape.max_stack);
    let mut failure: Option<&'static str> = None;

    for op in &tape.ops {
        let out = eval_op(op, tape, bindings, &mut stack);
        let out = match out {
            Ok(v) => v,
            Err(why) => {
                failure.get_or_insert(why);
                Approx64 {
                    val: f64::NAN,
                    err: f64::INFINITY,
                }
            }
        };
        record.push(out.val);
        stack.push(out);
    }
    let result = stack.pop().expect("tape leaves one value");
    match failure {
        Some(why) => Tier0Outcome::Escalate(why),
        None if result.val.is_finite() && result.err.is_finite() => Tier0Outcome::Ok(result),
        None => Tier0Outcome::Escalate("non-finite result"),
    }
}

fn eval_op(
    op: &Op,
    tape: &CompiledExpr,
    bindings: &[f64],
    stack: &mut Vec<Approx64>,
) -> Result<Approx64, &'static str> {
    Ok(match op {
        Op::Const(i) => const_approx(&tape.consts[*i as usize]),
        Op::Var(i) => Approx64 {
            val: *bindings.get(*i as usize).ok_or("unbound variable")?,
            err: 0.0,
        },
        Op::Pi => Approx64 {
            val: std::f64::consts::PI,
            err: std::f64::consts::PI * U,
        },
        Op::E => Approx64 {
            val: std::f64::consts::E,
            err: std::f64::consts::E * U,
        },
        Op::I => return Err("imaginary unit in the real tier"),
        Op::Root(i) => {
            let (poly, idx) = &tape.roots[*i as usize];
            let Some(z) = crate::rootof::numeric_root(poly, *idx) else {
                return Err("root isolation failed");
            };
            if z.im != 0.0 {
                return Err("complex root in the real tier");
            }
            // Real roots come from exact Sturm bisection refined past f64
            // resolution (upoly::refine_to_f64's stopping rule), so the
            // certified bound is a couple of ulps of the value's scale.
            Approx64 {
                val: z.re,
                err: (z.re.abs() + 2.0) * 2f64.powi(-52),
            }
        }
        Op::Add(n) => {
            let n = *n as usize;
            let start = stack.len() - n;
            let mut acc = stack[start];
            for x in &stack[start + 1..] {
                let s = acc.val + x.val;
                acc = Approx64 {
                    val: s,
                    err: acc.err + x.err + s.abs() * U,
                };
            }
            stack.truncate(start);
            acc
        }
        Op::Mul(n) => {
            let n = *n as usize;
            let start = stack.len() - n;
            let mut acc = stack[start];
            for x in &stack[start + 1..] {
                let p = acc.val * x.val;
                acc = Approx64 {
                    val: p,
                    err: acc.val.abs() * x.err
                        + x.val.abs() * acc.err
                        + acc.err * x.err
                        + p.abs() * U,
                };
            }
            stack.truncate(start);
            acc
        }
        Op::PowInt(k) => {
            let x = stack.pop().unwrap();
            if x.val == 0.0 && *k < 0 {
                return Err("0^negative");
            }
            let p = x.val.powi(i32::try_from(*k).map_err(|_| "exponent overflow")?);
            let rel_in = if x.val != 0.0 { x.err / x.val.abs() } else { 0.0 };
            let rel = (k.unsigned_abs() as f64) * (rel_in + U);
            Approx64 {
                val: p,
                err: p.abs() * rel + p.abs() * U,
            }
        }
        Op::Pow => {
            let b = stack.pop().unwrap();
            let a = stack.pop().unwrap();
            if a.val <= 0.0 {
                return Err("non-positive base of a real power");
            }
            let p = a.val.powf(b.val);
            let rel = b.val.abs() * (a.err / a.val) + a.val.ln().abs() * b.err + 2.0 * U;
            Approx64 {
                val: p,
                err: p.abs() * rel + p.abs() * U,
            }
        }
        Op::Call(id) => {
            let x = stack.pop().unwrap();
            let k = registry()[*id as usize];
            if !(k.domain)(x.val) {
                return Err("function domain edge");
            }
            let v = (k.f)(x.val);
            let d = (k.df)(x.val).abs();
            if !d.is_finite() {
                return Err("unbounded derivative at the argument");
            }
            Approx64 {
                val: v,
                err: d * x.err + v.abs() * U * 2.0,
            }
        }
    })
}

fn const_approx(n: &Number) -> Approx64 {
    let v = n.to_f64();
    // Exactly-representable values carry no error; everything else half an
    // ulp from the nearest-double conversion.
    let exact = match n {
        Number::Int(i) => i.unsigned_abs() < (1u64 << 53),
        Number::Float(_) => true,
        _ => false,
    };
    Approx64 {
        val: v,
        err: if exact { 0.0 } else { v.abs() * U },
    }
}

// ---- P4: complex Tier 0 ----

use num_complex::Complex64;

#[derive(Clone, Copy, Debug)]
pub struct CApprox64 {
    pub val: Complex64,
    /// Bound on |Δz| (absolute).
    pub err: f64,
}

pub enum CTier0Outcome {
    Ok(CApprox64),
    Escalate(&'static str),
}

/// Complex run of the tape (principal branches, matching `eval_complex`).
/// `record[i]` receives op i's complex value for the complex planner.
pub fn run_complex(
    tape: &CompiledExpr,
    bindings: &[f64],
    record: &mut Vec<Complex64>,
) -> CTier0Outcome {
    record.clear();
    record.reserve(tape.ops.len());
    let mut stack: Vec<CApprox64> = Vec::with_capacity(tape.max_stack);
    let mut failure: Option<&'static str> = None;
    for op in &tape.ops {
        let out = match ceval_op(op, tape, bindings, &mut stack) {
            Ok(v) => v,
            Err(why) => {
                failure.get_or_insert(why);
                CApprox64 {
                    val: Complex64::new(f64::NAN, f64::NAN),
                    err: f64::INFINITY,
                }
            }
        };
        record.push(out.val);
        stack.push(out);
    }
    let result = stack.pop().expect("tape leaves one value");
    match failure {
        Some(why) => CTier0Outcome::Escalate(why),
        None if result.val.re.is_finite() && result.val.im.is_finite() && result.err.is_finite() => {
            CTier0Outcome::Ok(result)
        }
        None => CTier0Outcome::Escalate("non-finite complex result"),
    }
}

fn ceval_op(
    op: &Op,
    tape: &CompiledExpr,
    bindings: &[f64],
    stack: &mut Vec<CApprox64>,
) -> Result<CApprox64, &'static str> {
    let real = |a: Approx64| CApprox64 {
        val: Complex64::new(a.val, 0.0),
        err: a.err,
    };
    Ok(match op {
        Op::Const(i) => real(const_approx(&tape.consts[*i as usize])),
        Op::Var(i) => CApprox64 {
            val: Complex64::new(*bindings.get(*i as usize).ok_or("unbound variable")?, 0.0),
            err: 0.0,
        },
        Op::Pi => real(Approx64 {
            val: std::f64::consts::PI,
            err: std::f64::consts::PI * U,
        }),
        Op::E => real(Approx64 {
            val: std::f64::consts::E,
            err: std::f64::consts::E * U,
        }),
        Op::I => CApprox64 {
            val: Complex64::new(0.0, 1.0),
            err: 0.0,
        },
        Op::Root(i) => {
            let (poly, idx) = &tape.roots[*i as usize];
            let Some(z) = crate::rootof::numeric_root(poly, *idx) else {
                return Err("root isolation failed");
            };
            if z.im == 0.0 {
                CApprox64 {
                    val: z,
                    err: (z.re.abs() + 2.0) * 2f64.powi(-52),
                }
            } else {
                // Rigorous simple-root bound |z − r| ≤ n·|p(z)/p′(z)|, with
                // f64 Horner rounding absorbed via the Σ|cᵢ||z|ⁱ majorant.
                let deg = poly.len() - 1;
                let (mut p, mut dp) = (Complex64::ZERO, Complex64::ZERO);
                let (mut pmax, mut dpmax) = (0.0f64, 0.0f64);
                let zn = z.norm();
                for (k, c) in poly.iter().enumerate().rev() {
                    let cf = c.to_f64();
                    if k > 0 {
                        dp = dp * z + Complex64::new(cf * k as f64, 0.0);
                        dpmax = dpmax * zn + cf.abs() * k as f64;
                    }
                    p = p * z + Complex64::new(cf, 0.0);
                    pmax = pmax * zn + cf.abs();
                }
                let slack = 2.0 * deg as f64 * U;
                let p_hi = p.norm() * (1.0 + 4.0 * U) + slack * pmax;
                let dp_lo = dp.norm() * (1.0 - 4.0 * U) - slack * dpmax;
                if dp_lo.is_nan() || dp_lo <= 0.0 {
                    return Err("root derivative unresolved at f64");
                }
                CApprox64 {
                    val: z,
                    err: deg as f64 * p_hi / dp_lo + (zn + 1.0) * 4.0 * U,
                }
            }
        }
        Op::Add(n) => {
            let n = *n as usize;
            let start = stack.len() - n;
            let mut acc = stack[start];
            for x in &stack[start + 1..] {
                let s = acc.val + x.val;
                acc = CApprox64 {
                    val: s,
                    err: acc.err + x.err + s.norm() * U,
                };
            }
            stack.truncate(start);
            acc
        }
        Op::Mul(n) => {
            let n = *n as usize;
            let start = stack.len() - n;
            let mut acc = stack[start];
            for x in &stack[start + 1..] {
                let p = acc.val * x.val;
                acc = CApprox64 {
                    val: p,
                    err: acc.val.norm() * x.err
                        + x.val.norm() * acc.err
                        + acc.err * x.err
                        + p.norm() * 2.0 * U,
                };
            }
            stack.truncate(start);
            acc
        }
        Op::PowInt(k) => {
            let x = stack.pop().unwrap();
            if x.val.norm() == 0.0 && *k < 0 {
                return Err("0^negative");
            }
            let p = x.val.powi(i32::try_from(*k).map_err(|_| "exponent overflow")?);
            let rel_in = if x.val.norm() != 0.0 {
                x.err / x.val.norm()
            } else {
                0.0
            };
            let rel = (k.unsigned_abs() as f64) * (rel_in + 2.0 * U);
            CApprox64 {
                val: p,
                err: p.norm() * rel + p.norm() * U,
            }
        }
        Op::Pow => {
            let b = stack.pop().unwrap();
            let a = stack.pop().unwrap();
            if a.val.norm() == 0.0 {
                return Err("0 base of a general power");
            }
            let p = a.val.powc(b.val);
            let rel = b.val.norm() * (a.err / a.val.norm())
                + a.val.ln().norm() * b.err
                + 4.0 * U;
            CApprox64 {
                val: p,
                err: p.norm() * rel + p.norm() * U,
            }
        }
        Op::Call(id) => {
            let x = stack.pop().unwrap();
            let k = registry()[*id as usize];
            let v = (k.cf)(x.val);
            let d = (k.cdfm)(x.val);
            if !d.is_finite() || !v.re.is_finite() || !v.im.is_finite() {
                return Err("complex function edge");
            }
            CApprox64 {
                val: v,
                err: d * x.err + v.norm() * U * 4.0,
            }
        }
    })
}
