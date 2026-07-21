//! Light normalization for *syntactic* equality — the port of JS
//! `equalsViaSyntax` (lib/expression/equality/syntax.js + the four
//! `normalize_*` passes in normalization/standard_form.js).
//!
//! Unlike [`canonicalize`](super::canonicalize), this deliberately does NOT
//! flatten, reorder, fold constants, combine like terms, or eliminate `Div`.
//! It only renames functions, moves powers/primes outside applications, tidies
//! negative-number placement, and sorts geometry-operator arguments — then
//! comparison is *order-sensitive* tree equality. This is the "is the answer in
//! the requested form?" check: `(x+y)+z` and `z+x+y` are NOT syntactically
//! equal, while `ln(x)` and `log(x)`, or `cos^(-1)(x)` and `arccos(x)`, are.

use crate::expr::{flatten, Expr};
use crate::num::Number;
use std::cmp::Ordering;

/// Apply the four `equalsViaSyntax` normalization passes, in JS order. Flattens
/// first: parsing is now faithful (keeps raw associative grouping), and this
/// order-sensitive comparison expects flat n-ary operators (mirrors the JS,
/// whose parser flattens before `equalsViaSyntax`).
pub fn normalize_syntactic(e: &Expr) -> Expr {
    let e = flatten(e.clone());
    let e = pass_function_names(&e);
    let e = pass_applied_functions(&e);
    let e = pass_negative_numbers(&e);
    pass_geometry_arg_order(&e)
}

// ---- Support tables (standard_form.js) ----
//
// The `function_normalizations`, `create_trig_inverses_for`, and
// `move_exponents_outside_for` tables now live on `FnDef` in
// `crate::functions` (aliases / `inverse` / `move_exponent_spellings`);
// the passes below query the registry.

// ---- Pass 1: normalize_function_names ----

fn pass_function_names(e: &Expr) -> Expr {
    match e {
        Expr::Apply(head, args) => {
            // sqrt/cbrt/nthroot rewrite to explicit powers.
            if let Expr::Sym(s) = head.as_ref() {
                match s.name().as_str() {
                    "sqrt" if args.len() == 1 => {
                        return pow(pass_function_names(&args[0]), half());
                    }
                    "cbrt" if args.len() == 1 => {
                        return pow(pass_function_names(&args[0]), one_over(Expr::int(3)));
                    }
                    "nthroot" => {
                        // JS: `nthroot(tuple(x, n))` → `x^(1/n)`; anything else is
                        // treated as a square root. Our applied form is the flat
                        // 2-arg `nthroot(x, n)`.
                        if args.len() == 2 {
                            return pow(
                                pass_function_names(&args[0]),
                                one_over(pass_function_names(&args[1])),
                            );
                        }
                        let inner = args.first().map(pass_function_names).unwrap_or(Expr::Blank);
                        return pow(inner, half());
                    }
                    _ => {}
                }
            }
            let head = normalize_head_name(head);
            let args = args.iter().map(pass_function_names).collect();
            Expr::Apply(Box::new(head), args)
        }
        // `e^x` → `exp(x)` (math.define_e defaults to true).
        Expr::Pow(base, exp) if is_sym(base, "e") => {
            Expr::Apply(Box::new(Expr::sym("exp")), vec![pass_function_names(exp)])
        }
        // `binom(n, k)` → `nCr(n, k)`.
        Expr::OtherOp(name, args) if name.name() == "binom" && args.len() == 2 => Expr::Apply(
            Box::new(Expr::sym("nCr")),
            args.iter().map(pass_function_names).collect(),
        ),
        _ => map_children(e, pass_function_names),
    }
}

/// Port of `normalize_function_names_sub`: rename a function head, turning
/// `f^(-1)` into `af` for the invertible trig/hyperbolic names.
fn normalize_head_name(head: &Expr) -> Expr {
    match head {
        Expr::Sym(s) => match crate::functions::canonical_name(&s.name()) {
            Some(canon) => Expr::sym(canon),
            None => head.clone(),
        },
        Expr::Pow(base, exp) if is_int(exp, -1) => {
            if let Expr::Sym(s) = base.as_ref() {
                if let Some(inv) = crate::functions::inverse_of(&s.name()) {
                    return Expr::sym(inv);
                }
            }
            Expr::Pow(
                Box::new(normalize_head_name(base)),
                Box::new(normalize_head_name(exp)),
            )
        }
        _ => map_children(head, normalize_head_name),
    }
}

// ---- Pass 2: normalize_applied_functions ----

fn pass_applied_functions(e: &Expr) -> Expr {
    if let Expr::Apply(head, args) = e {
        let args: Vec<Expr> = args.iter().map(pass_applied_functions).collect();
        match head.as_ref() {
            // Applied power: `f^n(x)` → `(f(x))^n` when n ≠ -1 and `f` is one of
            // the functions whose exponent conventionally sits outside.
            Expr::Pow(base, exp) => {
                if !is_int(exp, -1) && is_move_exponent(base) {
                    return Expr::Pow(Box::new(Expr::Apply(base.clone(), args)), exp.clone());
                }
                return Expr::Apply(Box::new(Expr::Pow(base.clone(), exp.clone())), args);
            }
            // Applied primes: `f''(x)` → `(f(x))''` — primes migrate outside.
            Expr::Prime(_) => {
                let (base, nprimes) = strip_primes(head);
                let mut out = Expr::Apply(Box::new(base), args);
                for _ in 0..nprimes {
                    out = Expr::Prime(Box::new(out));
                }
                return out;
            }
            _ => return Expr::Apply(head.clone(), args),
        }
    }
    map_children(e, pass_applied_functions)
}

fn strip_primes(mut head: &Expr) -> (Expr, usize) {
    let mut n = 0;
    while let Expr::Prime(inner) = head {
        n += 1;
        head = inner;
    }
    (head.clone(), n)
}

// ---- Pass 3: normalize_negative_numbers ----

fn pass_negative_numbers(e: &Expr) -> Expr {
    if let Expr::Neg(inner) = e {
        match inner.as_ref() {
            // `-(3)` → `-3`
            Expr::Num(n) if !n.is_negative() => return Expr::Num(n.neg()),
            // `-(3*x)` → `(-3)*x`
            Expr::Mul(factors) if !factors.is_empty() => {
                if let Some(first) = negate_leading(&factors[0]) {
                    let mut out = vec![first];
                    out.extend(factors[1..].iter().map(pass_negative_numbers));
                    return Expr::Mul(out);
                }
            }
            // `-(3/y)` → `(-3)/y`
            Expr::Div(num, den) => {
                if let Some(neg_num) = negate_leading(num) {
                    return Expr::Div(Box::new(neg_num), Box::new(pass_negative_numbers(den)));
                }
            }
            _ => {}
        }
        return Expr::Neg(Box::new(pass_negative_numbers(inner)));
    }
    map_children(e, pass_negative_numbers)
}

/// Port of `negate_leading_positive_number`: if `node` begins with a
/// non-negative number, return the node with that number negated (recursing
/// into `*`/`/` leaders); otherwise `None`.
fn negate_leading(node: &Expr) -> Option<Expr> {
    match node {
        Expr::Num(n) if !n.is_negative() => Some(Expr::Num(n.neg())),
        Expr::Mul(factors) if !factors.is_empty() => {
            if let Expr::Num(n) = &factors[0] {
                if !n.is_negative() {
                    let mut out = vec![Expr::Num(n.neg())];
                    out.extend(factors[1..].iter().map(pass_negative_numbers));
                    return Some(Expr::Mul(out));
                }
            }
            None
        }
        Expr::Div(num, den) => negate_leading(num)
            .map(|neg_num| Expr::Div(Box::new(neg_num), Box::new(pass_negative_numbers(den)))),
        _ => None,
    }
}

// ---- Pass 4: normalize_angle_linesegment_arg_order ----

fn pass_geometry_arg_order(e: &Expr) -> Expr {
    if let Expr::OtherOp(name, args) = e {
        // `angle(A,B,C)` is unoriented: reverse to `angle(C,B,A)` when out of
        // order (endpoints compared, vertex `B` fixed in the middle).
        if name.name() == "angle" && args.len() == 3 {
            let mut a: Vec<Expr> = args.iter().map(pass_geometry_arg_order).collect();
            if super::cmp(&a[0], &a[2]) == Ordering::Greater {
                a.reverse();
            }
            return Expr::OtherOp(*name, a);
        }
        // `linesegment(A,B) = linesegment(B,A)`.
        if name.name() == "linesegment" && args.len() == 2 {
            let mut a: Vec<Expr> = args.iter().map(pass_geometry_arg_order).collect();
            if super::cmp(&a[0], &a[1]) == Ordering::Greater {
                a.reverse();
            }
            return Expr::OtherOp(*name, a);
        }
    }
    map_children(e, pass_geometry_arg_order)
}

// ---- Small builders / predicates ----

fn half() -> Expr {
    Expr::Num(Number::rat(1, 2))
}

fn one_over(n: Expr) -> Expr {
    Expr::Div(Box::new(Expr::int(1)), Box::new(n))
}

fn pow(base: Expr, exp: Expr) -> Expr {
    Expr::Pow(Box::new(base), Box::new(exp))
}

fn is_sym(e: &Expr, name: &str) -> bool {
    matches!(e, Expr::Sym(s) if s.name() == name)
}

fn is_int(e: &Expr, v: i64) -> bool {
    matches!(e, Expr::Num(n) if *n == Number::Int(v))
}

fn is_move_exponent(base: &Expr) -> bool {
    matches!(base, Expr::Sym(s) if crate::functions::moves_exponent_outside(&s.name()))
}

/// Apply `f` to every immediate `Expr` child, rebuilding the node; leaves are
/// returned unchanged. Shared by the syntactic passes and `norm::simplify`
/// (generic over `FnMut` so callers can thread state, e.g. a change flag).
pub(crate) fn map_children<F: FnMut(&Expr) -> Expr>(e: &Expr, mut f: F) -> Expr {
    match e {
        Expr::Num(_)
        | Expr::Sym(_)
        | Expr::Const(_)
        | Expr::RootOf { .. }
        | Expr::Blank
        | Expr::Ldots => e.clone(),
        Expr::Add(xs) => Expr::Add(xs.iter().map(&mut f).collect()),
        Expr::Mul(xs) => Expr::Mul(xs.iter().map(&mut f).collect()),
        Expr::And(xs) => Expr::And(xs.iter().map(&mut f).collect()),
        Expr::Or(xs) => Expr::Or(xs.iter().map(&mut f).collect()),
        Expr::Union(xs) => Expr::Union(xs.iter().map(&mut f).collect()),
        Expr::Intersect(xs) => Expr::Intersect(xs.iter().map(&mut f).collect()),
        Expr::Div(a, b) => Expr::Div(Box::new(f(a)), Box::new(f(b))),
        Expr::Pow(a, b) => Expr::Pow(Box::new(f(a)), Box::new(f(b))),
        Expr::Index(a, b) => Expr::Index(Box::new(f(a)), Box::new(f(b))),
        Expr::Neg(x) => Expr::Neg(Box::new(f(x))),
        Expr::Not(x) => Expr::Not(Box::new(f(x))),
        Expr::Prime(x) => Expr::Prime(Box::new(f(x))),
        Expr::Apply(h, xs) => {
            let h = f(h);
            Expr::Apply(Box::new(h), xs.iter().map(&mut f).collect())
        }
        Expr::Seq(k, xs) => Expr::Seq(*k, xs.iter().map(&mut f).collect()),
        Expr::Interval { endpoints, closed } => Expr::Interval {
            endpoints: Box::new((f(&endpoints.0), f(&endpoints.1))),
            closed: *closed,
        },
        Expr::Relation { operands, ops } => Expr::Relation {
            operands: operands.iter().map(&mut f).collect(),
            ops: ops.clone(),
        },
        Expr::Matrix {
            rows,
            cols,
            entries,
        } => Expr::Matrix {
            rows: *rows,
            cols: *cols,
            entries: entries.iter().map(&mut f).collect(),
        },
        Expr::OtherOp(name, xs) => Expr::OtherOp(*name, xs.iter().map(&mut f).collect()),
    }
}
