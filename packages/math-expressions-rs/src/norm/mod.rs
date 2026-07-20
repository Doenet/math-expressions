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

pub(crate) mod expand;
pub(crate) mod order;
pub(crate) mod present;
pub(crate) mod simplify;
pub(crate) mod special_values;
pub(crate) mod syntactic;

use crate::expr::{Expr, MathConst, RelOp, SeqKind};
use crate::num::Number;
use std::cmp::Ordering;

pub(crate) use expand::expand_core;
pub(crate) use order::cmp;
pub(crate) use present::present;
pub(crate) use simplify::{simplify_canonical, simplify_core};
pub use expand::expand;
pub use simplify::{simplify, simplify_logical, simplify_with};
pub use special_values::fold_special_values;
pub use syntactic::normalize_syntactic;

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

        Expr::Num(_)
        | Expr::Sym(_)
        | Expr::Const(_)
        | Expr::RootOf { .. }
        | Expr::Blank
        | Expr::Ldots => e.clone(),

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
/// exactly, combine like terms (`3x + 2x → 5x`), drop zeros, sort. Literal
/// matrices of equal dimensions fold entrywise (MATRIX_PLAN §1a); mismatched
/// dimensions (and matrix + scalar) stay as separate unevaluated terms.
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
    // Entrywise accumulation per matrix dimension: (rows, cols, per-entry terms).
    let mut mats: Vec<(u32, u32, Vec<Vec<Expr>>)> = Vec::new();
    for t in flat {
        if let Expr::Matrix {
            rows,
            cols,
            entries,
        } = t
        {
            match mats.iter_mut().find(|(r, c, _)| *r == rows && *c == cols) {
                Some((_, _, acc)) => {
                    for (slot, e) in acc.iter_mut().zip(entries) {
                        slot.push(e);
                    }
                }
                None => mats.push((rows, cols, entries.into_iter().map(|e| vec![e]).collect())),
            }
            continue;
        }
        let (coeff, rest) = split_coeff(t);
        match rest {
            None => constant = constant.add(&coeff),
            Some(r) => match parts.iter_mut().find(|(k, _)| *k == r) {
                Some(slot) => slot.1 = slot.1.add(&coeff),
                None => parts.push((r, coeff)),
            },
        }
    }

    let mut out = Vec::with_capacity(parts.len() + mats.len() + 1);
    for (rows, cols, acc) in mats {
        out.push(Expr::Matrix {
            rows,
            cols,
            entries: acc.into_iter().map(add).collect(),
        });
    }
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
///
/// Matrix factors (MATRIX_PLAN §1a) split the product into a commutative
/// scalar segment (everything below) and an **order-preserving matrix
/// segment**: adjacent dimension-compatible literal matrices fold via matrix
/// multiplication, a fully-folded product absorbs the scalar part into its
/// entries, and anything unfoldable (dimension mismatch, unevaluated matrix
/// powers) stays as `Mul([scalars…, matrices-in-order…])`.
pub(crate) fn mul(factors: Vec<Expr>) -> Expr {
    let mut flat = Vec::with_capacity(factors.len());
    for f in factors {
        match f {
            Expr::Mul(xs) => flat.extend(xs),
            other => flat.push(other),
        }
    }

    if flat.iter().any(is_matrix_valued) {
        let (scalars, matrices): (Vec<Expr>, Vec<Expr>) =
            flat.into_iter().partition(|f| !is_matrix_valued(f));
        let scalar_part = mul(scalars); // no matrices: the commutative pipeline
        // Fold adjacent compatible literal matrices, left to right.
        let mut seq: Vec<Expr> = Vec::with_capacity(matrices.len());
        for m in matrices {
            match (seq.last(), &m) {
                (Some(Expr::Matrix { .. }), Expr::Matrix { .. }) => {
                    let prev = seq.pop().unwrap();
                    match matmul_literal(&prev, &m) {
                        Some(folded) => seq.push(folded),
                        None => {
                            seq.push(prev);
                            seq.push(m);
                        }
                    }
                }
                _ => seq.push(m),
            }
        }
        // Fully folded: the scalar part distributes into the entries.
        if seq.len() == 1 {
            if let Expr::Matrix {
                rows,
                cols,
                entries,
            } = &seq[0]
            {
                if !matches!(&scalar_part, Expr::Num(n) if n.is_one()) {
                    return Expr::Matrix {
                        rows: *rows,
                        cols: *cols,
                        entries: entries
                            .iter()
                            .map(|e| mul(vec![scalar_part.clone(), e.clone()]))
                            .collect(),
                    };
                }
                return seq.pop().unwrap();
            }
        }
        let mut out = match scalar_part {
            Expr::Num(n) if n.is_one() => Vec::new(),
            Expr::Mul(xs) => xs,
            other => vec![other],
        };
        out.extend(seq);
        return match out.len() {
            1 => out.pop().unwrap(),
            _ => Expr::Mul(out),
        };
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
    let mut refold = false;
    for (base, exp) in parts {
        match pow(base, exp) {
            // A folded power may collapse to a number (e.g. exponent 0 → 1).
            Expr::Num(n) => coeff = coeff.mul(&n),
            // A combined power may come back as a *product* (the integer
            // power-of-product rule: `(x·y)^(1/2)·(x·y)^(3/2)` combines to
            // `(x·y)^2`, which distributes to `x²·y²`). Its factors must merge
            // with the others — pushing it whole would nest a Mul inside a Mul
            // and break the flat canonical invariant.
            Expr::Mul(xs) => {
                refold = true;
                out.extend(xs);
            }
            other => out.push(other),
        }
    }
    if coeff.is_zero() {
        return Expr::Num(Number::zero());
    }
    // Re-run the combining pass so distributed factors pair up with the rest
    // (e.g. an existing `x⁻²` cancels the distributed `x²`). Terminates: the
    // distribution only fires for Mul bases, and its output powers have
    // non-Mul bases, so nesting strictly decreases each round.
    if refold {
        if !coeff.is_one() {
            out.push(Expr::Num(coeff));
        }
        return mul(out);
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
    // Matrix base (MATRIX_PLAN §1a): integer k ≥ 2 on a square matrix folds by
    // binary powering, k = 0 gives the identity, k = 1 the base. Everything
    // else (negative — inverse is Layer 2 —, symbolic, non-square) stays an
    // unevaluated Pow. Ordered before the scalar fast paths: `A^0` must be I,
    // not the scalar 1.
    if let Expr::Matrix { rows, cols, .. } = &base {
        let (rows, cols) = (*rows, *cols);
        match as_int(&exp) {
            Some(1) => return base,
            Some(0) if rows == cols => return identity_matrix(rows),
            Some(k)
                if rows == cols
                    && k >= 2
                    && k <= crate::resource_limits::current().max_expand_power =>
            {
                let mut acc = identity_matrix(rows);
                let mut sq = base.clone();
                let mut k = k as u64;
                loop {
                    if k & 1 == 1 {
                        match matmul_literal(&acc, &sq) {
                            Some(m) => acc = m,
                            None => return Expr::Pow(Box::new(base), Box::new(exp)),
                        }
                    }
                    k >>= 1;
                    if k == 0 {
                        return acc;
                    }
                    match matmul_literal(&sq, &sq) {
                        Some(m) => sq = m,
                        None => return Expr::Pow(Box::new(base), Box::new(exp)),
                    }
                }
            }
            // Negative integer power of an *invertible rational* matrix folds
            // through the exact inverse (MATRIX_PLAN §1b); symbolic or
            // singular matrices keep the unevaluated Pow (the assumption-
            // gated inverse is `matrix::matrix_inverse`).
            Some(k) if rows == cols && k < 0 && k > i64::MIN => {
                if let Some(inv) = crate::matrix::invert_rational_literal(&base) {
                    return pow(inv, Expr::int(-k));
                }
                return Expr::Pow(Box::new(base), Box::new(exp));
            }
            _ => return Expr::Pow(Box::new(base), Box::new(exp)),
        }
    }
    if let Expr::Num(e) = &exp {
        if e.is_zero() {
            return Expr::Num(Number::one()); // x^0 = 1, including 0^0
        }
        if e.is_one() {
            return base;
        }
    }
    // RootOf power reduction (MATRIX_PLAN §2d): an integer exponent ≥ deg p
    // (or negative) rewrites through t^n mod p, so polynomials in an abstract
    // root always stay below deg p — `p(RootOf(p,k)) = 0` falls out of this
    // plus like-term folding.
    if matches!(base, Expr::RootOf { .. }) {
        if let Some(k) = as_int(&exp) {
            if let Some(reduced) = crate::rootof::power_reduced(&base, k) {
                return reduced;
            }
        }
    }
    // Flatten a nested power when the OUTER exponent is an integer:
    // `(b^a)^k = b^(a·k)` (repeated multiplication/division), valid for any base
    // and integer `k`. Restricting to integer `k` avoids the `(x^2)^(1/2) = |x|`
    // trap. This lets e.g. `x·(x^2)^(-1)` collapse to `x^(-1)` and removable
    // singularities like `d/dx((y/x)·x)` reduce to 0. (§7d nested-Pow flatten.)
    if let Expr::Pow(inner_base, inner_exp) = &base {
        if as_int(&exp).is_some() {
            let combined = mul(vec![(**inner_exp).clone(), exp]);
            return pow((**inner_base).clone(), combined);
        }
    }
    // Distribute an integer power over a product: `(a·b)^k = a^k·b^k` (valid for
    // any factors and integer `k`). Extracts numeric coefficients (`(2x)^(-1) =
    // x^(-1)/2`) and enables cancellations like `x·(2x)^(-1) → 1/2`.
    if let Expr::Mul(factors) = &base {
        // Not valid over a non-commutative (matrix) product: (A·B)² ≠ A²·B².
        if as_int(&exp).is_some() && !factors.iter().any(is_matrix_valued) {
            return mul(factors.iter().map(|f| pow(f.clone(), exp.clone())).collect());
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

/// Is this canonical factor matrix-valued (a literal matrix, an unevaluated
/// matrix power, or an unfoldable matrix product)? Such factors must not
/// commute past each other and are excluded from scalar-only rewrites.
pub(crate) fn is_matrix_valued(e: &Expr) -> bool {
    match e {
        Expr::Matrix { .. } => true,
        Expr::Pow(b, _) => matches!(**b, Expr::Matrix { .. }),
        Expr::Mul(fs) => fs.iter().any(is_matrix_valued),
        _ => false,
    }
}

/// The n×n identity matrix.
pub(crate) fn identity_matrix(n: u32) -> Expr {
    let entries = (0..n)
        .flat_map(|r| (0..n).map(move |c| Expr::int(i64::from(r == c))))
        .collect();
    Expr::Matrix {
        rows: n,
        cols: n,
        entries,
    }
}

/// Multiply two literal matrices symbolically (entries built with the smart
/// constructors). `None` on dimension mismatch or when the work exceeds
/// `limits.max_expand_terms` (the caller keeps the product unevaluated).
pub(crate) fn matmul_literal(a: &Expr, b: &Expr) -> Option<Expr> {
    let (
        Expr::Matrix {
            rows: r1,
            cols: c1,
            entries: ea,
        },
        Expr::Matrix {
            rows: r2,
            cols: c2,
            entries: eb,
        },
    ) = (a, b)
    else {
        return None;
    };
    if c1 != r2 {
        return None;
    }
    let (r1, c1, c2) = (*r1 as usize, *c1 as usize, *c2 as usize);
    if r1.saturating_mul(c1).saturating_mul(c2) > crate::resource_limits::current().max_expand_terms {
        return None;
    }
    let mut entries = Vec::with_capacity(r1 * c2);
    for i in 0..r1 {
        for j in 0..c2 {
            let terms = (0..c1)
                .map(|k| mul(vec![ea[i * c1 + k].clone(), eb[k * c2 + j].clone()]))
                .collect();
            entries.push(add(terms));
        }
    }
    Some(Expr::Matrix {
        rows: r1 as u32,
        cols: c2 as u32,
        entries,
    })
}

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
