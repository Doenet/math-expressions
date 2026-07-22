//! ODE numerics (ODE_PLAN.md): the `numeric.dopri` replacement for Doenet's
//! `ODESystem`. Dormand–Prince RK5(4) with the PI step-size controller, the
//! free 4th-order dense-output interpolant, and §7f guards (step caps,
//! vanishing-step and non-finite detection → clean early termination at the
//! last accepted point — never a hang, never NaN samples).
//!
//! f64-only by design, like `src/numeric.rs`: this is plotting/animation
//! numerics, not CAS arithmetic.

use crate::expr::Expr;

// Dormand–Prince 5(4) tableau (identical to `numeric.dopri` / scipy RK45).
const C: [f64; 7] = [0.0, 1.0 / 5.0, 3.0 / 10.0, 4.0 / 5.0, 8.0 / 9.0, 1.0, 1.0];
const A: [[f64; 6]; 7] = [
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [1.0 / 5.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [3.0 / 40.0, 9.0 / 40.0, 0.0, 0.0, 0.0, 0.0],
    [44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0, 0.0, 0.0, 0.0],
    [
        19372.0 / 6561.0,
        -25360.0 / 2187.0,
        64448.0 / 6561.0,
        -212.0 / 729.0,
        0.0,
        0.0,
    ],
    [
        9017.0 / 3168.0,
        -355.0 / 33.0,
        46732.0 / 5247.0,
        49.0 / 176.0,
        -5103.0 / 18656.0,
        0.0,
    ],
    [
        35.0 / 384.0,
        0.0,
        500.0 / 1113.0,
        125.0 / 192.0,
        -2187.0 / 6784.0,
        11.0 / 84.0,
    ],
];
/// 5th-order weights (= the last A row: FSAL).
const B: [f64; 7] = [
    35.0 / 384.0,
    0.0,
    500.0 / 1113.0,
    125.0 / 192.0,
    -2187.0 / 6784.0,
    11.0 / 84.0,
    0.0,
];
/// Embedded 4th-order weights.
const BHAT: [f64; 7] = [
    5179.0 / 57600.0,
    0.0,
    7571.0 / 16695.0,
    393.0 / 640.0,
    -92097.0 / 339200.0,
    187.0 / 2100.0,
    1.0 / 40.0,
];
/// Hairer's dense-output D coefficients (dopri5.f).
const D: [f64; 7] = [
    -12715105075.0 / 11282082432.0,
    0.0,
    87487479700.0 / 32700410799.0,
    -10690763975.0 / 1880347072.0,
    701980252875.0 / 199316789632.0,
    -1453857185.0 / 822651844.0,
    69997945.0 / 29380423.0,
];

/// One accepted step's dense-output coefficients (Hairer's rcont1..rcont5),
/// flattened per component: `u(θ) = r1 + θ(r2 + (1−θ)(r3 + θ(r4 + (1−θ)r5)))`.
struct DenseSeg {
    t: f64,
    h: f64,
    rcont: Vec<[f64; 5]>, // one [r1..r5] per component
}

/// A computed trajectory with dense output.
pub struct OdeSolution {
    dim: usize,
    t0: f64,
    ts: Vec<f64>,
    ys: Vec<Vec<f64>>,
    segs: Vec<DenseSeg>,
    /// True when integration stopped before reaching `t1` (blow-up,
    /// vanishing step, or the step budget) — Doenet's warning path.
    pub terminated_early: bool,
}

impl OdeSolution {
    pub fn dim(&self) -> usize {
        self.dim
    }
    pub fn last_t(&self) -> f64 {
        *self.ts.last().unwrap_or(&self.t0)
    }
    pub fn last_y(&self) -> Vec<f64> {
        self.ys.last().cloned().unwrap_or_default()
    }
    pub fn times(&self) -> &[f64] {
        &self.ts
    }

    /// Dense output: the interpolated state at `t` (clamped to the computed
    /// span, matching `numeric.dopri`'s behavior of only answering inside
    /// the trajectory). Always a length-n vector, even for n = 1 (§5a).
    pub fn at(&self, t: f64) -> Vec<f64> {
        // NaN can't be ordered: the binary search below would panic (an abort
        // under wasm's panic=abort). Propagate NaN like any float function.
        if t.is_nan() {
            let n = self.ys.first().map_or(0, Vec::len);
            return vec![f64::NAN; n];
        }
        if self.segs.is_empty() {
            return self.ys.first().cloned().unwrap_or_default();
        }
        let forward = self.segs[0].h > 0.0;
        // Binary search for the segment containing t (clamped).
        let cmp_key = |seg: &DenseSeg| if forward { seg.t } else { -seg.t };
        let key = if forward { t } else { -t };
        let idx = match self
            .segs
            .binary_search_by(|s| cmp_key(s).partial_cmp(&key).unwrap())
        {
            Ok(i) => i,
            Err(0) => 0,
            Err(i) => i - 1,
        };
        let seg = &self.segs[idx.min(self.segs.len() - 1)];
        let theta = ((t - seg.t) / seg.h).clamp(0.0, 1.0);
        seg.rcont
            .iter()
            .map(|r| {
                let [r1, r2, r3, r4, r5] = *r;
                r1 + theta * (r2 + (1.0 - theta) * (r3 + theta * (r4 + (1.0 - theta) * r5)))
            })
            .collect()
    }
}

/// Integrate `y′ = f(t, y)` from `t0` to `t1` with adaptive DP5(4).
/// `f` writes the derivative into its out-slice and returns `false` to abort
/// (e.g. an evaluation failure) — treated like a non-finite state.
pub fn solve_ode_with<F>(
    mut f: F,
    t0: f64,
    t1: f64,
    y0: &[f64],
    tol: f64,
    max_steps: usize,
) -> OdeSolution
where
    F: FnMut(f64, &[f64], &mut [f64]) -> bool,
{
    let dim = y0.len();
    let tol = if tol.is_finite() && tol > 0.0 { tol } else { 1e-6 };
    let max_steps = max_steps.min(crate::resource_limits::current().max_ode_steps).max(1);
    let mut sol = OdeSolution {
        dim,
        t0,
        ts: vec![t0],
        ys: vec![y0.to_vec()],
        segs: Vec::new(),
        terminated_early: false,
    };
    if dim == 0 || !y0.iter().all(|v| v.is_finite()) || !t0.is_finite() || !t1.is_finite() {
        sol.terminated_early = true;
        return sol;
    }
    if t0 == t1 {
        return sol;
    }
    let dir = (t1 - t0).signum();
    let span = (t1 - t0).abs();

    let mut t = t0;
    let mut y = y0.to_vec();
    let mut k = vec![vec![0.0f64; dim]; 7];
    if !f(t, &y, &mut k[0]) || !k[0].iter().all(|v| v.is_finite()) {
        sol.terminated_early = true;
        return sol;
    }

    // Initial step: conservative fraction of the span, scaled by the slope.
    let mut h = {
        let ynorm = y.iter().fold(0.0f64, |m, v| m.max(v.abs())).max(1.0);
        let fnorm = k[0].iter().fold(0.0f64, |m, v| m.max(v.abs()));
        let by_slope = if fnorm > 0.0 { 0.01 * ynorm / fnorm } else { span };
        dir * by_slope.min(span / 10.0).max(span * 1e-8)
    };

    // PI controller state (Hairer's beta = 0.04).
    let beta = 0.04;
    let expo1 = 0.2 - beta * 0.75;
    let mut facold = 1e-4f64;

    let mut ynew = vec![0.0f64; dim];
    let mut ystage = vec![0.0f64; dim];
    for _ in 0..max_steps {
        if (t - t1).abs() <= 1e-14 * span || (dir > 0.0 && t >= t1) || (dir < 0.0 && t <= t1) {
            return sol; // reached the end
        }
        // Don't step past t1.
        if (dir > 0.0 && t + h > t1) || (dir < 0.0 && t + h < t1) {
            h = t1 - t;
        }
        // Vanishing step guard.
        if h.abs() < 16.0 * f64::EPSILON * t.abs().max(1.0) {
            sol.terminated_early = true;
            return sol;
        }
        // Stages 2..7 (k1 is fresh from FSAL or the initial evaluation).
        let mut ok = true;
        for s in 1..7 {
            for i in 0..dim {
                let mut acc = 0.0;
                for (j, kj) in k.iter().enumerate().take(s) {
                    acc += A[s][j] * kj[i];
                }
                ystage[i] = y[i] + h * acc;
            }
            if !f(t + C[s] * h, &ystage, &mut k[s]) || !k[s].iter().all(|v| v.is_finite()) {
                ok = false;
                break;
            }
        }
        if !ok {
            // Evaluation failed inside the step: shrink and retry; if the
            // step is already minimal, terminate cleanly.
            h *= 0.5;
            if h.abs() < 16.0 * f64::EPSILON * t.abs().max(1.0) {
                sol.terminated_early = true;
                return sol;
            }
            continue;
        }
        // 5th-order solution (k7 = f(t+h, ynew) by FSAL: A[6] = B).
        for i in 0..dim {
            let mut acc = 0.0;
            for (j, kj) in k.iter().enumerate().take(6) {
                acc += B[j] * kj[i];
            }
            ynew[i] = y[i] + h * acc;
        }
        // Error estimate against the embedded 4th-order weights.
        let mut err_sq = 0.0f64;
        for i in 0..dim {
            let mut e = 0.0;
            for (j, kj) in k.iter().enumerate() {
                e += (B[j] - BHAT[j]) * kj[i];
            }
            let sc = tol + tol * y[i].abs().max(ynew[i].abs());
            let r = h * e / sc;
            err_sq += r * r;
        }
        let err = (err_sq / dim as f64).sqrt();
        if !err.is_finite() || !ynew.iter().all(|v| v.is_finite()) {
            h *= 0.5;
            if h.abs() < 16.0 * f64::EPSILON * t.abs().max(1.0) {
                sol.terminated_early = true;
                return sol;
            }
            continue;
        }

        if err <= 1.0 {
            // Accept: build the dense segment, then advance (FSAL).
            // PI controller (Hairer): growth = safety / (err^expo1 / facold^beta),
            // clamped to [0.2, 5].
            let fac_raw = err.max(1e-16).powf(expo1) / facold.powf(beta);
            let growth = (0.9 / fac_raw).clamp(0.2, 5.0);
            facold = err.max(1e-4);
            if !f(t + h, &ynew, &mut k[6]) || !k[6].iter().all(|v| v.is_finite()) {
                sol.terminated_early = true;
                return sol;
            }
            let mut rcont = Vec::with_capacity(dim);
            for i in 0..dim {
                let ydiff = ynew[i] - y[i];
                let bspl = h * k[0][i] - ydiff;
                let mut dsum = 0.0;
                for (j, kj) in k.iter().enumerate() {
                    dsum += D[j] * kj[i];
                }
                rcont.push([
                    y[i],
                    ydiff,
                    bspl,
                    ydiff - h * k[6][i] - bspl,
                    h * dsum,
                ]);
            }
            sol.segs.push(DenseSeg { t, h, rcont });
            t += h;
            y.copy_from_slice(&ynew);
            k.swap(0, 6); // FSAL: k7 becomes next step's k1
            sol.ts.push(t);
            sol.ys.push(y.clone());
            h *= growth;
        } else {
            // Reject: shrink (no PI history update on rejections).
            let fac11 = err.powf(expo1);
            h *= (1.0 / (fac11 / 0.9)).clamp(0.1, 1.0);
        }
    }
    sol.terminated_early = true;
    sol
}

/// Expression-RHS front end (plan §5c): binds `ind_var → t` and
/// `state_vars[i] → y[i]` per stage. Uses the compiled evaluation tape when
/// every RHS compiles (O3 — no per-step allocation beyond the bindings
/// vector); otherwise falls back to `eval_complex` per call (O2).
#[allow(clippy::too_many_arguments)] // mirrors the plan's §5c signature
pub fn solve_ode_exprs(
    rhs: &[Expr],
    ind_var: &str,
    state_vars: &[String],
    t0: f64,
    t1: f64,
    y0: &[f64],
    tol: f64,
    max_steps: usize,
) -> Option<OdeSolution> {
    if rhs.len() != state_vars.len() || y0.len() != rhs.len() || rhs.is_empty() {
        return None;
    }
    let canon: Vec<Expr> = rhs.iter().map(crate::norm::canonicalize).collect();
    // All free variables must be the independent/state variables.
    for c in &canon {
        for v in crate::ops::variables(c) {
            if v != ind_var
                && !state_vars.contains(&v)
                && !crate::sym::is_constant_symbol(&v)
            {
                return None;
            }
        }
    }
    // Tape path: compile each RHS and map its variable slots onto
    // [t, y0, y1, …].
    let tapes: Option<Vec<(crate::precise::tape::CompiledExpr, Vec<usize>)>> = canon
        .iter()
        .map(|c| {
            let tape = crate::precise::compile(c).ok()?;
            let slots: Option<Vec<usize>> = tape
                .vars()
                .iter()
                .map(|name| {
                    if name == ind_var {
                        Some(0usize)
                    } else {
                        state_vars.iter().position(|s| s == name).map(|i| i + 1)
                    }
                })
                .collect();
            Some((tape, slots?))
        })
        .collect();
    match tapes {
        Some(tapes) => {
            let mut full = vec![0.0f64; state_vars.len() + 1];
            let mut bindings: Vec<Vec<f64>> =
                tapes.iter().map(|(_, s)| vec![0.0; s.len()]).collect();
            Some(solve_ode_with(
                move |t, y, out| {
                    full[0] = t;
                    full[1..].copy_from_slice(y);
                    for (i, (tape, slots)) in tapes.iter().enumerate() {
                        for (b, &s) in bindings[i].iter_mut().zip(slots.iter()) {
                            *b = full[s];
                        }
                        match tape.eval_f64(&bindings[i]) {
                            Some((v, _)) if v.is_finite() => out[i] = v,
                            _ => return false,
                        }
                    }
                    true
                },
                t0,
                t1,
                y0,
                tol,
                max_steps,
            ))
        }
        None => {
            // eval_complex fallback (unknown functions the tape can't take).
            let ind = ind_var.to_string();
            let states = state_vars.to_vec();
            Some(solve_ode_with(
                move |t, y, out| {
                    let mut env = crate::eval::Env::new();
                    env.insert(ind.clone(), num_complex::Complex64::new(t, 0.0));
                    for (name, &v) in states.iter().zip(y.iter()) {
                        env.insert(name.clone(), num_complex::Complex64::new(v, 0.0));
                    }
                    for (i, c) in canon.iter().enumerate() {
                        match crate::eval::eval_complex(c, &env) {
                            Some(z) if z.re.is_finite() && z.im.abs() < 1e-9 * z.re.abs().max(1.0) => {
                                out[i] = z.re
                            }
                            _ => return false,
                        }
                    }
                    true
                },
                t0,
                t1,
                y0,
                tol,
                max_steps,
            ))
        }
    }
}
