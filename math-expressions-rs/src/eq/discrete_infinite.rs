//! Stage 4 of `equals`: discrete infinite sets (PORTING_PLAN.md §10/§17).
//!
//! A *discrete infinite set* is a union of arithmetic progressions
//! `{offset + k·period : k ∈ ℤ}`, encoded as
//! `OtherOp("discrete_infinite_set", [Seq(Tuple, [offset, period, min_index,
//! max_index]), …])` — the shape `create_discrete_infinite_set` builds and the
//! JS `["discrete_infinite_set", ["tuple", o, p, min, max], …]` maps to. These
//! represent periodic solution sets (`x = π/4 + nπ`).
//!
//! Equality is mutual containment. Containment of one progression in a union
//! is decided by normalizing everything by the progression's period and
//! checking that all residue classes are covered (port of
//! `lib/expression/equality/discrete_infinite_set.js`). Offsets may be
//! symbolic (`a`, `a+3`): only *differences* and *ratios* must simplify to
//! numbers, which our canonical like-term/like-power combining does exactly
//! (`(π/4)/π → 1/4`, `(a+3)/3 − a/3 → 1`).
//!
//! Divergence (documented): the JS requires an explicit `c ≠ 0` assumption to
//! fold `2c/c`; our assumption-free canonicalizer folds it unconditionally, so
//! such pairs compare equal without assumptions (consistent with the crate's
//! `x/x → 1` divergence class). Bound (JS has none): normalized period
//! numerators are capped at 10 000 — beyond that we conservatively report
//! not-contained rather than loop over enormous residue classes.

use crate::expr::{Expr, MathConst, SeqKind};
// Canonical-shape variants: this module pattern-matches `Expr::Num` etc. on
// the results, so it must not receive the display (`present`) form.
use crate::norm::{expand_core, simplify_core};
use crate::num::Number;

use super::EqOptions;

/// Minimum listed elements for the `{a, a+p, a+2p, …}`-vs-list comparison.
const MIN_ELEMENTS_MATCH: usize = 3;

/// Boolean stage-4 entry for the `equals` chain. Symmetric: tries both
/// orientations (the JS method is receiver-oriented; a set compared with a
/// listed sequence works whichever side the set is on).
pub(super) fn equals_discrete_infinite(a: &Expr, b: &Expr, opts: &EqOptions) -> bool {
    match_discrete_infinite(a, b, opts, false) >= 1.0
        || match_discrete_infinite(b, a, opts, false) >= 1.0
}

/// Full/partial match score in `[0, 1]` (1 = equal; fractions arise only with
/// `match_partial`, mirroring the JS partial-credit API). `a` must be a
/// discrete infinite set; `b` is another set or a list ending in `…`.
pub fn match_discrete_infinite(a: &Expr, b: &Expr, opts: &EqOptions, match_partial: bool) -> f64 {
    let Some(a_tuples) = as_discrete_infinite_set(a) else {
        return 0.0;
    };

    if let Some(b_tuples) = as_discrete_infinite_set(b) {
        if match_partial {
            let m1 = contained_in_set(&a_tuples, &b_tuples, true);
            let m2 = contained_in_set(&b_tuples, &a_tuples, true);
            if m1 == 0.0 || m2 == 0.0 {
                return 0.0;
            }
            return m1.min(m2);
        }
        let both = contained_in_set(&a_tuples, &b_tuples, false) >= 1.0
            && contained_in_set(&b_tuples, &a_tuples, false) >= 1.0;
        return if both { 1.0 } else { 0.0 };
    }

    // `b` as a listed sequence: Seq(List, [e1, …, en, Ldots]).
    match sequence_matches_list(&a_tuples, b, opts) {
        true => 1.0,
        false => 0.0,
    }
}

/// One progression: offset + period·k for k in [min_index, max_index].
struct Progression<'a> {
    offset: &'a Expr,
    period: &'a Expr,
    min_index: &'a Expr,
    max_index: &'a Expr,
}

/// Is `e` a discrete infinite set (for the equals chain's type dispatch)?
pub(super) fn is_discrete_infinite_set(e: &Expr) -> bool {
    as_discrete_infinite_set(e).is_some()
}

/// Recognize the set shape and pull out its progressions.
fn as_discrete_infinite_set(e: &Expr) -> Option<Vec<Progression<'_>>> {
    let Expr::OtherOp(name, args) = e else {
        return None;
    };
    if name.name() != "discrete_infinite_set" || args.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(args.len());
    for a in args {
        let Expr::Seq(SeqKind::Tuple, xs) = a else {
            return None;
        };
        let [offset, period, min_index, max_index] = xs.as_slice() else {
            return None;
        };
        out.push(Progression {
            offset,
            period,
            min_index,
            max_index,
        });
    }
    Some(out)
}

/// Fraction of `a`'s progressions contained in the union `b` (1.0 = all).
/// With `match_partial`, a partially-covered progression contributes its
/// covered fraction (JS: `num_matches += match`); without it, any piece
/// short of full containment fails the whole set.
fn contained_in_set(a: &[Progression], b: &[Progression], match_partial: bool) -> f64 {
    let mut matched = 0.0f64;
    for piece in a {
        let m = progression_contained(piece, b, match_partial);
        if m < 1.0 && !match_partial {
            return 0.0;
        }
        matched += m;
    }
    matched / a.len() as f64
}

/// How much of the single progression `a` (over all of ℤ) is contained in the
/// union `b`: 1.0 = fully, and with `match_partial` the best fraction of
/// residue classes covered (0.0 otherwise). Everything is normalized by `a`'s
/// period; then `a` is the integer lattice `r0 + ℤ` and each `b` progression
/// covers the residue classes `j (mod p_i)` whose offset difference is a
/// multiple of its normalized period `p_i/q_i`.
fn progression_contained(a: &Progression, b: &[Progression], match_partial: bool) -> f64 {
    // Implemented (like the JS) only for full-ℤ progressions.
    if !is_neg_inf(a.min_index) || !is_pos_inf(a.max_index) {
        return 0.0;
    }

    let r0 = ratio(a.offset, a.period);

    // (p, q, normalized offset, normalized period value) per b-progression.
    let mut data: Vec<(i64, i64, Expr, f64)> = Vec::new();
    for t in b {
        if !is_neg_inf(t.min_index) || !is_pos_inf(t.max_index) {
            return 0.0;
        }
        let off = ratio(t.offset, a.period);
        let per = ratio(t.period, a.period);
        let Expr::Num(n) = &per else {
            return 0.0; // period ratio must be numeric
        };
        let Some((p, q)) = frac_parts(n) else {
            return 0.0;
        };
        if p == 0 || p > crate::resource_limits::current().max_residues {
            return 0.0; // degenerate or beyond the residue cap
        }
        let per_val = (p as f64) / (q as f64);
        data.push((p, q, off, per_val));
    }
    data.sort_by_key(|d| d.0);

    // Progressions whose normalized period divides 1 (p == 1): a single offset
    // match covers everything. Non-matching ones are dropped (as in the JS).
    while let Some(first) = data.first() {
        if first.0 != 1 {
            break;
        }
        let (_, _, off, per_val) = data.remove(0);
        if offsets_align(&off, &r0, 0, per_val) {
            return 1.0;
        }
        if data.is_empty() {
            return 0.0;
        }
    }

    // General covering: for each candidate residue count base_p, try to cover
    // all classes j (mod base_p) using progressions whose p divides base_p.
    // With `match_partial`, track the best covered fraction across base_ps.
    let mut ps: Vec<i64> = data.iter().map(|d| d.0).collect();
    ps.dedup();
    let mut max_fraction_covered = 0.0f64;
    for &base_p in &ps {
        let mut covered = vec![false; base_p as usize];
        for (p, _, off, per_val) in data.iter().filter(|d| base_p % d.0 == 0) {
            let m = base_p / p;
            for j in 0..*p {
                if offsets_align(off, &r0, j, *per_val) {
                    for k in 0..m {
                        covered[(j + k * p) as usize] = true;
                    }
                    if covered.iter().all(|&c| c) {
                        return 1.0;
                    }
                    break;
                }
            }
        }
        if match_partial {
            let fraction = covered.iter().filter(|&&c| c).count() as f64 / base_p as f64;
            max_fraction_covered = max_fraction_covered.max(fraction);
        }
    }
    if match_partial {
        max_fraction_covered
    } else {
        0.0
    }
}

/// Does `off − (r0 + j)` reduce to a numeric multiple of `period` (within the
/// JS's 1e-10 relative tolerance)? Symbolic parts must cancel exactly
/// (`(a+3)/3 − a/3 → 1`); a residual symbol means "cannot verify" → false.
fn offsets_align(off: &Expr, r0: &Expr, j: i64, period: f64) -> bool {
    let target = Expr::Add(vec![r0.clone(), Expr::int(j)]);
    let diff = simplify_core(&expand_core(&Expr::Add(vec![
        off.clone(),
        Expr::Neg(Box::new(target)),
    ])));
    let Expr::Num(n) = &diff else {
        return false;
    };
    let d = n.to_f64();
    let p = period.abs();
    if !d.is_finite() || !p.is_finite() || p == 0.0 {
        return false;
    }
    let m = d.rem_euclid(p);
    m.min(p - m) < 1e-10 * p
}

/// `simplify(e / period)` — exact when the symbolic parts cancel.
fn ratio(e: &Expr, period: &Expr) -> Expr {
    simplify_core(&Expr::Div(Box::new(e.clone()), Box::new(period.clone())))
}

/// |numerator| and denominator of an exact small rational.
fn frac_parts(n: &Number) -> Option<(i64, i64)> {
    match n {
        Number::Int(i) => Some((i.abs(), 1)),
        Number::Rat(num, den) => Some((num.abs(), *den)),
        _ => None, // Big/Float periods: conservatively unsupported
    }
}

fn is_pos_inf(e: &Expr) -> bool {
    matches!(e, Expr::Const(MathConst::Inf))
        || matches!(e, Expr::Num(n) if n.to_f64() == f64::INFINITY)
}

fn is_neg_inf(e: &Expr) -> bool {
    matches!(e, Expr::Const(MathConst::NegInf))
        || matches!(e, Expr::Num(n) if n.to_f64() == f64::NEG_INFINITY)
}

/// Compare a single-progression set (with integer `min_index`) against a
/// listed sequence `e1, e2, …, en, …` — the user-typable form. The first `n`
/// generated elements must equal the listed ones (via full `equals`).
fn sequence_matches_list(a: &[Progression], b: &Expr, opts: &EqOptions) -> bool {
    let Expr::Seq(SeqKind::List, xs) = b else {
        return false;
    };
    if xs.len() < 2 || !matches!(xs.last(), Some(Expr::Ldots)) {
        return false;
    }
    let listed = &xs[..xs.len() - 1];
    if listed.len() < MIN_ELEMENTS_MATCH {
        return false;
    }

    // Generation implemented only for a single progression with an integer
    // start index and no upper bound (like the JS).
    let [p] = a else { return false };
    if !is_pos_inf(p.max_index) {
        return false;
    }
    let Expr::Num(Number::Int(min)) = simplify_core(p.min_index) else {
        return false;
    };

    let generated: Vec<Expr> = (0..listed.len() as i64)
        .map(|i| {
            simplify_core(&Expr::Add(vec![
                Expr::Mul(vec![p.period.clone(), Expr::int(min + i)]),
                p.offset.clone(),
            ]))
        })
        .collect();

    super::equals(
        &Expr::Seq(SeqKind::List, generated),
        &Expr::Seq(SeqKind::List, listed.to_vec()),
        opts,
    )
}

/// Build a discrete infinite set from offsets/periods (port of
/// `lib/expression/sets.js`). `offsets` and `periods` may be lists: a list of
/// offsets shares one period unless `periods` is a matching-length list.
/// Defaults: indices over all of ℤ. Returns `None` for mismatched lists.
pub fn create_discrete_infinite_set(
    offsets: &Expr,
    periods: &Expr,
    min_index: Option<&Expr>,
    max_index: Option<&Expr>,
) -> Option<Expr> {
    let min = min_index
        .cloned()
        .unwrap_or(Expr::Const(MathConst::NegInf));
    let max = max_index.cloned().unwrap_or(Expr::Const(MathConst::Inf));

    let offsets_list: Vec<&Expr> = match offsets {
        Expr::Seq(SeqKind::List, xs) => xs.iter().collect(),
        other => vec![other],
    };
    let periods_list: Vec<&Expr> = match periods {
        Expr::Seq(SeqKind::List, xs) => xs.iter().collect(),
        other => vec![other],
    };
    if periods_list.len() != 1 && periods_list.len() != offsets_list.len() {
        return None;
    }

    let tuples = offsets_list
        .iter()
        .enumerate()
        .map(|(i, off)| {
            let period = if periods_list.len() == 1 {
                periods_list[0]
            } else {
                periods_list[i]
            };
            Expr::Seq(
                SeqKind::Tuple,
                vec![(*off).clone(), period.clone(), min.clone(), max.clone()],
            )
        })
        .collect();

    Some(Expr::OtherOp(
        crate::sym::Sym::new("discrete_infinite_set"),
        tuples,
    ))
}
