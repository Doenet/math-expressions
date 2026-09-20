//! `me.perform_vector_matrix_additions_scalar_multiplications` — the shape pass
//! answer grading runs *before* it slices an expression into components.
//!
//! It does not do arithmetic; it moves `+` and scalar `*` **inside** vector and
//! matrix containers so the container ends up on the outside:
//!
//! ```text
//! (1,2)+(3,4)  →  (1+3, 2+4)        componentwise, container stays a tuple
//! 3(1,2)       →  (1·3, 2·3)        scalar distributed, scalar on the right
//! a(x,y)       →  (x·a, y·a)
//! x+y          →  x+y               nothing to move
//! ```
//!
//! Why grading needs it: `checkEquality` reads `tree[0]` and branches on the
//! top-level operator to split the answer into per-component ASTs for partial
//! credit. Left as `["+", ["tuple",…], ["tuple",…]]`, the top is still `+`, the
//! componentwise path never engages, and grading collapses. This is a *shape*
//! precondition, not a math step — `simplify` would overshoot, folding `(1+3,
//! 2+4)` to `(4,6)` and changing what the tolerance comparison sees.
//!
//! Port of `perform_vector_matrix_additions_scalar_multiplications` /
//! `perform_vector_scalar_multiplications` / `perform_matrix_scalar_multiplications`
//! (legacy `simplify.js` / `transformation.js`). Faithful points worth knowing:
//!
//! - a scalar lands on the **right** of each component (`["*", comp, scalar]`),
//!   and several scalars nest outward from the container (`2·3·v`'s component is
//!   `(comp·3)·2`), matching the legacy pre/post-factor unwinding;
//! - a literal `1` factor is dropped rather than distributed, so `1·(1,2)` is
//!   `(1,2)` and not `(1·1, 2·1)` (legacy's `if (preFactors[i] !== 1)` guard);
//! - in a product holding several containers, the one that gets the scalars is
//!   the first that can actually absorb one — `(1,2)·⟨3,4⟩·5` distributes the
//!   `5` into the altvector, because the leading tuple is walled off from it;
//! - only `tuple`/`vector`/`altvector` are vector containers, and only the
//!   `["matrix", …]` spelling is a matrix — an array-of-arrays matrix is left
//!   alone, exactly as legacy did (the name oversells the matrix half);
//! - when unlike vector containers combine, the result type is `vector` if any
//!   addend was a vector (or a tuple and an altvector met), else `altvector` if
//!   any was one, else `tuple`.

use crate::expr::map_children;
use crate::expr::{Expr, Mat, SeqKind};

/// Move `+`/scalar-`*` inside vector and matrix containers, bottom-up.
///
/// The three legacy phases (matrix-scalar, vector-scalar, addition) each run as
/// their own bottom-up transform over the whole tree; applying all three at each
/// node in one post-order walk is equivalent, because every rewrite is local to
/// its node and reads only already-transformed children.
///
/// **Flattened first.** Every rewrite below reads the operand list of a *single*
/// `Add`/`Mul` node, but `Expr` keeps those left-nested the way the text parser
/// built them — `(1,2)+(3,4)+x` arrives as `Add[Add[(1,2),(3,4)], x]`, where no
/// one node sees both containers *and* the scalar. Without this the pass would
/// silently no-op on any sum or product of three or more operands built by
/// `parse_text` (the JS tree spelling is already n-ary, so `from_ast` input
/// happened to work — the two entry points disagreed).
pub fn perform_vector_matrix_additions_scalar_multiplications(e: &Expr) -> Expr {
    distribute(&crate::expr::flatten(e.clone()))
}

fn distribute(e: &Expr) -> Expr {
    let e = map_children(e, distribute);
    let e = matrix_scalar_mult(&e);
    let e = vector_scalar_mult(&e);
    vector_matrix_addition(&e)
}

/// A vector container's kind, or `None` for anything else. `list`/`set` are
/// sequences but *not* vectors here — only these three distribute.
fn vector_kind(e: &Expr) -> Option<SeqKind> {
    match e {
        Expr::Seq(k @ (SeqKind::Tuple | SeqKind::Vector | SeqKind::AltVector), _) => Some(*k),
        _ => None,
    }
}

/// Heads that stop scalar consumption: any sequence, a matrix, or an interval.
/// A factor that is not one of these is a scalar and gets folded into the
/// container's components.
fn is_container(e: &Expr) -> bool {
    matches!(e, Expr::Seq(..) | Expr::Matrix(_) | Expr::Interval { .. })
}

// ---- scalar × vector -------------------------------------------------------

/// Distribute scalar factors of a product into the vector container they
/// multiply. A product with no vector container, or whose containers are all
/// flanked entirely by other containers, is returned unchanged.
fn vector_scalar_mult(e: &Expr) -> Expr {
    let Expr::Mul(factors) = e else {
        return e.clone();
    };
    // Legacy ran this transform once per container kind — `vector`, then
    // `altvector`, then `tuple` — so a product whose *first* container is walled
    // off from every scalar still had a later one distributed into. Trying each
    // container in turn and keeping the first that actually absorbs a factor
    // reproduces that in one pass: a walled-off candidate consumes nothing and
    // yields to the next.
    for (pos, kind) in candidates(factors, vector_kind) {
        // `candidates` only yields positions where `vector_kind` matched, so
        // this destructure always succeeds; skipping beats asserting it, as in
        // the sibling `matrix_scalar_mult`. This crate compiles to wasm with
        // `panic = "abort"`, where an `unreachable!` takes down the worker.
        let Expr::Seq(_, data) = &factors[pos] else {
            continue;
        };
        let (pre, data, post) = consume_scalars(&factors[..pos], data.clone(), &factors[pos + 1..]);
        if let Some(data) = data {
            return rebuild_product(pre, Expr::Seq(kind, data), post);
        }
    }
    e.clone()
}

/// The positions in `factors` holding a container `classify` recognizes, paired
/// with what it recognized them as, in left-to-right order.
fn candidates<T>(factors: &[Expr], classify: fn(&Expr) -> Option<T>) -> Vec<(usize, T)> {
    factors
        .iter()
        .enumerate()
        .filter_map(|(i, f)| classify(f).map(|k| (i, k)))
        .collect()
}

// ---- scalar × matrix -------------------------------------------------------

/// The dimensions of a `["matrix", …]`, or `None` for anything else.
fn matrix_dims(e: &Expr) -> Option<(u32, u32)> {
    match e {
        Expr::Matrix(m) => Some((m.rows(), m.cols())),
        _ => None,
    }
}

/// Distribute scalar factors of a product into the `["matrix", …]` they
/// multiply, applied to every entry. Candidate selection works as in
/// [`vector_scalar_mult`].
fn matrix_scalar_mult(e: &Expr) -> Expr {
    let Expr::Mul(factors) = e else {
        return e.clone();
    };
    for (pos, (rows, cols)) in candidates(factors, matrix_dims) {
        // `candidates` only yields positions where `matrix_dims` matched, so
        // this destructure always succeeds; skipping beats asserting it.
        let Expr::Matrix(m) = &factors[pos] else {
            continue;
        };
        let (pre, entries, post) =
            consume_scalars(&factors[..pos], m.entries().to_vec(), &factors[pos + 1..]);
        // `consume_scalars` rewrites entries in place, so the count still
        // matches; if it ever did not, leaving the product alone is correct.
        if let Some(matrix) = entries.and_then(|es| Mat::new(rows, cols, es)) {
            return rebuild_product(pre, Expr::Matrix(matrix), post);
        }
    }
    e.clone()
}

/// Fold the scalar pre/post factors around a container into its `components`,
/// returning the factors that remain outside. Scalars nearest the container are
/// consumed first and land on the right of each component (`["*", comp, s]`),
/// stopping at the first flanking container.
///
/// The components come back as `Some` only when a factor was actually absorbed;
/// `None` means this container is walled off by other containers on both sides
/// and the caller should try the next one. A literal `1` is absorbed — removed
/// from the product — but not distributed, so `1·(1,2)` stays `(1,2)` instead of
/// becoming `(1·1, 2·1)` (legacy's `if (preFactors[i] !== 1)` guard).
///
/// An *empty* container absorbs nothing: distributing into zero components would
/// consume the scalar without recording it anywhere, so `3·()` would come back
/// as `()` — a silently dropped factor rather than a wrong shape.
fn consume_scalars(
    before: &[Expr],
    mut components: Vec<Expr>,
    after: &[Expr],
) -> (Vec<Expr>, Option<Vec<Expr>>, Vec<Expr>) {
    if components.is_empty() {
        return (before.to_vec(), None, after.to_vec());
    }
    let distribute = |components: Vec<Expr>, s: &Expr| {
        if matches!(s, Expr::Num(n) if n.is_one()) {
            return components;
        }
        components
            .into_iter()
            .map(|c| Expr::Mul(vec![c, s.clone()]))
            .collect()
    };
    // Pre-factors: consume right-to-left (nearest the container first).
    let mut pre = before.to_vec();
    while let Some(last) = pre.last() {
        if is_container(last) {
            break;
        }
        let s = pre.pop().unwrap();
        components = distribute(components, &s);
    }
    // Post-factors: consume left-to-right (nearest the container first).
    let mut post = after.to_vec();
    while let Some(first) = post.first() {
        if is_container(first) {
            break;
        }
        let s = post.remove(0);
        components = distribute(components, &s);
    }
    let consumed = pre.len() != before.len() || post.len() != after.len();
    (pre, consumed.then_some(components), post)
}

/// Reassemble `pre · container · post`, dropping the `*` entirely when nothing
/// remains outside the container.
fn rebuild_product(pre: Vec<Expr>, container: Expr, post: Vec<Expr>) -> Expr {
    if pre.is_empty() && post.is_empty() {
        return container;
    }
    let mut factors = pre;
    factors.push(container);
    factors.extend(post);
    Expr::Mul(factors)
}

// ---- componentwise addition ------------------------------------------------

/// Combine like vector/matrix addends of a sum componentwise. Vectors group by
/// length, matrices by dimensions; a group of two or more collapses into one
/// container of componentwise `+`s. Unlike or unpaired addends pass through, and
/// if nothing has a partner the sum is returned untouched.
fn vector_matrix_addition(e: &Expr) -> Expr {
    let Expr::Add(addends) = e else {
        return e.clone();
    };

    /// One output position, held in first-appearance order: an addend that
    /// combines with nothing, or the group a combinable addend opened.
    enum Slot {
        Through(Expr),
        Vectors(usize, Vec<Expr>),
        Matrices(u32, u32, Vec<Expr>),
    }

    // A combinable addend joins the group its *first* member opened, and a group
    // occupies that first member's position — so the sum keeps the order it was
    // written in. (Addition commutes, so reordering would still be
    // value-preserving, but the caller is a grading shape pass: it hands the
    // result to a componentwise comparison that reads positions, and legacy
    // emitted the written order.)
    let mut slots: Vec<Slot> = Vec::new();
    for a in addends {
        if let (Some(_), Expr::Seq(_, xs)) = (vector_kind(a), a) {
            let n = xs.len();
            match slots
                .iter_mut()
                .find(|s| matches!(s, Slot::Vectors(m, _) if *m == n))
            {
                Some(Slot::Vectors(_, g)) => g.push(a.clone()),
                _ => slots.push(Slot::Vectors(n, vec![a.clone()])),
            }
            continue;
        }
        if let Expr::Matrix(m) = a {
            let (r, c) = (m.rows(), m.cols());
            match slots
                .iter_mut()
                .find(|s| matches!(s, Slot::Matrices(sr, sc, _) if (*sr, *sc) == (r, c)))
            {
                Some(Slot::Matrices(_, _, g)) => g.push(a.clone()),
                _ => slots.push(Slot::Matrices(r, c, vec![a.clone()])),
            }
            continue;
        }
        slots.push(Slot::Through(a.clone()));
    }

    let any_pair = slots.iter().any(|s| match s {
        Slot::Vectors(_, g) | Slot::Matrices(_, _, g) => g.len() >= 2,
        Slot::Through(_) => false,
    });
    if !any_pair {
        return e.clone();
    }

    let mut out: Vec<Expr> = Vec::with_capacity(slots.len());
    for slot in slots {
        match slot {
            Slot::Through(x) => out.push(x),
            Slot::Vectors(n, g) if g.len() >= 2 => out.push(combine_vectors(n, &g)),
            Slot::Matrices(r, c, g) if g.len() >= 2 => out.push(combine_matrices(r, c, &g)),
            Slot::Vectors(_, g) | Slot::Matrices(_, _, g) => out.extend(g),
        }
    }

    if out.len() == 1 {
        out.pop().unwrap()
    } else {
        Expr::Add(out)
    }
}

/// The container kind for a combined vector group: `vector` if any member was a
/// vector or a tuple and an altvector both appear, else `altvector` if any was
/// one, else `tuple`.
fn combined_kind(group: &[Expr]) -> SeqKind {
    let has = |k: SeqKind| group.iter().any(|x| vector_kind(x) == Some(k));
    let (v, t, a) = (
        has(SeqKind::Vector),
        has(SeqKind::Tuple),
        has(SeqKind::AltVector),
    );
    if v || (t && a) {
        SeqKind::Vector
    } else if a {
        SeqKind::AltVector
    } else {
        SeqKind::Tuple
    }
}

/// Componentwise sum of `n`-length vector addends into one container.
fn combine_vectors(n: usize, group: &[Expr]) -> Expr {
    let kind = combined_kind(group);
    let components = (0..n)
        .map(|i| {
            Expr::Add(
                group
                    .iter()
                    // The group was formed from `n`-length containers only, so
                    // every index resolves. Substituting zero if one somehow
                    // did not keeps a malformed addend from aborting the
                    // worker — the same reasoning as `combine_matrices`.
                    .map(|x| match x {
                        Expr::Seq(_, xs) => xs.get(i).cloned().unwrap_or_else(|| Expr::int(0)),
                        _ => Expr::int(0),
                    })
                    .collect(),
            )
        })
        .collect();
    Expr::Seq(kind, components)
}

/// Componentwise sum of same-dimension matrix addends into one matrix.
fn combine_matrices(rows: u32, cols: u32, group: &[Expr]) -> Expr {
    // The group was formed from `rows`×`cols` matrices only, and `Mat` fixes
    // the entry count to match, so every `get` here resolves. Substituting zero
    // if one somehow did not keeps a malformed addend from aborting the worker,
    // which indexing (and the `unreachable!` this replaced) would have done.
    let entry = |m: &Expr, r: u32, c: u32| match m {
        Expr::Matrix(mm) => mm.get(r, c).cloned().unwrap_or_else(|| Expr::int(0)),
        _ => Expr::int(0),
    };
    Expr::Matrix(Mat::generate(rows, cols, |r, c| {
        Expr::Add(group.iter().map(|m| entry(m, r, c)).collect())
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{to_text, TextToAst};

    fn p(s: &str) -> Expr {
        TextToAst::new(Default::default())
            .convert(s)
            .unwrap_or_else(|e| panic!("parse {s:?}: {e:?}"))
    }
    /// Deliberately does *not* pre-flatten: the parser's left-nested output is
    /// exactly what the production caller passes in, and pre-flattening here is
    /// what hid the 3-addend no-op.
    fn run(s: &str) -> Expr {
        perform_vector_matrix_additions_scalar_multiplications(&p(s))
    }
    fn txt(e: &Expr) -> String {
        to_text(e, &Default::default())
    }

    #[test]
    fn adds_two_tuples_componentwise() {
        // Top-level operator becomes the container, which is the whole point.
        let r = run("(1,2)+(3,4)");
        assert!(matches!(r, Expr::Seq(SeqKind::Tuple, _)), "got {r:?}");
        assert_eq!(txt(&r), "(1 + 3, 2 + 4)");
    }

    #[test]
    fn does_not_fold_the_components() {
        // The pass must leave `1+3`, not `4` — grading compares components under
        // tolerance and folding would change what is compared.
        let r = run("(1,2)+(3,4)");
        let Expr::Seq(_, xs) = &r else { panic!() };
        assert!(matches!(xs[0], Expr::Add(_)), "component should stay a sum");
    }

    #[test]
    fn distributes_a_leading_scalar_on_the_right() {
        assert_eq!(txt(&run("3(1,2)")), "(1 * 3, 2 * 3)");
        assert_eq!(txt(&run("a(x,y)")), "(x a, y a)");
    }

    #[test]
    fn distributes_a_trailing_scalar() {
        assert_eq!(txt(&run("(x,y)*3")), "(x * 3, y * 3)");
    }

    /// Legacy pops a literal `1` off the factor list without distributing it
    /// (`if (preFactors[i] !== 1)`), so the components stay bare.
    #[test]
    fn a_literal_one_is_dropped_rather_than_distributed() {
        assert_eq!(txt(&run("1(1,2)")), "(1, 2)");
        assert_eq!(txt(&run("(x,y)*1")), "(x, y)");
        // Still distributed when a real scalar rides along with the 1.
        assert_eq!(txt(&run("1*3*(x,y)")), "(x * 3, y * 3)");
    }

    /// With several containers in one product, the scalars go to the first that
    /// can actually reach them — legacy got this from running the transform once
    /// per container kind.
    #[test]
    fn a_walled_off_container_yields_to_a_reachable_one() {
        // The leading tuple has no scalar neighbor; the altvector does.
        assert_eq!(txt(&run("(1,2)*⟨3,4⟩*5")), "(1, 2) ⟨3 * 5, 4 * 5⟩");
        // Same shape with two tuples.
        assert_eq!(txt(&run("(1,2)*(3,4)*5")), "(1, 2) (3 * 5, 4 * 5)");
        // A product of containers with no scalar at all is left alone.
        let both = run("(1,2)*⟨3,4⟩");
        assert!(matches!(both, Expr::Mul(_)), "got {both:?}");
    }

    #[test]
    fn leaves_non_vectors_untouched() {
        assert_eq!(run("x+y"), crate::expr::flatten(p("x+y")));
        assert_eq!(run("3x"), crate::expr::flatten(p("3x")));
    }

    /// The regression the pass existed to prevent and did not: `+` is parsed
    /// left-nested, so with three or more addends no single `Add` node saw both
    /// containers and the pass silently returned its input — leaving `+` on top,
    /// which is precisely what makes grading's componentwise branch not engage.
    #[test]
    fn combines_across_a_left_nested_sum() {
        for s in [
            "x+(1,2)+(3,4)",
            "(1,2)+x+(3,4)",
            "(1,2)+(3,4)+x",
            "1+(1,2)+(3,4)",
            "x+(1,2)+2(3,4)",
        ] {
            let r = run(s);
            let Expr::Add(xs) = &r else {
                panic!("{s:?} should stay a sum, got {r:?}")
            };
            assert!(
                xs.iter().any(|x| matches!(x, Expr::Seq(SeqKind::Tuple, _))),
                "{s:?} did not combine its tuples: {}",
                txt(&r)
            );
        }
        // And the same tree reached through `from_ast`'s already-flat spelling
        // must agree — the two entry points disagreeing was the bug.
        assert_eq!(
            run("x+(1,2)+(3,4)"),
            perform_vector_matrix_additions_scalar_multiplications(&Expr::Add(vec![
                p("x"),
                p("(1,2)"),
                p("(3,4)"),
            ]))
        );
    }

    /// Addition commutes, but this is a shape pass feeding a positional
    /// comparison: the sum must come back in the order it was written.
    #[test]
    fn preserves_addend_order() {
        assert_eq!(txt(&run("x+(1,2)+(3,4)")), "x + (1 + 3, 2 + 4)");
        assert_eq!(txt(&run("(1,2)+(3,4)+x")), "(1 + 3, 2 + 4) + x");
        assert_eq!(txt(&run("(1,2)+x+(3,4)")), "(1 + 3, 2 + 4) + x");
    }

    /// An empty container has nowhere to put a scalar, so it must not absorb
    /// one — distributing into zero components would delete the factor.
    #[test]
    fn an_empty_container_does_not_swallow_its_scalar() {
        let r = perform_vector_matrix_additions_scalar_multiplications(&Expr::Mul(vec![
            p("3"),
            Expr::Seq(SeqKind::Tuple, vec![]),
        ]));
        assert!(matches!(r, Expr::Mul(_)), "the 3 was dropped: {r:?}");
    }

    #[test]
    fn a_vector_plus_a_scalar_is_left_alone() {
        // No partner of the same length, so nothing combines.
        let r = run("(1,2)+3");
        assert!(matches!(r, Expr::Add(_)), "got {r:?}");
    }

    #[test]
    fn different_lengths_do_not_combine() {
        let r = run("(1,2)+(3,4,5)");
        assert!(matches!(r, Expr::Add(_)), "got {r:?}");
    }

    #[test]
    fn combines_three_addends_and_a_scalar_multiple() {
        // (1,2) + 2(3,4) + (5,6) → (1+3·2+5, 2+4·2+6)
        let r = run("(1,2)+2(3,4)+(5,6)");
        assert!(matches!(r, Expr::Seq(SeqKind::Tuple, _)), "got {r:?}");
        assert_eq!(txt(&r), "(1 + 3 * 2 + 5, 2 + 4 * 2 + 6)");
    }

    #[test]
    fn a_vector_beats_a_tuple_when_they_combine() {
        let r = run("(1,2)+⟨3,4⟩");
        // tuple + altvector → altvector only if no vector; here a tuple and an
        // altvector meet, so the result is a vector.
        assert!(matches!(r, Expr::Seq(SeqKind::Vector, _)), "got {r:?}");
    }

    #[test]
    fn adds_two_matrices_componentwise() {
        let m = |a, b, c, d| {
            Expr::Matrix(Mat::new(2, 2, vec![p(a), p(b), p(c), p(d)]).expect("2x2 has 4 entries"))
        };
        let sum = Expr::Add(vec![m("1", "2", "3", "4"), m("5", "6", "7", "8")]);
        let r = perform_vector_matrix_additions_scalar_multiplications(&sum);
        let Expr::Matrix(mat) = &r else {
            panic!("got {r:?}")
        };
        assert_eq!(txt(&mat.entries()[0]), "1 + 5");
        assert_eq!(txt(&mat.entries()[3]), "4 + 8");
    }

    #[test]
    fn recurses_into_subexpressions() {
        // The container move must happen wherever a sum-of-vectors appears.
        let r = run("f((1,2)+(3,4))");
        let Expr::Apply(_, args) = &r else {
            panic!("got {r:?}")
        };
        assert!(matches!(args[0], Expr::Seq(SeqKind::Tuple, _)));
    }
}
