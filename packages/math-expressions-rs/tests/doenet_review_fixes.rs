//! Regressions for the review of the Doenet-compatibility round (`c110a56..`).
//! Each test pins one defect the review found, so it cannot come back quietly.
//!
//! Three of them — the NaN aggregates, `0/∞`, and `nCr` on a float — produced
//! *wrong numbers* rather than errors, which is the failure shape
//! `active-plans/DOENET_INTEGRATION.md` argues is worst on a grading path: a
//! student sees a confident answer and nothing logs. The two cost tests exist
//! because student input is adversarial by construction.

use math_expressions::{
    is_integer, is_negative, is_nonnegative, is_nonpositive, is_positive, is_real, resource_limits,
    simplify, Assumptions, Expr, MathConst, TextToAst,
};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

fn p(s: &str) -> Expr {
    TextToAst::new(Default::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e:?}"))
}

fn js(json: &str) -> Expr {
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    math_expressions::expr::serde::try_from_js(&v).unwrap_or_else(|e| panic!("{json}: {e}"))
}

fn tree(e: &Expr) -> String {
    math_expressions::expr::serde::to_js(e).to_string()
}

// ---- annihilation: `0 · x` vs the indeterminate forms --------------------

/// `∞^(-1)` *is* `0`, so `0/∞` is a plain zero. The guard blocking `0·x → 0`
/// for non-finite factors read only the base and not the exponent, so the fix
/// that correctly made `0/0` indeterminate also made `0/∞` come back `NaN`.
#[test]
fn zero_over_infinity_is_zero() {
    for s in [
        r#"["/",0,{"$":"Inf"}]"#,
        r#"["/",0,{"$":"-Inf"}]"#,
        r#"["*",0,["^",{"$":"Inf"},-1]]"#,
        r#"["*",0,["^",{"$":"-Inf"},-2]]"#,
    ] {
        assert_eq!(tree(&simplify(&js(s))), "0", "{s} should annihilate to 0");
    }
}

/// The other half of the same guard: the genuinely indeterminate forms must
/// stay `NaN`. DoenetML computes an undefined slope as `0/0`, and reporting
/// that as `0` calls a degenerate line horizontal.
#[test]
fn indeterminate_products_stay_nan() {
    for s in [
        r#"["/",0,0]"#,
        r#"["*",0,{"$":"Inf"}]"#,
        r#"["*",0,{"$":"-Inf"}]"#,
        r#"["*",0,{"$":"NaN"}]"#,
        r#"["*",0,["^",{"$":"Inf"},2]]"#,
        // `{"$":"None"}` is DoenetML's "no value here". A product touching one
        // is undefined, not zero — plan item 3b, which is the whole reason
        // `evaluate_to_constant` guards on a `None` leaf before simplifying.
        r#"["*",0,{"$":"None"}]"#,
        r#"["*",0,["^",{"$":"None"},-1]]"#,
    ] {
        assert_eq!(
            simplify(&js(s)),
            Expr::Const(MathConst::NaN),
            "{s} is indeterminate and must be NaN"
        );
    }
}

/// A factor whose finiteness is merely *unknown* still annihilates — legacy's
/// third `undefined` state fell through to `0`, and narrowing that would break
/// the overwhelmingly common case.
#[test]
fn zero_times_an_unknown_factor_is_still_zero() {
    for s in ["0*x", "0*f(x)", "0/x", "0*x*y^2"] {
        assert_eq!(tree(&simplify(&p(s))), "0", "{s:?} should be 0");
    }
}

// ---- combinatorics on floats and on large arguments ----------------------

/// `n.re.round() as i64` saturates, so `nCr(1e20,3)` silently became
/// `nCr(i64::MAX,3)` — a confident answer ~1275× too small. The float path now
/// stays in f64, where `n - k` is at least correctly rounded.
#[test]
fn combinatorics_on_large_floats_are_not_saturated() {
    let near = |got: &Expr, want: f64, label: &str| {
        let Expr::Num(n) = got else {
            panic!("{label} did not fold: {got:?}")
        };
        let g = n.to_f64();
        assert!(
            (g / want - 1.0).abs() < 1e-9,
            "{label}: got {g:e}, want ~{want:e}"
        );
    };
    near(
        &simplify(&js(r#"["apply","nCr",["tuple",1e20,3]]"#)),
        1e60 / 6.0,
        "nCr(1e20,3)",
    );
    near(
        &simplify(&js(r#"["apply","nPr",["tuple",1e20,2]]"#)),
        1e40,
        "nPr(1e20,2)",
    );
    // Small float arguments are unaffected.
    near(
        &simplify(&js(r#"["apply","nCr",["tuple",5.0,3.0]]"#)),
        10.0,
        "nCr(5.0,3.0)",
    );
}

/// Bounding `r` alone left the *work* unbounded — a running product against a
/// 1661-bit `n` is quadratic in the result size. The value must be unchanged;
/// only the cost is. (A generous ceiling: this was ~4 s.)
#[test]
fn large_exact_combinatorics_are_fast() {
    for s in ["nCr(10^500,500)", "nPr(10^500,500)"] {
        let t = Instant::now();
        let got = simplify(&p(s));
        let dt = t.elapsed();
        assert!(
            matches!(got, Expr::Num(_)),
            "{s:?} should still fold exactly, got {got:?}"
        );
        assert!(dt.as_millis() < 1500, "{s:?} took {dt:?}");
    }
    // Still exactly right at a size that can be checked by hand.
    assert_eq!(tree(&simplify(&p("nCr(60,30)"))), "118264581564861424");
    assert_eq!(tree(&simplify(&p("nCr(5,3)"))), "10");
    assert_eq!(tree(&simplify(&p("nPr(5,3)"))), "60");
}

/// `integer_log` stripped one factor per iteration, which is quadratic in the
/// bit length: `log₂(2^200000)` — six characters of student input — took
/// seconds. Binary search on the exponent gives the same answers.
#[test]
fn exact_logarithms_are_fast_on_huge_powers() {
    let t = Instant::now();
    assert_eq!(tree(&simplify(&p("log_2(2^200000)"))), "200000");
    let dt = t.elapsed();
    assert!(dt.as_millis() < 1500, "log_2(2^200000) took {dt:?}");

    // The answers the binary search has to keep: exact powers fold, everything
    // else stays symbolic rather than becoming a float.
    assert_eq!(tree(&simplify(&p("log_10(1000)"))), "3");
    assert_eq!(tree(&simplify(&p("log10(100000)"))), "5");
    assert_eq!(tree(&simplify(&p("log_7(343)"))), "3");
    assert_eq!(tree(&simplify(&p("log_2(1)"))), "0");
    assert_eq!(tree(&simplify(&js(r#"["apply","log2",["/",1,8]]"#))), "-3");
    // Not an exact power: the value must stay symbolic rather than turn into a
    // float. `log10(3)` keeps its application; the *based* spellings hand off to
    // the change-of-base rewrite instead, which is still symbolic — and is what
    // decides that the based and unbased spellings are the same number.
    assert!(matches!(simplify(&p("log10(3)")), Expr::Apply(..)));
    for s in ["log_2(9)", "log_2(2^200000+1)"] {
        let simplified = simplify(&p(s));
        assert!(
            !matches!(simplified, Expr::Num(_)),
            "{s:?} is not an exact power and must not fold to a number"
        );
        let arg = s.trim_start_matches("log_2(").trim_end_matches(')');
        assert_eq!(
            simplified,
            simplify(&p(&format!("log({arg})/log(2)"))),
            "{s:?} should reduce by change of base"
        );
    }
}

// ---- `from_ast` diagnostics ----------------------------------------------

/// The message the DoenetML team lost a debugging cycle to: an object with no
/// `$` was reported as `unknown special None`, where that `None` was the
/// `Option` from the lookup — and `{"$":"None"}` is meanwhile a *legal* tree,
/// so it read as a valid input being rejected.
#[test]
fn from_ast_names_the_actual_problem() {
    let err = |json: &str| {
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        math_expressions::expr::serde::try_from_js(&v).unwrap_err()
    };
    let missing = err("{}");
    assert!(
        !missing.contains("None"),
        "a missing `$` must not be reported with the word None: {missing}"
    );
    assert!(
        missing.contains('$'),
        "should name the missing key: {missing}"
    );

    let unknown = err(r#"{"$":"Bogus"}"#);
    assert!(
        unknown.contains("Bogus"),
        "should name the offending tag: {unknown}"
    );

    // And the tag that *is* legal still round-trips.
    assert_eq!(js(r#"{"$":"None"}"#), Expr::Const(MathConst::None));
}

/// `is_single_delimited_group` (`print/latex.rs`) stepped one *byte* at a time
/// over anything that was not a `\left`/`\right` token, so a multi-byte
/// character inside the group put the next `&s[i..]` off a UTF-8 boundary and
/// panicked — an abort, in this crate, taking the worker with it.
///
/// `＿` (U+FF3F) is three bytes and is exactly what DoenetML leaves in a slot
/// the student has not filled, and the guard only runs on a rendering that
/// already contains a space, which `and` supplies. The text-printer twin
/// `is_single_paren_group` walks `char_indices` and never had this.
#[test]
fn latex_of_a_delimited_group_holding_a_blank_does_not_abort() {
    let latex =
        math_expressions::to_latex(&p("(_, _) and x"), &math_expressions::LatexOpts::default());
    assert!(latex.contains('＿'), "unexpected latex: {latex}");

    // The guard's own answer is unchanged for the ASCII case it was written
    // for: an operand that is itself a spaced compound keeps its delimiters,
    // so the re-parse binds it the same way.
    let compound = math_expressions::to_latex(
        &p("(a and b) or c"),
        &math_expressions::LatexOpts::default(),
    );
    assert!(
        compound.contains("\\left(a \\land b\\right)"),
        "unexpected latex: {compound}"
    );
}

// ---------------------------------------------------------------------------
// Fourth review cycle. Everything below reproduced against the public API
// before the fix and was re-checked after it.
// ---------------------------------------------------------------------------

/// `default_order` has to be a *normal form*: `simplify="normalizeOrder"` is
/// how two spellings of one answer are made to match, so an order that depends
/// on the input order grades the same answer differently depending on how the
/// student wrote it.
///
/// `append_unit` stringifies sort-key index 1, which for a `Seq`/`Array`/
/// `Interval` holds the operand count rather than a kind name, and `cmp_key`
/// compared that `Str` with the plain containers' `Num` numerically —
/// `"2_%".parse::<f64>()` is `NaN`, which tied it with *every* plain container
/// while the plain ones still ordered by length. Six input orderings gave three
/// different trees. (`sort_by` on a non-total comparator can also panic
/// outright, which in a `panic = "abort"` crate is a dead worker.)
#[test]
fn default_order_of_a_unit_annotated_container_is_a_normal_form() {
    let terms = ["(z,z)", "(y,y)%", "(x,x,x)"];
    let mut seen = std::collections::BTreeSet::new();
    for perm in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let src = format!(
            "{} + {} + {}",
            terms[perm[0]], terms[perm[1]], terms[perm[2]]
        );
        let ordered = math_expressions::default_order(&p(&src));
        seen.insert(math_expressions::to_text(&ordered, &Default::default()));
    }
    assert_eq!(seen.len(), 1, "not a normal form: {seen:?}");

    // The kind-name keys units were designed for are untouched: both sides are
    // strings there, so `5%` still sorts among the numbers.
    assert_eq!(
        math_expressions::to_text(
            &math_expressions::default_order(&p("x + 5% + 3")),
            &Default::default()
        ),
        math_expressions::to_text(
            &math_expressions::default_order(&p("3 + 5% + x")),
            &Default::default()
        )
    );
}

/// Pulling the sign out of an odd root picks the *real* branch, so it may not
/// be done over a radicand that is not real. `is_real` answers `None` for
/// `i·x` and `x + i` — it cannot rule out an imaginary `x` — and the rule
/// accepted anything not *provably* non-real, so `cbrt(-i·x)` became
/// `-cbrt(i·x)`, a different number.
#[test]
fn odd_root_sign_extraction_declines_over_an_imaginary_residual() {
    // `simplify` leaves these alone rather than moving the sign.
    for src in ["cbrt(-x*i)", "cbrt(-x-i)"] {
        let out = tree(&simplify(&p(src)));
        assert!(
            !out.starts_with(r#"["-",["apply","cbrt""#),
            "{src} should not extract the sign: {out}"
        );
    }

    // The convention that makes a *symbol* of unknown realness count as real
    // is unchanged — that is what `cbrt(-8)` folding to `-2` rests on.
    assert_eq!(
        tree(&simplify(&p("cbrt(-x)"))),
        r#"["-",["apply","cbrt","x"]]"#
    );
    assert_eq!(tree(&simplify(&p("cbrt(-8)"))), "-2");
    assert_eq!(
        tree(&simplify(&p("cbrt(-8x)"))),
        r#"["*",-2,["apply","cbrt","x"]]"#
    );
    // A provably non-real residual keeps the sign under the radical and lets
    // only the positive perfect power out, as before.
    assert_eq!(
        tree(&simplify(&p("cbrt(-8i)"))),
        r#"["*",2,["apply","cbrt",["-","i"]]]"#
    );
}

/// The siblings of the case above, which the `i`-spelling check alone did not
/// catch. Realness is not propagated through `+` or `*` — `combine::add` and
/// `combine::mul` infer `real` only as "every operand is real" and never emit
/// `Some(false)` — so a residual whose *parts* are provably non-real answers
/// `None` as a whole: `is_real(sqrt(-2))` is `Some(false)` but
/// `is_real(x·sqrt(-2))` is `None`, and none of these spellings mentions `i`.
/// Each therefore had its sign pulled out of the odd root and became a
/// different number.
///
/// The decisive evidence is that the engine contradicted *itself*: at `x = 1`
/// the residual is closed, `is_real(sqrt(-2))` is `Some(false)`, and
/// `cbrt(-sqrt(-2))` was correctly left alone — while the symbolic pair was
/// rewritten. Those are different numbers (`cbrt(-sqrt(-2)) = 0.972 − 0.561i`
/// against `-cbrt(sqrt(-2)) = -0.972 − 0.561i`), so one spelling of an answer
/// graded against another is a wrong answer on the grading path.
///
/// Note the assertions below are on the *tree*, deliberately. `equals` is
/// itself taken in by this — it answers `true` for
/// `cbrt(-x·sqrt(-2)) = -cbrt(x·sqrt(-2))` while answering `false` for the
/// `x = 1` instance — so an `equals`-based assertion here passes with the
/// defect in place and pins nothing.
#[test]
fn odd_root_sign_extraction_declines_over_a_non_real_part() {
    for src in [
        "cbrt(-x*sqrt(-2))",
        "cbrt(-x*ln(-1))",
        "cbrt(-x*arcsin(2))",
        "cbrt(-(x+sqrt(-2)))",
        "nthroot(-x*sqrt(-2),5)",
    ] {
        let out = tree(&simplify(&p(src)));
        assert!(
            !out.starts_with(r#"["-",["apply""#),
            "{src} should not extract the sign: {out}"
        );
    }

    // A positive perfect power still comes out, as it does over a residual
    // spelled with `i`: a positive real factor does not move the argument.
    assert_eq!(
        tree(&simplify(&p("cbrt(-8*x*sqrt(-2))"))),
        r#"["*",2,["apply","cbrt",["-",["*","x",["apply","sqrt",-2]]]]]"#
    );

    // Non-realness that only an assumption establishes, likewise: `x ∉ R`
    // makes `is_real(x)` false but says nothing about `x + 1` or `x·y`.
    let mut a = Assumptions::new();
    a.add(&p("x notelementof R"));
    for src in ["cbrt(-(x+1))", "cbrt(-x*y)"] {
        let out = tree(&math_expressions::simplify_with(&p(src), &a));
        assert!(
            !out.starts_with(r#"["-",["apply","cbrt""#),
            "{src} should not extract the sign under `x ∉ R`: {out}"
        );
    }

    // Still extracted where nothing is provably non-real.
    assert_eq!(
        tree(&simplify(&p("cbrt(-x*sqrt(2))"))),
        r#"["-",["apply","cbrt",["*","x",["apply","sqrt",2]]]]"#
    );
}

/// `∞ − ∞` written as two poles. `add` collects like terms, and the
/// additive-inverse identity it relies on does not hold for an infinite term:
/// `1/0 − 1/0` answered `0`, `1/0 + 2 − 1/0` answered `2` and `2/0 − 1/0`
/// answered `∞`. `simplify::is_infnan_constant` already counted a zero-pole;
/// the two layers disagreed and `constructors` ran first (it is
/// `canonicalize`, which precedes every rewrite), so `constructors` won.
#[test]
fn poles_do_not_cancel_as_like_terms() {
    for src in [
        "1/0 - 1/0",
        "x/0 - x/0",
        "1/0 + 2 - 1/0",
        "2/0 - 1/0",
        "1/0 - 1/0 + y",
        "1/(0^2) - 1/(0^2)",
    ] {
        assert_eq!(tree(&simplify(&p(src))), r#"{"$":"NaN"}"#, "{src}");
    }

    // Coefficients that all pull the same way are still collected: `c·∞` is
    // `∞` for positive `c`, so nothing here is indeterminate.
    assert_eq!(tree(&simplify(&p("1/0 + 1/0"))), r#"{"$":"Inf"}"#);
    assert_eq!(tree(&simplify(&p("3 + 1/0"))), r#"{"$":"Inf"}"#);
    // A pole *inside* a finite subexpression is finite and must still cancel:
    // `1/(1 + 1/0)` is an exact zero.
    assert_eq!(tree(&simplify(&p("1/(1+1/0) - 1/(1+1/0)"))), "0");
}

/// One operation the sweep below runs over each hand-built tree.
type PrintOp = Box<dyn Fn(&Expr)>;

/// `expr/serde.rs`'s catch-all builds an `OtherOp` for any unknown head with
/// no arity check, so `me.fromAst(["pm"])`, `["binom","x"]`, `["unit","x"]`
/// and `["d"]` are all constructible from JS — and both printers indexed
/// `args[0]`/`args[1]` without looking. In a `panic = "abort"` crate that is a
/// worker kill on the one operation every expression meets.
///
/// The transform layers were already clean; this sweep asserts that and pins
/// the printers alongside them.
#[test]
fn printers_survive_every_arity_of_every_notation_head() {
    let heads = [
        "pm",
        "forall",
        "exists",
        "implies",
        "impliedby",
        "iff",
        "rightarrow",
        "leftarrow",
        "leftrightarrow",
        "perp",
        "parallel",
        ":",
        "|",
        "binom",
        "vec",
        "linesegment",
        "angle",
        "unit",
        "d",
        "derivative_leibniz",
        "partial_derivative_leibniz",
        "wombat",
    ];
    let arities: [&[&str]; 4] = [&[], &["\"x\""], &["\"x\"", "2"], &["\"x\"", "2", "3"]];
    let wrappers = ["{}", r#"["+",{},1]"#, r#"["*",{},2]"#, r#"["^",{},2]"#];

    let mut trouble = Vec::new();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    for head in heads {
        for arity in arities {
            let inner = format!(
                "[\"{head}\"{}{}]",
                if arity.is_empty() { "" } else { "," },
                arity.join(",")
            );
            for wrapper in wrappers {
                let json = wrapper.replace("{}", &inner);
                let value: serde_json::Value = serde_json::from_str(&json).unwrap();
                let Ok(e) = math_expressions::expr::serde::try_from_js(&value) else {
                    continue;
                };
                let ops: [(&str, PrintOp); 6] = [
                    (
                        "to_text",
                        Box::new(|e: &Expr| {
                            math_expressions::to_text(e, &Default::default());
                        }),
                    ),
                    (
                        "to_latex",
                        Box::new(|e: &Expr| {
                            math_expressions::to_latex(e, &Default::default());
                        }),
                    ),
                    (
                        "simplify",
                        Box::new(|e: &Expr| {
                            simplify(e);
                        }),
                    ),
                    (
                        "default_order",
                        Box::new(|e: &Expr| {
                            math_expressions::default_order(e);
                        }),
                    ),
                    (
                        "canonicalize",
                        Box::new(|e: &Expr| {
                            math_expressions::canonicalize(e);
                        }),
                    ),
                    (
                        "expand",
                        Box::new(|e: &Expr| {
                            math_expressions::expand(e);
                        }),
                    ),
                ];
                for (label, op) in ops {
                    let e = e.clone();
                    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| op(&e))).is_err() {
                        trouble.push(format!("{label} {json}"));
                    }
                }
            }
        }
    }
    std::panic::set_hook(previous);
    assert!(trouble.is_empty(), "{} panics: {trouble:#?}", trouble.len());
}

/// `i64::MIN` has no positive counterpart in `i64`, and `Number::neg` negated
/// in place. Under `overflow-checks` that is an abort; without them it wraps
/// back to `i64::MIN`, which is a wrong number. Both reachable sites are
/// typeable: `rule_distribute_sign` and `present`'s `negated_exponent`.
#[test]
fn negating_i64_min_widens_instead_of_overflowing() {
    let big = "9223372036854775808";
    assert_eq!(
        math_expressions::to_text(
            &simplify(&p("-9223372036854775808(1-x)")),
            &Default::default()
        ),
        format!("{big} (x - 1)")
    );
    assert_eq!(
        math_expressions::to_text(
            &simplify(&p("2^(-9223372036854775808/1)")),
            &Default::default()
        ),
        format!("1/2^{big}")
    );
    // The exact value survives the widening rather than being rounded to f64.
    assert_eq!(
        math_expressions::to_text(
            &simplify(&p("-(-9223372036854775808)")),
            &Default::default()
        ),
        big
    );
}

/// `rule_gaussian`'s exponent cap is per node, and `rewrite` runs bottom-up,
/// so nesting compounds it: each level multiplies the operand size by up to
/// 64. `(((2+i)^64+1)^64+1)^64` is about thirty typeable characters and did
/// not finish in twenty seconds. Student input is adversarial by construction,
/// which is why the neighbouring passes (`max_expand_power`, `polynomial_pow`)
/// are bounded too.
///
/// Asserted on the *shape* of the result rather than on a clock, because a
/// clock cannot report this. Without the bound the offending call does not
/// return at all — it was still running after 400 s — so a trailing
/// `assert!(elapsed < 10s)` placed after the loop is never reached and the
/// regression surfaces only as a CI job timeout, on some other job's name.
/// The bound refuses to fold, which is directly observable: the power comes
/// back written as it was typed. The nested calls are kept as well, on a
/// worker thread joined with a timeout, so that a hang there is this test
/// failing rather than the suite wedging.
#[test]
fn nested_gaussian_powers_are_bounded() {
    // Straight at the limit, so the assertion is about the bound and not about
    // this machine. `2+i` is a 2-bit base, so `^64` asks for 128 bits of
    // result and is refused, while `^32` asks for exactly 64 and folds.
    let tight = resource_limits::ResourceLimits {
        max_gaussian_pow_bits: 64,
        ..Default::default()
    };
    resource_limits::with(tight, || {
        assert_eq!(tree(&simplify(&p("(2+i)^64"))), r#"["^",["+","i",2],64]"#);
        assert_eq!(
            tree(&simplify(&p("(2+i)^32"))),
            r#"["+",["*",116749235904,"i"],-98248054847]"#
        );
    });

    // The same refusal under the shipped limit, where it takes the nesting to
    // reach: the inner `((2+i)^64+1)^64` folds to a ~4.8 kbit Gaussian integer
    // and the outer `^64` would be ~305 kbit, so the outer power is left
    // standing. Limits are thread-local, so the worker below runs under the
    // defaults, which is what is wanted here.
    let (tx, rx) = mpsc::channel();
    let worker = thread::spawn(move || {
        let shapes: Vec<String> = ["(((2+i)^64+1)^64+1)^64", "((((2+i)^64+1)^64+1)^64+1)^64"]
            .iter()
            .map(|src| tree(&simplify(&p(src))))
            .collect();
        let _ = tx.send(shapes);
    });
    let start = Instant::now();
    let shapes = match rx.recv_timeout(Duration::from_secs(30)) {
        Ok(shapes) => shapes,
        // Deliberately not joined: the worker is wedged, and joining it would
        // wedge the harness in exactly the way this rewrite exists to avoid.
        Err(e) => panic!("nested Gaussian powers did not finish in {:?} ({e}) — the max_gaussian_pow_bits bound is not holding", start.elapsed()),
    };
    worker.join().unwrap();
    for shape in &shapes {
        assert!(
            shape.starts_with(r#"["^","#) && shape.ends_with(",64]"),
            "the outer power should have been left unfolded, got {shape}"
        );
    }

    // The bound is on the *result*, so the ordinary cases still fold exactly.
    assert_eq!(tree(&simplify(&p("(2+i)^4"))), r#"["+",["*",24,"i"],-7]"#);
    assert_eq!(tree(&simplify(&p("(2+i)(2-i)"))), "5");
}

/// `log_b(a) → log(a)/log(b)` is invalid at `b = 1`, where `log b` is `0`:
/// it answered `log_1(5) → ∞`, and `log_1(1) → 1` through the numeric pass
/// beside it. There is no base-1 logarithm, so it joins the other undefined
/// forms at `NaN`.
#[test]
fn log_base_one_is_undefined() {
    assert_eq!(tree(&simplify(&p("log_1(5)"))), r#"{"$":"NaN"}"#);
    assert_eq!(tree(&simplify(&p("log_1(1)"))), r#"{"$":"NaN"}"#);
    // Every other base is unaffected.
    assert_eq!(tree(&simplify(&p("log_2(8)"))), "3");
    assert_eq!(
        tree(&simplify(&p("log_2(5)"))),
        r#"["/",["apply","log",5],["apply","log",2]]"#
    );
}

/// A coefficient that folded to one *through a float* stayed written down.
///
/// `Number::is_one` recognized only `Int(1)`, where its sibling `is_zero` had
/// always accepted `Float(0.0)`. That predicate is what drops the identity
/// factor in `normalize::mul`, so `0.5 · 2 · x` came back `1·x` — but only
/// along the JSON path, because the text parser turns a decimal literal into an
/// exact rational (`Number::from_decimal_str`) while `expr::serde::try_from_js`
/// hands `0.5` to `Number::from_f64` and gets a `Float`.
///
/// Same mathematics, two answers, decided by which door the expression came in
/// through — and DoenetML comes in through `fromAst`. It reached grading:
/// `<math simplify expand>` of `0.5(2x-2)(x+1)` produced `1·x² − 1` where the
/// same answer typed `1/2(2x-2)(x+1)` produced `x² − 1`, so a correct response
/// failed a `symbolicEquality` comparison against the expected `x² − 1`.
#[test]
fn a_float_valued_one_is_still_the_multiplicative_identity() {
    // The JSON path, which is the one that was wrong.
    assert_eq!(tree(&simplify(&js(r#"["*",0.5,2,"x"]"#))), r#""x""#);
    assert_eq!(
        tree(&simplify(&js(r#"["+",["*",0.5,2,["^","x",2]],-1]"#))),
        r#"["+",["^","x",2],-1]"#
    );
    // It always agreed with the text parser on the answer's *value*; now it
    // agrees on the spelling too.
    assert_eq!(tree(&simplify(&p("0.5*2*x"))), r#""x""#);

    // The identity exponent is dropped by the same predicate.
    assert_eq!(tree(&simplify(&js(r#"["^","x",["*",0.5,2]]"#))), r#""x""#);
    // And a float-valued one is still a base that absorbs its exponent.
    assert_eq!(tree(&simplify(&js(r#"["^",["*",0.5,2],"x"]"#))), "1");

    // Nothing else moves: a float coefficient that is not one stays.
    assert_eq!(
        tree(&simplify(&js(r#"["*",0.5,3,"x"]"#))),
        r#"["*",1.5,"x"]"#
    );
}

/// An integer power of an imaginary number is a *real* number, and the
/// assumptions layer said it was not.
///
/// `eval_complex` fast-pathed to an exact `powi` only when the base was on the
/// real axis; an imaginary base fell through to `powc`, which goes via
/// `exp`/`ln` and returns `i² = -1 + 1.2246e-16i`. `Facts::of_constant` tests
/// `im != 0.0` *exactly*, so that residue classified `-1` as a genuine complex
/// value — and `Facts::normalize` then forced `integer`, `negative`, `nonneg`,
/// `positive`, `nonpos` all to `false` behind it. So `is_real(i^2)` was false
/// while `simplify(i^2)` was `-1`: the engine contradicting itself on the
/// public API. The legacy JS library answered `true`.
#[test]
fn integer_powers_of_an_imaginary_base_are_real() {
    let a = Assumptions::default();
    let re = |s: &str| is_real(&p(s), &a);

    // Negative real values: `i^2 = -1`, `(2i)^2 = -4`.
    for s in ["i*i", "i^2", "i^(-2)", "(2i)^2", "sqrt(-1)^2", "(1+i)^2*i"] {
        let e = p(s);
        assert_eq!(is_real(&e, &a), Some(true), "{s} is real");
        assert_eq!(is_negative(&e, &a), Some(true), "{s} is negative");
        assert_eq!(is_nonpositive(&e, &a), Some(true), "{s} is nonpositive");
        assert_eq!(is_positive(&e, &a), Some(false), "{s} is not positive");
        assert_eq!(
            is_nonnegative(&e, &a),
            Some(false),
            "{s} is not nonnegative"
        );
    }
    assert_eq!(
        is_integer(&p("i^2"), &a),
        Some(true),
        "i^2 is the integer -1"
    );
    assert_eq!(is_integer(&p("(2i)^2"), &a), Some(true), "(2i)^2 is -4");

    // And the positive half of the cycle.
    assert_eq!(re("i^4"), Some(true));
    assert_eq!(is_positive(&p("i^4"), &a), Some(true), "i^4 is 1");
    assert_eq!(is_integer(&p("i^4"), &a), Some(true));

    // The direction that must *not* move: a value genuinely off the real axis
    // still answers not-real, however small its imaginary part. Snapping the
    // residue at its source rather than putting an epsilon in `of_constant` is
    // what keeps these honest.
    for s in ["i", "i^3", "(1+i)^2", "2i", "i*10^(-300)", "1+i*10^(-300)"] {
        assert_eq!(is_real(&p(s), &a), Some(false), "{s} is not real");
        assert_eq!(is_integer(&p(s), &a), Some(false), "{s} is not an integer");
    }

    // The value the sampler computes is the value `simplify` folds to.
    assert_eq!(tree(&simplify(&p("i^2"))), "-1");
    assert_eq!(tree(&simplify(&p("(2i)^2"))), "-4");
    assert_eq!(tree(&simplify(&p("i^4"))), "1");
}
