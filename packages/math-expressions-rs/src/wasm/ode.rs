//! ODE solving (ODE_PLAN O1+O2): the dense-output `OdeSolution` handle and the
//! numeric (`js_sys::Function` RHS) and expression-RHS solvers.

use super::Expression;
use crate::Expr;
use wasm_bindgen::prelude::*;

/// A computed trajectory with dense output — the `numeric.dopri` result
/// contract: `at(t)` always returns a length-n Float64Array (§5a), and
/// `last_t`/`last_y` support ODESystem's chunk chaining (§5b).
#[wasm_bindgen]
pub struct OdeSolution(crate::ode::OdeSolution);

#[wasm_bindgen]
impl OdeSolution {
    pub fn at(&self, t: f64) -> Vec<f64> {
        self.0.at(t)
    }

    /// Batch sampling for plotting: the states at each `ts[i]`, flattened
    /// row-major (`n` values per abscissa).
    pub fn at_many(&self, ts: Vec<f64>) -> Vec<f64> {
        let mut out = Vec::with_capacity(ts.len() * self.0.dim());
        for t in ts {
            out.extend(self.0.at(t));
        }
        out
    }

    pub fn dim(&self) -> usize {
        self.0.dim()
    }
    pub fn last_t(&self) -> f64 {
        self.0.last_t()
    }
    pub fn last_y(&self) -> Vec<f64> {
        self.0.last_y()
    }
    /// True when integration stopped before t1 (blow-up / budget) —
    /// Doenet's warning path, never an exception or NaN samples.
    pub fn terminated_early(&self) -> bool {
        self.0.terminated_early
    }
    /// The accepted step times (diagnostics).
    pub fn times(&self) -> Vec<f64> {
        self.0.times().to_vec()
    }
}

/// Drop-in for `numeric.dopri(t0, t1, y0, f, tol, maxit)`: `f` is a JS
/// closure `(t, yArray) -> array`. One boundary crossing per RK stage; for
/// expression right-hand sides prefer [`solve_ode_expressions`], which
/// evaluates entirely inside wasm.
#[wasm_bindgen]
pub fn solve_ode(
    f: &js_sys::Function,
    t0: f64,
    t1: f64,
    y0: Vec<f64>,
    tol: f64,
    max_steps: usize,
) -> OdeSolution {
    let this = JsValue::NULL;
    let sol = crate::ode::solve_ode_with(
        |t, y, out| {
            let arr = js_sys::Float64Array::from(y);
            match f.call2(&this, &JsValue::from_f64(t), &arr.into()) {
                Ok(v) => {
                    let a = js_sys::Array::from(&v);
                    if a.length() as usize != out.len() {
                        return false;
                    }
                    for (i, slot) in out.iter_mut().enumerate() {
                        match a.get(i as u32).as_f64() {
                            Some(x) => *slot = x,
                            None => return false,
                        }
                    }
                    true
                }
                Err(_) => false,
            }
        },
        t0,
        t1,
        &y0,
        tol,
        max_steps,
    );
    OdeSolution(sol)
}

/// Expression-RHS solver (plan §5c): `rhs` is a tuple/vector Expression with
/// one component per state variable (or a single expression for n = 1),
/// evaluated inside wasm via the compiled tape — no boundary crossings.
/// `undefined` when the expressions reference unknown variables.
#[wasm_bindgen]
pub fn solve_ode_expressions(
    rhs: &Expression,
    ind_var: &str,
    state_vars: Vec<String>,
    t0: f64,
    t1: f64,
    y0: Vec<f64>,
    tol: f64,
    max_steps: usize,
) -> Option<OdeSolution> {
    let comps: Vec<Expr> = match &rhs.0 {
        Expr::Seq(_, xs) => xs.clone(),
        other => vec![other.clone()],
    };
    crate::ode::solve_ode_exprs(&comps, ind_var, &state_vars, t0, t1, &y0, tol, max_steps)
        .map(OdeSolution)
}
