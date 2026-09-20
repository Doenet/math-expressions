//! Numeric folding and rounding passes over the expression tree:
//! `evaluate_numbers`, rational-fraction cancellation (`reduce_rational`), and
//! the display-rounding passes (`round_numbers_*`, `set_small_zero`,
//! `constants_to_floats`).

use crate::constant_policy::{is_e, is_pi};
use crate::expr::map_children;
use crate::expr::Expr;
use crate::normalize::{canonicalize, present};
use crate::num::{BigNumber, Number, Spelling};
use crate::polynomials::kernel::{indeterminates, kernelize, Kernels};
use std::collections::BTreeSet;

/// Fold numeric subexpressions (`4 + x − 2` → `x + 2`) — the port of
/// `me.evaluate_numbers`. Ours is the exact canonical fold: rationals stay
/// exact where the JS produces floats; the ordering is the canonical one.
///
/// **Numeric only.** The canonical layer also collects like terms, and running
/// it unmodified here made `x² + 3x²` come back as `4x²` — a correct
/// simplification, but not a *numeric* one. It is the whole content of
/// DoenetML's `simplify="numbers"`, which is specified as "fold numeric
/// constants, leave the symbolic structure alone"; with like terms collected
/// that attribute was indistinguishable from `simplify="full"`. So the sum
/// constructor runs with collection switched off — see
/// [`without_like_term_collection`](crate::normalize::without_like_term_collection)
/// for exactly what that does and does not suppress.
pub fn evaluate_numbers(e: &Expr) -> Expr {
    evaluate_numbers_budget(e, MaxDigits::None)
}

/// [`evaluate_numbers`] under a digit budget. The default budget
/// ([`MaxDigits::None`]) is exactly [`evaluate_numbers`]; a non-`None` budget
/// additionally spends exact rationals into decimals *after* the fold — see
/// [`spend_rationals`] — which is why `1/3` reaches `0.333` under `Unlimited`
/// where floating the input `["/", 1, 3]` before the fold never could (its two
/// operands are integers and integers do not float; the `Rat(1, 3)` only exists
/// once the fold has combined them).
fn evaluate_numbers_budget(e: &Expr, budget: MaxDigits) -> Expr {
    if crate::equality::contains_blank(e) {
        return e.clone();
    }
    // Float contagion: a decimal or float already spent its precision, so exact
    // constants (`π`, `e`) and rationals sharing the expression fold to floats
    // too — `0.5·π` → `1.5707…`, matching mathjs. Purely exact input (`π/2`,
    // `1/3`) stays exact.
    // Fold only the *constants* (`π`, `e`) to floats when a decimal/float is
    // present; the decimal then floats the arithmetic around them by contagion
    // (`Rat·Float → Float`). Exact rationals are left exact, so `0.1 + 0.2`
    // still folds to an exact `0.3` rather than the `0.30000…4` a blanket float
    // conversion would give.
    //
    // An `Unlimited` budget floats the constants unconditionally, so `2π + π`
    // folds to one number even with no decimal in the input. A *finite* budget
    // does not: `π` is irrational, so no finite count of digits captures it, and
    // legacy left it symbolic (a written decimal still floats it, by contagion).
    let prepped = if matches!(budget, MaxDigits::Unlimited) || contains_inexact(e) {
        constants_to_floats(e)
    } else {
        e.clone()
    };
    // `fold_units` combines like-unit terms a numeric fold should join
    // (`50% + 75%` → `125%`); canonicalization leaves them written.
    // `fold_infnan_tree` folds division-by-zero poles and infinity arithmetic
    // (`1/0 → ∞`, `6/-0 → −∞`, `∞·2 → ∞`) — the same rule `simplify` applies,
    // which plain canonicalization leaves for later.
    // `fold_i_powers_tree` reduces `i^n` (`i·i` → `−1`). That is arithmetic on a
    // number, not a symbolic identity — the imaginary unit's integer powers are
    // exact and have no branch cut to choose — so it belongs to a pass that
    // claims to evaluate the numbers. It is re-canonicalized because the fold
    // produces a plain `−1` that a surrounding product must absorb: `2i·3i` is
    // `6·i²`, and only after the fold and a re-canonicalization is it `−6`.
    crate::normalize::without_like_term_collection(|| {
        let canon = canonicalize(&prepped);
        let folded = canonicalize(&fold_infnan_tree(&canon));
        let folded = canonicalize(&fold_i_powers_tree(&folded));
        let united = crate::normalize::fold_units(&folded);
        // Spend the budget on the *folded* rationals, on the canonical form: a
        // coefficient like the `1/3` in `x/3` is a single `Rat` here, so the
        // leaf rule below reaches it, and `present` restores the display order
        // (`["*", 0.333, "x"]`, number first).
        let spent = match budget {
            MaxDigits::None => united,
            _ => spend_rationals(&united, budget),
        };
        present(&spent)
    })
}

/// Apply the ∞/NaN + pole fold ([`rule_infnan`](crate::normalize)) bottom-up
/// across the tree, so `evaluate_numbers` folds `1/0 → ∞` and `∞·2 → ∞` the way
/// `simplify` does. Children fold first, so a pole revealed inside a product is
/// seen by the enclosing node.
fn fold_infnan_tree(e: &Expr) -> Expr {
    let e = map_children(e, fold_infnan_tree);
    crate::normalize::rule_infnan(&e).unwrap_or(e)
}

/// Apply `i^n → {1, i, −1, −i}`
/// ([`fold_imaginary_power`](crate::normalize::fold_imaginary_power)) bottom-up,
/// so `evaluate_numbers` reduces powers of the imaginary unit the way the JS
/// library does (`i·i` → `−1`, `i³` → `−i`).
///
/// Children fold first, so a power revealed underneath — the `i²` inside
/// `(2i)(3i)` once canonicalization has gathered the two `i` factors — is seen
/// by the enclosing product.
fn fold_i_powers_tree(e: &Expr) -> Expr {
    let e = map_children(e, fold_i_powers_tree);
    match &e {
        Expr::Pow(b, x) => crate::normalize::fold_imaginary_power(b, x).unwrap_or(e),
        _ => e,
    }
}

/// Whether any number in `e` is inexact — a `Float`, or an exact rational the
/// author spelled as a decimal (`0.5` parses to `Rat(1, 2, Decimal)`). This is
/// what triggers float contagion in [`evaluate_numbers`].
fn contains_inexact(e: &Expr) -> bool {
    if let Expr::Num(n) = e {
        return matches!(n, Number::Float(_)) || n.spelling() == Spelling::Decimal;
    }
    e.children().iter().any(|c| contains_inexact(c))
}

/// [`evaluate_numbers`] plus the special-value folds, so a function applied to
/// a numeric argument evaluates: `sin(0) + 2` → `2`. This is the
/// `evaluate_functions` option of the JS `evaluate_numbers`, and what
/// DoenetML's `simplify="full"` needs.
///
/// Two passes are added on top of [`evaluate_numbers`]:
/// [`fold_special_values`](crate::normalize::fold_special_values) for the exact
/// identities (`sin(0) → 0`), then
/// [`fold_numeric_applications_approx`](crate::normalize::fold_numeric_applications_approx)
/// for the rest, which evaluates a function of numeric arguments to a float
/// when it has no exact value (`log(31) → 3.4339…`).
///
/// The float step is what "evaluate functions" means to the callers — `<round>`
/// asks for this precisely so it has a number to round — and it is why this
/// pass is *not* part of `simplify`, which must not trade an exact value for a
/// float. Nothing with a free variable is touched: every argument has to be a
/// number already, so `f(x)` is never "evaluated" at a guessed point.
///
/// Like-term collection stays suppressed for the same reason
/// [`evaluate_numbers`] suppresses it: the difference between this and plain
/// `evaluate_numbers` should be function evaluation and nothing else.
pub fn evaluate_numbers_evaluate_functions(e: &Expr) -> Expr {
    if crate::equality::contains_blank(e) {
        return e.clone();
    }
    crate::normalize::without_like_term_collection(|| {
        let folded = crate::normalize::fold_special_values(e);
        present(&crate::normalize::fold_numeric_applications_approx(&folded))
    })
}

/// How far an *exact* value may be spent into a decimal before folding — the
/// `max_digits` option of `me.evaluate_numbers`.
///
/// The budget exists because folding is not free: `1/3` has no finite decimal,
/// so turning it into `0.3333333333333333` trades an exact value for an
/// approximation. Legacy therefore asked how many significant digits the caller
/// was willing to spend, and only converted a value that fits.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MaxDigits {
    /// Never introduce a decimal: exact values stay exact. The default, and
    /// what plain [`evaluate_numbers`] does.
    #[default]
    None,
    /// Convert without limit, `π` and `1/3` included. `Infinity` on the JS
    /// side, and what DoenetML's grading path passes so that a response typed
    /// as `6.28318` can be compared against a target that still holds `2π`.
    Unlimited,
    /// Spend up to `n` **significant digits**: an exact value becomes a decimal
    /// only when its shortest exact decimal fits within `n` significant figures.
    /// `1/2` folds at any `n ≥ 1` (it is `0.5`), but `1/3` never does — its
    /// decimal does not terminate, so no finite budget can hold it — and `π`
    /// stays symbolic (an irrational is never captured by a finite count).
    /// Legacy's positive-integer `max_digits`. `Finite(0)` is legacy's
    /// "integers only": nothing non-integral fits.
    Finite(u32),
}

/// [`evaluate_numbers`] under a digit budget.
///
/// With [`MaxDigits::None`] this is exactly [`evaluate_numbers`]. With
/// [`MaxDigits::Unlimited`] the constants `π`, `e` and every folded rational
/// become floats, so a variable-free subtree collapses to one number: `2π + π +
/// 6` is `15.42477796076938`, `1/3` is `0.3333333333333333`, and `x/3` is
/// `0.3333333333333333 x`. A [`MaxDigits::Finite`] budget converts only the
/// rationals whose decimal fits (`1/2 → 0.5`, but `1/3` stays exact).
///
/// `i` is left alone throughout. It is a constant symbol like `π`, but it has no
/// real value to become, and the imaginary unit surviving the pass is what keeps
/// `0.5i + 0.75` a complex number rather than nonsense.
pub fn evaluate_numbers_with_digits(e: &Expr, max_digits: MaxDigits) -> Expr {
    evaluate_numbers_budget(e, max_digits)
}

/// [`evaluate_numbers_evaluate_functions`] under a digit budget.
///
/// The budget is spent on both sides of the fold, because this is the form that
/// *creates* constants: `sin⁻¹(1)` evaluates to `π/2`, and a `π` the fold
/// introduced was never in the tree a pre-pass walked. Spending only before left
/// `asin(1)` at `π/2` while the target it was compared against — a `π/2` the
/// author wrote — came out as `1.5707963267948966`, and syntactic equality
/// reads two spellings of the same number as unequal. The trailing
/// [`evaluate_numbers_budget`] pass is what folds and then spends the rationals.
pub fn evaluate_numbers_evaluate_functions_with_digits(e: &Expr, max_digits: MaxDigits) -> Expr {
    if crate::equality::contains_blank(e) {
        return e.clone();
    }
    // `constants_to_floats` up front so a function of a constant (`cos(pi)`)
    // sees the number; only for `Unlimited`, matching `evaluate_numbers_budget`.
    let prepped = match max_digits {
        MaxDigits::Unlimited => constants_to_floats(e),
        _ => e.clone(),
    };
    let folded = evaluate_numbers_evaluate_functions(&prepped);
    match max_digits {
        MaxDigits::None => folded,
        _ => evaluate_numbers_budget(&folded, max_digits),
    }
}

/// [`evaluate_numbers_preserve_order`](crate::ops::evaluate_numbers_preserve_order)
/// under a digit budget. Order-preserving folding produces the faithful display
/// form (`Div`/`Neg` kept), so the budget is spent on that form afterwards —
/// [`spend_rationals`] handles a division bar directly and never reorders.
pub fn evaluate_numbers_preserve_order_with_digits(e: &Expr, max_digits: MaxDigits) -> Expr {
    if crate::equality::contains_blank(e) {
        return e.clone();
    }
    let prepped = match max_digits {
        MaxDigits::Unlimited => constants_to_floats(e),
        _ => e.clone(),
    };
    let folded = crate::ops::evaluate_numbers_preserve_order(&prepped);
    match max_digits {
        MaxDigits::None => folded,
        _ => spend_rationals(&folded, max_digits),
    }
}

/// Spend a digit budget by turning exact rationals into decimals where the
/// budget allows — the post-fold half of the `max_digits` option.
///
/// Only *non-integral* values convert. An integer that stays an integer is what
/// a dozen rules key on — `log_2(2^x)` collapses, `e^3` is exact, `sin^(-1)`
/// reads as an inverse function, `int(x·x)` integrates — and floating them was
/// measured against the suite as a net loss (25 tests when every value floated,
/// 6 even with exponents exempt). Exponents are exempt for the same reason:
/// `x^(1/2)` must stay a root, not become `x^0.5`, so a `Pow` spends its base
/// only.
///
/// A written decimal (`Spelling::Decimal`) always converts — the author already
/// accepted float precision — while a written fraction converts only when its
/// decimal fits the budget ([`fits`]). `Unlimited` converts every non-integral
/// value; `NegZero` is left alone as the one exact value whose *identity*
/// carries information a float cannot (`1/(−0)` is `−∞`).
fn spend_rationals(e: &Expr, budget: MaxDigits) -> Expr {
    if matches!(budget, MaxDigits::None) {
        return e.clone();
    }
    match e {
        Expr::Num(n) => match spend_number(n, budget) {
            Some(f) => Expr::Num(f),
            None => e.clone(),
        },
        // Exponents never float (`x^(1/2)` stays a root); spend the base only.
        Expr::Pow(base, exp) => Expr::Pow(Box::new(spend_rationals(base, budget)), exp.clone()),
        // Faithful-layer division (the display / order-preserving form): fold a
        // bare `number / integer` into a decimal, or pull a `1/integer`
        // coefficient in front of a symbolic numerator (`x/3 → 0.333·x`).
        Expr::Div(a, b) => {
            let a = spend_rationals(a, budget);
            let b = spend_rationals(b, budget);
            spend_division(&a, &b, budget).unwrap_or(Expr::Div(Box::new(a), Box::new(b)))
        }
        _ => map_children(e, |c| spend_rationals(c, budget)),
    }
}

/// A single non-integral number spent to a float when the budget allows;
/// `None` to leave it exact. See [`spend_rationals`].
fn spend_number(n: &Number, budget: MaxDigits) -> Option<Number> {
    if is_integer_valued(n) || matches!(n, Number::Float(_) | Number::NegZero) {
        return None;
    }
    let v = n.to_f64();
    let convert = match budget {
        MaxDigits::None => false,
        MaxDigits::Unlimited => true,
        MaxDigits::Finite(d) => n.spelling() == Spelling::Decimal || fits(v, d),
    };
    convert.then(|| Number::from_f64(v))
}

/// Spend `a / b` where `b` is an integer, for the faithful `Div` form.
/// `number / integer` becomes one decimal; `symbolic / integer` becomes the
/// reciprocal times the numerator. Returns `None` (leave the bar in place) when
/// the resulting decimal does not fit the budget.
fn spend_division(a: &Expr, b: &Expr, budget: MaxDigits) -> Option<Expr> {
    let denom = match b {
        Expr::Num(n) if is_integer_valued(n) && !n.is_zero() => n.to_f64(),
        _ => return None,
    };
    match a {
        Expr::Num(n) if is_integer_valued(n) => {
            let q = n.to_f64() / denom;
            budget_fits(q, budget).then(|| Expr::Num(Number::from_f64(q)))
        }
        _ => {
            let recip = 1.0 / denom;
            budget_fits(recip, budget)
                .then(|| Expr::Mul(vec![Expr::Num(Number::from_f64(recip)), a.clone()]))
        }
    }
}

/// Whether `v` may be spent under `budget` — always under `Unlimited`, and under
/// a finite budget when its decimal fits ([`fits`]). Only called on values that
/// are already known non-integral.
fn budget_fits(v: f64, budget: MaxDigits) -> bool {
    match budget {
        MaxDigits::None => false,
        MaxDigits::Unlimited => true,
        MaxDigits::Finite(d) => fits(v, d),
    }
}

/// Whether `v` is exactly representable in `digits` significant figures — the
/// finite-`max_digits` test, ported from legacy's `evalf(c, d) === evalf(c, 14)`
/// (14 standing in for "as exact as a float gets"). `digits == 0` is legacy's
/// "integers only", so a non-integral value never fits it.
fn fits(v: f64, digits: u32) -> bool {
    if digits == 0 {
        return false;
    }
    round_sig(v, digits) == round_sig(v, 14)
}

/// Round `v` to `digits` significant figures (ties away from zero, as `f64::round`).
fn round_sig(v: f64, digits: u32) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let places = digits as i32 - 1 - v.abs().log10().floor() as i32;
    let factor = 10f64.powi(places);
    (v * factor).round() / factor
}

/// Whether `n` holds a whole-number value (`Int`, or a `Big` integer).
fn is_integer_valued(n: &Number) -> bool {
    matches!(n, Number::Int(_)) || matches!(n, Number::Big(b) if matches!(&**b, BigNumber::Int(_)))
}

/// Cancel common polynomial factors in fractions — the port of
/// `me.reduce_rational` (`(x²−1)/(x−1)` → `x+1`, `(x²−5x+6)/(x²−4)` →
/// `(x−3)/(x+2)`, multivariate included). Applied bottom-up at every node;
/// non-polynomial fractions (`sin x / x`) are left unchanged. Backed by the
/// polynomial layer (recursive dense GCD over ℚ, bounded by resource limits).
pub fn reduce_rational(e: &Expr) -> Expr {
    let canon = canonicalize(e);
    // Bottom-up reduction, then re-canonicalize so in-place reductions merge
    // with their surroundings (`1 + (x²−1)/(x−1)` → `x + 2`).
    present(&canonicalize(&reduce_node(&canon)))
}

/// The spelling a value computed from `e` should read back with: `Decimal` as
/// soon as any number in `e` is decimal, `Fraction` otherwise. The polynomial
/// layer works in `BigRational`, which carries no spelling, so a pass that
/// round-trips through it has to restore one — otherwise
/// `(1.5x + 1.5)/(x + 1)` reduces to `3/2` instead of `1.5`.
fn spelling_of(e: &Expr) -> Spelling {
    match e {
        Expr::Num(n) => n.spelling(),
        _ => e
            .children()
            .into_iter()
            .fold(Spelling::Fraction, |acc, c| acc.join(spelling_of(c))),
    }
}

/// `e` with every number re-spelled. Sound only alongside [`spelling_of`],
/// which is why the two are used as a pair.
fn respell(e: &Expr, spelling: Spelling) -> Expr {
    map_numbers(e, &|n| n.with_spelling(spelling))
}

/// Maximum distinct indeterminates (variables + kernels) [`reduce_node`] will
/// build a dense polynomial over.
///
/// The recursive dense model is exponential in this count — `y/∏ᵏ(xᵢ+1)` costs
/// 0.5 s at 10, 4.8 s at 12, 15.7 s at 13, tripling per variable, and the
/// per-variable [`MAX_DEGREE`](crate::polynomials) cap does not touch it. That
/// hole is as old as the pass, but it used to be hard to fall into: anything
/// with a `sin` in it was refused before it got this far. Kernels remove that
/// accidental shield, so the guard has to be explicit.
///
/// 10 is measured, not principled — an order of magnitude above the six a real
/// rational function needs, and cheap at the boundary. `ratform` caps at 6
/// because `together` multiplies denominators together and starts from a worse
/// place; this pass only cancels a fraction that already exists.
const MAX_INDETERMINATES: usize = 10;

fn reduce_node(e: &Expr) -> Expr {
    let e = map_children(e, reduce_node);
    let Expr::Mul(factors) = &e else { return e };
    let spelling = spelling_of(&e);

    // Split canonical `Mul` factors into numerator parts and denominator
    // bases: a factor `Pow(b, −k)` (integer k>0) contributes `b^k` below.
    let mut num_parts: Vec<Expr> = Vec::new();
    let mut den_parts: Vec<Expr> = Vec::new();
    for f in factors {
        if let Expr::Pow(b, x) = f {
            if let Expr::Num(Number::Int(k)) = &**x {
                if *k < 0 {
                    den_parts.push(crate::normalize::pow(
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
    let num = crate::normalize::mul(num_parts);
    let den = crate::normalize::mul(den_parts);

    // The polynomial ring is over ℚ in named variables, so anything else that
    // sits in the fraction — `π`, `e`, `cos x`, `√y` — has to become an
    // indeterminate before the converter will look at it. That is exactly what
    // cancellation needs: it is a polynomial *identity* (`num = g·qn`,
    // `den = g·qd`), and identities survive specialization, so it never matters
    // what the indeterminates later stand for. Without this the converter
    // simply refuses, and `(a+b)(c+d) / ((e+f)(c+d))` comes back uncancelled
    // for no better reason than the letter `e`.
    //
    // One `Kernels` spans both halves: give the same `cos x` two names and the
    // common factor goes unseen.
    let mut kernels = Kernels::default();
    let num = kernelize(&num, &mut kernels);
    let den = kernelize(&den, &mut kernels);

    // Common indeterminate list, in a fixed order.
    let vars = indeterminates(&num)
        .into_iter()
        .chain(indeterminates(&den))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<String>>();
    if vars.is_empty() {
        return e; // pure numeric fraction — Number arithmetic already reduced it
    }
    if vars.len() > MAX_INDETERMINATES {
        return e;
    }

    let (Some(pn), Some(pd)) = (
        crate::polynomials::expr_to_poly(&num, &vars),
        crate::polynomials::expr_to_poly(&den, &vars),
    ) else {
        return e;
    };
    let Some(g) = crate::polynomials::gcd(&pn, &pd, vars.len()) else {
        return e;
    };
    if crate::polynomials::is_trivial(&g) {
        return e;
    }
    let (Some(qn), Some(qd)) = (
        crate::polynomials::exact_div_top(&pn, &g, vars.len()),
        crate::polynomials::exact_div_top(&pd, &g, vars.len()),
    ) else {
        return e;
    };
    // Normalize the quotients' rational content into a single scalar on the
    // numerator, so `(2x+4)/2` comes out as `x+2` rather than `½·(2x+4)`.
    let (qn, qd) = crate::polynomials::normalize_fraction_sign(qn, qd);
    let (cn, qn) = crate::polynomials::strip_rational_content(&qn);
    let (cd, qd) = crate::polynomials::strip_rational_content(&qd);
    let scalar = Expr::Num(Number::from_bigrational_spelled(cn / cd, spelling));
    let new_num = crate::normalize::mul(vec![
        scalar,
        respell(&crate::polynomials::poly_to_expr(&qn, &vars), spelling),
    ]);
    let new_den = respell(&crate::polynomials::poly_to_expr(&qd, &vars), spelling);
    // Respell first, restore second: the numbers the quotients carry are the
    // ring's, and are the ones that need a spelling back. The numbers *inside* a
    // kernel are the author's own (`cos(0.5)`) and are none of our business.
    let new_num = kernels.restore(&new_num);
    let new_den = kernels.restore(&new_den);
    canonicalize(&Expr::Div(Box::new(new_num), Box::new(new_den)))
}

/// Replace the constant symbols `pi` and `e` with their floating-point values
/// (`i` is left as the imaginary unit). Matches `me.constants_to_floats`.
///
/// The base of `e^x` is exempt: that `e` is not a constant standing in for a
/// number, it is half the spelling of the exponential function, and floating it
/// leaves a tree nothing downstream recognizes as `exp`. The exponent still
/// converts.
pub fn constants_to_floats(e: &Expr) -> Expr {
    match e {
        Expr::Pow(base, exp) if is_e(base) => {
            Expr::Pow(base.clone(), Box::new(constants_to_floats(exp)))
        }
        // Only a *declared* constant floats: an undeclared `pi` is a variable
        // name, and turning it into 3.14159… would be a substitution, not a
        // conversion.
        _ if is_pi(e) => Expr::Num(Number::from_f64(std::f64::consts::PI)),
        _ if is_e(e) => Expr::Num(Number::from_f64(std::f64::consts::E)),
        _ => map_children(e, constants_to_floats),
    }
}

/// A rational the *author wrote as a fraction* — as opposed to one that is a
/// decimal quantity ([`Spelling::Decimal`]) or an integer.
///
/// Display rounding leaves these alone. The JS library had no rational type, so
/// a written `2/3` was the tree `["/", 2, 3]` and rounding — which maps over
/// *numbers* — found two integers that were already whole and changed nothing.
/// A fraction reached the reader as a fraction however few digits were asked
/// for. Folding `2/3` into a single exact `Rat` is a strictly better
/// representation, but it silently turned that display into `0.67`, and only
/// for values that had been through `simplify`: `<math>2/3</math>` still showed
/// the fraction while `<point>(2/3,3)</point>` did not, in the same document.
///
/// [`Spelling`] is exactly the distinction needed, and it is why it is carried:
/// a `Fraction`-spelled rational is what legacy held as `["/", a, b]`, and a
/// `Decimal`-spelled one (`0.5`, or anything a decimal took part in) is what it
/// held as a float. So `<round>0.5</round>` still rounds, and the rule needs no
/// separate display-only entry point.
///
/// Supersedes the narrower rule in upstream request 16 ("if rounding would not
/// change the value, return it unchanged"), which kept `5/2` but could not keep
/// `1/3` — it assumed legacy decimalized a non-terminating fraction, and legacy
/// had no way to.
fn is_written_as_fraction(n: &Number) -> bool {
    match n {
        Number::Rat(_, _, sp) => *sp == Spelling::Fraction,
        Number::Big(b) => matches!(&**b, BigNumber::Rat(_, sp) if *sp == Spelling::Fraction),
        _ => false,
    }
}

/// Round every number in `e` to `decimals` decimal places (ties away from zero).
pub fn round_numbers_to_decimals(e: &Expr, decimals: i32) -> Expr {
    map_numbers(e, &|n| {
        if is_written_as_fraction(n) {
            return n.clone();
        }
        n.round_to_decimals(decimals)
    })
}

/// `me.set_small_zero`: replace every number whose magnitude is `< tolerance`
/// with exact `0`. The float-noise cleanup applied after numeric evaluation
/// (default tolerance `1e-14` on the JS side — callers pass it explicitly).
pub fn set_small_zero(e: &Expr, tolerance: f64) -> Expr {
    let tol = tolerance.abs();
    map_numbers(e, &|n| {
        if n.to_f64().abs() < tol {
            Number::Int(0)
        } else {
            n.clone()
        }
    })
}

/// Round every number in `e` to `sig_figs` significant figures.
pub fn round_numbers_to_precision(e: &Expr, sig_figs: i32) -> Expr {
    map_numbers(e, &|n| {
        if sig_figs < 1 || is_written_as_fraction(n) {
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
            if is_written_as_fraction(n) {
                return n.clone();
            }
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
