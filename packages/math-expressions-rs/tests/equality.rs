//! Equality-tester cases, mirroring the equivalence pairs in the JS
//! `slow_math-expressions.spec.js`. Equal pairs resolve either at the exact
//! canonical stage (stage 1) or by numerical sampling (stage 3).

use math_expressions::{equals, equals_syntactic, EqOptions, Expr, TextToAst, TextToAstOptions};

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

fn eq(a: &str, b: &str) -> bool {
    equals(&parse(a), &parse(b), &EqOptions::default())
}

#[test]
fn exact_stage_equalities() {
    // These resolve structurally (stage 1), no sampling needed.
    assert!(eq("1/3 + 1/6", "1/2"));
    assert!(eq("0.1 + 0.2", "0.3"));
    assert!(eq("x + x", "2x"));
    assert!(eq("x + y", "y + x"));
    assert!(eq("a*b*c", "c*a*b"));
    assert!(eq("x*x", "x^2"));
    assert!(eq("x - x", "0"));
    assert!(eq("2*3 + 4", "10"));
}

#[test]
fn numerical_stage_equalities() {
    // Algebraic identities canonicalize differently but agree numerically.
    assert!(eq("(x+1)^2", "x^2 + 2x + 1"));
    assert!(eq("x^2 - 1", "(x-1)(x+1)"));
    assert!(eq("(x+y)^2", "x^2 + 2x y + y^2"));
    assert!(eq("sin^2 x + cos^2 x", "1"));
    assert!(eq("2 sin(x) cos(x)", "sin(2x)"));
    assert!(eq("exp(x) exp(y)", "exp(x+y)"));
}

#[test]
fn inequalities() {
    assert!(!eq("x", "y"));
    assert!(!eq("x + 1", "x + 2"));
    assert!(!eq("1/3", "1/2"));
    assert!(!eq("sin(x)", "cos(x)"));
    assert!(!eq("(x+1)^2", "x^2 + 1"));
    assert!(!eq("x^2", "x^3"));
    assert!(!eq("2x", "3x"));
}

#[test]
fn log_and_branch_cut_identities() {
    // These hold only off a branch cut, so they are accepted by the lenient
    // region sampler — made safe by the finite-field rejection stage.
    assert!(eq("log(a^2*b)", "2*log(a)+log(b)"));
    assert!(eq("log(x^2*y/z)", "2*log(x) + log(y) - log(z)"));
    assert!(eq("x*log(y)", "log(y^x)"));
    assert!(eq("(-1)^n*cos(x)^n", "(-cos(x))^n"));
    // Constant transcendental values that reduce to zero.
    assert!(eq("sin(pi)", "0"));
    assert!(eq("cos(pi/2)", "0"));
}

#[test]
fn finite_field_rejects_near_misses() {
    // Differences the *lenient* sampler alone would mask (a small or
    // magnitude-dwarfed additive term) must still be rejected. The finite-field
    // stage evaluates exactly in ℤ/pℤ, where magnitude cannot hide them.
    assert!(!eq("e^(10x)", "e^(10x)+C"));
    assert!(!eq("e^(10x)", "e^(10x)+0.0000001"));
    assert!(!eq("sin(10x)", "sin(10x)+C"));
    assert!(!eq("1/8 e^(2x) + 2e^(-2x)", "1/8 e^(2x)"));
    // This one differs by exactly 1 (`sin²+cos²`), caught in the field.
    assert!(!eq("e^(10x)", "e^(10x)+sin^2(x)+cos^2(x)"));
    // Underflow guard: `x^sin(x)` underflows to exactly 0 across whole regions,
    // which must not count as agreement — with another underflowing function OR
    // with a literal 0 (one-sided zeros would match via tolerance_for_zero).
    assert!(!eq("x^(sin(x))", "x^(cos(x))"));
    assert!(!eq("x^(sin(x))", "0"));
    // ...but genuine near-equal float coefficients (−4π + π = −3π) stay equal:
    // the field skips high-precision decimals, so sampling (with tolerance) decides.
    assert!(eq(
        "-12.566370614359172 y^2 + 3.141592653589793 (-y)^2",
        "-9.42477796076938 y^2"
    ));
}

#[test]
fn exactness_beats_float_slop() {
    // §3a payoff: these are distinct exact integers even though they collapse
    // to the same f64. The JS float path calls them equal; we do not.
    assert!(!eq("10^20 + 1", "10^20 + 2"));
    // ...but the genuinely-equal huge integers still compare equal.
    assert!(eq("10^20 + 1", "1 + 10^20"));
}

#[test]
fn blanks_are_never_equal() {
    // A missing operand (`x^` → x^blank) makes equality undefined.
    assert!(!eq("x^", "x^"));
    let opts = EqOptions {
        allow_blanks: true,
        ..EqOptions::default()
    };
    // With allow_blanks, identical blank-bearing trees compare structurally.
    assert!(equals(&parse("x^"), &parse("x^"), &opts));
}

#[test]
fn commutativity_and_tuple_coercion() {
    // Tuple/array coercion on by default.
    assert!(eq("(1, 2)", "[1, 2]"));
    let no_coerce = EqOptions {
        coerce_tuples_arrays: false,
        ..EqOptions::default()
    };
    assert!(!equals(&parse("(1,2)"), &parse("[1,2]"), &no_coerce));
}

#[test]
fn scaling_units() {
    // `%` and `deg` scale away to plain numbers (units.js: `only_scales`), so
    // they are equal to their scaled values under full equality.
    assert!(eq("50%", "1/2"));
    assert!(eq("50% + 1", "1.5"));
    assert!(eq("x%", "x/100"));
    assert!(eq("180 deg", "pi"));

    // `$` is a `prefix` unit that only marks its value: it becomes a free
    // factor, so like-`$` quantities combine (`$3 + $2 = $5`)...
    assert!(eq("$5", "$3+$2"));
    assert!(eq("$5", "$9-$4"));
    assert!(eq("$xy+a$b", "$(xy+ab)"));
    // ...but a `$` quantity is never equal to a bare number.
    assert!(!eq("$5", "5"));
    assert!(!eq("$x", "x"));
}

#[test]
fn scaling_units_stay_syntactically_distinct() {
    // Syntactic (`equalsViaSyntax`) equality does NOT desugar units, so a
    // scaled unit and its numeric value remain structurally different — even
    // though full `equals` treats them as equal above.
    let o = EqOptions::default();
    assert!(!equals_syntactic(&parse("50%"), &parse("1/2"), &o));
    assert!(!equals_syntactic(&parse("180 deg"), &parse("pi"), &o));
}

// Helper for the form-check tests: syntactic equality with default options.
fn syn(a: &str, b: &str) -> bool {
    equals_syntactic(&parse(a), &parse(b), &EqOptions::default())
}

#[test]
fn syntactic_equality_is_a_form_check() {
    // `equals_syntactic` is `equalsViaSyntax`: a "is the answer in this *form*?"
    // check. It normalizes only lightly (function-name spelling, exponents/primes
    // outside applications, negative-number placement, geometry arg order) and
    // then compares trees order-sensitively. It must NOT flatten, reorder, fold,
    // combine like terms, or eliminate division.
    assert!(!syn("(x+y)+z", "z+x+y")); // reordering is a different form
    assert!(!syn("3+2", "5")); // no constant folding
    assert!(!syn("(-1*2)+3*4", "10")); // no evaluation
    assert!(!syn("(-x)*(-x)", "x^2")); // no simplification
    assert!(!syn("(a*b)/c", "a*(b/c)")); // division not rearranged
    assert!(!syn("++2", "2")); // structure preserved
    assert!(!syn("x*(5+x)", "(x+5)*x")); // operand order matters
}

#[test]
fn syntactic_equality_light_normalizations() {
    // The four passes that DO apply, so equivalent *spellings* of the same form
    // still match.
    assert!(syn("ln(x)", "log(x)")); // function-name table
    assert!(syn("arccos(x)", "acos(x)"));
    assert!(syn("cos^(-1)(x)", "arccos(x)")); // inverse notation → a-name
    assert!(syn("e^x", "exp(x)")); // e^x → exp(x)
    assert!(syn("binom(n,k)", "nCr(n,k)"));
    assert!(syn("sin^2(x)", "(sin(x))^2")); // exponent moves outside
    assert!(syn("linesegment(A,B)", "linesegment(B,A)")); // unoriented
    assert!(syn("angle(A,B,C)", "angle(C,B,A)"));
    // ...but identical trees are of course still equal.
    assert!(syn("sin(x) + cos(x)", "sin(x) + cos(x)"));
}

#[test]
fn equation_and_inequality_equivalence() {
    // Equations compare by standard form up to any nonzero scalar: `a=b` ≡ `c=d`
    // when `a-b` is proportional to `c-d`.
    assert!(eq("5x + 2y = 3", "6-4y = 10x")); // factor -1/2
    assert!(eq("5x + 2y = 3", "-(6-4y) = -10x")); // factor 1/2

    // Inequalities need a *positive* factor: a negative one reverses direction.
    assert!(eq("5q-9z < 2u+9z", "27z -5q > -4u + 5q-9z"));
    assert!(eq("5q-9z <= 2u+9z", "27z -5q >= -4u + 5q-9z"));

    // Same coefficients, opposite direction: factor -1, so not equal.
    assert!(!eq("5q < 9z", "5q > 9z"));
    assert!(!eq("5q <= 9z", "-5q <= -9z"));
    // Different constants are not proportional.
    assert!(!eq("x > 1000", "x > 1001"));
    // A shift by a free constant / tiny number is not proportional either.
    assert!(!eq("e^(10x)=0", "e^(10x)+C=0"));
    assert!(!eq("cos(10x) < 0", "cos(10x)+0.0000001 < 0"));
}

#[test]
fn equation_form_is_preserved_for_syntactic_equality() {
    // The proportional-standard-form equivalence above is a *mathematical*
    // check; it must NOT leak into syntactic equality, so a teacher grading the
    // required form still distinguishes `5x+2y=3` from its rearrangement.
    let o = EqOptions::default();
    assert!(!equals_syntactic(
        &parse("5x + 2y = 3"),
        &parse("6-4y = 10x"),
        &o
    ));
    assert!(!equals_syntactic(
        &parse("5q-9z < 2u+9z"),
        &parse("27z -5q > -4u + 5q-9z"),
        &o
    ));
}

#[test]
fn logs_roots_and_factorials() {
    // Single-arg nthroot is a square root.
    assert!(eq("nthroot(x)", "sqrt(x)"));
    // Subscripted log evaluates by change of base (`log_b(x) = ln x / ln b`).
    assert!(eq("log_2(8)", "3"));
    assert!(eq("log_a(b)", "log(b)/log(a)"));
    assert!(!eq("log_2(8)", "4"));
    // Factorials evaluate via the gamma function, so the gamma recurrence
    // `(n+1)·Γ(n+1) = Γ(n+2)` makes these hold at sampled (non-integer) points.
    assert!(eq("(n+1)*n!", "(n+1)!"));
    assert!(eq("n/n!", "1/(n-1)!"));
    assert!(!eq("n!", "(n+1)!"));
    // Exact integer factorial folding still applies.
    assert!(eq("5!", "120"));
}

#[test]
fn coercion_reaches_nested_positions() {
    // Sequence coercion must apply inside relations (and other containers),
    // not just at the top level.
    assert!(eq("(1,2) = x", "[1,2] = x"));
    assert!(eq("x = y", "y = x"));
}

#[test]
fn infnan_folds_are_conservative() {
    // Regression tests from the 2026-07-17 review: ∞/NaN folding fires only on
    // all-constant sums/products. A symbolic factor has unknown sign (`x·∞` is
    // ±∞ or NaN depending on x), so these must NOT compare equal.
    assert!(!eq("x*Infinity", "Infinity"));
    assert!(!eq("x*Infinity", "y*Infinity"));
    assert!(!eq("x/0", "1/0"));
    assert!(!eq("x/0", "y/0"));
    // A tuple is not a scalar: ∞·(a,b) must not collapse to ∞.
    assert!(!eq("Infinity*(a,b)", "Infinity"));
    // ∞ − ∞ absorbs only constant terms — never a free variable.
    assert!(!eq("x + Infinity - Infinity", "y + Infinity - Infinity"));
    // All-constant folds still work (matching JS .simplify()).
    assert!(eq("Infinity + 3", "Infinity"));
    assert!(eq("Infinity*i", "Infinity"));
    assert!(eq("1/0", "Infinity"));
    assert!(eq("1/Infinity", "0"));
    // NegInf base folds by parity; negative exponent gives 0.
    assert!(eq("(-Infinity)^2", "Infinity"));
    assert!(eq("(-Infinity)^3", "-Infinity"));
    assert!(eq("1/(-Infinity)", "0"));
}

#[test]
fn radical_extraction_is_bounded() {
    // Regression: sqrt(<19-digit prime>) previously trial-divided up to
    // ~3·10^9 iterations inside equals() (multi-second stall). The perfect
    // power case is now O(log) and partial extraction is capped, so this
    // completes instantly (the test itself is the timing assertion — it would
    // time out otherwise).
    assert!(!eq("sqrt(9223372036854775783)", "2"));
    // Perfect powers of any size still fold exactly.
    assert!(eq("sqrt(4611686014132420609)", "2147483647")); // (2^31-1)^2
    assert!(eq("sqrt(12)", "2*sqrt(3)"));
    assert!(eq("(-8)^(1/3)", "-2"));
}

#[test]
fn mixed_seq_kinds_combine_componentwise() {
    // Regression: coercion must run BEFORE simplify so a coerced Array
    // combines with a Tuple componentwise.
    assert!(eq("[1,2]+(3,4)", "[4,6]"));
    assert!(eq("[1,2]+(3,4)", "(4,6)"));
    assert!(eq("[2x,y^2]", "(x+x, y*y)"));
}

#[test]
fn applied_function_power_spellings_unify() {
    // canon_apply moves a function-head exponent outside the application
    // (MOVE_EXPONENT_OUTSIDE), so both spellings share one canonical form and
    // compare equal at stage 1 — even nested where sampling cannot reach.
    assert!(eq("sin^2(x)", "sin(x)^2"));
    assert!(eq("x ∈ [3, sin^2(x)]", "x ∈ [3, sin(x)^2]"));
}

#[test]
fn reciprocal_powers_stay_pole_safe() {
    // Since nested-pow flattening, 1/x^a canonicalizes to x^(-1·a) — the
    // finite-field filter must stay pole-conservative on the zero-base case
    // (regression guard for the flattened-reciprocal shape).
    assert!(eq("1/x^a", "x^(-a)"));
    assert!(!eq("1/x^a", "1/x^(a+1)"));
    assert!(!eq("1/x^a", "1/y^a"));
}

#[test]
fn allowed_error_in_numbers() {
    // JS-oracle verdicts (probed against me.equals with the same options).
    let fuzzy = |err: f64| EqOptions {
        allowed_error_in_numbers: err,
        ..EqOptions::default()
    };
    let feq = |a: &str, b: &str, o: &EqOptions| equals(&parse(a), &parse(b), o);

    assert!(feq("3.14", "pi", &fuzzy(0.01)));
    assert!(!feq("3.1", "pi", &fuzzy(0.001)));
    assert!(feq("2.0001*x", "2*x", &fuzzy(1e-3)));
    assert!(!feq("2.0001*x", "2*x", &fuzzy(1e-6)));
    assert!(feq("3.14*sin(x)", "pi*sin(x)", &fuzzy(0.01)));
    assert!(feq("1/3.14", "1/pi", &fuzzy(0.01)));
    assert!(!feq("5", "5.05", &fuzzy(0.001)));
    // Exponents are exempt unless included explicitly.
    assert!(!feq("x^2.0002", "x^2", &fuzzy(1e-3)));
    let with_exp = EqOptions {
        include_error_in_number_exponents: true,
        ..fuzzy(1e-3)
    };
    assert!(feq("x^2.0002", "x^2", &with_exp));
    // Absolute mode.
    let abs = EqOptions {
        allowed_error_is_absolute: true,
        ..fuzzy(0.1)
    };
    assert!(feq("5", "5.05", &abs));
    // Default (0) keeps exact semantics.
    assert!(!feq("3.14", "pi", &EqOptions::default()));
}

#[test]
fn sqrt_and_half_power_are_equal() {
    // `sqrt(x)` (Apply) and `x^(1/2)` (Pow) stay distinct canonical trees —
    // matching JS, which keeps them distinct at the tree level and relies on the
    // equality pipeline to reconcile them. The full `equals` must still resolve
    // them as equal (lock-in against a canonicalize-level merge — a gratuitous
    // divergence from JS — or a regression that stops treating them as equal).
    assert!(eq("sqrt(x)", "x^(1/2)"));
    assert!(eq("x^(1/2)", "sqrt(x)"));
    assert!(eq("sqrt(x*y)", "(x*y)^(1/2)"));
}
