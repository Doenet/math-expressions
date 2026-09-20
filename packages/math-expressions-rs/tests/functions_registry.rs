//! Registry snapshot tests (tmp/IMPROVEMENT_PLAN.md Phase 1 step 1).
//!
//! The `crate::special_functions` registry replaced per-subsystem tables in
//! `parse/`, `normalize/`, `calculus/diff.rs`, and `calculus/integrate/`. These
//! tests pin the derived views to the *historical* table contents, so registry
//! edits that would silently change parser defaults or normalization behavior
//! fail loudly. When a change is intentional, update the literals here.

use math_expressions::eval_numeric::certified_digits::kernels;
use math_expressions::special_functions;

/// The text parser's default `applied_function_symbols` exactly as it stood
/// before the registry migration (parse/text.rs history).
const OLD_TEXT_APPLIED: &[&str] = &[
    "abs", "exp", "log", "ln", "log10", "sign", "sqrt", "cbrt", "nthroot", "mod", "erf", "cos",
    "cosh", "acos", "acosh", "arccos", "arccosh", "cot", "coth", "acot", "acoth", "arccot",
    "arccoth", "csc", "csch", "acsc", "acsch", "arccsc", "arccsch", "sec", "sech", "asec", "asech",
    "arcsec", "arcsech", "sin", "sinh", "asin", "asinh", "arcsin", "arcsinh", "tan", "tanh",
    "atan", "atan2", "atanh", "arctan", "arctanh", "arg", "conj", "re", "im", "det", "trace",
    "nPr", "nCr", "floor", "ceil", "round", "rootof",
];

/// The LaTeX parser's historical default list (parse/latex.rs history).
const OLD_LATEX_APPLIED: &[&str] = &[
    "abs", "exp", "log", "ln", "log10", "sign", "sqrt", "erf", "cos", "cosh", "acos", "acosh",
    "arccos", "arccosh", "cot", "coth", "acot", "acoth", "arccot", "arccoth", "csc", "csch",
    "acsc", "acsch", "arccsc", "arccsch", "sec", "sech", "asec", "asech", "arcsec", "arcsech",
    "sin", "sinh", "asin", "asinh", "arcsin", "arcsinh", "tan", "tanh", "atan", "atan2", "atanh",
    "arctan", "arctanh", "arg", "conj", "Re", "Im", "det", "trace", "nPr", "nCr", "floor", "ceil",
    "round",
    // Beyond the historical JS list (deliberate): `rootof` is Rust-only
    // notation for the RootOf leaf; registering it closes the LaTeX
    // `\operatorname{rootof}(p, k)` round-trip. JS has no RootOf, so there is
    // no parity to preserve.
    "rootof",
];

/// standard_form.js `function_normalizations`, verbatim.
const OLD_NORMALIZATIONS: &[(&str, &str)] = &[
    ("ln", "log"),
    ("arccos", "acos"),
    ("arccosh", "acosh"),
    ("arcsin", "asin"),
    ("arcsinh", "asinh"),
    ("arctan", "atan"),
    ("arctanh", "atanh"),
    ("arcsec", "asec"),
    ("arcsech", "asech"),
    ("arccsc", "acsc"),
    ("arccsch", "acsch"),
    ("arccot", "acot"),
    ("arccoth", "acoth"),
    ("cosec", "csc"),
];

/// The historical `inverse_function_name` table (normalize/mod.rs history).
const OLD_INVERSES: &[(&str, &str)] = &[
    ("sin", "asin"),
    ("cos", "acos"),
    ("tan", "atan"),
    ("sec", "asec"),
    ("csc", "acsc"),
    ("cot", "acot"),
    ("sinh", "asinh"),
    ("cosh", "acosh"),
    ("tanh", "atanh"),
    ("sech", "asech"),
    ("csch", "acsch"),
    ("coth", "acoth"),
];

/// The historical MOVE_EXPONENT_OUTSIDE set (normalize/syntactic.rs history).
const OLD_MOVE_EXPONENT: &[&str] = &[
    "cos", "cosh", "sin", "sinh", "tan", "tanh", "sec", "sech", "csc", "csch", "cot", "coth",
    "log", "ln",
];

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

#[test]
fn text_parser_defaults_match_historical_list() {
    assert_eq!(
        sorted(special_functions::applied_text_names()),
        sorted(OLD_TEXT_APPLIED.iter().map(|s| s.to_string()).collect()),
    );
}

#[test]
fn latex_parser_defaults_match_historical_list() {
    assert_eq!(
        sorted(special_functions::applied_latex_names()),
        sorted(OLD_LATEX_APPLIED.iter().map(|s| s.to_string()).collect()),
    );
}

#[test]
fn canonical_name_matches_historical_normalizations() {
    for (alias, canon) in OLD_NORMALIZATIONS {
        assert_eq!(
            special_functions::canonical_name(alias),
            Some(*canon),
            "normalization of {alias:?}"
        );
    }
    // Canonical spellings and unknown names return None — the old tables'
    // contract (callers use None to mean "leave the head untouched").
    for name in ["log", "sin", "asin", "csc", "sqrt", "notafunction", ""] {
        assert_eq!(special_functions::canonical_name(name), None, "{name:?}");
    }
}

#[test]
fn inverse_of_matches_historical_table() {
    for (name, inv) in OLD_INVERSES {
        assert_eq!(
            special_functions::inverse_of(name),
            Some(*inv),
            "inverse of {name:?}"
        );
    }
    // Only the 12 canonical trig/hyperbolic spellings have notated inverses;
    // aliases (cosec) and inverse names themselves do not.
    for name in [
        "cosec",
        "asin",
        "arcsin",
        "log",
        "exp",
        "abs",
        "notafunction",
    ] {
        assert_eq!(special_functions::inverse_of(name), None, "{name:?}");
    }
}

#[test]
fn move_exponent_matches_historical_set() {
    for name in OLD_MOVE_EXPONENT {
        assert!(
            special_functions::moves_exponent_outside(name),
            "{name:?} must move exponents outside"
        );
    }
    // Spellings deliberately NOT in the historical set: `cosec` (alias of
    // csc but never listed), the inverse functions, and non-trig functions.
    for name in [
        "cosec",
        "asin",
        "arcsin",
        "exp",
        "sqrt",
        "abs",
        "notafunction",
    ] {
        assert!(!special_functions::moves_exponent_outside(name), "{name:?}");
    }
}

#[test]
fn precise_kernels_cover_historical_registry() {
    // The names the old eval_numeric/certified_digits/kernels.rs REGISTRY rows
    // listed, alias spellings included, all resolve — and to the same id as
    // their canonical spelling.
    for name in [
        "sqrt", "exp", "ln", "log", "abs", "sin", "cos", "tan", "asin", "arcsin", "acos", "arccos",
        "atan", "arctan", "sinh", "cosh", "tanh", "log10",
    ] {
        assert!(kernels::lookup(name).is_some(), "{name:?} lost its kernel");
    }
    assert_eq!(kernels::lookup("ln"), kernels::lookup("log"));
    assert_eq!(kernels::lookup("arcsin"), kernels::lookup("asin"));
    // Functions that never had precise kernels.
    for name in [
        "sec",
        "csc",
        "cosec",
        "sign",
        "floor",
        "atan2",
        "notafunction",
    ] {
        assert!(kernels::lookup(name).is_none(), "{name:?}");
    }
    // Every registered kernel id must be a valid index into the runtime
    // registry (the Op::Call id space).
    let n = kernels::registry().len();
    assert_eq!(n, 14, "kernel-bearing definitions");
    for def in special_functions::ALL {
        if let Some(k) = def.kernel {
            let id = kernels::lookup(def.name).expect("kernel def resolves");
            assert!(std::ptr::eq(kernels::registry()[id as usize], k));
        }
    }
}

#[test]
fn registry_has_no_duplicate_names_or_aliases() {
    let mut seen = std::collections::HashSet::new();
    for def in special_functions::ALL {
        for key in std::iter::once(&def.name).chain(def.aliases) {
            assert!(seen.insert(*key), "{key:?} registered twice");
        }
    }
}

#[test]
fn parse_spellings_are_name_or_alias() {
    // Every parser-default spelling must be the def's own name or one of its
    // aliases — a typo here would silently register a phantom function.
    // (`re`/`im` capitalize in LaTeX; the capitalized forms are aliases.)
    for def in special_functions::ALL {
        for s in def.parse_text.iter().chain(def.parse_latex) {
            assert!(
                *s == def.name || def.aliases.contains(s) || s.eq_ignore_ascii_case(def.name),
                "parse spelling {s:?} unrelated to function {:?}",
                def.name
            );
        }
    }
}

#[test]
fn derivative_templates_are_alias_aware() {
    // Spot checks against the historical calculus/diff.rs table, both spellings.
    assert_eq!(
        special_functions::derivative_template("sin"),
        Some("cos(x)")
    );
    assert_eq!(
        special_functions::derivative_template("arcsin"),
        special_functions::derivative_template("asin")
    );
    assert_eq!(special_functions::derivative_template("ln"), Some("1/x"));
    assert_eq!(special_functions::derivative_template("log"), Some("1/x"));
    // log10 historically had NO derivative template (prime-notation fallback).
    assert_eq!(special_functions::derivative_template("log10"), None);
    assert_eq!(special_functions::derivative_template("notafunction"), None);
}

#[test]
fn latex_commands_match_historical_tables() {
    // The function portion of the old ALLOWED_LATEX_SYMBOLS list plus the
    // old convert_latex_symbol arc-conversions (print/latex.rs history).
    for (spelling, cmd) in [
        ("sin", "sin"),
        ("csc", "csc"),
        ("sinh", "sinh"),
        ("ln", "ln"),
        ("log", "log"),
        ("log10", "log10"),
        ("sqrt", "sqrt"),
        ("abs", "abs"),
        ("erf", "erf"),
        ("arg", "arg"),
        ("det", "det"),
        ("Re", "Re"),
        ("Im", "Im"),
        ("asin", "arcsin"),
        ("arcsin", "arcsin"),
        ("acot", "arccot"),
        ("arccot", "arccot"),
    ] {
        assert_eq!(
            special_functions::latex_command(spelling),
            Some(cmd),
            "latex command for {spelling:?}"
        );
    }
    // Never had control words: inverse hyperbolics, lowercase re/im, cosec,
    // shape-rendered functions (floor renders \lfloor, not \floor).
    for spelling in [
        "asinh", "arcsinh", "re", "im", "cosec", "floor", "conj", "trace",
    ] {
        assert_eq!(
            special_functions::latex_command(spelling),
            None,
            "{spelling:?}"
        );
    }
}

#[test]
fn eval_coverage_matches_historical_known_function() {
    // The historical eval_numeric/complex.rs `known_function` arity-1 list…
    for name in [
        "sin",
        "cos",
        "tan",
        "sinh",
        "cosh",
        "tanh",
        "asin",
        "acos",
        "atan",
        "asinh",
        "acosh",
        "atanh",
        "sec",
        "csc",
        "cot",
        "sech",
        "csch",
        "coth",
        "asec",
        "acsc",
        "acot",
        "asech",
        "acsch",
        "acoth",
        "exp",
        "log",
        "log10",
        "sqrt",
        "cbrt",
        "abs",
        "sign",
        "conj",
        "re",
        "im",
        "arg",
        "floor",
        "ceil",
        "round",
        "trace",
        "factorial",
        "erf",
        // `det` joined the list in review: like `trace` its kernel is mathjs's
        // scalar identity (`det(2) = 2`), and without one `det(x)` was sampled
        // as an unknown, so it compared unequal to `x` where legacy said equal.
        // A `Matrix` argument does not come here at all — see
        // `matrix::scalar_reduction`, and `matrix.rs`'s equality tests.
        "det",
    ] {
        assert!(
            special_functions::eval1(name).is_some(),
            "{name:?} must evaluate"
        );
    }
    // …the arity-2 list…
    for name in ["atan2", "nthroot", "nCr", "nPr", "mod"] {
        assert!(
            special_functions::eval2(name).is_some(),
            "{name:?} must evaluate"
        );
    }
    // …and names deliberately NOT evaluable: aliases (evaluation runs on
    // canonicalized trees) and `rootof`, whose `Apply` spelling `canonicalize`
    // turns back into the `Expr::RootOf` leaf before any evaluator sees it.
    //
    // This half of the test pins a *decision*, not a fact about the outside
    // world, so it cannot notice a head that ought to evaluate and does not —
    // which is how `erf` and then `det` sat here while legacy evaluated both.
    // The check with outside authority is
    // `math-expressions-js-compat/spec/quick_doenet_compat_pr84.spec.ts`,
    // which runs the same expressions the JS library answered.
    for name in ["arcsin", "ln", "cosec", "rootof", "notafunction"] {
        assert!(special_functions::eval1(name).is_none(), "{name:?}");
    }
}

#[test]
fn antiderivative_builders_cover_historical_table() {
    // The 14 heads the integrator's elementary table matched, plus alias
    // spellings, minus everything else.
    for name in [
        "sin", "cos", "tan", "cot", "exp", "ln", "log", "log10", "sqrt", "sinh", "cosh", "tanh",
        "atan", "arctan", "asin", "arcsin", "acos", "arccos",
    ] {
        assert!(
            special_functions::antiderivative_builder(name).is_some(),
            "{name:?} lost its antiderivative"
        );
    }
    for name in ["sec", "csc", "asec", "abs", "cbrt", "notafunction"] {
        assert!(
            special_functions::antiderivative_builder(name).is_none(),
            "{name:?}"
        );
    }
}
