//! Simplifier corpus (PORTING_PLAN.md §7e).
//!
//! `fixtures/simplify-corpus.json` holds 342 real text inputs harvested from
//! `spec/slow_simplify.spec.js`, each paired with the tree JS `.simplify()`
//! produces (regenerate with `node scripts/generate-simplify-corpus.mjs`).
//!
//! The oracle is **own-reducedness**, not tree-match to JS (see §7e). For each
//! input we check three things about our `simplify`:
//!
//! - **meaning-preserving** — `equals(simplify(input), input)`, widened by the
//!   certified sample-point check ([`certified_verdicts`]) for the folds
//!   float sampling cannot confirm, and *narrowed* by the same check: a
//!   certified counterexample fails the test even when the result matches JS.
//!   A failure here is a correctness bug and is never acceptable (asserted, no
//!   snapshot).
//! - **reduced (fixpoint)** — `simplify(simplify(input)) == simplify(input)`
//!   structurally. Also a hard invariant of the design (asserted).
//! - **JS agreement (advisory)** — `equals(simplify(input), <JS's tree>)`. This is
//!   the reduction-progress signal: how often we reach something equal to JS's
//!   reduced form. It is *reported*, and its remaining gaps are snapshotted in
//!   `fixtures/simplify-known-failures.json` so we catch regressions and can
//!   shrink the list as clusters land (same pattern as the equality corpus).
//!
//! Regenerate the snapshot after an intended change:
//!   UPDATE_KNOWN_FAILURES=1 cargo test --test simplify_corpus

mod common;

use math_expressions::assumptions::Assumptions;
use math_expressions::{
    contains_blank, equals, eval_exact, expr, simplify, EqOptions, Expr, TextToAst,
    TextToAstOptions,
};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

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
    common::caught(f)
}

/// Does the tree involve a value outside `equals`'s finite-sampling domain — an
/// ∞/NaN constant, or a `Pow(0, negative)` division-by-zero pole?
fn involves_nonfinite(e: &Expr) -> bool {
    use math_expressions::Expr::{Const, Num, Pow};
    use math_expressions::MathConst::{Inf, NaN, NegInf};
    e.any_subexpr(&|c| match c {
        Const(k) => matches!(k, Inf | NegInf | NaN),
        Pow(b, x) => {
            matches!(&**b, Num(n) if n.to_f64() == 0.0)
                && matches!(&**x, Num(n) if n.to_f64() < 0.0)
        }
        _ => false,
    })
}

/// The integers the free symbols are pinned at, one round per entry (symbol
/// `i` of a round gets `base + 2i`, so a round's assignment is injective).
/// Deliberately not 0, ±1 or 2: `2 + 2 = 2·2 = 2² = 4`, so a symbol pinned at 2
/// makes a whole family of wrong rewrites agree.
const CERTIFY_POINTS: [i64; 3] = [3, 5, 11];

/// Exactly evaluate `input − simplified` once per entry in [`CERTIFY_POINTS`],
/// returning [`eval_exact::is_zero`]'s verdict for each: `Some(true)` =
/// certified zero there, `Some(false)` = certified *nonzero* there, `None` =
/// undecided (what a pole such as `1/(x−3)` at `x = 3` yields, and why a
/// missing verdict is tolerated rather than treated as failure).
///
/// The two directions are **not** equally strong, and the caller uses them
/// differently:
///
/// * one `Some(false)` is a *proof* that the rewrite changed the value — the
///   exact tower fully decided both sides at that point and they differ;
/// * all-`Some(true)` is *evidence only*. Agreement at finitely many points
///   cannot certify equality (`x·(x−3)` also vanishes at 3), so it is used
///   solely to widen the accepted set, layered on top of the many float points
///   `equals` already sampled — never as the sole warrant for a new rule.
///
/// It exists for the folds `equals` structurally cannot confirm: against a
/// folded `0` the residue `sin(π) ≈ 1.2e-16` has no relative tolerance that
/// closes. Deliberately not built out of `simplify`, which would let a
/// consistently-wrong special-value table rubber-stamp its own output:
/// `eval_exact::is_zero` runs `expand` → `canonicalize` → structural /
/// rational-normal-form / exact-constant stages and never invokes
/// `fold_special_values`, so the *pass* under test is not part of its own
/// oracle. The honest limit of that independence: `fold_special_values` reads
/// `eval_exact`'s trig/constant tables, so a wrong entry in **those** would
/// still be self-confirming. Independence is at the pass level, not the table
/// level — which is why the tables have their own direct tests
/// (`tests/exact_is_zero.rs`, `tests/special_values.rs`).
///
/// Each entry is paired with a rendering of the assignment it used, so a
/// failure can name the point rather than just asserting one exists.
fn certified_verdicts(input: &Expr, simplified: &Expr) -> Vec<(String, Option<bool>)> {
    let diff = Expr::Add(vec![input.clone(), Expr::Neg(Box::new(simplified.clone()))]);
    // `variables` reports the named constants (`pi`, `e`, `i`) too; substituting
    // those would destroy the very special values this check exists to certify.
    // Same filter `eval_exact` itself uses to decide what counts as free.
    let free: Vec<String> = math_expressions::variables(&diff)
        .into_iter()
        .filter(|v| !math_expressions::expr::sym::is_constant_symbol(v))
        .collect();
    CERTIFY_POINTS
        .iter()
        .map(|base| {
            let subs: HashMap<String, Expr> = free
                .iter()
                .enumerate()
                .map(|(i, v)| (v.clone(), Expr::int(base + 2 * i as i64)))
                .collect();
            let at = math_expressions::substitute(&diff, &subs);
            let where_ = if free.is_empty() {
                "no free symbols".to_string()
            } else {
                free.iter()
                    .enumerate()
                    .map(|(i, v)| format!("{v} = {}", base + 2 * i as i64))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            (where_, eval_exact::is_zero(&at, &Assumptions::new()))
        })
        .collect()
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
        let simplified = match catch(|| simplify(&parsed)) {
            Some(s) => s,
            None => {
                // A panicking simplify must fail the invariants test loudly —
                // silently skipping the case would hide crashes on real inputs.
                assert!(
                    !assert_invariants,
                    "simplify PANICKED on corpus input {:?}",
                    c.input
                );
                continue;
            }
        };
        let want = expr::serde::try_from_js(&c.tree).expect("fixture tree");
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
            // real-domain correctness oracle (§7e).
            //
            // The `agrees ||` escape used to be an unchecked hole: a rewrite
            // that changes meaning but happens to reproduce JS's tree could not
            // be told apart from a sanctioned real-domain identity. The
            // certified verdicts below now close it for everything the exact
            // tower can decide — a certified counterexample fails the test
            // regardless of what JS produced. What remains uncovered is the
            // class `eval_exact` answers `None` on; if a rule is ever authored
            // by pattern-matching JS output, tighten the escape to an explicit
            // allowlist of sanctioned divergences (or a real-domain `equals`
            // mode) instead. `Blank` inputs are exempt (the equals stage-0
            // guard rejects them outright), as are non-finite results
            // (∞/NaN/poles): `equals` samples finite complex points and has no
            // verdict there, so it cannot judge meaning either way.
            let judgeable = !contains_blank(&parsed)
                && !contains_blank(&simplified)
                && !involves_nonfinite(&parsed)
                && !involves_nonfinite(&simplified);
            if judgeable {
                let verdicts =
                    catch(|| certified_verdicts(&parsed, &simplified)).unwrap_or_default();
                // Proof direction: `input − simplified` is certified *nonzero*
                // at a point, so the rewrite is wrong however JS spells it.
                // This is the one check with no snapshot escape, so the message
                // has to carry everything the person who trips it needs.
                if let Some((at, _)) = verdicts.iter().find(|(_, v)| *v == Some(false)) {
                    panic!(
                        "simplify changed the VALUE of {:?}\n  \
                         result:      {:?}\n  \
                         refuted at:  {}\n\
                         `input - simplified` evaluates to a certified NONZERO constant there \
                         (eval_exact decided both sides exactly), so this is a proof the rewrite \
                         is wrong -- not a sampling tolerance artifact, and not excused by \
                         matching JS's tree.\n\
                         There is deliberately no known-failures snapshot for this direction: \
                         fix the rule. If the divergence really is a sanctioned real-domain \
                         identity that the complex-domain exact tower is entitled to refute, \
                         narrow the `judgeable` guard above explicitly and record why.",
                        c.input, simplified, at,
                    );
                }
                // Evidence direction: certified zero at every sample point.
                // This is what accepts the sound special-value folds
                // (`sin(π)·x → 0`) that the float-sampling `equals` cannot
                // confirm — `sin(π)` samples as ~1e-16, and against a folded
                // `0` the relative comparison never closes. JS never folded
                // them either, so `agrees` is false too.
                let certified =
                    !verdicts.is_empty() && verdicts.iter().all(|(_, v)| *v == Some(true));
                let preserves = agrees
                    || catch(|| equals(&simplified, &parsed, &opts)).unwrap_or(false)
                    || certified;
                assert!(
                    preserves,
                    "simplify changed the meaning of {:?}: got {:?}\n  \
                     JS agreement: {agrees}; certified verdicts: {verdicts:?}\n\
                     (`None` = eval_exact could not decide the difference at that point, e.g. a \
                     pole or an opaque function head; all-`Some(true)` would have accepted.)",
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
    collect_js_gaps(true);
}

/// The advisory JS-agreement gaps, guarded against regression by a snapshot.
#[test]
fn simplify_no_js_agreement_regressions() {
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
