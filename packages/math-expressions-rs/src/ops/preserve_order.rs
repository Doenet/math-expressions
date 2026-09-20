//! Order-preserving numeric folding: the `skip_ordering` form of
//! `me.evaluate_numbers`, backing DoenetML's `simplify="numberspreserveorder"`.

use crate::expr::{map_children, Expr, MathConst};
use crate::num::Number;

/// Fold numeric subexpressions **without reordering operands**: `1 + x + 2`
/// stays `1 + x + 2`, where the canonical [`evaluate_numbers`] gives `x + 3`.
///
/// This is a separate pass rather than the canonical one with its sort switched
/// off. In the canonical layer ordering is not a final tidy-up step, it is the
/// *definition* of structural equality — `add`/`mul` merge operands into a
/// keyed accumulator that has already discarded positional information by the
/// time any sort runs, so there is no seam to skip. What is implemented here is
/// the legacy `evaluate_numbers_sub` rule under `skip_ordering`: a number
/// combines only with an *adjacent* number, never with one reached by hopping
/// over an intervening symbolic term.
///
/// Deliberately weaker than the canonical pass in three further ways, each
/// matching legacy:
/// - like symbolic terms are never collected — `x + x` and `2x + 3x` stay as
///   written (contrast [`evaluate_numbers`], which gives `2x` and `5x`);
/// - a quotient or power folds only when *both* operands are already numeric,
///   so `x/2/3` stays as written rather than becoming `x/6`;
/// - nothing is distributed, and no term is moved between factors.
///
/// Known narrower than legacy in one place: legacy cancels a numeric factor
/// across a fraction bar when one side is a bare number and the division is
/// exact (`2x/2 → x`, `4/(2x) → 2/x`, but not `3x/2` or `6x/(3y)`). We leave
/// those as written. The results are equal-valued and compare equal, so this
/// costs some tidiness rather than correctness.
///
/// [`evaluate_numbers`]: crate::evaluate_numbers
pub fn evaluate_numbers_preserve_order(e: &Expr) -> Expr {
    // An expression with an operand missing is returned as written; see
    // `simplify_with` for why folding across a blank is not allowed. Checked
    // once here rather than inside `fold`, which recurses.
    if crate::equality::contains_blank(e) {
        return e.clone();
    }
    fold(e)
}

fn fold(e: &Expr) -> Expr {
    // Bottom-up: children fold first, so an inner `2*3` has already become `6`
    // by the time the surrounding sum looks for adjacent numbers — which is
    // what makes `1 + 2*3 + 4` reach `11` in a single pass.
    let e = map_children(e, fold);
    match e {
        Expr::Add(terms) => sum(terms),
        Expr::Mul(factors) => product(factors),
        Expr::Neg(x) => negate(*x),
        Expr::Div(a, b) => divide(*a, *b),
        Expr::Pow(b, x) => power(*b, *x),
        other => other,
    }
}

/// Merge every maximal run of *adjacent* numeric operands with `op`, leaving
/// everything else — and the order of what remains — untouched. This one
/// function is the whole order-preserving rule: `[1, x, 2]` contains no run
/// longer than one and comes back unchanged, while `[1, 2, x]` folds its
/// leading pair.
fn fold_runs(operands: Vec<Expr>, op: fn(&Number, &Number) -> Number) -> Vec<Expr> {
    let mut out: Vec<Expr> = Vec::with_capacity(operands.len());
    for t in operands {
        match (out.last_mut(), &t) {
            (Some(Expr::Num(acc)), Expr::Num(n)) => *acc = op(acc, n),
            _ => out.push(t),
        }
    }
    out
}

fn sum(terms: Vec<Expr>) -> Expr {
    let mut out = fold_runs(flatten(terms, as_add), Number::add);
    // Drop the additive identity, as legacy does (`0 + x` → `x`). Note this is
    // the one place a non-adjacent operand disappears, and it is safe precisely
    // because removing a zero cannot change what its neighbours sum to.
    out.retain(|t| !matches!(t, Expr::Num(n) if n.is_zero()));
    match out.len() {
        0 => Expr::Num(Number::zero()),
        1 => out.pop().unwrap(),
        _ => Expr::Add(out),
    }
}

fn product(factors: Vec<Expr>) -> Expr {
    let mut out = fold_runs(flatten(factors, as_mul), Number::mul);
    // A literal zero annihilates the whole product regardless of adjacency
    // (`x·0·y` → `0`), except against a factor that is provably non-finite,
    // where `0·∞` is indeterminate. The canonical `mul` decides this over
    // `(base, exp)` pairs (`is_infinite_factor`, reached through
    // `peel_nonzero_scaling`); this pass keeps trees unflattened and un-peeled,
    // so it has to look through the wrappers itself — see `is_non_finite`.
    if out.iter().any(|f| matches!(f, Expr::Num(n) if n.is_zero())) {
        return if out.iter().any(is_non_finite) {
            Expr::Const(MathConst::NaN)
        } else {
            Expr::Num(Number::zero())
        };
    }
    out.retain(|f| !matches!(f, Expr::Num(n) if n.is_one()));
    // A leading coefficient of exactly −1 spells as a negation (`−x`, not
    // `(−1)·x`), the convention the canonical `present` also uses. Any other
    // negative coefficient stays a literal, so `−9·(…)·8·(…)·(−3)` keeps its
    // `−9` rather than growing a wrapper.
    if out.len() > 1
        && matches!(out.first(), Some(Expr::Num(n)) if n.is_negative() && n.abs().is_one())
    {
        let rest = out.split_off(1);
        return Expr::Neg(Box::new(collapse(rest, Expr::Mul)));
    }
    match out.len() {
        0 => Expr::Num(Number::one()),
        1 => out.pop().unwrap(),
        _ => Expr::Mul(out),
    }
}

/// A one-element operand list is just that operand; anything longer keeps the
/// operator. (Never called with an empty list.)
fn collapse(mut operands: Vec<Expr>, op: fn(Vec<Expr>) -> Expr) -> Expr {
    if operands.len() == 1 {
        operands.pop().unwrap()
    } else {
        op(operands)
    }
}

fn negate(e: Expr) -> Expr {
    match absorb_negation(e) {
        Ok(folded) => folded,
        Err(original) => Expr::Neg(Box::new(original)),
    }
}

/// Push a negation into a leading numeric coefficient — legacy's
/// `try_evaluate_negate_number`, which is why `−2·cbrt(2x)` comes back as
/// `["*", −2, …]` rather than a `Neg` wrapped around a positive product. `Err`
/// hands the term back untouched when there is no number to absorb into (`−q`),
/// leaving the caller to keep an explicit wrapper.
fn absorb_negation(e: Expr) -> Result<Expr, Expr> {
    match e {
        Expr::Num(n) => Ok(Expr::Num(n.neg())),
        // `−(−x)` cancels rather than stacking wrappers.
        Expr::Neg(x) => Ok(*x),
        Expr::Mul(mut factors) => match factors.first_mut() {
            Some(Expr::Num(n)) => {
                *n = n.neg();
                Ok(product(factors))
            }
            _ => Err(Expr::Mul(factors)),
        },
        // Only the numerator carries the sign: `−5/e^t` is `(−5)/e^t`.
        Expr::Div(a, b) => match absorb_negation(*a) {
            Ok(a) => Ok(Expr::Div(Box::new(a), b)),
            Err(a) => Err(Expr::Div(Box::new(a), b)),
        },
        other => Err(other),
    }
}

/// Fold `a/b` only when both sides are already numeric — legacy's
/// `try_evaluate_quotient_of_numbers`, which is why `x/2/3` stays as written
/// rather than collapsing to `x/6`. A zero divisor yields the same constants
/// the canonical layer produces: `0/0` is indeterminate, anything else over
/// zero is a pole whose sign is `sign(n) · sign(d)` — a `−0` divisor flips it,
/// so `6/(−0) → −∞`.
fn divide(a: Expr, b: Expr) -> Expr {
    if let (Expr::Num(n), Expr::Num(d)) = (&a, &b) {
        if d.is_zero() {
            return Expr::Const(if n.is_zero() {
                MathConst::NaN
            } else if n.is_negative() ^ d.is_neg_zero() {
                MathConst::NegInf
            } else {
                MathConst::Inf
            });
        }
        if let Some(q) = n.checked_div(d) {
            return Expr::Num(q);
        }
    }
    Expr::Div(Box::new(a), Box::new(b))
}

/// Fold `b^x` only for a numeric base and an *integer* numeric exponent, so
/// `2^3` becomes `8` while `2^(1/2)` is left alone — legacy folds a power only
/// when the exact result is a number, and a fractional exponent generally is
/// not one.
fn power(b: Expr, x: Expr) -> Expr {
    // `1^t` is `1` whatever the exponent is — the one fold that does not need
    // a numeric exponent.
    if matches!(&b, Expr::Num(n) if n.is_one()) {
        return Expr::Num(Number::one());
    }
    if let (Expr::Num(n), Expr::Num(Number::Int(k))) = (&b, &x) {
        // `0^0` is indeterminate, and this pass has to say so itself. Every
        // other indeterminate form on this path is caught by `is_non_finite`
        // above, which looks for an infinity; `0^0` has none, so it fell
        // through to `checked_pow_int` and came back `1` — and then vanished,
        // because `x·1` is `x`. `simplify` answers `NaN` under the default
        // policy, and this is the path DoenetML's equality checking runs, so
        // the two must not disagree about it.
        if n.is_zero() && *k == 0 {
            return Expr::Const(MathConst::NaN);
        }
        if let Some(v) = n.checked_pow_int(*k) {
            return Expr::Num(v);
        }
    }
    Expr::Pow(Box::new(b), Box::new(spell_exponent(x)))
}

/// Spell a non-integer exact rational *exponent* as a quotient — `x^(3/2)`,
/// not the emitter's terminating-decimal `x^1.5`. This mirrors `present`'s
/// `present_exponent`, deliberately including its restriction to exponents:
/// elsewhere a rational stays a plain number so that exact decimal folds still
/// come out as decimals (`$9.5` is `9.5`, not `19/2`). We cannot call
/// `present_exponent` itself because its fallthrough re-sorts.
fn spell_exponent(x: Expr) -> Expr {
    if let Expr::Num(n) = &x {
        let (neg, num, den) = crate::normalize::split_number(n);
        if !den.is_one() {
            let frac = Expr::Div(Box::new(Expr::Num(num)), Box::new(Expr::Num(den)));
            return if neg { Expr::Neg(Box::new(frac)) } else { frac };
        }
    }
    x
}

/// A factor whose value is *provably* non-finite, and so blocks zero
/// annihilation. Finiteness that is merely unknown (a bare symbol, `1/x`) does
/// not count — legacy's `is_nonzero` returned a third "undefined" state there
/// and fell through to `0`.
///
/// The canonical `mul` decides this after `peel_nonzero_scaling` has already
/// flattened the product and split every factor into a `(base, exponent)` pair,
/// so `constructors::is_infinite_factor` only ever sees a bare leaf. This pass
/// deliberately does neither, to keep the operand order it was asked to
/// preserve, so it has to walk the wrappers itself: `0·(−∞)`, `0·(∞/2)`,
/// `0·(∞+1)` and `0·(0^(-1))` are every bit as indeterminate as `0·∞`, and
/// folding any of them to `0` is a wrong *number* on the `skip_ordering` path
/// DoenetML's equality checking, `MathOperators` and `Parabola` all use —
/// the failure shape `constructors::annihilate` exists to prevent.
fn is_non_finite(e: &Expr) -> bool {
    match e {
        Expr::Const(MathConst::Inf | MathConst::NegInf | MathConst::NaN | MathConst::None) => true,
        // Sign never makes a non-finite value finite.
        Expr::Neg(inner) => is_non_finite(inner),
        // A non-finite operand carries out through a sum or a product: `∞+x` is
        // `∞` or `NaN`, `∞·x` is `±∞` or `NaN`, and all of those are non-finite.
        Expr::Add(xs) | Expr::Mul(xs) => xs.iter().any(is_non_finite),
        // `∞/x` is non-finite whatever `x` is, and `x/0` is the pole that makes
        // `0·(1/0)` indeterminate. `x/∞` is `0`, so a non-finite *denominator*
        // deliberately falls through as finite.
        Expr::Div(num, den) => is_non_finite(num) || matches!(&**den, Expr::Num(n) if n.is_zero()),
        // Mirrors `constructors::is_infinite_factor`: the exponent's sign
        // decides both poles (`0^(-1)` is `±∞`) and their reciprocals
        // (`∞^(-1)` is `0`, which annihilates like any other zero).
        Expr::Pow(base, exp) => {
            let negative_exponent = is_negative_literal(exp);
            if matches!(&**base, Expr::Num(n) if n.is_zero()) {
                return negative_exponent;
            }
            is_non_finite(base) && !negative_exponent
        }
        _ => false,
    }
}

/// Is `e` a literal negative number? Written to see through the `Neg` wrappers
/// this pass preserves, which the canonical layer has already folded into the
/// number itself by the time `is_infinite_factor` runs.
fn is_negative_literal(e: &Expr) -> bool {
    let (mut e, mut negated) = (e, false);
    while let Expr::Neg(inner) = e {
        negated = !negated;
        e = inner;
    }
    match e {
        Expr::Num(n) if negated => n.is_positive(),
        Expr::Num(n) => n.is_negative(),
        _ => false,
    }
}

fn as_add(e: Expr) -> Result<Vec<Expr>, Expr> {
    match e {
        Expr::Add(xs) => Ok(xs),
        other => Err(other),
    }
}

fn as_mul(e: Expr) -> Result<Vec<Expr>, Expr> {
    match e {
        Expr::Mul(xs) => Ok(xs),
        other => Err(other),
    }
}

/// Splice same-operator children into the parent's operand list, in place.
/// Order-preserving, and load-bearing rather than cosmetic: it is what makes
/// the inner constants of `1 + (2 + x)` adjacent to the outer ones, so that
/// folds to `3 + x` while `1 + (x + 2)` correctly does not.
fn flatten(operands: Vec<Expr>, split: fn(Expr) -> Result<Vec<Expr>, Expr>) -> Vec<Expr> {
    let mut out = Vec::with_capacity(operands.len());
    for t in operands {
        match split(t) {
            Ok(xs) => out.extend(flatten(xs, split)),
            Err(other) => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TextToAst;

    /// Fold `s` and print the result the way the JS tree spells it, so the
    /// expectations below can be read straight off the legacy oracle.
    fn run(s: &str) -> String {
        let e = TextToAst::new(Default::default()).convert(s).unwrap();
        crate::expr::serde::to_js(&evaluate_numbers_preserve_order(&e)).to_string()
    }

    /// The defining property: a constant merges with an adjacent constant but
    /// never hops over a symbolic term. Every expectation here was taken from
    /// the legacy library running `evaluate_numbers({skip_ordering: true})`.
    #[test]
    fn constants_merge_only_across_adjacent_positions() {
        assert_eq!(run("1+x+2"), r#"["+",1,"x",2]"#);
        assert_eq!(run("1+2+x"), r#"["+",3,"x"]"#);
        assert_eq!(run("x+1+2"), r#"["+","x",3]"#);
        assert_eq!(run("1+x+2+3+y+4"), r#"["+",1,"x",5,"y",4]"#);
        assert_eq!(run("2*x*3"), r#"["*",2,"x",3]"#);
        assert_eq!(run("2*3*x"), r#"["*",6,"x"]"#);
        assert_eq!(run("2*(x*3)*4"), r#"["*",2,"x",12]"#);
    }

    /// Nested same-operator nodes splice in before the run scan, which is what
    /// separates `1+(2+x)` from `1+(x+2)`.
    #[test]
    fn nesting_is_flattened_before_runs_are_found() {
        assert_eq!(run("1+(2+x)"), r#"["+",3,"x"]"#);
        assert_eq!(run("1+(x+2)"), r#"["+",1,"x",2]"#);
        assert_eq!(run("1+(2+(3+x))"), r#"["+",6,"x"]"#);
        assert_eq!(run("x*(y*2)*3"), r#"["*","x","y",6]"#);
    }

    /// Identities drop and zero annihilates, exactly as legacy does.
    #[test]
    fn identities_and_annihilation() {
        assert_eq!(run("0+x"), r#""x""#);
        assert_eq!(run("x+0"), r#""x""#);
        assert_eq!(run("1*x*2"), r#"["*","x",2]"#);
        assert_eq!(run("0*x"), "0");
        assert_eq!(run("x*0*y"), "0");
        assert_eq!(run("3+0*x"), "3");
    }

    /// Like symbolic terms are *not* collected — the distinction
    /// `simplify="numbers"` is supposed to preserve.
    #[test]
    fn like_terms_are_left_alone() {
        assert_eq!(run("x+x"), r#"["+","x","x"]"#);
        assert_eq!(run("2*x+3*x"), r#"["+",["*",2,"x"],["*",3,"x"]]"#);
        assert_eq!(run("x*x"), r#"["*","x","x"]"#);
    }

    /// Quotients and powers fold only when both operands are numeric.
    #[test]
    fn quotients_and_powers_fold_only_when_fully_numeric() {
        assert_eq!(run("2^3+x"), r#"["+",8,"x"]"#);
        assert_eq!(run("2^(1/2)+x"), r#"["+",["^",2,["/",1,2]],"x"]"#);
        assert_eq!(run("x/2/3"), r#"["/",["/","x",2],3]"#);
        // A fraction of integers reads back as a fraction, matching legacy —
        // the spelling is carried on the value (`num::Spelling`), so it
        // survives the fold rather than depending on which pass produced it.
        assert_eq!(run("2/4+x"), r#"["+",["/",1,2],"x"]"#);
        assert_eq!(run("x+1/2+1/2"), r#"["+","x",1]"#);
        assert_eq!(run("2*3+4*5"), "26");
        assert_eq!(run("1+2*3+4"), "11");
    }

    /// Subtraction reaches the fold as a negative term, so `2-3+x` merges its
    /// leading pair while `1+x-2` cannot.
    #[test]
    fn negation_participates_in_runs() {
        assert_eq!(run("2-3+x"), r#"["+",-1,"x"]"#);
        assert_eq!(run("x-3-4"), r#"["+","x",-7]"#);
        assert_eq!(run("1+x-2"), r#"["+",1,"x",-2]"#);
    }

    /// A negation is absorbed by a leading numeric coefficient where there is
    /// one, and a coefficient of exactly −1 spells back as a negation.
    #[test]
    fn negation_folds_into_a_leading_coefficient() {
        assert_eq!(
            run("-2 cbrt(2x)"),
            r#"["*",-2,["apply","cbrt",["*",2,"x"]]]"#
        );
        assert_eq!(run("-5/e^t"), r#"["/",-5,["^","e","t"]]"#);
        assert_eq!(run("(-1+2-2)x"), r#"["-","x"]"#);
        // Nothing numeric to absorb into — the wrapper stays.
        assert_eq!(run("-q+2v"), r#"["+",["-","q"],["*",2,"v"]]"#);
    }

    /// A pole is a constant, not an annihilated zero (see the `0/0` case in
    /// the canonical layer).
    #[test]
    fn division_by_zero_yields_constants() {
        assert_eq!(run("1/0+x"), r#"["+",{"$":"Inf"},"x"]"#);
        assert_eq!(run("0/0+x"), r#"["+",{"$":"NaN"},"x"]"#);
        assert_eq!(run("0*(1/0)"), r#"{"$":"NaN"}"#);
    }

    /// `0^0` is the indeterminate form with no infinity in it, so nothing in
    /// the annihilation guard above catches it: it folded to `1` here and then
    /// vanished into whatever it multiplied, while `simplify` answers `NaN`.
    /// That is two different answers for one expression on the two paths
    /// DoenetML compares along.
    #[test]
    fn zero_to_the_zero_is_indeterminate_here_too() {
        assert_eq!(run("0^0"), r#"{"$":"NaN"}"#);
        // Left in place rather than collapsed, as this pass leaves every other
        // non-finite operand: the point is that it is *visible*, where `1` was
        // dropped by the `is_one` retain below and the expression read as `x`.
        assert_eq!(run("x*0^0"), r#"["*","x",{"$":"NaN"}]"#);
        assert_eq!(run("0^0+x"), r#"["+",{"$":"NaN"},"x"]"#);
        // The neighbouring powers are unaffected.
        assert_eq!(run("0^2"), "0");
        assert_eq!(run("2^0"), "1");
        // A negative exponent is not an integer power this pass folds at all,
        // so the pole stays written out; `is_non_finite` recognises it there.
        assert_eq!(run("0^(-1)"), r#"["^",0,-1]"#);
    }

    /// A non-finite factor blocks annihilation however it is *spelled*. This
    /// pass keeps the wrappers the canonical path peels off before it decides,
    /// so matching only `Expr::Const(…)` let every one of these fold to `0` —
    /// a wrong number, not a visible failure, on the `skip_ordering` path
    /// DoenetML's equality checking uses.
    #[test]
    fn a_wrapped_infinity_blocks_annihilation() {
        assert_eq!(run("0*(-infinity)"), r#"{"$":"NaN"}"#);
        assert_eq!(run("0*(-(-infinity))"), r#"{"$":"NaN"}"#);
        assert_eq!(run("x*0*(-infinity)"), r#"{"$":"NaN"}"#);
        assert_eq!(run("0*infinity^2"), r#"{"$":"NaN"}"#);
        assert_eq!(run("0*(infinity/2)"), r#"{"$":"NaN"}"#);
        assert_eq!(run("0*(infinity+1)"), r#"{"$":"NaN"}"#);
        assert_eq!(run("0*(0^(-1))"), r#"{"$":"NaN"}"#);
        assert_eq!(run("x*0*infinity^3"), r#"{"$":"NaN"}"#);
        // Still annihilates when nothing is provably non-finite: an unknown
        // symbol, and the reciprocal of an infinity, which is a plain zero.
        assert_eq!(run("0*(-x)"), "0");
        assert_eq!(run("0*infinity^(-1)"), "0");
        assert_eq!(run("0*(1/infinity)"), "0");
    }
}
