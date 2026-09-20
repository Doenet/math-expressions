//! Fold numeric function applications: `floor(55.33) → 55`,
//! `sum(3, 17, 5−4) → 21`, `log10(10³) → 3`.
//!
//! Like [`fold_special_values`](super::fold_special_values), this is a pass in
//! its own right rather than one of the base rewrite clusters in `simplify`:
//! the base rounds never run it, and the public `simplify` reaches it only
//! through the `full_simplify` fixpoint driver. That keeps `equals` — which
//! goes through `simplify_canonical` — byte-stable.
//!
//! # Only exact folds
//!
//! An application folds only when every argument is an exact rational *and*
//! the result is one too; [`FnDef::fold_exact`] decides, returning `None` to
//! leave the node alone. So `floor(55.33)` and `mean(1,2,4)` fold (to `55` and
//! `7/3`) while `sqrt(2)`, `log10(3)` and `asin(1)` stay symbolic.
//!
//! The legacy library reached similar answers by a different route: evaluate
//! in floating point, then try to *recover* a fraction from the result. That
//! is why its `log(1000, 10)` is `2.9999999999999996` while its `log10(1000)`
//! is `3` — the difference being only that V8 has a dedicated `Math.log10`.
//! Deciding exactness instead of estimating it removes that whole class of
//! answer, and is why `log_10(1000)` folds here too.
//!
//! [`FnDef::fold_exact`]: crate::special_functions::FnDef::fold_exact

use crate::expr::{map_children, Expr};
use crate::num::{Number, Spelling};
use crate::special_functions::fold_exact;
use num_rational::BigRational;

/// Fold every numeric application in `e` that has an exact value. Bottom-up,
/// so an inner fold feeds the one above it (`abs(floor(-2.5)) → 3`).
///
/// The input is canonicalized first and the output is canonical, matching
/// [`fold_special_values`](super::fold_special_values). That is not just
/// tidiness: the exactness gate below only recognizes a `Num` leaf, so
/// `sum(3, 17, 5−4)` and `log₂(1/8)` fold only once `5−4` and `1/8` have
/// become single numbers.
pub fn fold_numeric_applications(e: &Expr) -> Expr {
    let canon = super::canonicalize(e);
    let folded = fold_nodes(&canon);
    // Folding swaps an `Apply` node for a `Num` leaf *in place*, which breaks
    // the canonical invariant of whatever contained it: `floor(55.33) + 3`
    // came back as `["+",55,3]`, two numeric terms left uncombined. Re-establish
    // it rather than relying on the caller running its own simplify afterwards
    // — `full_simplify` does, but this function is public and its contract says
    // canonical-out.
    if folded == canon {
        canon
    } else {
        super::canonicalize(&folded)
    }
}

fn fold_nodes(e: &Expr) -> Expr {
    let e = map_children(e, fold_nodes);
    let Expr::Apply(head, args) = &e else {
        return e;
    };
    // `det`/`trace` of a literal matrix reduce to a number (`\det[[1,2],[3,4]]`
    // → −2). The reduction returns a symbolic `Expr` for a symbolic matrix —
    // keep the application in that case, only fold when it lands on a number.
    // (The equality sampler consults the same helper, and does *not* stop at a
    // number: see `matrix::scalar_reduction`.)
    if let Some(r @ Expr::Num(_)) = crate::matrix::scalar_reduction(head, args) {
        return r;
    }
    if let Some(v) = fold_application(head, args) {
        return Expr::Num(v);
    }
    // The integer-valued functions can fold through an argument that has no
    // exact value of its own: `ceil(log(31.1))` is 4, whatever log(31.1) is to
    // the last digit. Their *result* is exact even when their input is not,
    // which is what makes this a legitimate exception to the exact-only rule
    // above — the float never survives the fold.
    //
    // The rounding itself stays exact on whatever value it is handed. Nudging
    // a near-integer argument onto the integer first would repair accumulated
    // f64 error (which is what the JS library's decimals always carried), but
    // it breaks `floor(x) ≤ x`, and a decimal here is an exact rational: there
    // is no representation error to repair.
    if args.len() == 1 && !matches!(args[0], Expr::Num(_)) && is_integer_valued(head) {
        let approx = fold_nodes_approx(&args[0]);
        if matches!(approx, Expr::Num(_)) {
            if let Some(v) = fold_application(head, std::slice::from_ref(&approx)) {
                return Expr::Num(v);
            }
        }
    }
    e
}

/// Functions whose value is an integer for every input they accept, so folding
/// them loses nothing even when the argument had to be evaluated numerically.
fn is_integer_valued(head: &Expr) -> bool {
    matches!(head, Expr::Sym(s) if matches!(s.name().as_str(), "floor" | "ceil" | "round"))
}

/// [`fold_numeric_applications`], but a function of numeric arguments that has
/// no *exact* value folds to a float instead of staying symbolic
/// (`log(31) → 3.4339…`, where the exact pass leaves it alone because 31 is not
/// a power of e).
///
/// Lossy by construction, so it is deliberately not part of `simplify` and not
/// reachable from `equals`. It exists for one caller: the `evaluate_functions`
/// option of `evaluate_numbers`, whose entire purpose is to turn function
/// applications into numbers — `<round>log(31)</round>` has to get a float
/// before it can round it.
pub fn fold_numeric_applications_approx(e: &Expr) -> Expr {
    let canon = super::canonicalize(e);
    let folded = fold_nodes_approx(&canon);
    if folded == canon {
        canon
    } else {
        super::canonicalize(&folded)
    }
}

fn fold_nodes_approx(e: &Expr) -> Expr {
    let e = map_children(e, fold_nodes_approx);
    let Expr::Apply(head, args) = &e else {
        return e;
    };
    // Exact first: `floor(55.33)` must stay the integer 55, and `mean(1,2)` the
    // rational 3/2, rather than picking up a float spelling on this path.
    if let Some(n) = fold_application(head, args) {
        return Expr::Num(n);
    }
    let args = effective_args(head, args);
    // Only when every argument is already numeric — otherwise `f(x)` with a
    // free variable would be "evaluated" at whatever the sampler happened to
    // pick.
    if !args.iter().all(|a| matches!(a, Expr::Num(_))) {
        return e;
    }
    fold_approximately(head, &args).map_or(e, Expr::Num)
}

/// The argument list a fold should actually reduce over: a head that takes a
/// list is spread into that list's elements, everything else is left alone.
fn effective_args(head: &Expr, args: &[Expr]) -> Vec<Expr> {
    spread_list_argument(head, args).unwrap_or_else(|| args.to_vec())
}

/// `f([a, b])` read as `f(a, b)` — an application whose sole argument is a
/// sequence of values — or `None` when this head and argument list are not
/// that shape.
///
/// The parenthesized spelling `f((a, b))` no longer arrives here: the parsers
/// flatten a lone `Tuple` argument (`parse::common::apply`), the way
/// `expr::serde::try_from_js` always has, so `mod((7,3))` *is* `mod(7,3)`
/// before normalization sees it. What is left for this to do is the other
/// list kinds, which do survive both the parsers and the JS AST: `mod([7,3])`
/// and `["apply","mod",["list",7,3]]` are still one sequence argument.
///
/// **Both layers that read an application must call this**, which is why it is
/// `pub(crate)` rather than private to the fold. `normalize::fold_apply` folds
/// the spread form; the equality sampler in `eval_numeric::complex` decides
/// from the same question whether an application has a value at all or is an
/// opaque atom to draw a random sample for. When only the fold spread,
/// `simplify(mod([7,3]))` was `1` while `equals(mod([7,3]), 1)` was `false` —
/// the same split that made a determinant differ from its own value
/// (`matrix::scalar_reduction`).
///
/// `fold_exact` is the membership test because it is the set of heads that
/// reduce a list of numbers to one number: the nine variadic aggregates plus
/// the fixed-arity `mod`, `nPr`, `nCr`, `abs`, `sign`, `floor`, `ceil`,
/// `log10`, `log2`, `round`. Spreading into a fixed-arity head is not a
/// mistake — it is what makes `mod([7,3])` legal — because the arity check
/// still happens downstream, on the spread list.
pub(crate) fn spread_list_argument(head: &Expr, args: &[Expr]) -> Option<Vec<Expr>> {
    match (head, args) {
        (Expr::Sym(s), [Expr::Seq(kind, xs)])
            if is_list_like(*kind) && fold_exact(&s.name()).is_some() =>
        {
            Some(xs.clone())
        }
        _ => None,
    }
}

/// Sequence kinds that stand for "these values", so an aggregate over one of
/// them reduces over its elements. `Set` is excluded: a set is unordered and
/// deduplicated, so `count({1,1,2})` is a question about the *set*, not a
/// three-element sample.
fn is_list_like(kind: crate::expr::SeqKind) -> bool {
    use crate::expr::SeqKind::*;
    matches!(kind, Tuple | Array | List | Vector | AltVector)
}

/// Whether [`fold_numeric_applications`] would replace this application with a
/// literal. Read by the change-of-base rewrite in
/// [`special_values`](super::special_values), which runs *earlier* in the
/// `full_simplify` round and must not pre-empt an exact fold.
pub(super) fn folds_to_a_number(head: &Expr, args: &[Expr]) -> bool {
    fold_application(head, args).is_some()
}

fn fold_application(head: &Expr, args: &[Expr]) -> Option<Number> {
    let spread = effective_args(head, args);
    let args = &spread[..];

    // Nothing folds unless every argument is already a number.
    let numbers: Vec<&Number> = args
        .iter()
        .map(|a| match a {
            Expr::Num(n) => Some(n),
            _ => None,
        })
        .collect::<Option<_>>()?;

    // How the result should read back: a function of decimals produces a
    // decimal (`abs(-3.5)` is `3.5`, not `7/2`), a function of integers or
    // fractions produces a fraction (`mean(1,2,3,4)` is `5/2`). Computed here
    // because `fold_exact` works in `BigRational`, which carries no spelling.
    let spelling = numbers
        .iter()
        .fold(Spelling::Fraction, |acc, n| acc.join(n.spelling()));

    // `to_bigrational` is the exactness gate: it returns `None` for
    // `Number::Float`. When every argument clears it we are in exact
    // territory and only an exact result is acceptable.
    match numbers.iter().map(|n| n.to_bigrational()).collect() {
        Some(rationals) => Some(fold_exactly(head, rationals)?.with_spelling(spelling)),
        // An argument that is *already* a float — `floor(55.33)` arriving
        // through the JSON tree, where a non-integer literal is an f64. The
        // value is inexact before we touch it, so folding cannot lose
        // exactness and the float evaluator decides. This is legacy's
        // "contains a decimal" escape hatch, and it is why `asin(0.5)` folds
        // to a number while `asin(1)` stays symbolic in both libraries.
        None => fold_approximately(head, args),
    }
}

fn fold_exactly(head: &Expr, rationals: Vec<BigRational>) -> Option<Number> {
    let value = match head {
        Expr::Sym(s) => fold_exact(&s.name())?(&rationals)?,
        // `log_b(x)` — the parsers spell a based logarithm as an `Index` head
        // rather than a two-argument apply.
        Expr::Index(f, base) => {
            let (Expr::Sym(f), Expr::Num(base)) = (&**f, &**base) else {
                return None;
            };
            let (name, [x]) = (f.name(), rationals.as_slice()) else {
                return None;
            };
            if name != "log" {
                return None;
            }
            crate::special_functions::exp_log::exact_log(x, &base.to_bigrational()?)?
        }
        _ => return None,
    };
    Some(Number::from_bigrational(value))
}

/// Fold through the float evaluator, for an application that already holds an
/// inexact argument. Only a finite *real* value is accepted — a complex result
/// (`sqrt(-4.5)`) leaves the application as written rather than silently
/// dropping an imaginary part.
fn fold_approximately(head: &Expr, args: &[Expr]) -> Option<Number> {
    let node = Expr::Apply(Box::new(head.clone()), args.to_vec());
    let z = crate::eval_numeric::complex::eval_complex(&node, &Default::default())?;
    (z.im == 0.0 && z.re.is_finite()).then(|| Number::from_f64(z.re))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TextToAst;

    /// Fold `s` and print the JS tree spelling, so expectations read the way
    /// the legacy oracle reports them.
    fn run(s: &str) -> String {
        let e = TextToAst::new(Default::default()).convert(s).unwrap();
        crate::expr::serde::to_js(&fold_numeric_applications(&e)).to_string()
    }

    /// Folding a tree built directly, which is how the aggregates arrive —
    /// they have no parser spelling (see `special_functions::aggregate`).
    fn run_js(json: &str) -> String {
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let e = crate::expr::serde::try_from_js(&v).unwrap();
        crate::expr::serde::to_js(&fold_numeric_applications(&e)).to_string()
    }

    #[test]
    fn rounding_and_magnitude_functions_fold_exactly() {
        // 55.33 is an exact rational (5533/100), never a float, so this is
        // decided rather than rounded.
        assert_eq!(run("floor(55.33)"), "55");
        assert_eq!(run("ceil(2.1)"), "3");
        assert_eq!(run("floor(-2.5)"), "-3");
        assert_eq!(run("ceil(-2.5)"), "-2");
        assert_eq!(run("abs(-3)"), "3");
        assert_eq!(run("abs(-3.5)"), "3.5");
        assert_eq!(run("sign(-4)"), "-1");
        assert_eq!(run("mod(7,3)"), "1");
    }

    #[test]
    fn logarithms_fold_only_on_exact_powers_of_the_base() {
        assert_eq!(run("log10(1000)"), "3");
        assert_eq!(run("log10(100000)"), "5");
        assert_eq!(run_js(r#"["apply","log2",8]"#), "3");
        assert_eq!(run_js(r#"["apply","log2",1024]"#), "10");
        // Negative exponents work the same way.
        assert_eq!(run_js(r#"["apply","log2",["/",1,8]]"#), "-3");
        // A based logarithm reaches the same helper through an `Index` head —
        // the shape where legacy returned 2.9999999999999996.
        assert_eq!(run("log_10(1000)"), "3");
        assert_eq!(run("log_2(8)"), "3");
        assert_eq!(run("log_7(343)"), "3");
        // Not a power of the base: left symbolic, not turned into a float.
        assert_eq!(run("log10(3)"), r#"["apply","log10",3]"#);
        assert_eq!(run("log_2(9)"), r#"["apply",["_","log",2],9]"#);
    }

    #[test]
    fn combinatorics_fold_exactly() {
        assert_eq!(run("nCr(5,3)"), "10");
        assert_eq!(run("nPr(5,3)"), "60");
        // Well past f64's exact-integer range: the float rule would lose
        // digits here, the exact one does not.
        assert_eq!(run("nCr(60,30)"), "118264581564861424");
    }

    #[test]
    fn aggregates_fold_over_their_whole_argument_list() {
        assert_eq!(run_js(r#"["apply","sum",["tuple",3,17,["+",5,-4]]]"#), "21");
        assert_eq!(run_js(r#"["apply","prod",["tuple",2,3,4]]"#), "24");
        assert_eq!(run_js(r#"["apply","mean",["tuple",1,2,3]]"#), "2");
        assert_eq!(
            run_js(r#"["apply","mean",["tuple",1,2,4]]"#),
            r#"["/",7,3]"#
        );
        // A fraction of integers reads back as a fraction whether or not its
        // decimal expansion terminates — `5/2` is not `2.5` here, because
        // nothing decimal went into it (`num::Spelling`).
        assert_eq!(
            run_js(r#"["apply","median",["tuple",1,2,3,4]]"#),
            r#"["/",5,2]"#
        );
        assert_eq!(run_js(r#"["apply","variance",["tuple",1,2,3]]"#), "1");
        assert_eq!(run_js(r#"["apply","std",["tuple",1,2,3]]"#), "1");
        assert_eq!(run_js(r#"["apply","count",["tuple",1,2,3]]"#), "3");
        assert_eq!(run_js(r#"["apply","max",["tuple",1,5,3]]"#), "5");
        assert_eq!(run_js(r#"["apply","min",["tuple",1,5,3]]"#), "1");
        assert_eq!(run_js(r#"["apply","sum",3]"#), "3");
    }

    /// The spreading above is reachable only from a tree built directly: the
    /// JS deserializer turns `["apply","sum",["tuple",1,2,3]]` into a
    /// three-argument apply before any of this runs, so the `run_js` cases
    /// above never take the `spread_list_argument` branch. Take it explicitly,
    /// so the branch has a test that fails if it is narrowed.
    #[test]
    fn an_aggregate_spreads_a_tuple_argument() {
        let tuple = Expr::Seq(
            crate::expr::SeqKind::Tuple,
            vec![Expr::int(1), Expr::int(2), Expr::int(3)],
        );
        for (name, expected) in [("sum", "6"), ("count", "3"), ("max", "3")] {
            let applied = Expr::Apply(Box::new(Expr::sym(name)), vec![tuple.clone()]);
            let folded = fold_numeric_applications(&applied);
            assert_eq!(
                crate::expr::serde::to_js(&folded).to_string(),
                expected,
                "{name} must spread its tuple"
            );
        }
    }

    /// …and so does a fixed-arity head. The parenthesized spelling reaches the
    /// same value without this branch — the parsers flatten a lone `Tuple`, so
    /// `mod((7,3))` *is* `mod(7,3)` — but a bracketed list does not, and it is
    /// a spelling both the text parser and the JS AST can carry.
    ///
    /// The arity check still happens, on the spread list: a head whose folder
    /// does not take that many arguments is left alone rather than forced.
    #[test]
    fn a_fixed_arity_head_spreads_a_list_argument_too() {
        // These take the branch: an `Array` argument is not flattened anywhere.
        assert_eq!(run("mod([7,3])"), "1");
        assert_eq!(run("nPr([5,2])"), "20");
        assert_eq!(run("nCr([5,2])"), "10");
        assert_eq!(run_js(r#"["apply","mod",["list",7,3]]"#), "1");
        // The parenthesized and two-argument spellings, for the same values;
        // both are the same tree by the time they arrive.
        assert_eq!(run("mod((7,3))"), "1");
        assert_eq!(run("mod(7,3)"), "1");
        assert_eq!(run("nPr(5,2)"), "20");
        assert_eq!(run("nCr(5,2)"), "10");
        // Spreading is not forcing: `abs` takes one argument, so a two-element
        // list leaves it symbolic rather than folding to something.
        assert_eq!(run("abs([-3,5])"), r#"["apply","abs",["array",-3,5]]"#);
        assert_eq!(
            run("log10([100,5])"),
            r#"["apply","log10",["array",100,5]]"#
        );
    }

    /// The whole point of the exactness gate: an irrational value keeps its
    /// symbolic form rather than collapsing to a float.
    #[test]
    fn irrational_and_symbolic_applications_are_left_alone() {
        assert_eq!(run("log10(3)"), r#"["apply","log10",3]"#);
        assert_eq!(run("asin(1)"), r#"["apply","asin",1]"#);
        assert_eq!(run("floor(x)"), r#"["apply","floor","x"]"#);
        assert_eq!(
            run_js(r#"["apply","std",["tuple",1,2,4]]"#),
            r#"["apply","std",["tuple",1,2,4]]"#
        );
        assert_eq!(
            run_js(r#"["apply","sum",["tuple","x","y"]]"#),
            r#"["apply","sum",["tuple","x","y"]]"#
        );
        // An unknown name has no folder and is untouched.
        assert_eq!(run("g(2)"), r#"["apply","g",2]"#);
    }

    /// An argument that is already an inexact float cannot be made *less*
    /// exact by folding, so the float evaluator decides there. This is the
    /// only route by which a fold introduces a non-rational number, and it
    /// mirrors legacy's "input contained a decimal" escape hatch.
    #[test]
    fn an_already_inexact_argument_folds_through_floats() {
        // 55.33 is an exact rational from the text parser but an f64 when it
        // arrives as a JSON literal — both reach 55.
        assert_eq!(run("floor(55.33)"), "55");
        assert_eq!(run_js(r#"["apply","floor",55.33]"#), "55");
        assert_eq!(run_js(r#"["apply","ceil",2.1]"#), "3");
        assert_eq!(run_js(r#"["apply","abs",-3.5]"#), "3.5");
        assert_eq!(run_js(r#"["apply","max",["tuple",1.5,2.5]]"#), "2.5");
        // Exact arguments still refuse: this is what keeps `asin(1)` symbolic
        // while `asin(0.5)` becomes a number, in both libraries.
        assert_eq!(run("asin(1)"), r#"["apply","asin",1]"#);
        assert!(run_js(r#"["apply","asin",0.5]"#).starts_with("0.523"));
        // A complex result is refused rather than silently losing its
        // imaginary part.
        assert_eq!(
            run_js(r#"["apply","sqrt",-4.5]"#),
            r#"["apply","sqrt",-4.5]"#
        );
    }

    /// Bottom-up, so a fold feeds the application above it.
    #[test]
    fn nested_applications_fold_inside_out() {
        assert_eq!(run("abs(floor(-2.5))"), "3");
        assert_eq!(
            run_js(r#"["apply","sum",["tuple",["apply","abs",-2],3]]"#),
            "5"
        );
    }

    /// The contract this pass documents: canonical in, canonical out. Replacing
    /// an application with a number leaves its *parent* uncanonical, so the
    /// numbers a fold exposes must still get combined and sorted.
    #[test]
    fn the_output_is_canonical() {
        assert_eq!(run("floor(55.33) + 3"), "58");
        assert_eq!(run("2*floor(55.33)*x"), r#"["*",110,"x"]"#);
        assert_eq!(run("floor(55.33) * x * 2"), r#"["*",110,"x"]"#);
        // Idempotent, which is what "canonical-out" buys the caller.
        for s in [
            "floor(55.33) + 3",
            "abs(floor(-2.5))",
            "log10(3) + 1",
            "x + 1",
        ] {
            let e = TextToAst::new(Default::default()).convert(s).unwrap();
            let once = fold_numeric_applications(&e);
            assert_eq!(
                fold_numeric_applications(&once),
                once,
                "not idempotent on {s:?}"
            );
        }
    }
}
