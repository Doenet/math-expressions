//! Normalization (PORTING_PLAN.md §7, redesign note): a pure
//! faithful-layer → canonical-layer transform. Canonical form eliminates the
//! display-only variants (`Div`, `Neg`), flattens and sorts commutative
//! operators, folds constants *exactly* (the §3a payoff), and combines like
//! terms and like powers — so two equal canonical expressions are identical
//! trees and structural equality is tree comparison.
//!
//! `canonicalize` is confluent, cheap, and assumption-free. Heuristic
//! simplification (root pulling, trig/log identities) is a separate, deferred
//! layer that needs the assumptions system.

pub(crate) mod order;

use crate::expr::{Expr, MathConst, RelOp, SeqKind};
use crate::num::Number;

pub(crate) use order::cmp;

/// Bottom-up canonicalization: canonicalize children, then apply the smart
/// constructor for the node.
pub fn canonicalize(e: &Expr) -> Expr {
    match e {
        Expr::Num(_) | Expr::Sym(_) | Expr::Const(_) | Expr::Blank | Expr::Ldots => e.clone(),

        Expr::Add(ts) => add(ts.iter().map(canonicalize).collect()),
        Expr::Mul(fs) => mul(fs.iter().map(canonicalize).collect()),
        // Display-only variants rewrite into the algebraic core.
        Expr::Div(a, b) => mul(vec![
            canonicalize(a),
            pow(canonicalize(b), Expr::Num(Number::Int(-1))),
        ]),
        Expr::Neg(x) => mul(vec![Expr::Num(Number::Int(-1)), canonicalize(x)]),
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
            let cargs: Vec<Expr> = args.iter().map(canonicalize).collect();
            // `binom(n,k)` and the applied `nCr(n,k)` denote the same thing;
            // unify on the applied form so they compare equal.
            if name.name() == "binom" && cargs.len() == 2 {
                canon_apply(Expr::sym("nCr"), cargs)
            } else {
                Expr::OtherOp(*name, cargs)
            }
        }
    }
}

/// The three scaling units from lib/expression/units.js.
enum Unit {
    /// `$` — a `prefix` unit that only marks its value (`scale: x => x`), so it
    /// survives desugaring as a free factor.
    Dollar,
    /// `%` — `only_scales`, `scale: x => x / 100`.
    Percent,
    /// `deg` — `only_scales`, `scale: x => x * pi / 180`.
    Deg,
}

/// Match the `["unit", …]` operand layout the parsers emit: prefix `$` is
/// `[unit, value]`; postfix `%`/`deg` is `[value, unit]` (mirrors
/// `get_unit_value_of_tree` in lib/expression/units.js).
fn unit_value(args: &[Expr]) -> Option<(Unit, &Expr)> {
    if args.len() != 2 {
        return None;
    }
    if let Expr::Sym(s) = &args[0] {
        if s.name() == "$" {
            return Some((Unit::Dollar, &args[1]));
        }
    }
    if let Expr::Sym(s) = &args[1] {
        match s.name().as_str() {
            "%" => return Some((Unit::Percent, &args[0])),
            "deg" => return Some((Unit::Deg, &args[0])),
            _ => {}
        }
    }
    None
}

/// Rewrite scaling-unit nodes into plain arithmetic. This is the equality-time
/// analogue of JS `remove_scaling_units` (lib/expression/simplify.js) combined
/// with numerical unit removal:
///
/// - `n %`   → `n / 100`
/// - `n deg` → `n * pi / 180`
/// - `$ n`   → `$ * n`  (the `$` becomes an ordinary factor)
///
/// Making `$` a plain multiplication by the symbol `$` is what preserves the JS
/// semantics with no special-casing downstream: the like-term folding in [`add`]
/// then gives `$3 + $2 → $5`, while the numerical stage samples `$` as a free
/// variable, so `$5` never equals a bare `5`. It is applied only in the full
/// [`equals`](crate::equals) path — never in `equalsViaSyntax` — so `50%` and
/// `1/2` stay *syntactically* distinct even though they are numerically equal.
pub fn desugar_units(e: &Expr) -> Expr {
    match e {
        Expr::OtherOp(name, args) if name.name() == "unit" => match unit_value(args) {
            Some((Unit::Dollar, v)) => Expr::Mul(vec![Expr::sym("$"), desugar_units(v)]),
            Some((Unit::Percent, v)) => {
                Expr::Div(Box::new(desugar_units(v)), Box::new(Expr::int(100)))
            }
            Some((Unit::Deg, v)) => Expr::Div(
                Box::new(Expr::Mul(vec![
                    desugar_units(v),
                    Expr::Const(MathConst::Pi),
                ])),
                Box::new(Expr::int(180)),
            ),
            // An `OtherOp("unit", …)` that does not match a known unit shape is
            // left structurally intact (recurse into its operands).
            None => Expr::OtherOp(*name, args.iter().map(desugar_units).collect()),
        },

        Expr::Num(_) | Expr::Sym(_) | Expr::Const(_) | Expr::Blank | Expr::Ldots => e.clone(),

        Expr::Add(xs) => Expr::Add(xs.iter().map(desugar_units).collect()),
        Expr::Mul(xs) => Expr::Mul(xs.iter().map(desugar_units).collect()),
        Expr::And(xs) => Expr::And(xs.iter().map(desugar_units).collect()),
        Expr::Or(xs) => Expr::Or(xs.iter().map(desugar_units).collect()),
        Expr::Union(xs) => Expr::Union(xs.iter().map(desugar_units).collect()),
        Expr::Intersect(xs) => Expr::Intersect(xs.iter().map(desugar_units).collect()),

        Expr::Div(a, b) => Expr::Div(Box::new(desugar_units(a)), Box::new(desugar_units(b))),
        Expr::Pow(a, b) => Expr::Pow(Box::new(desugar_units(a)), Box::new(desugar_units(b))),
        Expr::Index(a, b) => Expr::Index(Box::new(desugar_units(a)), Box::new(desugar_units(b))),
        Expr::Neg(x) => Expr::Neg(Box::new(desugar_units(x))),
        Expr::Not(x) => Expr::Not(Box::new(desugar_units(x))),
        Expr::Prime(x) => Expr::Prime(Box::new(desugar_units(x))),

        Expr::Apply(h, xs) => Expr::Apply(
            Box::new(desugar_units(h)),
            xs.iter().map(desugar_units).collect(),
        ),
        Expr::Seq(k, xs) => Expr::Seq(*k, xs.iter().map(desugar_units).collect()),
        Expr::Interval { endpoints, closed } => Expr::Interval {
            endpoints: Box::new((desugar_units(&endpoints.0), desugar_units(&endpoints.1))),
            closed: *closed,
        },
        Expr::Relation { operands, ops } => Expr::Relation {
            operands: operands.iter().map(desugar_units).collect(),
            ops: ops.clone(),
        },
        Expr::Matrix {
            rows,
            cols,
            entries,
        } => Expr::Matrix {
            rows: *rows,
            cols: *cols,
            entries: entries.iter().map(desugar_units).collect(),
        },
        Expr::OtherOp(name, args) => Expr::OtherOp(*name, args.iter().map(desugar_units).collect()),
    }
}

// ---- Smart constructors (assume children are already canonical) ----

/// Build a canonical sum from canonical terms: flatten, fold the numeric part
/// exactly, combine like terms (`3x + 2x → 5x`), drop zeros, sort.
pub(crate) fn add(terms: Vec<Expr>) -> Expr {
    let mut flat = Vec::with_capacity(terms.len());
    for t in terms {
        match t {
            Expr::Add(xs) => flat.extend(xs),
            other => flat.push(other),
        }
    }

    let mut constant = Number::zero();
    // (rest, coefficient) for each distinct non-constant term.
    let mut parts: Vec<(Expr, Number)> = Vec::new();
    for t in flat {
        let (coeff, rest) = split_coeff(t);
        match rest {
            None => constant = constant.add(&coeff),
            Some(r) => match parts.iter_mut().find(|(k, _)| *k == r) {
                Some(slot) => slot.1 = slot.1.add(&coeff),
                None => parts.push((r, coeff)),
            },
        }
    }

    let mut out = Vec::with_capacity(parts.len() + 1);
    if !constant.is_zero() {
        out.push(Expr::Num(constant));
    }
    for (rest, coeff) in parts {
        if coeff.is_zero() {
            continue;
        }
        out.push(mul(vec![Expr::Num(coeff), rest]));
    }
    out.sort_by(cmp);
    match out.len() {
        0 => Expr::Num(Number::zero()),
        1 => out.pop().unwrap(),
        _ => Expr::Add(out),
    }
}

/// Build a canonical product from canonical factors: flatten, fold the numeric
/// coefficient exactly, annihilate on zero, combine like powers
/// (`x² · x³ → x⁵`), drop ones, sort.
pub(crate) fn mul(factors: Vec<Expr>) -> Expr {
    let mut flat = Vec::with_capacity(factors.len());
    for f in factors {
        match f {
            Expr::Mul(xs) => flat.extend(xs),
            other => flat.push(other),
        }
    }

    let mut coeff = Number::one();
    // (base, summed exponent) for each distinct base.
    let mut parts: Vec<(Expr, Expr)> = Vec::new();
    for f in flat {
        if let Expr::Num(n) = &f {
            coeff = coeff.mul(n);
            continue;
        }
        let (base, exp) = split_pow(f);
        match parts.iter_mut().find(|(b, _)| *b == base) {
            Some(slot) => slot.1 = add(vec![std::mem::replace(&mut slot.1, Expr::Blank), exp]),
            None => parts.push((base, exp)),
        }
    }

    if coeff.is_zero() {
        return Expr::Num(Number::zero());
    }

    let mut out = Vec::with_capacity(parts.len() + 1);
    for (base, exp) in parts {
        match pow(base, exp) {
            // A folded power may collapse to a number (e.g. exponent 0 → 1).
            Expr::Num(n) => coeff = coeff.mul(&n),
            other => out.push(other),
        }
    }
    if coeff.is_zero() {
        return Expr::Num(Number::zero());
    }
    out.sort_by(cmp);
    if out.is_empty() {
        return Expr::Num(coeff);
    }
    // The numeric coefficient sorts first (Num has the lowest rank).
    if !coeff.is_one() {
        out.insert(0, Expr::Num(coeff));
    }
    if out.len() == 1 {
        out.pop().unwrap()
    } else {
        Expr::Mul(out)
    }
}

/// Build a canonical power, applying the identities and constant folding that
/// hold without assumptions. `0` to a negative power is left unfolded (an
/// exact division by zero — §3.6 of the redesign note).
pub(crate) fn pow(base: Expr, exp: Expr) -> Expr {
    if let Expr::Num(e) = &exp {
        if e.is_zero() {
            return Expr::Num(Number::one()); // x^0 = 1, including 0^0
        }
        if e.is_one() {
            return base;
        }
    }
    if let Expr::Num(b) = &base {
        if b.is_one() {
            return Expr::Num(Number::one()); // 1^x = 1
        }
        // `as_int` matches only an integer exponent, the case we fold.
        if let Some(k) = as_int(&exp) {
            if let Some(v) = b.checked_pow_int(k) {
                return Expr::Num(v);
            }
            // 0^negative: fall through, stays a Pow node.
        }
    }
    Expr::Pow(Box::new(base), Box::new(exp))
}

// ---- helpers ----

/// A term split into (coefficient, remaining factor). `None` remainder means
/// the term is a pure number.
fn split_coeff(t: Expr) -> (Number, Option<Expr>) {
    match t {
        Expr::Num(n) => (n, None),
        Expr::Mul(xs) => {
            if let Some(Expr::Num(n)) = xs.first() {
                let n = n.clone();
                let rest = mul(xs[1..].to_vec());
                (n, Some(rest))
            } else {
                (Number::one(), Some(Expr::Mul(xs)))
            }
        }
        other => (Number::one(), Some(other)),
    }
}

/// A factor split into (base, exponent).
fn split_pow(f: Expr) -> (Expr, Expr) {
    match f {
        Expr::Pow(b, e) => (*b, *e),
        other => (other, Expr::Num(Number::one())),
    }
}

fn as_int(n: &Expr) -> Option<i64> {
    match n {
        Expr::Num(Number::Int(i)) => Some(*i),
        _ => None,
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

    let head = normalize_head(head);

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
    // Cap the fold: canonicalization must stay cheap on any user input, and
    // `(10^12)!` would otherwise loop forever. Beyond the cap the node stays
    // an application (structural equality still works).
    if !(0..=10_000).contains(&n) {
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
fn inverse_function_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "sin" => "asin",
        "cos" => "acos",
        "tan" => "atan",
        "sec" => "asec",
        "csc" => "acsc",
        "cot" => "acot",
        "sinh" => "asinh",
        "cosh" => "acosh",
        "tanh" => "atanh",
        "sech" => "asech",
        "csch" => "acsch",
        "coth" => "acoth",
        // Already-inverse names spelled `arc…` normalize elsewhere.
        _ => return None,
    })
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
/// standard_form.js `function_normalizations`).
fn normalize_function_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "ln" => "log",
        "arccos" => "acos",
        "arccosh" => "acosh",
        "arcsin" => "asin",
        "arcsinh" => "asinh",
        "arctan" => "atan",
        "arctanh" => "atanh",
        "arcsec" => "asec",
        "arcsech" => "asech",
        "arccsc" => "acsc",
        "arccsch" => "acsch",
        "arccot" => "acot",
        "arccoth" => "acoth",
        "cosec" => "csc",
        _ => return None,
    })
}
