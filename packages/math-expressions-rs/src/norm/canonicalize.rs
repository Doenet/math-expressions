//! Bottom-up canonicalization dispatch, plus the function-application and
//! relation canonicalization it delegates to.

use super::{add, cmp, mul, pow};
use crate::expr::{Expr, RelOp, SeqKind};
use crate::num::Number;
use std::cmp::Ordering;

/// Bottom-up canonicalization: canonicalize children, then apply the smart
/// constructor for the node.
pub fn canonicalize(e: &Expr) -> Expr {
    match e {
        Expr::Num(_) | Expr::Sym(_) | Expr::Const(_) | Expr::Blank | Expr::Ldots => e.clone(),
        // Re-establish the canonical invariant (primitive integer squarefree
        // coefficients) for RootOf leaves built outside the smart
        // constructors, e.g. deserialized trees. An unrepresentable one
        // (bad index, degree cap) is kept as-is — it is still a leaf.
        Expr::RootOf { poly, index } => crate::rootof::coeffs_to_upoly(poly)
            .and_then(|p| crate::rootof::make_rootof(&p, *index))
            .unwrap_or_else(|| e.clone()),

        Expr::Add(ts) => add(ts.iter().map(canonicalize).collect()),
        Expr::Mul(fs) => mul(fs.iter().map(canonicalize).collect()),
        // Display-only variants rewrite into the algebraic core.
        Expr::Div(a, b) => mul(vec![
            canonicalize(a),
            pow(canonicalize(b), Expr::Num(Number::Int(-1))),
        ]),
        Expr::Neg(x) => {
            let cx = canonicalize(x);
            // −(±y) → ±y: the value set {y, −y} is closed under negation, so the
            // outer sign is absorbed (port of JS simplify's pm negation rule).
            if crate::pm::is_pm(&cx) {
                cx
            } else {
                mul(vec![Expr::Num(Number::Int(-1)), cx])
            }
        }
        Expr::Pow(b, e) => pow(canonicalize(b), canonicalize(e)),

        Expr::And(xs) => assoc_sorted(Variant::And, xs),
        Expr::Or(xs) => assoc_sorted(Variant::Or, xs),
        Expr::Union(xs) => assoc_sorted(Variant::Union, xs),
        Expr::Intersect(xs) => assoc_sorted(Variant::Intersect, xs),
        Expr::Not(x) => Expr::Not(Box::new(canonicalize(x))),

        Expr::Apply(head, args) => {
            canon_apply(canonicalize(head), args.iter().map(canonicalize).collect())
        }
        // Push primes inside an application so `f(x)'` and `f'(x)` agree:
        // Prime(Apply(h, args)) → Apply(Prime(h), args). Recursion handles
        // repeated primes (`f'''`).
        Expr::Prime(x) => match canonicalize(x) {
            Expr::Apply(h, args) => Expr::Apply(Box::new(Expr::Prime(h)), args),
            other => Expr::Prime(Box::new(other)),
        },
        Expr::Index(a, b) => Expr::Index(Box::new(canonicalize(a)), Box::new(canonicalize(b))),

        Expr::Seq(kind, xs) => {
            let mut v: Vec<Expr> = xs.iter().map(canonicalize).collect();
            // A set is unordered; sort so `{a, b}` and `{b, a}` canonicalize
            // alike. Ordered sequences keep their positions.
            if *kind == SeqKind::Set {
                v.sort_by(cmp);
            }
            Expr::Seq(*kind, v)
        }
        Expr::Interval { endpoints, closed } => Expr::Interval {
            endpoints: Box::new((canonicalize(&endpoints.0), canonicalize(&endpoints.1))),
            closed: *closed,
        },
        Expr::Relation { operands, ops } => {
            canon_relation(operands.iter().map(canonicalize).collect(), ops.clone())
        }
        Expr::Matrix {
            rows,
            cols,
            entries,
        } => Expr::Matrix {
            rows: *rows,
            cols: *cols,
            entries: entries.iter().map(canonicalize).collect(),
        },
        Expr::OtherOp(name, args) => {
            let mut cargs: Vec<Expr> = args.iter().map(canonicalize).collect();
            // `binom(n,k)` and the applied `nCr(n,k)` denote the same thing;
            // unify on the applied form so they compare equal.
            if name.name() == "binom" && cargs.len() == 2 {
                return canon_apply(Expr::sym("nCr"), cargs);
            }
            // Unoriented geometry: `angle(A,B,C) = angle(C,B,A)` (endpoints
            // swap, vertex fixed) and `linesegment(A,B) = linesegment(B,A)`.
            // JS applies `normalize_angle_linesegment_arg_order` here too.
            if name.name() == "angle"
                && cargs.len() == 3
                && cmp(&cargs[0], &cargs[2]) == Ordering::Greater
            {
                cargs.swap(0, 2);
            } else if name.name() == "linesegment"
                && cargs.len() == 2
                && cmp(&cargs[0], &cargs[1]) == Ordering::Greater
            {
                cargs.swap(0, 1);
            }
            Expr::OtherOp(*name, cargs)
        }
    }
}

enum Variant {
    And,
    Or,
    Union,
    Intersect,
}

/// Flatten same-variant children and sort (for commutative boolean/set ops).
fn assoc_sorted(variant: Variant, xs: &[Expr]) -> Expr {
    let mut flat: Vec<Expr> = Vec::with_capacity(xs.len());
    for x in xs.iter().map(canonicalize) {
        match (&variant, &x) {
            (Variant::And, Expr::And(v))
            | (Variant::Or, Expr::Or(v))
            | (Variant::Union, Expr::Union(v))
            | (Variant::Intersect, Expr::Intersect(v)) => flat.extend(v.iter().cloned()),
            _ => flat.push(x),
        }
    }
    flat.sort_by(cmp);
    match variant {
        Variant::And => Expr::And(flat),
        Variant::Or => Expr::Or(flat),
        Variant::Union => Expr::Union(flat),
        Variant::Intersect => Expr::Intersect(flat),
    }
}

/// Build a canonical function application (head and args already canonical),
/// applying the notation/fold rules that hold without assumptions:
/// inverse-function notation (`sin^(-1) → asin`), name normalization
/// (`arcsin → asin`, `ln → log`), and exact folding of `n!`.
fn canon_apply(head: Expr, args: Vec<Expr>) -> Expr {
    // `f^(-1)(x)` is the inverse function, not a reciprocal, for the invertible
    // functions (contrast `sin^2(x)`, which is a power). Only exponent −1.
    if let Expr::Pow(inner, exp) = &head {
        if matches!(&**exp, Expr::Num(Number::Int(-1))) {
            if let Expr::Sym(f) = &**inner {
                if let Some(inv) = inverse_function_name(&f.name()) {
                    return Expr::Apply(Box::new(Expr::sym(inv)), args);
                }
            }
        }
    }

    // `f^n(x)` (n ≠ −1, f in the move set): the exponent moves outside the
    // application — `sin^2(x)` → `sin(x)^2` — so both spellings share ONE
    // canonical form and downstream rules (e.g. the trig Pythagorean rule)
    // match a single shape. Mirrors `pass_applied_functions` in syntactic.rs,
    // using the same `move_exponent_spellings` registry facet.
    if let Expr::Pow(inner, exp) = &head {
        if let Expr::Sym(f) = &**inner {
            if crate::functions::moves_exponent_outside(&f.name())
                && !matches!(&**exp, Expr::Num(Number::Int(-1)))
            {
                let exp = (**exp).clone();
                return pow(canon_apply(Expr::Sym(*f), args), exp);
            }
        }
    }

    let head = normalize_head(head);

    // `rootof(p, k)` with a recognizable single-variable polynomial becomes
    // the RootOf leaf (MATRIX_PLAN §2a); anything else stays an unevaluated
    // application.
    if let Expr::Sym(s) = &head {
        if s.name() == "rootof" {
            if let Some(r) = crate::rootof::from_apply_args(&args) {
                return r;
            }
        }
    }

    // A single-argument `nthroot(x)` is a square root (JS
    // `normalize_function_names`); unify with `sqrt(x)` so they compare equal.
    if let Expr::Sym(s) = &head {
        if s.name() == "nthroot" && args.len() == 1 {
            return Expr::Apply(Box::new(Expr::sym("sqrt")), args);
        }
    }

    // Fold `n!` for a non-negative integer `n` (exact, promotes to Big).
    if let Expr::Sym(s) = &head {
        if s.name() == "factorial" {
            if let [Expr::Num(Number::Int(n))] = args.as_slice() {
                if let Some(v) = factorial_of(*n) {
                    return Expr::Num(v);
                }
            }
        }
    }

    Expr::Apply(Box::new(head), args)
}

/// Normalize a relation:
/// - converse pairs flip to one canonical direction, so `a ⊃ b` ≡ `b ⊂ a`,
///   `x ∈ A` ≡ `A ∋ x`, `a > b` ≡ `b < a` (binary only; chains keep order);
/// - fully symmetric relations (`=`, `≠`, including chained `a=b=c`) sort
///   their operands, so `x = y` ≡ `y = x`.
fn canon_relation(mut operands: Vec<Expr>, ops: Vec<RelOp>) -> Expr {
    use RelOp::*;
    if let ([_, _], [op]) = (operands.as_slice(), ops.as_slice()) {
        if let Some(converse) = match op {
            Gt => Some(Lt),
            Ge => Some(Le),
            Superset => Some(Subset),
            SupersetEq => Some(SubsetEq),
            NotSuperset => Some(NotSubset),
            NotSupersetEq => Some(NotSubsetEq),
            Ni => Some(In),
            NotNi => Some(NotIn),
            _ => None,
        } {
            operands.swap(0, 1);
            return Expr::Relation {
                operands,
                ops: vec![converse],
            };
        }
    }
    if ops.iter().all(|o| matches!(o, Eq)) || matches!(ops.as_slice(), [Ne]) {
        operands.sort_by(cmp);
    }
    Expr::Relation { operands, ops }
}

fn factorial_of(n: i64) -> Option<Number> {
    // Cap the fold (resource_limits::current().max_factorial): canonicalization must
    // stay cheap on any user input, and `(10^12)!` would otherwise loop
    // forever. Beyond the cap the node stays an application (structural
    // equality still works).
    if !(0..=crate::resource_limits::current().max_factorial).contains(&n) {
        return None;
    }
    let mut acc = Number::one();
    for k in 2..=n {
        acc = acc.mul(&Number::Int(k));
    }
    Some(acc)
}

/// The inverse of an invertible (trig/hyperbolic) function, using the
/// normalized `a…` spelling. `None` for functions without a notated inverse.
/// (Table: `FnDef::inverse` in `crate::functions`.)
fn inverse_function_name(name: &str) -> Option<&'static str> {
    crate::functions::inverse_of(name)
}

/// Canonicalize a function head's name (`arcsin → asin`, `ln → log`, …). The
/// head is otherwise already canonical; only a bare function symbol is renamed.
fn normalize_head(head: Expr) -> Expr {
    if let Expr::Sym(s) = &head {
        if let Some(canonical) = normalize_function_name(&s.name()) {
            return Expr::sym(canonical);
        }
    }
    head
}

/// The function-name normalization table (lib/expression/normalization/
/// standard_form.js `function_normalizations`; now `FnDef::aliases` in
/// `crate::functions`).
fn normalize_function_name(name: &str) -> Option<&'static str> {
    crate::functions::canonical_name(name)
}
