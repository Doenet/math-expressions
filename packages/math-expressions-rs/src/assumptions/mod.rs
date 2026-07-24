//! Assumptions: variable facts and three-valued
//! sign/set inference.
//!
//! Storage mirrors the JS `initialize_assumptions` shape: per-variable facts
//! (`by_var`) added via [`Assumptions::add`], retrieved with
//! [`Assumptions::get`], removed with [`Assumptions::remove`]. A fact is a
//! canonical relation `Expr` (`x > 0`, `n ∈ Z`, `x ≠ 0`, `x = 3`, chains split
//! on `And`). Generic assumptions (JS `add_generic_assumption`) are not
//! ported.
//!
//! Queries are the eight three-valued predicates of
//! `lib/assumptions/element_of_sets.js` — `is_integer`, `is_real`,
//! `is_complex`, `is_nonzero`, `is_nonnegative`, `is_positive`, `is_negative`,
//! `is_nonpositive` — returning `Some(true)` / `Some(false)` / `None`
//! (unknown), the JS `true/false/undefined`. The inference is a clean-slate
//! bottom-up pass over the canonical tree (the JS is ~2100 lines of AST
//! matching); behaviour is validated against the JS oracle by the assumptions
//! corpus. Deliberately mirrored JS conservatisms: an unassumed variable is
//! fully unknown (no default-real), odd powers of negatives get no sign
//! (`x³ | x<0` → unknown), and sums do no interval arithmetic
//! (`x−3 | x>4` → unknown sign).

use crate::expr::{Expr, MathConst, RelOp};
use crate::norm::canonicalize;
use crate::num::Number;
use std::collections::HashMap;

/// Three-valued logic: `Some(true)` / `Some(false)` / `None` = unknown.
pub type Tri = Option<bool>;

/// Per-variable assumption store, plus generic assumptions: patterns in the
/// designated variable `x` that apply to every variable with no specific
/// facts (JS `add_generic_assumption`).
#[derive(Debug, Clone, Default)]
pub struct Assumptions {
    by_var: HashMap<String, Vec<Expr>>,
    generic: Vec<Expr>,
}

impl Assumptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an assumption (a relation, or an `And` of relations, in any parse
    /// form). Each conjunct is canonicalized and filed under every variable it
    /// mentions.
    pub fn add(&mut self, assumption: &Expr) {
        let canon = canonicalize(assumption);
        for conjunct in conjuncts(&canon) {
            let mut vars = std::collections::BTreeSet::new();
            crate::eval::free_symbols(conjunct, &mut vars);
            for v in vars {
                self.by_var.entry(v).or_default().push(conjunct.clone());
            }
        }
    }

    /// All facts mentioning `var`, combined with `And` (or the single fact),
    /// mirroring `get_assumptions`. `None` when nothing is known.
    pub fn get(&self, var: &str) -> Option<Expr> {
        let facts = self.by_var.get(var)?;
        match facts.as_slice() {
            [] => None,
            [one] => Some(one.clone()),
            many => Some(Expr::And(many.to_vec())),
        }
    }

    /// Remove a previously-added assumption (structural equality on the
    /// canonical form), from every variable it was filed under.
    pub fn remove(&mut self, assumption: &Expr) {
        let canon = canonicalize(assumption);
        for conjunct in conjuncts(&canon) {
            for facts in self.by_var.values_mut() {
                facts.retain(|f| f != conjunct);
            }
        }
        self.by_var.retain(|_, v| !v.is_empty());
    }

    pub fn clear(&mut self) {
        self.by_var.clear();
        self.generic.clear();
    }

    /// No facts stored at all?
    pub fn is_empty(&self) -> bool {
        self.by_var.is_empty() && self.generic.is_empty()
    }

    /// Add a generic assumption: a pattern in the variable `x` applied to any
    /// variable without specific facts (`x > 0` ⇒ every unassumed variable is
    /// positive). Conjuncts not mentioning `x` are ignored (JS parity).
    pub fn add_generic(&mut self, assumption: &Expr) {
        let canon = canonicalize(assumption);
        for conjunct in conjuncts(&canon) {
            let mut vars = std::collections::BTreeSet::new();
            crate::eval::free_symbols(conjunct, &mut vars);
            if vars.contains("x") {
                self.generic.push(conjunct.clone());
            }
        }
    }

    /// Remove a generic assumption added with [`Assumptions::add_generic`].
    pub fn remove_generic(&mut self, assumption: &Expr) {
        let canon = canonicalize(assumption);
        for conjunct in conjuncts(&canon) {
            self.generic.retain(|f| f != conjunct);
        }
    }

    /// The facts in effect for `var`: its specific facts, or — when none exist
    /// — the generic patterns with `x` substituted by `var` (unless the
    /// pattern itself mentions `var` as a different symbol, JS parity).
    fn facts_for(&self, var: &str) -> Vec<Expr> {
        if let Some(facts) = self.by_var.get(var) {
            return facts.clone();
        }
        if self.generic.is_empty() {
            return Vec::new();
        }
        let subs = HashMap::from([("x".to_string(), Expr::sym(var))]);
        self.generic
            .iter()
            .filter(|f| {
                if var == "x" {
                    return true;
                }
                let mut vs = std::collections::BTreeSet::new();
                crate::eval::free_symbols(f, &mut vs);
                !vs.contains(var)
            })
            .map(|f| canonicalize(&crate::ops::substitute(f, &subs)))
            .collect()
    }
}

fn conjuncts(e: &Expr) -> Vec<&Expr> {
    match e {
        Expr::And(xs) => xs.iter().flat_map(conjuncts).collect(),
        other => vec![other],
    }
}

// ---- the eight queries ----

pub fn is_integer(e: &Expr, a: &Assumptions) -> Tri {
    facts(&canonicalize(e), a).integer
}
pub fn is_real(e: &Expr, a: &Assumptions) -> Tri {
    facts(&canonicalize(e), a).real
}
pub fn is_complex(e: &Expr, a: &Assumptions) -> Tri {
    facts(&canonicalize(e), a).complex
}
pub fn is_nonzero(e: &Expr, a: &Assumptions) -> Tri {
    facts(&canonicalize(e), a).nonzero
}
pub fn is_nonnegative(e: &Expr, a: &Assumptions) -> Tri {
    facts(&canonicalize(e), a).nonneg
}
pub fn is_positive(e: &Expr, a: &Assumptions) -> Tri {
    facts(&canonicalize(e), a).positive
}
pub fn is_negative(e: &Expr, a: &Assumptions) -> Tri {
    facts(&canonicalize(e), a).negative
}
pub fn is_nonpositive(e: &Expr, a: &Assumptions) -> Tri {
    facts(&canonicalize(e), a).nonpos
}

/// What is known about one (sub)expression. Every field is three-valued.
#[derive(Debug, Clone, Copy, Default)]
struct Facts {
    integer: Tri,
    real: Tri,
    complex: Tri,
    nonzero: Tri,
    nonneg: Tri,
    positive: Tri,
    negative: Tri,
    nonpos: Tri,
}

impl Facts {
    /// Everything unknown.
    fn unknown() -> Facts {
        Facts::default()
    }

    /// Facts of an exact number.
    fn of_number(n: &Number) -> Facts {
        let v = n.to_f64();
        if v.is_nan() {
            return Facts::unknown();
        }
        let is_int = match n {
            Number::Int(_) => true,
            Number::Rat(..) => false,
            Number::Big(_) => n.magnitude_log10().is_some() && is_big_int(n),
            Number::Float(_) => v.fract() == 0.0,
        };
        Facts {
            integer: Some(is_int),
            real: Some(true),
            complex: Some(true),
            nonzero: Some(v != 0.0),
            nonneg: Some(v >= 0.0),
            positive: Some(v > 0.0),
            negative: Some(v < 0.0),
            nonpos: Some(v <= 0.0),
        }
    }

    /// A real, positive, non-integer constant (`pi`, `e`).
    fn positive_transcendental() -> Facts {
        Facts {
            integer: Some(false),
            real: Some(true),
            complex: Some(true),
            nonzero: Some(true),
            nonneg: Some(true),
            positive: Some(true),
            negative: Some(false),
            nonpos: Some(false),
        }
    }

    /// The imaginary unit: complex, not real; sign predicates are all false
    /// (JS reports F, not undefined, for `i`).
    fn imaginary_unit() -> Facts {
        Facts {
            integer: Some(false),
            real: Some(false),
            complex: Some(true),
            nonzero: Some(true),
            nonneg: Some(false),
            positive: Some(false),
            negative: Some(false),
            nonpos: Some(false),
        }
    }
}

fn is_big_int(n: &Number) -> bool {
    matches!(n, Number::Big(b) if matches!(&**b, crate::num::BigNumber::Int(_)))
}

/// Bottom-up fact inference over a canonical expression.
fn facts(e: &Expr, a: &Assumptions) -> Facts {
    match e {
        Expr::Num(n) => Facts::of_number(n),
        Expr::Const(MathConst::Pi | MathConst::E) => Facts::positive_transcendental(),
        Expr::Const(MathConst::I) => Facts::imaginary_unit(),
        Expr::Const(_) => Facts::unknown(),

        Expr::Sym(s) => match s.name().as_str() {
            "pi" | "e" => Facts::positive_transcendental(),
            "i" => Facts::imaginary_unit(),
            name => variable_facts(name, a),
        },

        Expr::Add(ts) => {
            let fs: Vec<Facts> = ts.iter().map(|t| facts(t, a)).collect();
            combine_add(&fs)
        }
        Expr::Mul(fs_) => {
            let fs: Vec<Facts> = fs_.iter().map(|t| facts(t, a)).collect();
            combine_mul(&fs)
        }
        Expr::Pow(b, x) => combine_pow(&facts(b, a), x, a),
        Expr::Apply(head, args) => apply_facts(head, args, a),

        _ => Facts::unknown(),
    }
}

/// Facts about a bare variable, derived from its stored assumptions.
fn variable_facts(name: &str, a: &Assumptions) -> Facts {
    let mut out = Facts::unknown();
    for fact in &a.facts_for(name) {
        let Expr::Relation { operands, ops } = fact else {
            continue;
        };
        let ([lhs, rhs], [op]) = (operands.as_slice(), ops.as_slice()) else {
            continue;
        };
        // Which side is the variable, which the bound? (Canonicalization
        // rewrites `>`/`≥` to `<`/`≤` with swapped operands, and sorts `=`.)
        let (var_on_left, other) = match (lhs, rhs) {
            (Expr::Sym(s), o) if s.name() == name => (true, o),
            (o, Expr::Sym(s)) if s.name() == name => (false, o),
            _ => continue, // compound fact (e.g. x + y < 2): no direct bound
        };

        match op {
            RelOp::Eq => {
                // Known value: adopt the literal's own facts wholesale.
                if let Expr::Num(n) = other {
                    return Facts::of_number(n);
                }
            }
            RelOp::Ne => {
                if matches!(other, Expr::Num(n) if n.is_zero()) {
                    out.nonzero = Some(true);
                }
            }
            RelOp::Lt | RelOp::Le => {
                let strict = matches!(op, RelOp::Lt);
                let Expr::Num(n) = other else { continue };
                let c = n.to_f64();
                // A one-sided real bound implies realness.
                out.real = Some(true);
                out.complex = Some(true);
                if var_on_left {
                    // name < c (or ≤): upper bound.
                    if c < 0.0 || (c == 0.0 && strict) {
                        out.negative = Some(true);
                        out.nonpos = Some(true);
                        out.nonzero = Some(true);
                        out.positive = Some(false);
                        out.nonneg = Some(false);
                    } else if c == 0.0 {
                        out.nonpos = Some(true);
                        out.positive = Some(false);
                    }
                } else {
                    // c < name (or ≤): lower bound.
                    if c > 0.0 || (c == 0.0 && strict) {
                        out.positive = Some(true);
                        out.nonneg = Some(true);
                        out.nonzero = Some(true);
                        out.negative = Some(false);
                        out.nonpos = Some(false);
                    } else if c == 0.0 {
                        out.nonneg = Some(true);
                        out.negative = Some(false);
                    }
                }
            }
            RelOp::In
                // `name ∈ Z / Q / R / C` (the JS set names).
                if var_on_left => {
                    if let Expr::Sym(set) = other {
                        match set.name().as_str() {
                            "Z" => {
                                out.integer = Some(true);
                                out.real = Some(true);
                                out.complex = Some(true);
                            }
                            "Q" | "R" => {
                                out.real = Some(true);
                                out.complex = Some(true);
                            }
                            "C" => out.complex = Some(true),
                            _ => {}
                        }
                    }
                }
            _ => {}
        }
    }
    out
}

/// Three-valued "all of them": T if every entry is T, F never inferred here,
/// U otherwise.
fn all(fs: &[Facts], get: impl Fn(&Facts) -> Tri) -> Tri {
    if fs.iter().all(|f| get(f) == Some(true)) {
        Some(true)
    } else {
        None
    }
}

fn combine_add(fs: &[Facts]) -> Facts {
    let mut out = Facts::unknown();
    out.integer = all(fs, |f| f.integer);
    // Exactly one definitely-non-integer term among integers → not integer
    // (`3 + π`); two or more non-integers could cancel, so stay unknown.
    if out.integer.is_none()
        && fs.iter().filter(|f| f.integer == Some(false)).count() == 1
        && fs.iter().all(|f| f.integer.is_some())
    {
        out.integer = Some(false);
    }
    out.real = all(fs, |f| f.real);
    out.complex = all(fs, |f| f.complex);

    // Sign: only from uniform term signs (no interval arithmetic, like JS).
    let all_nonneg = fs.iter().all(|f| f.nonneg == Some(true));
    let all_nonpos = fs.iter().all(|f| f.nonpos == Some(true));
    let any_pos = fs.iter().any(|f| f.positive == Some(true));
    let any_neg = fs.iter().any(|f| f.negative == Some(true));
    if all_nonneg {
        out.nonneg = Some(true);
        out.negative = Some(false);
        if any_pos {
            out.positive = Some(true);
            out.nonzero = Some(true);
            out.nonpos = Some(false);
        }
    }
    if all_nonpos {
        out.nonpos = Some(true);
        out.positive = Some(false);
        if any_neg {
            out.negative = Some(true);
            out.nonzero = Some(true);
            out.nonneg = Some(false);
        }
    }
    out
}

fn combine_mul(fs: &[Facts]) -> Facts {
    let mut out = Facts::unknown();
    out.integer = all(fs, |f| f.integer);
    out.real = all(fs, |f| f.real);
    out.complex = all(fs, |f| f.complex);
    // Nonzero is multiplicative in any field — no realness required
    // (`x ≠ 0` alone makes `2x`, `x·x` nonzero).
    if fs.iter().all(|f| f.nonzero == Some(true)) {
        out.nonzero = Some(true);
    }

    // Definite sign only when every factor is real with a definite strict-or-
    // zero-allowed sign.
    if out.real == Some(true) {
        let mut sign_known = true;
        let mut negatives = 0usize;
        let mut may_be_zero = false;
        for f in fs {
            if f.positive == Some(true) {
                // positive factor: no change
            } else if f.negative == Some(true) {
                negatives += 1;
            } else if f.nonneg == Some(true) {
                may_be_zero = true;
            } else if f.nonpos == Some(true) {
                negatives += 1;
                may_be_zero = true;
            } else {
                sign_known = false;
                break;
            }
        }
        if sign_known {
            let positive_product = negatives.is_multiple_of(2);
            if positive_product {
                out.nonneg = Some(true);
                out.negative = Some(false);
                if !may_be_zero {
                    out.positive = Some(true);
                    out.nonzero = Some(true);
                    out.nonpos = Some(false);
                }
            } else {
                out.nonpos = Some(true);
                out.positive = Some(false);
                if !may_be_zero {
                    out.negative = Some(true);
                    out.nonzero = Some(true);
                    out.nonneg = Some(false);
                }
            }
        }
    }
    out
}

fn combine_pow(base: &Facts, exp: &Expr, a: &Assumptions) -> Facts {
    let ef = facts(exp, a);
    let mut out = Facts::unknown();

    // Integer exponent as a literal (the only case with parity information).
    let lit = match exp {
        Expr::Num(Number::Int(k)) => Some(*k),
        _ => None,
    };

    // b^k for integer k: real base stays real (nonzero if k may be negative);
    // integer base with k ≥ 0 stays integer.
    if let Some(k) = lit {
        if base.real == Some(true) && (k >= 0 || base.nonzero == Some(true)) {
            out.real = Some(true);
            out.complex = Some(true);
        }
        if base.integer == Some(true) && k >= 0 {
            out.integer = Some(true);
        }
        if base.nonzero == Some(true) {
            out.nonzero = Some(true);
        }
        // Reciprocal of a definite-sign real: sign carries through a negative
        // odd exponent (JS reaches this via its division rule; positive bases
        // are covered below). Positive odd exponents deliberately stay
        // unknown for negative bases, matching the JS Pow path.
        if k < 0 && k % 2 != 0 && base.negative == Some(true) {
            out.real = Some(true);
            out.complex = Some(true);
            out.negative = Some(true);
            out.nonpos = Some(true);
            out.nonzero = Some(true);
            out.positive = Some(false);
            out.nonneg = Some(false);
        }
        // Odd positive power of a nonnegative real stays nonnegative (JS
        // infers this, though not the negative-base analogue).
        if k > 0 && k % 2 != 0 && base.nonneg == Some(true) && base.real == Some(true) {
            out.nonneg = Some(true);
            out.negative = Some(false);
        }
        // Even power of a real: nonnegative; positive iff base nonzero.
        // (Odd powers of negatives deliberately stay unknown, like JS.)
        if k != 0 && k % 2 == 0 && base.real == Some(true) {
            out.nonneg = Some(true);
            out.negative = Some(false);
            if base.nonzero == Some(true) {
                out.positive = Some(true);
                out.nonzero = Some(true);
                out.nonpos = Some(false);
            }
        }
    }

    // Positive real base: positive for any real exponent (covers 1/x, sqrt
    // as x^(1/2), and symbolic real exponents).
    if base.positive == Some(true) && (ef.real == Some(true) || lit.is_some() || is_real_exponent_shape(exp))
    {
        out.real = Some(true);
        out.complex = Some(true);
        out.positive = Some(true);
        out.nonneg = Some(true);
        out.nonzero = Some(true);
        out.negative = Some(false);
        out.nonpos = Some(false);
    }
    out
}

/// A numeric (rational) exponent — real by construction even though it is not
/// an integer literal (e.g. the `1/2` in `sqrt` written as a power).
fn is_real_exponent_shape(e: &Expr) -> bool {
    matches!(e, Expr::Num(n) if !n.to_f64().is_nan())
}

fn apply_facts(head: &Expr, args: &[Expr], a: &Assumptions) -> Facts {
    let (Expr::Sym(f), [arg]) = (head, args) else {
        return Facts::unknown();
    };
    let af = facts(arg, a);
    let mut out = Facts::unknown();
    match f.name().as_str() {
        "abs" => {
            // |z| ≠ 0 iff z ≠ 0, in any field.
            if af.nonzero == Some(true) {
                out.nonzero = Some(true);
            }
            if af.real == Some(true) {
                out.real = Some(true);
                out.complex = Some(true);
                out.nonneg = Some(true);
                out.negative = Some(false);
                if af.nonzero == Some(true) {
                    out.positive = Some(true);
                    out.nonzero = Some(true);
                    out.nonpos = Some(false);
                }
            }
        }
        "exp" => {
            if af.real == Some(true) {
                out.real = Some(true);
                out.complex = Some(true);
                out.positive = Some(true);
                out.nonneg = Some(true);
                out.nonzero = Some(true);
                out.negative = Some(false);
                out.nonpos = Some(false);
            }
        }
        "sqrt" => {
            if af.positive == Some(true) {
                out.real = Some(true);
                out.complex = Some(true);
                out.positive = Some(true);
                out.nonneg = Some(true);
                out.nonzero = Some(true);
                out.negative = Some(false);
                out.nonpos = Some(false);
            } else if af.nonneg == Some(true) && af.real == Some(true) {
                out.real = Some(true);
                out.complex = Some(true);
                out.nonneg = Some(true);
                out.negative = Some(false);
            } else if af.real == Some(true) {
                // √ of any real is a complex number.
                out.complex = Some(true);
            }
        }
        "sin" | "cos" | "tan" => {
            if af.real == Some(true) {
                out.real = Some(true);
                out.complex = Some(true);
            }
        }
        // Both spellings: `log` is canonical but the registry's builders and
        // un-normalized user trees say `ln` (see functions/exp_log.rs).
        "log" | "ln" => {
            if af.positive == Some(true) {
                out.real = Some(true);
                out.complex = Some(true);
            } else if af.nonzero == Some(true) || af.real == Some(true) {
                // log of a nonzero (or any real, matching JS's looseness even
                // at 0) is complex.
                out.complex = Some(true);
            }
        }
        _ => {}
    }
    out
}
