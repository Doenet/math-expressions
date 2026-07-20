//! Differential corpus for the assumptions queries (PORTING_PLAN.md §11).
//! Each case: an assumption context, an expression, and the JS verdict
//! ("T"/"F"/"U") for all eight three-valued queries. Our inference is a
//! clean-slate engine, so exact agreement is not required — but it must be
//! SOUND relative to JS: wherever both sides are definite (T/F) they must
//! agree, and our-definite-where-JS-unknown is allowed only as a snapshot
//! (each such case is an intentional strengthening or a divergence to
//! review). Regenerate corpus: `node scripts/generate-assumptions-corpus.mjs`;
//! snapshot: `UPDATE_KNOWN_FAILURES=1 cargo test --test assumptions_corpus`.

use math_expressions::{
    is_complex, is_integer, is_negative, is_nonnegative, is_nonpositive, is_nonzero, is_positive,
    is_real, Assumptions, Expr, TextToAst, TextToAstOptions,
};
use std::collections::BTreeSet;

fn parse(s: &str) -> Expr {
    TextToAst::new(TextToAstOptions::default())
        .convert(s)
        .unwrap_or_else(|e| panic!("parse {s:?}: {e}"))
}

#[derive(serde::Deserialize)]
struct Case {
    assume: Option<String>,
    expr: String,
    verdicts: std::collections::BTreeMap<String, String>,
}

const CORPUS: &str = include_str!("fixtures/assumptions-corpus.json");
const KNOWN: &str = include_str!("fixtures/assumptions-known-divergences.json");

fn tri_to_str(v: Option<bool>) -> &'static str {
    match v {
        Some(true) => "T",
        Some(false) => "F",
        None => "U",
    }
}

fn run_query(name: &str, e: &Expr, a: &Assumptions) -> Option<bool> {
    match name {
        "integer" => is_integer(e, a),
        "real" => is_real(e, a),
        "complex" => is_complex(e, a),
        "nonzero" => is_nonzero(e, a),
        "nonnegative" => is_nonnegative(e, a),
        "positive" => is_positive(e, a),
        "negative" => is_negative(e, a),
        "nonpositive" => is_nonpositive(e, a),
        _ => unreachable!(),
    }
}

/// (conflicts, divergences): a conflict is T-vs-F (unsoundness — always
/// asserted empty); a divergence is definite-vs-unknown in either direction
/// (snapshot-guarded).
fn collect() -> (Vec<String>, BTreeSet<String>) {
    let cases: Vec<Case> = serde_json::from_str(CORPUS).unwrap();
    let mut conflicts = Vec::new();
    let mut divergences = BTreeSet::new();
    for c in &cases {
        let mut a = Assumptions::new();
        if let Some(s) = &c.assume {
            a.add(&parse(s));
        }
        let e = parse(&c.expr);
        for (query, js) in &c.verdicts {
            let ours = tri_to_str(run_query(query, &e, &a));
            let js = js.as_str();
            if ours == js {
                continue;
            }
            let key = format!(
                "{} | {} | {query}: js={js} rust={ours}",
                c.assume.as_deref().unwrap_or("-"),
                c.expr
            );
            if ours != "U" && js != "U" {
                conflicts.push(key);
            } else {
                divergences.insert(key);
            }
        }
    }
    (conflicts, divergences)
}

#[test]
fn no_definite_conflicts_with_js() {
    let (conflicts, _) = collect();
    assert!(
        conflicts.is_empty(),
        "{} definite T-vs-F conflicts with the JS oracle:\n{}",
        conflicts.len(),
        conflicts.join("\n"),
    );
}

#[test]
fn divergences_are_snapshotted() {
    let (_, divergences) = collect();
    if std::env::var("UPDATE_KNOWN_FAILURES").is_ok() {
        let list: Vec<&String> = divergences.iter().collect();
        std::fs::write(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/assumptions-known-divergences.json"
            ),
            serde_json::to_string_pretty(&list).unwrap() + "\n",
        )
        .unwrap();
        eprintln!("updated snapshot: {} divergences", divergences.len());
        return;
    }
    let known: BTreeSet<String> = serde_json::from_str::<Vec<String>>(KNOWN)
        .unwrap()
        .into_iter()
        .collect();
    let new: Vec<&String> = divergences.difference(&known).collect();
    let fixed: Vec<&String> = known.difference(&divergences).collect();
    if !fixed.is_empty() {
        eprintln!(
            "{} known divergences now agree — prune (UPDATE_KNOWN_FAILURES=1)",
            fixed.len()
        );
    }
    assert!(
        new.is_empty(),
        "{} NEW divergences from the JS oracle:\n{}",
        new.len(),
        new.iter().take(40).map(|k| format!("  {k}")).collect::<Vec<_>>().join("\n"),
    );
}

#[test]
fn assumptions_corpus_agreement_rate() {
    let cases: Vec<Case> = serde_json::from_str(CORPUS).unwrap();
    let total: usize = cases.iter().map(|c| c.verdicts.len()).sum();
    let (conflicts, divergences) = collect();
    eprintln!(
        "assumptions corpus: {}/{} verdicts agree with JS ({} conflicts, {} definite/unknown divergences)",
        total - conflicts.len() - divergences.len(),
        total,
        conflicts.len(),
        divergences.len(),
    );
}
