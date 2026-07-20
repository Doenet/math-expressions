//! Expression utilities: `substitute` and `variables` (PORTING_PLAN.md §15).
//! Small, self-contained ports of the corresponding `me.*` methods.

use crate::eval::{eval_complex, Env};
use crate::expr::Expr;
use crate::norm::{canonicalize, present, simplify_core, syntactic::map_children};
use crate::num::Number;
use num_complex::Complex64;
use std::collections::{BTreeSet, HashMap, HashSet};

/// Fold numeric subexpressions (`4 + x − 2` → `x + 2`) — the port of
/// `me.evaluate_numbers`. Ours is the exact canonical fold (§3a): rationals
/// stay exact where the JS produces floats; the combined/ordered shape is the
/// canonical one.
pub fn evaluate_numbers(e: &Expr) -> Expr {
    present(&canonicalize(e))
}

/// Cancel common polynomial factors in fractions — the port of
/// `me.reduce_rational` (`(x²−1)/(x−1)` → `x+1`, `(x²−5x+6)/(x²−4)` →
/// `(x−3)/(x+2)`, multivariate included). Applied bottom-up at every node;
/// non-polynomial fractions (`sin x / x`) are left unchanged. Backed by the
/// §8 polynomial layer (recursive dense GCD over ℚ, bounded per §7f).
pub fn reduce_rational(e: &Expr) -> Expr {
    let canon = canonicalize(e);
    // Bottom-up reduction, then re-canonicalize so in-place reductions merge
    // with their surroundings (`1 + (x²−1)/(x−1)` → `x + 2`).
    present(&canonicalize(&reduce_node(&canon)))
}

fn reduce_node(e: &Expr) -> Expr {
    let e = map_children(e, reduce_node);
    let Expr::Mul(factors) = &e else { return e };

    // Split canonical `Mul` factors into numerator parts and denominator
    // bases: a factor `Pow(b, −k)` (integer k>0) contributes `b^k` below.
    let mut num_parts: Vec<Expr> = Vec::new();
    let mut den_parts: Vec<Expr> = Vec::new();
    for f in factors {
        if let Expr::Pow(b, x) = f {
            if let Expr::Num(Number::Int(k)) = &**x {
                if *k < 0 {
                    den_parts.push(crate::norm::pow(
                        (**b).clone(),
                        Expr::Num(Number::Int(-k)),
                    ));
                    continue;
                }
            }
        }
        num_parts.push(f.clone());
    }
    if den_parts.is_empty() {
        return e;
    }
    let num = crate::norm::mul(num_parts);
    let den = crate::norm::mul(den_parts);

    // Common variable list (order fixed by BTreeSet). Constant symbols are
    // rejected by the converter, so `pi/x` style fractions pass through.
    let mut vars = BTreeSet::new();
    collect_var_names(&num, &mut vars);
    collect_var_names(&den, &mut vars);
    let vars: Vec<String> = vars.into_iter().collect();
    if vars.is_empty() {
        return e; // pure numeric fraction — Number arithmetic already reduced it
    }

    let (Some(pn), Some(pd)) = (
        crate::poly::expr_to_poly(&num, &vars),
        crate::poly::expr_to_poly(&den, &vars),
    ) else {
        return e;
    };
    let Some(g) = crate::poly::gcd(&pn, &pd, vars.len()) else {
        return e;
    };
    if crate::poly::is_trivial(&g) {
        return e;
    }
    let (Some(qn), Some(qd)) = (
        crate::poly::exact_div_top(&pn, &g, vars.len()),
        crate::poly::exact_div_top(&pd, &g, vars.len()),
    ) else {
        return e;
    };
    // Normalize the quotients' rational content into a single scalar on the
    // numerator, so `(2x+4)/2` comes out as `x+2` rather than `½·(2x+4)`.
    let (cn, qn) = crate::poly::strip_rational_content(&qn);
    let (cd, qd) = crate::poly::strip_rational_content(&qd);
    let scalar = Expr::Num(Number::from_bigrational(cn / cd));
    let new_num = crate::norm::mul(vec![scalar, crate::poly::poly_to_expr(&qn, &vars)]);
    let new_den = crate::poly::poly_to_expr(&qd, &vars);
    canonicalize(&Expr::Div(Box::new(new_num), Box::new(new_den)))
}

/// The applied function names in `e`, first-appearance order, de-duplicated —
/// the port of `me.functions` (`sin(x)+f(y)` → `["sin","f"]`). Only bare-Sym
/// application heads count (a `Pow` head like `sin^2` contributes its inner
/// name via the canonical faithful tree's head structure being Sym-rooted).
pub fn functions(e: &Expr) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    fn walk(e: &Expr, out: &mut Vec<String>, seen: &mut HashSet<String>) {
        if let Expr::Apply(head, _) = e {
            // Dig a bare name out of the head (`sin`, and `sin` inside `sin^2`).
            fn head_name(h: &Expr) -> Option<String> {
                match h {
                    Expr::Sym(s) => Some(s.name()),
                    Expr::Pow(b, _) => head_name(b),
                    Expr::Prime(x) => head_name(x),
                    _ => None,
                }
            }
            if let Some(name) = head_name(head) {
                if seen.insert(name.clone()) {
                    out.push(name);
                }
            }
        }
        for c in e.children() {
            walk(c, out, seen);
        }
    }
    walk(e, &mut out, &mut seen);
    out
}

/// The operator heads used in `e`, first-appearance order, de-duplicated (JS
/// tree spelling: `+`, `-`, `*`, `/`, `^`, `apply`-less) — the port of
/// `me.operators` on the faithful tree.
pub fn operators(e: &Expr) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    fn push(name: &str, out: &mut Vec<String>, seen: &mut HashSet<String>) {
        if seen.insert(name.to_string()) {
            out.push(name.to_string());
        }
    }
    fn walk(e: &Expr, out: &mut Vec<String>, seen: &mut HashSet<String>) {
        match e {
            Expr::Add(_) => push("+", out, seen),
            Expr::Neg(_) => push("-", out, seen),
            Expr::Mul(_) => push("*", out, seen),
            Expr::Div(..) => push("/", out, seen),
            Expr::Pow(..) => push("^", out, seen),
            Expr::And(_) => push("and", out, seen),
            Expr::Or(_) => push("or", out, seen),
            Expr::Not(_) => push("not", out, seen),
            Expr::Union(_) => push("union", out, seen),
            Expr::Intersect(_) => push("intersect", out, seen),
            _ => {}
        }
        for c in e.children() {
            walk(c, out, seen);
        }
    }
    walk(e, &mut out, &mut seen);
    out
}

/// The `i`-th (0-based) component of a tuple/vector/list, port of
/// `me.get_component`. `None` for non-sequences or out-of-range.
pub fn get_component(e: &Expr, i: usize) -> Option<Expr> {
    match e {
        Expr::Seq(_, xs) => xs.get(i).cloned(),
        _ => None,
    }
}

/// Replace the `i`-th component of a sequence, port of
/// `me.substitute_component`. `None` for non-sequences or out-of-range.
pub fn substitute_component(e: &Expr, i: usize, value: &Expr) -> Option<Expr> {
    match e {
        Expr::Seq(k, xs) if i < xs.len() => {
            let mut xs = xs.clone();
            xs[i] = value.clone();
            Some(Expr::Seq(*k, xs))
        }
        _ => None,
    }
}

/// Collapse simple subscripts into flat symbol names, port of
/// `me.subscripts_to_strings`: `x_1` (`Index(x, 1)`) → the symbol `x_1`.
/// Only bare-symbol bases with a number or bare-symbol index convert;
/// everything else is left structural.
pub fn subscripts_to_strings(e: &Expr) -> Expr {
    if let Expr::Index(base, idx) = e {
        if let Expr::Sym(b) = &**base {
            let suffix = match &**idx {
                Expr::Sym(s) => Some(s.name()),
                Expr::Num(n) => n.terminating_decimal(),
                _ => None,
            };
            if let Some(sfx) = suffix {
                return Expr::sym(&format!("{}_{}", b.name(), sfx));
            }
        }
    }
    map_children(e, subscripts_to_strings)
}

/// Inverse of [`subscripts_to_strings`]: a symbol containing `_` splits at the
/// first underscore into `Index(base, index)`, with a numeric suffix parsed as
/// a number (`x_1` → `Index(x, 1)`, `y_a` → `Index(y, a)`).
pub fn strings_to_subscripts(e: &Expr) -> Expr {
    if let Expr::Sym(s) = e {
        let name = s.name();
        if let Some(pos) = name.find('_') {
            let (base, sfx) = (&name[..pos], &name[pos + 1..]);
            if !base.is_empty() && !sfx.is_empty() {
                let idx = match sfx.parse::<i64>() {
                    Ok(n) => Expr::int(n),
                    Err(_) => Expr::sym(sfx),
                };
                return Expr::Index(Box::new(Expr::sym(base)), Box::new(idx));
            }
        }
        return e.clone();
    }
    map_children(e, strings_to_subscripts)
}

/// Convert 2-element tuples/arrays into interval notation, port of
/// `me.to_intervals`: `(1,2)` → the open interval, `[1,2]` → the closed one
/// (half-open forms already parse as intervals). Recurses everywhere; other
/// shapes are untouched.
pub fn to_intervals(e: &Expr) -> Expr {
    use crate::expr::SeqKind;
    if let Expr::Seq(kind, xs) = e {
        if xs.len() == 2 && matches!(kind, SeqKind::Tuple | SeqKind::Array) {
            let closed = matches!(kind, SeqKind::Array);
            return Expr::Interval {
                endpoints: Box::new((to_intervals(&xs[0]), to_intervals(&xs[1]))),
                closed: (closed, closed),
            };
        }
    }
    map_children(e, to_intervals)
}

fn collect_var_names(e: &Expr, out: &mut BTreeSet<String>) {
    if let Expr::Sym(s) = e {
        let name = s.name();
        if !crate::sym::is_constant_symbol(&name) {
            out.insert(name);
        }
    }
    for c in e.children() {
        collect_var_names(c, out);
    }
}

/// Replace the constant symbols `pi` and `e` with their floating-point values
/// (`i` is left as the imaginary unit). Matches `me.constants_to_floats`.
pub fn constants_to_floats(e: &Expr) -> Expr {
    match e {
        Expr::Sym(s) => match s.name().as_str() {
            "pi" => Expr::Num(Number::from_f64(std::f64::consts::PI)),
            "e" => Expr::Num(Number::from_f64(std::f64::consts::E)),
            _ => e.clone(),
        },
        _ => map_children(e, constants_to_floats),
    }
}

/// Round every number in `e` to `decimals` decimal places (ties away from zero).
pub fn round_numbers_to_decimals(e: &Expr, decimals: i32) -> Expr {
    map_numbers(e, &|n| n.round_to_decimals(decimals))
}

/// Round every number in `e` to `sig_figs` significant figures.
pub fn round_numbers_to_precision(e: &Expr, sig_figs: i32) -> Expr {
    map_numbers(e, &|n| {
        if sig_figs < 1 {
            return n.clone();
        }
        // Decimal place of the leading significant digit, then round so that
        // `sig_figs` digits survive. `magnitude_log10` is finite for every
        // nonzero value — including exact rationals outside f64 range like a
        // pasted `1e-400` or a 350-digit integer — and the i64 arithmetic +
        // saturating narrow avoid the i32 overflow those extremes caused.
        // (`round_to_decimals` clamps its argument again internally.)
        let Some(k) = n.magnitude_log10() else {
            return n.clone(); // zero / NaN
        };
        let d = (i64::from(sig_figs) - 1 - k).clamp(i64::from(i32::MIN), i64::from(i32::MAX));
        n.round_to_decimals(d as i32)
    })
}

/// Round every number to `digits` significant figures but never below
/// `decimals` decimal places — the port of
/// `me.round_numbers_to_precision_plus_decimals` (Doenet's display rounding:
/// "4 significant digits, at least 2 decimals"). Parameters are `f64` because
/// the JS callers pass `±Infinity` to disable one of the modes: `digits < 1`
/// (incl. `-Infinity`) → decimals-only; `digits > 15` (incl. `Infinity`) →
/// unchanged; non-finite `decimals` → precision-only.
pub fn round_numbers_to_precision_plus_decimals(e: &Expr, digits: f64, decimals: f64) -> Expr {
    let use_precision = digits >= 1.0;
    let sig_figs = digits.round();
    if use_precision && sig_figs > 15.0 {
        return e.clone();
    }
    let use_decimals = decimals.is_finite();
    // No need to go much beyond the limits of double precision (JS clamps ±330).
    let nd = decimals.round().clamp(-330.0, 330.0) as i64;

    match (use_precision, use_decimals) {
        (true, true) => map_numbers(e, &|n| {
            let Some(k) = n.magnitude_log10() else {
                return n.clone(); // zero / NaN
            };
            let d = (sig_figs as i64 - 1 - k)
                .max(nd)
                .clamp(i64::from(i32::MIN), i64::from(i32::MAX));
            n.round_to_decimals(d as i32)
        }),
        (true, false) => round_numbers_to_precision(e, sig_figs as i32),
        (false, true) => round_numbers_to_decimals(e, nd as i32),
        (false, false) => e.clone(),
    }
}

/// Apply `f` to every `Num` leaf, recursing through the whole tree.
fn map_numbers(e: &Expr, f: &dyn Fn(&Number) -> Number) -> Expr {
    match e {
        Expr::Num(n) => Expr::Num(f(n)),
        _ => map_children(e, |c| map_numbers(c, f)),
    }
}

/// Evaluate `e` at real variable bindings, returning its (possibly complex)
/// numeric value. `None` if a needed variable is unbound, the expression is not
/// numerically meaningful, or the result is non-finite (`me.evaluate` returns
/// `null` for e.g. `1/0`). Uses the complex principal branch, matching mathjs:
/// `x^(1/3)` at `x = -8` is `1 + i√3`, not the real root `-2`.
pub fn evaluate(e: &Expr, bindings: &HashMap<String, f64>) -> Option<Complex64> {
    let env: Env = bindings
        .iter()
        .map(|(k, v)| (k.clone(), Complex64::new(*v, 0.0)))
        .collect();
    finite(eval_complex(e, &env)?)
}

/// Evaluate a closed expression to its numeric constant, or `None`. Matches
/// `me.evaluate_to_constant`: `None` if the *original* expression mentions any
/// genuine free variable (the constants `pi`/`e`/`i` don't count, and it does
/// NOT cancel first — so `x − x` is `None`, not `0`); otherwise simplify (real-
/// domain reductions apply, `(-8)^(1/3)` → `-2` — contrast [`evaluate`]'s
/// complex-principal branch) and evaluate, `None` if non-finite.
pub fn evaluate_to_constant(e: &Expr) -> Option<Complex64> {
    if variables(e)
        .iter()
        .any(|v| !crate::sym::is_constant_symbol(v))
    {
        return None;
    }
    finite(eval_complex(&simplify_core(e), &Env::new())?)
}

fn finite(v: Complex64) -> Option<Complex64> {
    (v.re.is_finite() && v.im.is_finite()).then_some(v)
}

/// Simultaneously replace each `Sym(name)` with `subs[name]`. Substitution is
/// one-pass and simultaneous — a replacement is not itself re-substituted, so
/// `{x: y, y: x}` swaps — and does NOT simplify (`x^2` with `x → 2` gives
/// `2^2`, not `4`), matching `me.substitute`. Recurses into every subexpression,
/// including function arguments.
pub fn substitute(e: &Expr, subs: &HashMap<String, Expr>) -> Expr {
    match e {
        Expr::Sym(s) => match subs.get(&s.name()) {
            Some(rep) => rep.clone(),
            None => e.clone(),
        },
        _ => map_children(e, |c| substitute(c, subs)),
    }
}

/// The free variable names of `e`, in first-appearance order, de-duplicated.
/// Matches `me.variables`: the constant symbols `pi`/`e`/`i` ARE included (they
/// are ordinary symbols here), but a function-application head (`sin` in
/// `sin(x)`, `f` in `f(x)`) is NOT.
pub fn variables(e: &Expr) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    collect(e, &mut out, &mut seen);
    out
}

fn collect(e: &Expr, out: &mut Vec<String>, seen: &mut HashSet<String>) {
    match e {
        Expr::Sym(s) => {
            let name = s.name();
            if seen.insert(name.clone()) {
                out.push(name);
            }
        }
        Expr::Num(_) | Expr::Const(_) | Expr::RootOf { .. } | Expr::Blank | Expr::Ldots => {}

        // An application head is never a variable source — JS drops the head
        // wholesale (`tree.slice(2)` in lib/expression/variables.js), even a
        // compound one like `f'` or `sin^2` — so `f'(x)` has variables `[x]`,
        // not `[f, x]`.
        Expr::Apply(_, args) => {
            for a in args {
                collect(a, out, seen);
            }
        }

        Expr::Add(xs)
        | Expr::Mul(xs)
        | Expr::And(xs)
        | Expr::Or(xs)
        | Expr::Union(xs)
        | Expr::Intersect(xs)
        | Expr::Seq(_, xs)
        | Expr::OtherOp(_, xs) => {
            for c in xs {
                collect(c, out, seen);
            }
        }
        Expr::Div(a, b) | Expr::Pow(a, b) | Expr::Index(a, b) => {
            collect(a, out, seen);
            collect(b, out, seen);
        }
        Expr::Neg(x) | Expr::Not(x) | Expr::Prime(x) => collect(x, out, seen),
        Expr::Interval { endpoints, .. } => {
            collect(&endpoints.0, out, seen);
            collect(&endpoints.1, out, seen);
        }
        Expr::Relation { operands, .. } => {
            for c in operands {
                collect(c, out, seen);
            }
        }
        Expr::Matrix { entries, .. } => {
            for c in entries {
                collect(c, out, seen);
            }
        }
    }
}
