//! `default_order`: **sort, do not evaluate** — a faithful port of the JS
//! `trees/default_order.js` as it stands in `math-expressions@2.0.0-alpha94`,
//! the version DoenetML pins.
//!
//! This is not [`canonicalize`](super::canonicalize), and the difference is the
//! whole point. Canonical form folds as it goes: `0·x²` disappears, `7+4`
//! becomes `11`, `1·x²` loses its coefficient. `default_order` touches nothing
//! but the *arrangement* — operands of the commutative operators are sorted,
//! comparison directions are flipped to point one way, and negatives are pulled
//! out of factors. Every term survives, spelled as it was written.
//!
//! That is what DoenetML's `simplify="normalizeOrder"` means, and what makes it
//! usable for grading: an author asking for it wants `1x²+2-0x²+3` to match the
//! same terms in any order and *not* to match `x²+5`, which is a different
//! answer written by a student who did more work than they were asked to.
//!
//! # The ordering is the JS one, deliberately
//!
//! [`super::order::cmp`] is this crate's canonical order and is a better
//! comparator — typed, allocation-free, stable across sessions. It is also a
//! *different* order, and the order here is observable: `simplify="normalizeOrder"`
//! feeds straight into `valueForDisplay`, so this decides the term sequence an
//! expression *prints* in. Reproducing the JS key is what keeps existing
//! documents rendering as their authors saw them.
//!
//! # Sums do not use the sort key
//!
//! The one part of this that is not a comparator over keys is the sum. JS calls
//! it a "kludge to get sort order closer to lexographic order", and it is: a
//! sum's terms are sorted by *descending exponent* of each variable in turn
//! (variables taken in alphabetical order), and only then by the sort key of
//! what is left over as a coefficient. That is what puts `x³` before `x²`
//! before `x` before the constants, instead of the key order, which would lead
//! with every bare number. See [`coeff_factors_from_term`].
//!
//! An earlier port of this file took its algorithm from a *different* JS
//! lineage, whose sum sort was the plain key comparator. It read as a tidier
//! implementation of the same idea and was not: it printed `-3-3+4-2x²+…`
//! where every DoenetML document written against the pinned library shows
//! `-2x²+0x²+1x²+5x²-3-3+4`.

use std::collections::BTreeMap;

use crate::expr::{Expr, MathConst, RelOp, SeqKind};

/// Sort an expression into the JS library's default order.
pub fn default_order(e: &Expr) -> Expr {
    let t = normalize_negatives(&flatten(e));
    normalize_negatives(&sort_ast(&t))
}

/// The comparator [`default_order`] sorts commutative operands with, exposed
/// for callers that arrange trees themselves rather than through a whole-tree
/// pass — the compat polynomial engine picks its leading variable this way, and
/// it has to be *this* order (the legacy JS key below) and not
/// [`super::order::cmp`], or two polynomials disagree about which of their
/// variables comes first.
pub fn cmp_default_order(a: &Expr, b: &Expr) -> std::cmp::Ordering {
    cmp_key(&sort_key(a, false), &sort_key(b, false))
}

/// Merge nested same-operator `Add`/`Mul`/`And`/`Or`/`Union`/`Intersect` nodes
/// into their parent, so the sort sees one flat operand list. The parsers
/// already produce flat trees; a tree built by hand through the AST boundary
/// need not be.
pub(super) fn flatten(e: &Expr) -> Expr {
    fn flat_children(xs: &[Expr], same: impl Fn(&Expr) -> Option<Vec<Expr>>) -> Vec<Expr> {
        let mut out = Vec::with_capacity(xs.len());
        for x in xs {
            let fx = flatten(x);
            match same(&fx) {
                Some(inner) => out.extend(inner),
                None => out.push(fx),
            }
        }
        out
    }
    match e {
        Expr::Add(xs) => Expr::Add(flat_children(xs, |x| match x {
            Expr::Add(inner) => Some(inner.clone()),
            _ => None,
        })),
        Expr::Mul(xs) => Expr::Mul(flat_children(xs, |x| match x {
            Expr::Mul(inner) => Some(inner.clone()),
            _ => None,
        })),
        Expr::And(xs) => Expr::And(flat_children(xs, |x| match x {
            Expr::And(inner) => Some(inner.clone()),
            _ => None,
        })),
        Expr::Or(xs) => Expr::Or(flat_children(xs, |x| match x {
            Expr::Or(inner) => Some(inner.clone()),
            _ => None,
        })),
        Expr::Union(xs) => Expr::Union(flat_children(xs, |x| match x {
            Expr::Union(inner) => Some(inner.clone()),
            _ => None,
        })),
        Expr::Intersect(xs) => Expr::Intersect(flat_children(xs, |x| match x {
            Expr::Intersect(inner) => Some(inner.clone()),
            _ => None,
        })),
        other => crate::expr::map_children(other, flatten),
    }
}

// ---------------------------------------------------------------------------
// Negative normalization (JS `normalize_negatives`)
// ---------------------------------------------------------------------------

/// Drop double negatives, pull a negative out of any factor, then put the sign
/// back onto a leading numeric coefficient. Run before *and* after sorting, as
/// JS does: the sort's own `-`-into-product rewrite creates a new inner
/// negative that the second run resolves.
fn normalize_negatives(e: &Expr) -> Expr {
    let e = remove_duplicate_negatives(e);
    let e = negatives_out_of_factors(&e);
    let e = remove_duplicate_negatives(&e);
    normalize_negative_numbers(&e)
}

fn remove_duplicate_negatives(e: &Expr) -> Expr {
    if let Expr::Neg(inner) = e {
        if let Expr::Neg(inner2) = inner.as_ref() {
            return remove_duplicate_negatives(inner2);
        }
        // Only an already-*negative* literal folds here (`["-", -3] → 3`); a
        // negated positive is left for `normalize_negative_numbers`, which runs
        // last and so gets the final say on where the sign sits. Doing it in
        // both places would be harmless but doing it *only* here would fold
        // `["-", 0]` to `NegZero`, a different leaf that sorts elsewhere, and
        // the pass would stop being idempotent.
        if let Expr::Num(n) = inner.as_ref() {
            if n.is_negative() {
                return Expr::Num(n.neg());
            }
        }
    }
    crate::expr::map_children(e, remove_duplicate_negatives)
}

/// Rebuilds a node from the factors [`negatives_out_of_factors`] pulled out of
/// it, so the two shapes that carry factors share one sign-stripping pass.
type Rebuild = fn(Vec<Expr>) -> Expr;

fn negatives_out_of_factors(e: &Expr) -> Expr {
    let e = crate::expr::map_children(e, negatives_out_of_factors);
    let (factors, rebuild): (Vec<Expr>, Rebuild) = match &e {
        Expr::Mul(xs) => (xs.clone(), Expr::Mul),
        Expr::Div(a, b) => (vec![a.as_ref().clone(), b.as_ref().clone()], |mut v| {
            Expr::Div(Box::new(v.remove(0)), Box::new(v.remove(0)))
        }),
        _ => return e,
    };
    let mut negative = false;
    let stripped: Vec<Expr> = factors
        .into_iter()
        .map(|f| match f {
            Expr::Neg(inner) => {
                negative = !negative;
                *inner
            }
            // A negative *number* as a factor counts too: `(-2)·x` and `-(2x)`
            // are one product with the sign in two places, and the sort must
            // see them alike.
            Expr::Num(n) if n.is_negative() => {
                negative = !negative;
                Expr::Num(n.neg())
            }
            other => other,
        })
        .collect();
    let result = rebuild(stripped);
    if negative {
        Expr::Neg(Box::new(result))
    } else {
        result
    }
}

/// JS `normalize_negative_numbers`: `["-", 3] → -3`, `["-", ["*", 3, x]] →
/// ["*", -3, x]`, `["-", ["/", 3, x]] → ["/", -3, x]`. The inverse of the
/// pull-out above, and the reason the two together are a fixpoint rather than a
/// loop: the sign ends up on the leading number when there is one, and outside
/// the product when there is not.
fn normalize_negative_numbers(e: &Expr) -> Expr {
    if let Expr::Neg(inner) = e {
        match inner.as_ref() {
            Expr::Num(n) if !n.is_negative() => return Expr::Num(n.neg()),
            Expr::Mul(factors) if !factors.is_empty() => {
                if let Some(first) = negate_leading_positive_number(&factors[0]) {
                    let mut out = Vec::with_capacity(factors.len());
                    out.push(first);
                    out.extend(factors[1..].iter().map(normalize_negative_numbers));
                    return Expr::Mul(out);
                }
            }
            Expr::Div(num, den) => {
                if let Some(negated) = negate_leading_positive_number(num) {
                    return Expr::Div(Box::new(negated), Box::new(normalize_negative_numbers(den)));
                }
            }
            _ => {}
        }
    }
    crate::expr::map_children(e, normalize_negative_numbers)
}

/// `Some(-node)` when `node` leads with a non-negative number that can carry
/// the sign, `None` when there is nowhere for it to go.
fn negate_leading_positive_number(node: &Expr) -> Option<Expr> {
    match node {
        Expr::Num(n) if !n.is_negative() => Some(Expr::Num(n.neg())),
        Expr::Mul(factors) => match factors.first() {
            Some(Expr::Num(n)) if !n.is_negative() => {
                let mut out = Vec::with_capacity(factors.len());
                out.push(Expr::Num(n.neg()));
                out.extend(factors[1..].iter().map(normalize_negative_numbers));
                Some(Expr::Mul(out))
            }
            _ => None,
        },
        Expr::Div(num, den) => negate_leading_positive_number(num)
            .map(|negated| Expr::Div(Box::new(negated), Box::new(normalize_negative_numbers(den)))),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// The sort itself
// ---------------------------------------------------------------------------

fn sort_ast(e: &Expr) -> Expr {
    let e = crate::expr::map_children(e, sort_ast);
    match e {
        // Sums get the exponent-first ordering, not the sort key.
        Expr::Add(xs) => Expr::Add(sort_sum_terms(xs)),
        // A product sorts by the key, except that the operands with a
        // meaningful order of their own — a tuple, a matrix, an interval — keep
        // theirs and move to the end, so `M·(e,f)` never becomes `(e,f)·M`.
        Expr::Mul(xs) => {
            let (mut sortable, fixed): (Vec<Expr>, Vec<Expr>) =
                xs.into_iter().partition(|x| !order_is_meaningful(x));
            sort_by_key(&mut sortable);
            sortable.extend(fixed);
            Expr::Mul(sortable)
        }
        Expr::And(mut xs) => {
            sort_by_key(&mut xs);
            Expr::And(xs)
        }
        Expr::Or(mut xs) => {
            sort_by_key(&mut xs);
            Expr::Or(xs)
        }
        Expr::Union(mut xs) => {
            sort_by_key(&mut xs);
            Expr::Union(xs)
        }
        Expr::Intersect(mut xs) => {
            sort_by_key(&mut xs);
            Expr::Intersect(xs)
        }
        // `=` and `ne` are commutative too; the ordered comparisons instead get
        // turned around so every one of them points the same way, and the
        // containment relations so the larger set is always on the right.
        Expr::Relation { mut operands, ops } => {
            if ops.iter().all(|o| matches!(o, RelOp::Eq))
                || ops.iter().all(|o| matches!(o, RelOp::Ne))
            {
                sort_by_key(&mut operands);
                return Expr::Relation { operands, ops };
            }
            if ops.iter().all(|o| {
                matches!(
                    o,
                    RelOp::Gt
                        | RelOp::Ge
                        | RelOp::Ni
                        | RelOp::NotNi
                        | RelOp::Superset
                        | RelOp::NotSuperset
                        | RelOp::SupersetEq
                        | RelOp::NotSupersetEq
                )
            }) {
                operands.reverse();
                let flipped: Vec<RelOp> = ops.iter().rev().map(|o| mirror(*o)).collect();
                return Expr::Relation {
                    operands,
                    ops: flipped,
                };
            }
            Expr::Relation { operands, ops }
        }
        // Negating a product puts the sign on its first factor, so `-(2x)` and
        // `(-2)x` reach the same tree.
        Expr::Neg(inner) => match *inner {
            Expr::Mul(mut xs) if !xs.is_empty() => {
                let first = xs.remove(0);
                xs.insert(0, Expr::Neg(Box::new(first)));
                Expr::Mul(xs)
            }
            other => Expr::Neg(Box::new(other)),
        },
        other => other,
    }
}

/// Operands whose position carries meaning, which a product's sort must not
/// disturb. JS tests the operator name against this exact list.
fn order_is_meaningful(e: &Expr) -> bool {
    matches!(e, Expr::Seq(..) | Expr::Interval { .. } | Expr::Matrix(_))
}

/// The same relation read right-to-left: `a > b` is `b < a`. Not
/// [`RelOp::negate`], which keeps the operand order and complements the
/// meaning; this keeps the meaning and reverses the operands.
fn mirror(op: RelOp) -> RelOp {
    match op {
        RelOp::Gt => RelOp::Lt,
        RelOp::Ge => RelOp::Le,
        RelOp::Lt => RelOp::Gt,
        RelOp::Le => RelOp::Ge,
        RelOp::Ni => RelOp::In,
        RelOp::NotNi => RelOp::NotIn,
        RelOp::In => RelOp::Ni,
        RelOp::NotIn => RelOp::NotNi,
        RelOp::Superset => RelOp::Subset,
        RelOp::NotSuperset => RelOp::NotSubset,
        RelOp::SupersetEq => RelOp::SubsetEq,
        RelOp::NotSupersetEq => RelOp::NotSubsetEq,
        RelOp::Subset => RelOp::Superset,
        RelOp::NotSubset => RelOp::NotSuperset,
        RelOp::SubsetEq => RelOp::SupersetEq,
        RelOp::NotSubsetEq => RelOp::NotSupersetEq,
        RelOp::Eq => RelOp::Eq,
        RelOp::Ne => RelOp::Ne,
    }
}

fn sort_by_key(xs: &mut [Expr]) {
    // A decorate-sort-undecorate: building the key is the expensive part, and
    // a comparison sort would rebuild it O(n log n) times per operand.
    let mut keyed: Vec<(Key, Expr)> = xs.iter().map(|x| (sort_key(x, false), x.clone())).collect();
    keyed.sort_by(|a, b| cmp_key(&a.0, &b.0));
    for (slot, (_, e)) in xs.iter_mut().zip(keyed) {
        *slot = e;
    }
}

// ---------------------------------------------------------------------------
// Sum ordering — the "kludge to get sort order closer to lexographic order"
// ---------------------------------------------------------------------------

/// Split a term into "which variables it contains, to what power" and "what is
/// left over as a coefficient". Only the shapes JS recognises are decomposed —
/// a product of symbols and integer powers of symbols, possibly negated or
/// divided. Anything else (a function call, a power of a sum) contributes no
/// variables and *is* its own coefficient, which is why `sqrt(x)+x+1` orders
/// `x` first and then sorts `1` and `sqrt(x)` by key.
fn coeff_factors_from_term(term: &Expr, string_factors: &mut Vec<String>) -> (Exponents, Expr) {
    fn index_of(name: String, string_factors: &mut Vec<String>) -> usize {
        match string_factors.iter().position(|s| *s == name) {
            Some(i) => i,
            None => {
                string_factors.push(name);
                string_factors.len() - 1
            }
        }
    }

    // JS asks `typeof term === "string"`, which is true of a variable, of a
    // named constant (`pi` is the string `"pi"` in that tree) and of the blank.
    if let Some(name) = js_string_leaf(term) {
        let ind = index_of(name, string_factors);
        return (Exponents::single(ind, 1.0), Expr::int(1));
    }

    match term {
        Expr::Mul(factors) => {
            let mut exps = Exponents::default();
            let mut coeff: Vec<Expr> = Vec::new();
            for factor in factors {
                if let Some(name) = js_string_leaf(factor) {
                    let ind = index_of(name, string_factors);
                    exps.add(ind, 1.0);
                    continue;
                }
                if matches!(factor, Expr::Pow(..) | Expr::Neg(_)) {
                    let (sub_exps, sub_coeff) = coeff_factors_from_term(factor, string_factors);
                    exps.merge(&sub_exps);
                    if !is_literal_one(&sub_coeff) {
                        coeff.push(sub_coeff);
                    }
                    continue;
                }
                coeff.push(factor.clone());
            }
            let coeff = match coeff.len() {
                0 => Expr::int(1),
                1 => coeff.into_iter().next().expect("length checked"),
                _ => Expr::Mul(coeff),
            };
            (exps, coeff)
        }
        // Only `variable ^ finite-number` is a power for these purposes.
        Expr::Pow(base, exp) => {
            if let (Some(name), Expr::Num(n)) = (js_string_leaf(base), exp.as_ref()) {
                let v = n.to_f64();
                if v.is_finite() {
                    let ind = index_of(name, string_factors);
                    return (Exponents::single(ind, v), Expr::int(1));
                }
            }
            (Exponents::default(), term.clone())
        }
        Expr::Neg(inner) => {
            let (exps, coeff) = coeff_factors_from_term(inner, string_factors);
            let coeff = match &coeff {
                Expr::Num(n) => Expr::Num(n.neg()),
                other => Expr::Neg(Box::new(other.clone())),
            };
            (exps, coeff)
        }
        Expr::Div(num, den) => {
            let (exps, coeff) = coeff_factors_from_term(num, string_factors);
            (
                exps,
                Expr::Div(Box::new(coeff), Box::new(den.as_ref().clone())),
            )
        }
        _ => (Exponents::default(), term.clone()),
    }
}

/// A term's variable exponents, keyed by index into the shared `string_factors`
/// list. Sparse, like the JS array it stands in for: an absent entry is 0.
#[derive(Default, Clone)]
struct Exponents(BTreeMap<usize, f64>);

impl Exponents {
    fn single(ind: usize, v: f64) -> Exponents {
        let mut m = BTreeMap::new();
        m.insert(ind, v);
        Exponents(m)
    }
    fn add(&mut self, ind: usize, v: f64) {
        *self.0.entry(ind).or_insert(0.0) += v;
    }
    fn merge(&mut self, other: &Exponents) {
        for (&ind, &v) in &other.0 {
            self.add(ind, v);
        }
    }
    fn get(&self, ind: usize) -> f64 {
        self.0.get(&ind).copied().unwrap_or(0.0)
    }
}

fn sort_sum_terms(terms: Vec<Expr>) -> Vec<Expr> {
    let mut string_factors: Vec<String> = Vec::new();
    let mut exps_by_term: Vec<Exponents> = Vec::with_capacity(terms.len());
    let mut coeffs: Vec<Expr> = Vec::with_capacity(terms.len());
    for term in &terms {
        let (exps, coeff) = coeff_factors_from_term(term, &mut string_factors);
        exps_by_term.push(exps);
        coeffs.push(coeff);
    }

    // Variables are compared in alphabetical order, whatever order they were
    // discovered in.
    let mut var_order: Vec<usize> = (0..string_factors.len()).collect();
    var_order.sort_by(|&a, &b| string_factors[a].cmp(&string_factors[b]));

    // Descending exponent per variable, then the coefficient's own sort key.
    let mut keyed: Vec<(Key, Expr)> = terms
        .into_iter()
        .enumerate()
        .map(|(i, term)| {
            let mut parts: Vec<Key> = var_order
                .iter()
                .map(|&v| Key::Num(-exps_by_term[i].get(v)))
                .collect();
            parts.push(sort_key(&coeffs[i], false));
            (Key::Arr(parts), term)
        })
        .collect();
    keyed.sort_by(|a, b| cmp_key(&a.0, &b.0));
    keyed.into_iter().map(|(_, t)| t).collect()
}

fn is_literal_one(e: &Expr) -> bool {
    matches!(e, Expr::Num(n) if n.to_f64() == 1.0)
}

/// The name this node carries as a bare *string* in the JS tree — a variable, a
/// named constant, or the blank. `None` for everything else, including the
/// specials (`Inf`, `NaN`) which serialize as objects rather than strings.
fn js_string_leaf(e: &Expr) -> Option<String> {
    match e {
        Expr::Sym(s) => Some(s.name()),
        Expr::Const(MathConst::Pi) => Some("pi".to_string()),
        Expr::Const(MathConst::E) => Some("e".to_string()),
        Expr::Const(MathConst::I) => Some("i".to_string()),
        Expr::Blank => Some("\u{ff3f}".to_string()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Sort keys
// ---------------------------------------------------------------------------

/// A JS sort key: a nested array of numbers, strings and booleans. Kept as a
/// tree rather than flattened to a string because alpha94 compares these with
/// its own recursive `arrayCompare` — a scalar sorts before an array, and a
/// shorter array before a longer one with the same prefix — which string
/// coercion would not reproduce.
#[derive(Clone, Debug)]
enum Key {
    Num(f64),
    Str(String),
    Bool(bool),
    Arr(Vec<Key>),
}

/// JS `arrayCompare`, made *total*.
///
/// JS compares two key entries with `a < b ? -1 : a > b ? 1 : 0`, which ties
/// whenever the comparison coerces to `NaN` — so its comparator is not a
/// strict weak ordering and its `sort` is therefore not a normal form. This
/// crate cannot copy that: [`default_order`] backs `simplify="normalizeOrder"`,
/// where two spellings of one expression must reduce to the same tree, and
/// Rust's `sort_by` *panics* on a detected order violation, which under
/// `panic = "abort"` is a dead worker.
///
/// Totality is restored by stratifying the key domain — numbers and booleans
/// below strings below arrays — and comparing numerically only *within* the
/// first stratum. That is invisible to every key this file builds except one:
/// [`append_unit`] stringifies index 1, which for a `Seq`/`Array`/`Interval`
/// holds the operand count rather than a kind name, so a unit-annotated
/// container's `Str` there met a plain container's `Num`. JS ties those
/// (`"2_%" < 3` is `false` both ways) and so ranked one unit-annotated
/// container equal to *every* plain one while the plain ones still ordered by
/// length; `(z,z) + (y,y)% + (x,x,x)` came out three different ways depending
/// on which of its six input orderings it was given. Units reach this from
/// ordinary text and LaTeX (`(1,2)%`, `(30,45)deg`).
///
/// The kind-name keys units were designed for are unaffected, because both
/// sides are strings there: `5%` still keys `[0, "number_%", 5]` beside
/// `[0, "number", 5]`, and `$x` still keys `[1, "symbol_$", "x"]`.
fn cmp_key(a: &Key, b: &Key) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (Key::Arr(xs), Key::Arr(ys)) => {
            for (x, y) in xs.iter().zip(ys.iter()) {
                let c = cmp_key(x, y);
                if c != Ordering::Equal {
                    return c;
                }
            }
            xs.len().cmp(&ys.len())
        }
        // Two strings compare as strings, as JS does.
        (Key::Str(x), Key::Str(y)) => x.cmp(y),
        // Numbers and booleans compare numerically, as JS does after
        // ToPrimitive — except that a `NaN` sorts last within the stratum
        // instead of tying with everything in it.
        (Key::Num(_) | Key::Bool(_), Key::Num(_) | Key::Bool(_)) => {
            let (x, y) = (scalar_f64(a), scalar_f64(b));
            x.partial_cmp(&y)
                .unwrap_or_else(|| y.is_nan().cmp(&x.is_nan()))
        }
        // Mixed types never share a key index in the schema this file builds,
        // save for `append_unit`'s stringification; rank them so the whole
        // relation is a strict weak ordering.
        _ => stratum(a).cmp(&stratum(b)),
    }
}

/// The rank of a key entry's type: numbers and booleans, then strings, then
/// arrays. Ordering across strata is by rank alone, which is what makes
/// [`cmp_key`] total; the array rank also reproduces JS's "a non-array comes
/// before an array".
fn stratum(k: &Key) -> u8 {
    match k {
        Key::Num(_) | Key::Bool(_) => 0,
        Key::Str(_) => 1,
        Key::Arr(_) => 2,
    }
}

/// JS `ToNumber` for the two types [`cmp_key`] compares numerically. `NaN` for
/// anything else, which only the stratum ranking can reach.
fn scalar_f64(k: &Key) -> f64 {
    match k {
        Key::Num(v) => *v,
        Key::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Key::Str(_) | Key::Arr(_) => f64::NAN,
    }
}

/// JS `sort_key`. `ignore_negatives` is the caller-supplied flag; note that JS
/// loses it in the branches that recurse through `operands.map(sort_key,
/// params)`, because `map` passes the array index where the function expects
/// its options. That quirk is reproduced rather than fixed — it is the order
/// existing documents were authored against — so the generic operator branch
/// and the `Num`-as-quotient branch build their nested keys with `false`.
///
/// The branches JS does *not* route through `map` keep it: `Pow`, `Apply` and
/// the `unit` prefix all pass `ignore_negatives` down, matching their JS
/// counterparts, which recurse by direct call.
///
/// All three of this crate's call sites pass `false` today (`cmp_default_order`
/// and the two `sort_by`s), so the distinction is currently unobservable; it is
/// documented because the parameter is the one lever that would change it.
fn sort_key(e: &Expr, ignore_negatives: bool) -> Key {
    // Every branch returns an array, which is what lets `^` splice two keys
    // together below.
    if let Some(name) = js_string_leaf(e) {
        // `+` and `-` occur as bare strings inside `pm` expressions.
        if name == "-" || name == "+" {
            return arr3(8.0, "plus_minus_string", Key::Str(name));
        }
        return arr3(1.0, "symbol", Key::Str(name));
    }
    // JS `sort_key`'s `unit` branch, taken before the operator tail below
    // exactly as it is there. A unit-annotated quantity keys as its *value's*
    // key with the unit appended to the kind string, so `5%` sorts among the
    // numbers (`[0,"number_%",5]`) and `$x` among the symbols
    // (`[1,"symbol_$","x"]`) rather than landing in the `[10, …]` catch-all
    // every unrecognized operator falls into — which put a unit-annotated term
    // last in a sum where JS puts it first.
    //
    // A unit node whose symbol is not one of JS's three keys as any other
    // operator does. That is the one deliberate divergence: JS destructures
    // `get_unit_value_of_tree`'s `null` and *throws*, so its `if (unit)` guard
    // is unreachable. Sorting is not a place to panic — it runs inside the
    // wasm worker on author-supplied trees — and the only nodes that reach it
    // are hand-built (`circ` is the one unit symbol this crate accepts that JS
    // has no entry for, and the LaTeX parser substitutes it to `deg`).
    if let Expr::OtherOp(s, args) = e {
        if s.name() == "unit" {
            if let Some((unit, value)) = crate::normalize::scaling_unit_and_value(args) {
                return append_unit(sort_key(value, ignore_negatives), unit);
            }
        }
    }
    match e {
        // A `Num` that crosses to JS as a *tree* keys as that tree, not as a
        // number. JS has no exact-rational leaf, so `-2/3` is `["/", -2, 3]`
        // there and `sort_key` reaches it through the two-operand branch as
        // `[4, "quotient", …]` — behind every symbol, whose key is
        // `[1, "symbol", …]`. Keying it `[0, "number", …]` sorted a fraction
        // *ahead* of a symbol and turned `x = -2/3` into `-2/3 = x`, where the
        // oracle leaves the variable on the left.
        //
        // Asking the JS spelling rather than the `Number` variant is what makes
        // this exact: a decimal-spelled rational (`19.9` is `Rat(199, 10)`)
        // crosses as the plain number `19.9`, so it keeps the number key — the
        // same split `number_to_js` makes.
        Expr::Num(n) => match crate::expr::serde::to_js(e) {
            serde_json::Value::Array(parts) => {
                let factor_keys: Vec<Key> = parts[1..]
                    .iter()
                    // `ignore_negatives` is dropped on the way in, matching what
                    // the generic branch below does and why (JS's
                    // `operands.map(sort_key, params)` passes `params` as
                    // `map`'s *thisArg*, so the nested call never sees it).
                    .map(|v| arr3(0.0, "number", Key::Num(v.as_f64().unwrap_or(0.0))))
                    .collect();
                Key::Arr(vec![
                    Key::Num(4.0),
                    Key::Str("quotient".to_string()),
                    Key::Num(factor_keys.len() as f64),
                    Key::Arr(factor_keys),
                ])
            }
            _ => {
                let v = n.to_f64();
                arr3(
                    0.0,
                    "number",
                    Key::Num(if ignore_negatives { v.abs() } else { v }),
                )
            }
        },
        Expr::Bool(b) => arr3(1.0, "boolean", Key::Bool(*b)),
        // A power keys as its base, then the marker, then its exponent — so
        // `x`, `x^2` and `x^3` land next to each other rather than being
        // scattered among the other two-operand operators.
        Expr::Pow(base, exp) => {
            let mut parts = key_items(sort_key(base, ignore_negatives));
            parts.push(Key::Str("power".to_string()));
            parts.extend(key_items(sort_key(exp, ignore_negatives)));
            Key::Arr(parts)
        }
        Expr::Apply(head, args) => apply_key(head, args, ignore_negatives),
        _ => {
            let operands = js_operands(e);
            let n = operands.len() as f64;
            // The nested keys deliberately drop `ignore_negatives`; see the doc
            // comment.
            let factor_keys = Key::Arr(operands.iter().map(|o| sort_key(o, false)).collect());
            match e {
                Expr::Mul(_) => Key::Arr(vec![
                    Key::Num(4.0),
                    Key::Str("product".to_string()),
                    Key::Num(n),
                    factor_keys,
                ]),
                Expr::Div(..) => Key::Arr(vec![
                    Key::Num(4.0),
                    Key::Str("quotient".to_string()),
                    Key::Num(n),
                    factor_keys,
                ]),
                Expr::Add(_) => Key::Arr(vec![
                    Key::Num(5.0),
                    Key::Str("sum".to_string()),
                    Key::Num(n),
                    factor_keys,
                ]),
                Expr::Neg(_) if ignore_negatives => first_item(factor_keys),
                Expr::Neg(_) => Key::Arr(vec![
                    Key::Num(6.0),
                    Key::Str("minus".to_string()),
                    Key::Num(n),
                    factor_keys,
                ]),
                Expr::OtherOp(s, _) if s.name() == "pm" => {
                    if ignore_negatives {
                        first_item(factor_keys)
                    } else {
                        Key::Arr(vec![
                            Key::Num(6.0),
                            Key::Str("pm".to_string()),
                            Key::Num(n),
                            factor_keys,
                        ])
                    }
                }
                // Shapes that can be coerced into each other sort together, so
                // that a tuple and an open interval — or an array and a closed
                // one — land in the same place. The operator name is left out
                // of the key for exactly that reason.
                Expr::Seq(SeqKind::Tuple | SeqKind::Vector | SeqKind::AltVector, _) => {
                    Key::Arr(vec![Key::Num(7.0), Key::Num(n), factor_keys])
                }
                Expr::Seq(SeqKind::Array, _) => {
                    Key::Arr(vec![Key::Num(9.0), Key::Num(n), factor_keys])
                }
                Expr::Interval { closed, .. } => match closed {
                    (false, false) => {
                        let mut parts = vec![Key::Num(7.0)];
                        parts.extend(tail_of_first(&factor_keys));
                        Key::Arr(parts)
                    }
                    (true, true) => {
                        let mut parts = vec![Key::Num(9.0)];
                        parts.extend(tail_of_first(&factor_keys));
                        Key::Arr(parts)
                    }
                    _ => Key::Arr(vec![Key::Num(8.0), Key::Num(n), factor_keys]),
                },
                other => Key::Arr(vec![
                    Key::Num(10.0),
                    Key::Str(legacy_operator(other)),
                    Key::Num(n),
                    factor_keys,
                ]),
            }
        }
    }
}

fn apply_key(head: &Expr, args: &[Expr], ignore_negatives: bool) -> Key {
    // `sqrt`, `cbrt` and `nthroot` are one family spelled three ways, and key
    // as the family plus its degree so they sort together and by degree.
    let mut key = match head {
        Expr::Sym(s) if s.name() == "sqrt" => {
            vec![Key::Num(5.0), Key::Str("root".to_string()), Key::Num(2.0)]
        }
        Expr::Sym(s) if s.name() == "cbrt" => {
            vec![Key::Num(5.0), Key::Str("root".to_string()), Key::Num(3.0)]
        }
        Expr::Sym(s) if s.name() == "nthroot" => {
            let degree = match args.get(1) {
                Some(Expr::Num(n)) => Key::Num(n.to_f64()),
                Some(other) => raw_key(other),
                None => Key::Num(2.0),
            };
            vec![Key::Num(5.0), Key::Str("root".to_string()), degree]
        }
        _ => vec![
            Key::Num(2.0),
            Key::Str("function".to_string()),
            raw_key(head),
        ],
    };

    // JS reads the argument slot straight out of the tree: one argument that is
    // itself an array has its head dropped and its operands read as the
    // argument list. That is why `f(x+y)` keys as a two-argument call — a quirk
    // of the encoding, faithfully kept, since it only affects ordering.
    let (n_args, arg_keys) = match args {
        [only] => match js_operands_opt(only) {
            Some(ops) => (
                ops.len(),
                ops.iter().map(|a| sort_key(a, ignore_negatives)).collect(),
            ),
            None => (1, vec![sort_key(only, ignore_negatives)]),
        },
        many => (
            many.len(),
            many.iter().map(|a| sort_key(a, ignore_negatives)).collect(),
        ),
    };
    key.push(Key::Arr(vec![Key::Num(n_args as f64), Key::Arr(arg_keys)]));
    Key::Arr(key)
}

fn arr3(tag: f64, kind: &str, value: Key) -> Key {
    Key::Arr(vec![Key::Num(tag), Key::Str(kind.to_string()), value])
}

/// JS `key[1] += "_" + unit` — the last step of the `unit` branch.
///
/// A *string* concatenation whatever index 1 held. For the keys this actually
/// meets it is the kind (`"number"` → `"number_%"`), but a tuple or array key
/// carries its operand count there and JS turns that into `"2_%"`; the tag at
/// index 0 is untouched either way, which is what keeps the unit sorting with
/// the kind of thing it annotates.
///
/// The container case is why [`cmp_key`] ranks a `Str` above a `Num` rather
/// than coercing: JS's own comparison of `"2_%"` with `3` is `NaN` in both
/// directions, which ties a unit-annotated container with every plain one and
/// leaves the sort without a normal form.
fn append_unit(key: Key, unit: &str) -> Key {
    match key {
        Key::Arr(mut items) if items.len() > 1 => {
            items[1] = Key::Str(format!("{}_{}", js_to_string(&items[1]), unit));
            Key::Arr(items)
        }
        other => other,
    }
}

/// A key scalar as JS stringifies it when concatenated onto a string.
fn js_to_string(k: &Key) -> String {
    match k {
        Key::Str(s) => s.clone(),
        Key::Num(v) => crate::num::js_f64_to_string(*v),
        Key::Bool(b) => b.to_string(),
        // Index 1 is never an array in any key this file builds.
        Key::Arr(_) => String::new(),
    }
}

fn key_items(k: Key) -> Vec<Key> {
    match k {
        Key::Arr(xs) => xs,
        other => vec![other],
    }
}

fn first_item(k: Key) -> Key {
    match k {
        Key::Arr(mut xs) if !xs.is_empty() => xs.remove(0),
        other => other,
    }
}

/// The first factor key with its leading tag dropped — JS `factor_keys[0].slice(1)`,
/// used so an interval keys identically to the tuple or array it can be read as.
fn tail_of_first(factor_keys: &Key) -> Vec<Key> {
    match factor_keys {
        Key::Arr(xs) => match xs.first() {
            Some(Key::Arr(inner)) if !inner.is_empty() => inner[1..].to_vec(),
            Some(other) => vec![other.clone()],
            None => Vec::new(),
        },
        other => vec![other.clone()],
    }
}

/// The raw JS tree value, as a key. Used where JS pushes an operand into the
/// key without keying it — a function's name, which is usually a string but is
/// an array for `f'`.
fn raw_key(e: &Expr) -> Key {
    if let Some(name) = js_string_leaf(e) {
        return Key::Str(name);
    }
    match e {
        Expr::Num(n) => Key::Num(n.to_f64()),
        Expr::Bool(b) => Key::Bool(*b),
        other => {
            let mut parts = vec![Key::Str(legacy_operator(other))];
            parts.extend(js_operands(other).iter().map(raw_key));
            Key::Arr(parts)
        }
    }
}

/// The operand list this node would have had in the JS tree, or `None` when the
/// JS tree holds a bare scalar there rather than an array.
fn js_operands_opt(e: &Expr) -> Option<Vec<Expr>> {
    match e {
        Expr::Num(_) | Expr::Bool(_) => None,
        _ if js_string_leaf(e).is_some() => None,
        // The specials serialize as tagged objects, not arrays.
        Expr::Const(_) => None,
        Expr::RootOf { .. } => None,
        _ => Some(js_operands(e)),
    }
}

pub(crate) fn js_operands(e: &Expr) -> Vec<Expr> {
    match e {
        Expr::Add(xs)
        | Expr::Mul(xs)
        | Expr::And(xs)
        | Expr::Or(xs)
        | Expr::Union(xs)
        | Expr::Intersect(xs)
        | Expr::Seq(_, xs)
        | Expr::OtherOp(_, xs) => xs.clone(),
        Expr::Div(a, b) | Expr::Pow(a, b) | Expr::Index(a, b) => {
            vec![a.as_ref().clone(), b.as_ref().clone()]
        }
        Expr::Neg(a) | Expr::Not(a) | Expr::Prime(a) => vec![a.as_ref().clone()],
        // `["apply", f, arg]` — a single argument sits bare, several become a
        // tuple.
        Expr::Apply(head, args) => vec![
            head.as_ref().clone(),
            match args.as_slice() {
                [only] => only.clone(),
                many => Expr::Seq(SeqKind::Tuple, many.to_vec()),
            },
        ],
        Expr::Interval { endpoints, closed } => vec![
            Expr::Seq(
                SeqKind::Tuple,
                vec![endpoints.0.clone(), endpoints.1.clone()],
            ),
            Expr::Seq(
                SeqKind::Tuple,
                vec![Expr::Bool(closed.0), Expr::Bool(closed.1)],
            ),
        ],
        // A two-operand relation is `[op, a, b]`; a chained one is
        // `["lts", <operands tuple>, <strictness tuple>]`.
        Expr::Relation { operands, ops } => {
            if ops.len() <= 1 {
                operands.clone()
            } else {
                vec![
                    Expr::Seq(SeqKind::Tuple, operands.clone()),
                    Expr::Seq(
                        SeqKind::Tuple,
                        ops.iter()
                            .map(|o| Expr::Bool(matches!(o, RelOp::Lt | RelOp::Gt)))
                            .collect(),
                    ),
                ]
            }
        }
        Expr::Matrix(m) => m.entries().to_vec(),
        _ => Vec::new(),
    }
}

pub(crate) fn legacy_operator(e: &Expr) -> String {
    match e {
        Expr::Pow(..) => "^".to_string(),
        Expr::Prime(_) => "prime".to_string(),
        Expr::Index(..) => "_".to_string(),
        Expr::Not(_) => "not".to_string(),
        Expr::And(_) => "and".to_string(),
        Expr::Or(_) => "or".to_string(),
        Expr::Union(_) => "union".to_string(),
        Expr::Intersect(_) => "intersect".to_string(),
        Expr::Add(_) => "+".to_string(),
        Expr::Mul(_) => "*".to_string(),
        Expr::Div(..) => "/".to_string(),
        Expr::Neg(_) => "-".to_string(),
        Expr::Apply(..) => "apply".to_string(),
        Expr::Seq(k, _) => k.js_name().to_string(),
        Expr::Interval { .. } => "interval".to_string(),
        // A chained relation keys under `lts`, as the JS tree's head would have
        // been; a two-operand one under its own operator.
        Expr::Relation { ops, .. } => {
            if ops.len() > 1 {
                "lts".to_string()
            } else {
                ops.first()
                    .map(|o| o.js_name().to_string())
                    .unwrap_or_else(|| "=".to_string())
            }
        }
        Expr::Matrix(_) => "matrix".to_string(),
        Expr::OtherOp(s, _) => s.name(),
        Expr::RootOf { .. } => "rootof".to_string(),
        Expr::Ldots => "ldots".to_string(),
        _ => "unknown".to_string(),
    }
}
