//! Symbolic differentiation (PORTING_PLAN.md §15 Phase 8).
//!
//! Ports the behaviour of `me.derivative(var)`, which upstream delegates to
//! mathjs's `derivative` (NOT the hand-written `derivative_with_story`, which is
//! only the pedagogical step-by-step version). So the rules are the standard
//! ones — sum / product / quotient / power / chain — plus a function-derivative
//! table matching mathjs (extracted directly from its output). The result is
//! `simplify`d for a clean, canonical form.
//!
//! Correctness is checked against the JS oracle via `equals` (semantic), so the
//! output need not match mathjs's tree shape — only its value.

use crate::expr::Expr;
use crate::norm::simplify;
use crate::parse::text::{TextToAst, TextToAstOptions};

/// d/d`var` of `e`, simplified. `var` is the differentiation variable's name.
pub fn derivative(e: &Expr, var: &str) -> Expr {
    simplify(&diff(e, var))
}

/// The raw (unsimplified) derivative in the faithful layer.
fn diff(e: &Expr, var: &str) -> Expr {
    // Anything not mentioning the variable is constant → 0. This subsumes
    // numbers, other variables, and `pi`/`e` used as constants.
    if !contains_var(e, var) {
        return Expr::int(0);
    }

    match e {
        Expr::Sym(s) => Expr::int(if s.name() == var { 1 } else { 0 }),

        // Sum rule: d(Σ tᵢ) = Σ d(tᵢ).
        Expr::Add(ts) => Expr::Add(ts.iter().map(|t| diff(t, var)).collect()),
        Expr::Neg(a) => Expr::Neg(Box::new(diff(a, var))),

        // Product rule: d(∏ fᵢ) = Σᵢ (∏_{j≠i} fⱼ)·d(fᵢ).
        Expr::Mul(fs) => {
            let terms = (0..fs.len())
                .map(|i| {
                    let factors = fs
                        .iter()
                        .enumerate()
                        .map(|(j, f)| if i == j { diff(f, var) } else { f.clone() })
                        .collect();
                    Expr::Mul(factors)
                })
                .collect();
            Expr::Add(terms)
        }

        // Quotient rule: d(f/g) = (f'·g − f·g') / g².
        Expr::Div(f, g) => {
            let num = sub(
                mul2(diff(f, var), (**g).clone()),
                mul2((**f).clone(), diff(g, var)),
            );
            Expr::Div(Box::new(num), Box::new(pow2((**g).clone(), Expr::int(2))))
        }

        Expr::Pow(base, exp) => power_rule(base, exp, var),
        Expr::Apply(head, args) => apply_rule(head, args, var),

        // Shapes with no differentiation rule that DO contain the variable
        // (sequences/tuples, relations, matrices, subscripts, …): return an
        // opaque `derivative(e, var)` node rather than asserting a wrong `0`
        // (same policy as apply_rule's prime fallback). It samples as an
        // opaque atom in `equals` and renders as `derivative(…, var)`.
        _ => Expr::OtherOp(
            crate::sym::Sym::new("derivative"),
            vec![e.clone(), Expr::sym(var)],
        ),
    }
}

/// d/dx of `base^exp`, by which of base/exp contain the variable.
fn power_rule(base: &Expr, exp: &Expr, var: &str) -> Expr {
    let exp_has = contains_var(exp, var);
    let base_has = contains_var(base, var);

    if !exp_has {
        // Constant exponent: n·base^(n−1)·base'  (base must contain the var,
        // else the whole node is constant and never reaches here).
        let reduced = pow2((base).clone(), sub((exp).clone(), Expr::int(1)));
        return mul3((exp).clone(), reduced, diff(base, var));
    }
    if is_e(base) {
        // d(e^u) = e^u · u'.
        return mul2(pow2((base).clone(), (exp).clone()), diff(exp, var));
    }
    if !base_has {
        // Constant base a: d(a^u) = a^u · log(a) · u'.
        return mul3(
            pow2((base).clone(), (exp).clone()),
            log_of((base).clone()),
            diff(exp, var),
        );
    }
    // General u^v: d = u^v · (v'·log(u) + v·u'/u).
    let bracket = Expr::Add(vec![
        mul2(diff(exp, var), log_of((base).clone())),
        Expr::Div(
            Box::new(mul2((exp).clone(), diff(base, var))),
            Box::new((base).clone()),
        ),
    ]);
    mul2(pow2((base).clone(), (exp).clone()), bracket)
}

/// d/dx of a function application, by the chain rule.
fn apply_rule(head: &Expr, args: &[Expr], var: &str) -> Expr {
    // Only bare single-argument function symbols are handled specially.
    if let (Expr::Sym(f), [arg]) = (head, args) {
        let inner = diff(arg, var);
        if let Some(outer) = outer_derivative(&f.name(), arg) {
            return mul2(outer, inner);
        }
        // Unknown single-arg function → prime notation: f'(u)·u'. Matches the
        // upstream "story" fallback for functions with no known derivative.
        let fprime = Expr::Apply(Box::new(Expr::Prime(Box::new(Expr::Sym(*f)))), vec![arg.clone()]);
        return mul2(fprime, inner);
    }
    // Multi-argument or non-symbol heads are not differentiated (rare; e.g.
    // atan2, log-with-base). Leave an opaque prime rather than a wrong value.
    Expr::Apply(Box::new(Expr::Prime(Box::new(head.clone()))), args.to_vec())
}

/// The outer derivative `f'(arg)`: the mathjs derivative-table entry for `f`
/// (as a text template in the placeholder `x`) with the placeholder replaced by
/// the actual argument. `None` for functions with no table entry.
///
/// Each template is parsed **once per thread** and cached. A thread-local
/// cache (not a global `OnceLock`) is load-bearing: `Sym` interning is
/// thread-local, so a globally cached `Expr` would carry one thread's symbol
/// ids into another thread's interner.
fn outer_derivative(fname: &str, arg: &Expr) -> Option<Expr> {
    thread_local! {
        static TEMPLATE_CACHE: std::cell::RefCell<std::collections::HashMap<&'static str, Expr>> =
            std::cell::RefCell::new(std::collections::HashMap::new());
    }
    let template = template_for(fname)?;
    let parsed = TEMPLATE_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .entry(template)
            .or_insert_with(|| {
                TextToAst::new(TextToAstOptions::default())
                    .convert(template)
                    .expect("derivative-table template must parse")
            })
            .clone()
    });
    let subs = std::collections::HashMap::from([("x".to_string(), arg.clone())]);
    Some(crate::ops::substitute(&parsed, &subs))
}

/// The mathjs `d/dx f(x)` output for each supported `f`, as a text template in
/// the placeholder `x`. The table is `FnDef::derivative` in
/// `crate::functions` (alias-aware, so `arc*` spellings find the `a*` entry).
fn template_for(fname: &str) -> Option<&'static str> {
    crate::functions::derivative_template(fname)
}

// ---- small faithful-layer builders ----

fn mul2(a: Expr, b: Expr) -> Expr {
    Expr::Mul(vec![a, b])
}
fn mul3(a: Expr, b: Expr, c: Expr) -> Expr {
    Expr::Mul(vec![a, b, c])
}
fn pow2(base: Expr, exp: Expr) -> Expr {
    Expr::Pow(Box::new(base), Box::new(exp))
}
fn sub(a: Expr, b: Expr) -> Expr {
    Expr::Add(vec![a, Expr::Neg(Box::new(b))])
}
fn log_of(a: Expr) -> Expr {
    Expr::Apply(Box::new(Expr::sym("log")), vec![a])
}

/// Is `e` the constant `e` (either spelling: the `e` symbol or `MathConst::E`)?
fn is_e(e: &Expr) -> bool {
    matches!(e, Expr::Sym(s) if s.name() == "e")
        || matches!(e, Expr::Const(crate::expr::MathConst::E))
}

/// Does `e` mention the variable `var` anywhere (full recursion — unlike
/// `eval::free_symbols`, which stops at opaque transcendental subtrees)?
fn contains_var(e: &Expr, var: &str) -> bool {
    e.any_subexpr(&|c| matches!(c, Expr::Sym(s) if s.name() == var))
}
