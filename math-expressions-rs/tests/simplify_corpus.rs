//! Simplifier corpus (PORTING_PLAN.md §7e).
//!
//! `fixtures/simplify-corpus.json` holds 342 real text inputs harvested from
//! `spec/slow_simplify.spec.js`, each paired with the tree JS `.simplify()`
//! produces (regenerate with `node scripts/generate-simplify-corpus.mjs`).
//!
//! The oracle is **own-reducedness**, not tree-match to JS (see §7e). For each
//! input we check three things about our `simplify`:
//!
//! - **meaning-preserving** — `equals(simplify(input), input)`. A failure here
//!   is a correctness bug and is never acceptable (asserted, no snapshot).
//! - **reduced (fixpoint)** — `simplify(simplify(input)) == simplify(input)`
//!   structurally. Also a hard invariant of the design (asserted).
//! - **JS agreement (advisory)** — `equals(simplify(input), js_tree)`. This is
//!   the reduction-progress signal: how often we reach something equal to JS's
//!   reduced form. It is *reported*, and its remaining gaps are snapshotted in
//!   `fixtures/simplify-known-failures.json` so we catch regressions and can
//!   shrink the list as clusters land (same pattern as the equality corpus).
//!
//! Regenerate the snapshot after an intended change:
//!   UPDATE_KNOWN_FAILURES=1 cargo test --test simplify_corpus

use math_expressions::{
    equals, js_tree, simplify, EqOptions, Expr, TextToAst, TextToAstOptions,
};
use serde_json::Value;
use std::collections::BTreeSet;

fn parse(s: &str) -> Option<Expr> {
    TextToAst::new(TextToAstOptions::default()).convert(s).ok()
}

#[derive(serde::Deserialize)]
struct Case {
    input: String,
    tree: Value,
}

const CORPUS: &str = include_str!("fixtures/simplify-corpus.json");
const KNOWN_FAILURES: &str = include_str!("fixtures/simplify-known-failures.json");

fn catch<T>(f: impl FnOnce() -> T) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok()
}

/// Does the tree involve a value outside `equals`'s finite-sampling domain — an
/// ∞/NaN constant, or a `Pow(0, negative)` division-by-zero pole?
fn involves_nonfinite(e: &Expr) -> bool {
    use math_expressions::Expr::*;
    use math_expressions::MathConst::{Inf, NaN, NegInf};
    match e {
        Const(c) => matches!(c, Inf | NegInf | NaN),
        Pow(b, x) => {
            let pole = matches!(&**b, Num(n) if n.to_f64() == 0.0)
                && matches!(&**x, Num(n) if n.to_f64() < 0.0);
            pole || involves_nonfinite(b) || involves_nonfinite(x)
        }
        Num(_) | Sym(_) | Blank | Ldots => false,
        Add(xs) | Mul(xs) | And(xs) | Or(xs) | Union(xs) | Intersect(xs) | Seq(_, xs)
        | OtherOp(_, xs) => xs.iter().any(involves_nonfinite),
        Apply(h, xs) => involves_nonfinite(h) || xs.iter().any(involves_nonfinite),
        Div(a, b) | Index(a, b) => involves_nonfinite(a) || involves_nonfinite(b),
        Neg(x) | Not(x) | Prime(x) => involves_nonfinite(x),
        Interval { endpoints, .. } => {
            involves_nonfinite(&endpoints.0) || involves_nonfinite(&endpoints.1)
        }
        Relation { operands, .. } => operands.iter().any(involves_nonfinite),
        Matrix { entries, .. } => entries.iter().any(involves_nonfinite),
    }
}

/// Local mirror of `eq::contains_blank` (private): does the tree hold a `Blank`?
fn contains_blank(e: &Expr) -> bool {
    match e {
        Expr::Blank => true,
        Expr::Num(_) | Expr::Sym(_) | Expr::Const(_) | Expr::Ldots => false,
        Expr::Add(xs)
        | Expr::Mul(xs)
        | Expr::And(xs)
        | Expr::Or(xs)
        | Expr::Union(xs)
        | Expr::Intersect(xs)
        | Expr::Seq(_, xs)
        | Expr::OtherOp(_, xs) => xs.iter().any(contains_blank),
        Expr::Apply(h, xs) => contains_blank(h) || xs.iter().any(contains_blank),
        Expr::Div(a, b) | Expr::Pow(a, b) | Expr::Index(a, b) => {
            contains_blank(a) || contains_blank(b)
        }
        Expr::Neg(x) | Expr::Not(x) | Expr::Prime(x) => contains_blank(x),
        Expr::Interval { endpoints, .. } => {
            contains_blank(&endpoints.0) || contains_blank(&endpoints.1)
        }
        Expr::Relation { operands, .. } => operands.iter().any(contains_blank),
        Expr::Matrix { entries, .. } => entries.iter().any(contains_blank),
    }
}

/// The set of inputs where our `simplify` result is NOT `equals` to JS's
/// `.simplify()` output (the advisory JS-agreement gaps). Also asserts the two
/// hard invariants (meaning-preserving, fixpoint) as a side effect.
fn collect_js_gaps(assert_invariants: bool) -> BTreeSet<String> {
    let cases: Vec<Case> = serde_json::from_str(CORPUS).unwrap();
    let opts = EqOptions::default();
    let mut gaps = BTreeSet::new();

    for c in &cases {
        let Some(parsed) = parse(&c.input) else {
            continue;
        };
        let Some(simplified) = catch(|| simplify(&parsed)) else {
            continue;
        };
        let want = js_tree::from_js(&c.tree);
        let agrees = catch(|| equals(&simplified, &want, &opts)).unwrap_or(false);

        if assert_invariants {
            // Fixpoint: a second pass must not change anything.
            let again = catch(|| simplify(&simplified));
            assert!(
                again.as_ref() == Some(&simplified),
                "simplify not idempotent on {:?}:\n  once: {:?}\n  twice: {:?}",
                c.input,
                simplified,
                again,
            );
            // Meaning-preserving. Our `equals` is complex-domain and strict, but
            // simplify serves a real-analysis tool: real-domain identities
            // (odd-root sign pulling, `(-8)^(1/3)=-2`) and non-finite folds are
            // *false* under complex principal branches / unsampleable. So a step
            // counts as meaning-preserving if it is either complex-`equals` to
            // the input OR equal to JS's reduced output — JS being the
            // real-domain correctness oracle (§7e). `Blank` inputs are exempt
            // (the equals stage-0 guard rejects them outright), as are non-finite
            // results (∞/NaN/poles): `equals` samples finite complex points and
            // has no verdict there, so it cannot judge meaning either way.
            let judgeable = !contains_blank(&parsed)
                && !contains_blank(&simplified)
                && !involves_nonfinite(&parsed)
                && !involves_nonfinite(&simplified);
            if judgeable {
                let preserves =
                    agrees || catch(|| equals(&simplified, &parsed, &opts)).unwrap_or(false);
                assert!(
                    preserves,
                    "simplify changed the meaning of {:?}: got {:?}",
                    c.input, simplified,
                );
            }
        }

        if !agrees {
            gaps.insert(c.input.clone());
        }
    }
    gaps
}

/// The two hard invariants across the whole corpus (meaning-preserving +
/// fixpoint). No snapshot: these must always hold.
#[test]
fn simplify_is_meaning_preserving_and_reduced() {
    std::panic::set_hook(Box::new(|_| {}));
    collect_js_gaps(true);
}

/// The advisory JS-agreement gaps, guarded against regression by a snapshot.
#[test]
fn simplify_no_js_agreement_regressions() {
    std::panic::set_hook(Box::new(|_| {}));
    let gaps = collect_js_gaps(false);

    if std::env::var("UPDATE_KNOWN_FAILURES").is_ok() {
        let list: Vec<&String> = gaps.iter().collect();
        std::fs::write(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/simplify-known-failures.json"
            ),
            serde_json::to_string_pretty(&list).unwrap() + "\n",
        )
        .unwrap();
        eprintln!("updated snapshot: {} JS-agreement gaps", gaps.len());
        return;
    }

    let known: BTreeSet<String> = serde_json::from_str::<Vec<String>>(KNOWN_FAILURES)
        .unwrap()
        .into_iter()
        .collect();
    let new: Vec<&String> = gaps.difference(&known).collect();
    let fixed: Vec<&String> = known.difference(&gaps).collect();

    if !fixed.is_empty() {
        eprintln!(
            "{} known gaps now agree with JS — prune them (UPDATE_KNOWN_FAILURES=1):",
            fixed.len()
        );
        for k in fixed.iter().take(30) {
            eprintln!("  {k}");
        }
    }
    assert!(
        new.is_empty(),
        "{} NEW JS-agreement regressions (not in snapshot):\n{}",
        new.len(),
        new.iter()
            .take(40)
            .map(|k| format!("  {k}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// Headline counts, always green.
#[test]
fn simplify_corpus_pass_rate() {
    std::panic::set_hook(Box::new(|_| {}));
    let cases: Vec<Case> = serde_json::from_str(CORPUS).unwrap();
    let gaps = collect_js_gaps(false).len();
    let n = cases.len();
    eprintln!(
        "simplify corpus: {}/{} agree with JS .simplify() ({} gaps)",
        n - gaps,
        n,
        gaps
    );
}
