//! Equality-tester cases, mirroring the equivalence pairs in the JS
//! `slow_math-expressions.spec.js`. Equal pairs resolve either at the exact
//! canonical stage (stage 1) or by numerical sampling (stage 3).

use math_expressions::{
    equals, equals_syntactic, simplify, EqOptions, Expr, MathConst, Number, TextToAst,
    TextToAstOptions,
};

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
    // ∞ − ∞ is NaN, and NaN poisons the whole sum — a free variable does not
    // rescue it, so `x + ∞ − ∞` folds to NaN (the infinities do not cancel).
    // `equals` treats NaN as never-equal (IEEE), so verify the fold structurally.
    assert!(matches!(
        simplify(&parse("x + Infinity - Infinity")),
        Expr::Const(MathConst::NaN)
    ));
    // All-constant folds still work (matching JS .simplify()).
    assert!(eq("Infinity + 3", "Infinity"));
    // `.simplify()` matches mathjs here (`∞·i → ∞`); `evaluate_to_constant`
    // reports the complex NaN separately (a different oracle).
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
fn a_tolerance_survives_the_term_reordering_it_causes() {
    // Term order is decided by the *values* of the numbers in the tree, so a
    // perturbation small enough to forgive at a leaf can still move the term
    // that contains it: `xy` has total degree 2 and `q^1.99991` has 1.99991, so
    // the sum comes back in the other order and an order-sensitive walk then
    // compares `0.01xy` against `1000q^…` — after having already accepted every
    // number it looked at. The tell is that perturbing the exponent *upward*
    // always passed; nothing about the arithmetic is direction-dependent.
    let o = EqOptions {
        allowed_error_in_numbers: 1e-4,
        include_error_in_number_exponents: true,
        ..EqOptions::default()
    };
    let form = |a: &str, b: &str| equals_syntactic(&simplify(&parse(a)), &simplify(&parse(b)), &o);

    assert!(form("exp(0.01xy+1000q^2)", "exp(0.01xy+1000q^(2+.00009))"));
    assert!(form("exp(0.01xy+1000q^2)", "exp(0.01xy+1000q^(2-.00009))"));
    // Only the exact degree tie was ever fragile — a companion term of degree 1
    // or 3 cannot be crossed by 9e-5, and those passed all along. They are the
    // controls that pin the diagnosis to the ordering rather than the arithmetic.
    assert!(form("0.01x+1000q^2", "0.01x+1000q^(2-.00009)"));
    assert!(form("0.01xyz+1000q^2", "0.01xyz+1000q^(2-.00009)"));

    // Re-matching terms does not mean forgiving a term that is simply wrong.
    assert!(!form("0.01xy+1000q^2", "0.01xy+1000q^(2-.5)"));
    assert!(!form("0.01xy+1000q^2", "0.01xy+2000q^(2-.00009)"));
    // Nor does it let one term stand in for two. The pairing is a *matching*,
    // not a covering: every term on the left needs its own partner. Built as
    // raw sums because `simplify` would collect `x + x` into `2x` and the two
    // sides would then differ by a coefficient, which proves nothing about the
    // pairing. Here each `x` on the left is fuzzy-equal to the single `x` on the
    // right, so a cover-style check would say yes and be wrong.
    let two_x = Expr::Add(vec![parse("x"), parse("x")]);
    let x_and_y = Expr::Add(vec![parse("x"), parse("y")]);
    assert!(!equals_syntactic(&two_x, &x_and_y, &o));
}

#[test]
fn order_sensitivity_survives_a_tolerance_that_was_never_spent() {
    // The re-match above is justified only by "forgiving ε may have moved the
    // term holding it". A permutation that needs no ε is not sort drift — it is
    // the two expressions being written in different orders — so requesting a
    // tolerance anywhere must not silently turn a form check into an unordered
    // one.
    //
    // DoenetML is where this bit: `<answer symbolicEquality
    // allowedErrorInNumbers="0.001">` is documented and tested as refusing a
    // reordered response, and `e·25.602348230 + 2.15234262π` against
    // `2.15234262π + e·25.602348230` — every number identical — graded correct.
    let o = EqOptions {
        allowed_error_in_numbers: 1e-3,
        ..EqOptions::default()
    };
    let form = |a: &str, b: &str| equals_syntactic(&parse(a), &parse(b), &o);

    assert!(!form("x+y", "y+x"));
    assert!(!form("2x+3y", "3y+2x"));
    assert!(!form(
        "2.15234262pi+e*25.602348230",
        "e*25.602348230+2.15234262pi"
    ));
    // Written in the same order, it still matches — the tolerance is doing its
    // ordinary job on the numbers.
    assert!(form(
        "2.15234262pi+e*25.602348230",
        "2.15234263pi+e*25.602348230"
    ));
    // And a reordering that the tolerance *did* pay for is still forgiven: this
    // pair needs the allowance to pair its second terms up at all.
    let reordered_and_fuzzed = EqOptions {
        allowed_error_in_numbers: 1e-4,
        include_error_in_number_exponents: true,
        ..EqOptions::default()
    };
    assert!(equals_syntactic(
        &simplify(&parse("exp(0.01xy+1000q^2)")),
        &simplify(&parse("exp(0.01xy+1000q^(2-.00009))")),
        &reordered_and_fuzzed
    ));
}

#[test]
fn order_sensitivity_is_intact_without_a_tolerance() {
    // The unordered re-match is gated on a nonzero `allowed_error_in_numbers`.
    // With none set, the order is a function of numbers being compared exactly,
    // so it cannot drift — and `equals_syntactic`'s documented order-sensitivity
    // has to survive untouched, since that is the whole point of a form check.
    let a = Expr::Add(vec![parse("x"), parse("y"), parse("z")]);
    let b = Expr::Add(vec![parse("z"), parse("x"), parse("y")]);
    assert!(!equals_syntactic(&a, &b, &EqOptions::default()));

    // Products are excluded even under a tolerance: multiplication is not
    // commutative here, so re-matching factors would grade `AB` equal to `BA`.
    let o = EqOptions {
        allowed_error_in_numbers: 1e-4,
        ..EqOptions::default()
    };
    let ab = Expr::Mul(vec![parse("[[1,2],[3,4]]"), parse("[[5,6],[7,8]]")]);
    let ba = Expr::Mul(vec![parse("[[5,6],[7,8]]"), parse("[[1,2],[3,4]]")]);
    assert!(!equals_syntactic(&ab, &ba, &o));
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

// ===================== bare numbers: exact vs computed =====================

/// A `Float` — what numerical evaluation produces, and what a JS caller's
/// `fromAst(0.1)` lands on. Parsed decimals are *not* this: they are exact
/// rationals (PORTING_PLAN §3a).
fn float(v: f64) -> Expr {
    Expr::Num(Number::from_f64(v))
}

#[test]
fn exact_bare_numbers_compare_exactly() {
    // Two numbers a person wrote are exact quantities, and stage 1 is
    // definitive about them — no f64 slop may override it. `exactness_beats_
    // float_slop` above covers the integer case; these are the decimals,
    // which are exact too (PORTING_PLAN §3a) and so must not pick up the
    // float tolerance below.
    assert!(!eq("0.3", "0.30000000000000004"));
    assert!(!eq("1/3", "0.3333333333333333"));
    assert!(eq("0.1 + 0.2", "0.3"), "exact arithmetic, exactly equal");
}

#[test]
fn a_computed_float_compares_within_the_relative_tolerance() {
    // A `Float`'s low digits record the route taken, not the value: they are
    // the residue of inexact arithmetic. The JS library had no exact numbers
    // and compared every numeric pair against a 1e-12 relative epsilon
    // (`lib/expression/equality/numerical.js`), and callers depend on that —
    // DoenetML's `<sequence type="math" from=".1" step=".1" exclude=".3">`
    // drops its third term by comparing it to `.3`.
    let opts = EqOptions::default();
    let three_tenths = parse("0.3"); // exact: Rat(3,10)

    // The sequence's third term, arrived at the way the component does.
    let computed = simplify(&Expr::Add(vec![
        float(0.1),
        Expr::Mul(vec![float(0.1), Expr::Num(Number::Int(2))]),
    ]));
    assert!(
        format!("{:?}", computed).contains("Float"),
        "precondition: the sum is inexact, not folded to a rational"
    );
    assert!(equals(&three_tenths, &computed, &opts));
    assert!(equals(&computed, &three_tenths, &opts), "symmetric");

    // Same story stated directly, in both operand orders.
    assert!(equals(&float(0.3), &float(0.30000000000000004), &opts));
    assert!(equals(&float(0.30000000000000004), &float(0.3), &opts));
    assert!(equals(&three_tenths, &float(0.30000000000000004), &opts));

    // The tolerance is relative and tight — this is not "floats are equal".
    assert!(!equals(&float(0.3), &float(0.3000000001), &opts));
    assert!(!equals(&float(1.0), &float(1.0000001), &opts));
    assert!(!equals(&float(2.0), &float(3.0), &opts));
}

#[test]
fn the_form_check_carries_the_same_relative_float_floor() {
    // `equals_syntactic` compares *trees*, but its number leaves are still
    // numbers: JS `trees/basic.js equal` floors its tolerance at a 1e-14
    // relative difference before `allowed_error_in_numbers` is consulted, so
    // "no allowance" never meant "bit-identical" on this path either.
    //
    // The floor is what a whole grading route rests on. DoenetML spends its
    // digit budget (`max_digits: Infinity`) *before* like terms are collected,
    // so the same coefficient reaches the comparison by two arithmetic routes:
    // `x² − x²/3` collects as `1 − 0.3333333333333333` and `2x²/3` folds as
    // `2/3`. Those are the same number and adjacent f64s, and a form check that
    // read the last ULP as a different form would grade a correct answer wrong.
    let opts = EqOptions::default();
    let x = || parse("x");
    let coeff = |c: f64| Expr::Mul(vec![float(c), x()]);
    assert_ne!(
        0.6666666666666667_f64.to_bits(),
        0.6666666666666666_f64.to_bits(),
        "precondition: these are distinct f64s"
    );
    assert!(equals_syntactic(
        &coeff(0.6666666666666667),
        &coeff(0.6666666666666666),
        &opts
    ));

    // Tight, and still relative: a difference a person could have typed is a
    // different form, and the structure around the number must match exactly.
    assert!(!equals_syntactic(&coeff(0.6667), &coeff(0.6666), &opts));
    assert!(!equals_syntactic(&float(1.0), &float(1.0000001), &opts));
    assert!(
        !syn("x/3", "0.3333333333333333 x"),
        "a bar is not a decimal"
    );
    assert!(!syn("3+2", "5"), "still no folding");
}

// ===================== intervals, and the finite-field floor =====================

#[test]
fn a_float_constant_does_not_blind_the_finite_field_stage() {
    // The field is the only stage that can tell `e^(10x)` from `e^(10x) + C`:
    // sampling cannot, because `e^(10x)` ranges over so many orders of
    // magnitude that a small offset is invisible at almost every point.
    // Skipping *every* float there meant the answer depended on whether the
    // constant had been through a JSON round trip — written `0.0000001` the
    // pair was correctly unequal, read back as the f64 `1e-7` it was not.
    let e10x = parse("e^(10x)");
    let opts = EqOptions::default();
    for offset in [1e-7, 1e-3, 0.5] {
        let shifted = Expr::Add(vec![e10x.clone(), float(offset)]);
        assert!(
            !equals(&e10x, &shifted, &opts),
            "e^(10x) must not equal e^(10x) + {offset}"
        );
    }
    // The bound that keeps the field honest still holds: a float that only
    // *approximates* something is not taken at face value.
    assert!(eq("x + 3.141592653589793", "x + pi"));
}

#[test]
fn intervals_and_the_tuple_spellings_of_them() {
    let opts = EqOptions::default();
    let open = |s: &str| crate_to_intervals(parse(s));
    // `(1,2) union (3,4)` and the same text parsed with intervals built are
    // the same set — each written the only way its parse can write it.
    assert!(equals(
        &parse("(1,2) union (3,4)"),
        &open("(1,2) union (3,4)"),
        &opts
    ));
    assert!(equals(
        &open("(1,2) union [3,4]"),
        &parse("(1,2) union [3,4]"),
        &opts
    ));
    // Open/closed still has to agree.
    assert!(!equals(&parse("(1,2)"), &open("[1,2]"), &opts));
    assert!(!equals(&parse("[1,2]"), &open("(1,2)"), &opts));
    // A *vector* is not an interval, however the vector flag is set.
    let v = math_expressions::tuples_to_vectors(&parse("(1,2)"));
    assert!(!equals(&v, &open("(1,2)"), &opts));
    // The form check reads them the same way, so `symbolicEquality` grading
    // agrees with numeric grading here.
    assert!(equals_syntactic(
        &parse("(1,2) union (3,4)"),
        &open("(1,2) union (3,4)"),
        &opts
    ));
    // Off with the coercion flag.
    let strict = EqOptions {
        coerce_tuples_arrays: false,
        ..EqOptions::default()
    };
    assert!(!equals(&parse("(1,2)"), &open("(1,2)"), &strict));
}

fn crate_to_intervals(e: Expr) -> Expr {
    math_expressions::to_intervals(&e)
}

#[test]
fn an_exponent_is_exempt_from_the_allowed_error_only_when_it_is_a_typed_number() {
    // The rule protects against slack on an exponent a *student typed*.
    let fuzzy = EqOptions {
        allowed_error_in_numbers: 1e-4,
        ..EqOptions::default()
    };
    assert!(!equals_syntactic(
        &parse("x^2.00002"),
        &parse("x^2"),
        &fuzzy
    ));
    let with_exp = EqOptions {
        include_error_in_number_exponents: true,
        ..fuzzy.clone()
    };
    assert!(equals_syntactic(
        &parse("x^2.00002"),
        &parse("x^2"),
        &with_exp
    ));

    // A compound exponent is not that case: `e^(…)` is an exponential and its
    // "exponent" is an argument, which `normalize_function_names` puts there by
    // folding `exp`. Its numbers are ordinary numbers.
    let a = math_expressions::normalize_function_names(&parse("10000exp(7.00002x/y)"));
    let b = math_expressions::normalize_function_names(&parse("10000exp(7x/y)"));
    assert!(equals_syntactic(&a, &b, &fuzzy));
}

#[test]
fn rounding_functions_fold_through_an_inexact_argument() {
    // `ceil(log(31.1))` is 4 whatever log(31.1) is to the last digit: the
    // *result* is exact even when the argument has no exact value, which is
    // what lets these fold where the exact-only rule leaves them alone.
    let simp = |s: &str| math_expressions::simplify(&parse(s));
    let num = |n: i64| Expr::Num(Number::Int(n));
    assert_eq!(simp("ceil(log(31.1))"), num(4));
    assert_eq!(simp("floor(log(31.1))"), num(3));
    // The rounding itself stays exact. A decimal parses to an exact rational,
    // so `3.999999999999999` *is* that number and its floor is 3 — nudging it
    // onto 4 would repair an f64 error that is not there, at the cost of
    // `floor(x) ≤ x`. (The JS library did nudge: its decimals were floats.)
    assert_eq!(simp("floor(3.999999999999999)"), num(3));
    assert_eq!(simp("ceil(-6999.999999999999)"), num(-6999));
    assert_eq!(simp("floor(3.99)"), num(3));
    assert_eq!(simp("ceil(2.01)"), num(3));
}

#[test]
fn mathjs_named_constants_evaluate() {
    // The JS library evaluated through mathjs's scope, so these names had
    // values wherever a closed expression was reduced to a number. DoenetML
    // depends on it: `1E-300` typed into a `<mathInput>` with scientific
    // notation off parses as `1·E − 300`, and `<isNumber>` answers yes.
    use math_expressions::evaluate_to_constant;
    let expr = Expr::Add(vec![
        Expr::Mul(vec![Expr::Num(Number::Int(1)), Expr::sym("E")]),
        Expr::Num(Number::Int(-300)),
    ]);
    let v = evaluate_to_constant(&expr).expect("E is Euler's number here");
    assert!((v.re - (std::f64::consts::E - 300.0)).abs() < 1e-9);
    assert!(evaluate_to_constant(&Expr::sym("PI")).is_some());
    assert!(evaluate_to_constant(&Expr::sym("SQRT2")).is_some());
    // Not language constants, though: they stay ordinary variables, so
    // `variables()` lists them and a sampler binds them.
    assert!(math_expressions::variables(&Expr::sym("E")).contains(&"E".to_string()));
}

/// The two layers that read an application must agree about whether it has a
/// value: `normalize::fold_apply` folds, and the equality sampler in
/// `eval_numeric::complex` decides from `known_function` whether to evaluate an
/// application or draw a random value for it as an opaque atom. When they
/// disagree, `equals` says an expression differs from the number `simplify`
/// turns it into.
///
/// `det` was the first instance (`tests/matrix.rs`). This is the second:
/// `f([a,b])` read as `f(a,b)`, which the fold did and the sampler did not, so
/// `simplify(mod([7,3]))` was `1` while `equals(mod([7,3]), 1)` was `false` and
/// `evaluate_to_constant` was `None`. Both now go through
/// `normalize::spread_list_argument`.
///
/// The bracketed spelling is the one that still reaches that helper. The
/// parenthesized `mod((7,3))` is checked alongside it because it is the
/// spelling the defect was first found in, but it no longer depends on the
/// helper at all: the parsers flatten a lone `Tuple` argument, so it is the
/// same tree as `mod(7,3)` — which is what legacy produced for both, and why
/// both were `1` there. Measured against `math-expressions@2.0.0-alpha94`.
#[test]
fn a_sequence_argument_does_not_split_the_folder_from_the_sampler() {
    use math_expressions::evaluate_to_constant;
    for (parenthesized, plain, value) in [
        ("mod([7,3])", "mod(7,3)", "1"),
        ("nPr([5,2])", "nPr(5,2)", "20"),
        ("nCr([5,2])", "nCr(5,2)", "10"),
        ("mod((7,3))", "mod(7,3)", "1"),
        ("nPr((5,2))", "nPr(5,2)", "20"),
        ("nCr((5,2))", "nCr(5,2)", "10"),
    ] {
        assert!(
            eq(parenthesized, value),
            "{parenthesized} must equal {value}"
        );
        assert!(eq(plain, value), "{plain} must equal {value}");
        assert!(
            eq(parenthesized, plain),
            "{parenthesized} must equal {plain}"
        );
        assert_eq!(simplify(&parse(parenthesized)), parse(value));
        assert!(
            evaluate_to_constant(&parse(parenthesized)).is_some(),
            "{parenthesized} must have a numeric value, so <number> can read it"
        );
    }
    // Spreading does not force an arity the head does not have: `abs` takes one
    // argument, so `abs((-3,5))` stays symbolic — on *both* layers, which is
    // the property that matters here. It is its own opaque atom and equal to
    // itself, and claims no value.
    for symbolic in ["abs([-3,5])", "log10([100,5])", "floor([1.5,2.5])"] {
        assert!(eq(symbolic, symbolic), "{symbolic} must equal itself");
        assert!(
            equals(
                &parse(symbolic),
                &simplify(&parse(symbolic)),
                &EqOptions::default()
            ),
            "{symbolic} must equal its own simplification"
        );
        assert!(evaluate_to_constant(&parse(symbolic)).is_none());
    }
}
