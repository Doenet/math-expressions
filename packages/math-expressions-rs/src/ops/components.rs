//! Component access — the port of `me.get_component` / `me.substitute_component`.
//!
//! A *component index* indexes the operand list of the array spelling: component
//! `i` of `["tuple", a, b, c]` is `tree[i + 1]`. So indexing is 0-based over
//! operands and the operator head is never addressable. The JS API takes either
//! a bare index or an array of them (`expr.get_component([2, 1, 2])` walks into
//! nested tuples); a bare index is just the one-element path, and the empty path
//! selects the whole expression.
//!
//! The operand list is read off the **JS tree spelling**, not the Rust variant's
//! own fields, because that is what the call sites were written against. Where
//! the two disagree, this follows the JS:
//!
//! - a matrix is `["matrix", ["tuple", rows, cols], ["tuple", <row-tuples>]]`,
//!   so its component `0` is the dimension pair and `1` the tuple of rows —
//!   an individual entry is the path `[1, row, col]`;
//! - a multi-argument application is `["apply", f, ["tuple", x, y]]`, so its
//!   component `1` is the whole argument tuple.
//!
//! Shapes whose JS spelling carries boolean flags — `interval`, and the
//! `lts`/`gts` mixed relation chains — have no component list here, because
//! `Expr` has no boolean leaf to hand back (issue #83 R2). They return `None`,
//! which is what they did before component paths existed.

use crate::expr::{Expr, Mat, RelOp, SeqKind};
use crate::num::Number;

/// The component at `path`, or `None` if any step is out of range or lands on
/// a shape with no component list. An empty path is the identity.
pub fn get_component(e: &Expr, path: &[usize]) -> Option<Expr> {
    let mut cur = flat(e);
    for &i in path {
        cur = components(&cur)?.into_iter().nth(i)?;
    }
    Some(cur)
}

/// `e` with the component at `path` replaced by `value`. `None` under the same
/// conditions as [`get_component`]; an empty path replaces the whole
/// expression.
pub fn substitute_component(e: &Expr, path: &[usize], value: &Expr) -> Option<Expr> {
    substitute_in(&flat(e), path, &flat(value))
}

/// The tree component paths are taken over. `Expr` keeps associative operators
/// as the parser nested them (`x+y+z` is `Add[Add[x, y], z]`), but the JS tree
/// the call sites index — `expr.tree`, i.e. `expr::serde::to_js` — is flattened
/// first, so `["+", "x", "y", "z"]` has *three* components, not two. Indexing
/// anything else would silently disagree with what the caller sees.
fn flat(e: &Expr) -> Expr {
    crate::expr::flatten(e.clone())
}

/// `substitute_component` over an already-flattened tree (a flattened tree's
/// components are themselves flattened, so this only has to happen once).
fn substitute_in(e: &Expr, path: &[usize], value: &Expr) -> Option<Expr> {
    let Some((&i, rest)) = path.split_first() else {
        return Some(value.clone());
    };
    let mut parts = components(e)?;
    let inner = substitute_in(parts.get(i)?, rest, value)?;
    parts[i] = inner;
    rebuild(e, parts)
}

/// The operands of `e`'s JS spelling, in order.
fn components(e: &Expr) -> Option<Vec<Expr>> {
    Some(match e {
        Expr::Add(xs)
        | Expr::Mul(xs)
        | Expr::And(xs)
        | Expr::Or(xs)
        | Expr::Union(xs)
        | Expr::Intersect(xs)
        | Expr::Seq(_, xs)
        | Expr::OtherOp(_, xs) => xs.clone(),

        Expr::Div(a, b) | Expr::Pow(a, b) | Expr::Index(a, b) => {
            vec![(**a).clone(), (**b).clone()]
        }
        Expr::Neg(x) | Expr::Not(x) | Expr::Prime(x) => vec![(**x).clone()],

        Expr::Apply(head, args) => vec![(**head).clone(), apply_argument(args)],

        // Binary (`["<", a, b]`) and all-equality (`["=", a, b, c]`) chains
        // spell their operands directly; a mixed chain does not (see the module
        // note on boolean flags).
        Expr::Relation { operands, ops }
            if ops.len() == 1 || ops.iter().all(|o| *o == RelOp::Eq) =>
        {
            operands.clone()
        }

        Expr::Matrix(m) if m.cols() > 0 => vec![
            tuple(vec![count(m.rows()), count(m.cols())]),
            tuple(
                m.entries()
                    .chunks(m.cols() as usize)
                    .map(|row| tuple(row.to_vec()))
                    .collect(),
            ),
        ],

        _ => return None,
    })
}

/// Rebuild `e` around a replacement component list. The head and arity are
/// unchanged — `substitute_component` only ever swaps one element — so the
/// length checks here are belt-and-braces against a malformed list.
fn rebuild(e: &Expr, parts: Vec<Expr>) -> Option<Expr> {
    Some(match e {
        Expr::Add(_) => Expr::Add(parts),
        Expr::Mul(_) => Expr::Mul(parts),
        Expr::And(_) => Expr::And(parts),
        Expr::Or(_) => Expr::Or(parts),
        Expr::Union(_) => Expr::Union(parts),
        Expr::Intersect(_) => Expr::Intersect(parts),
        Expr::Seq(kind, _) => Expr::Seq(*kind, parts),
        Expr::OtherOp(name, _) => Expr::OtherOp(*name, parts),

        Expr::Div(..) => {
            let (a, b) = two(parts)?;
            Expr::Div(Box::new(a), Box::new(b))
        }
        Expr::Pow(..) => {
            let (a, b) = two(parts)?;
            Expr::Pow(Box::new(a), Box::new(b))
        }
        Expr::Index(..) => {
            let (a, b) = two(parts)?;
            Expr::Index(Box::new(a), Box::new(b))
        }

        Expr::Neg(_) => Expr::Neg(Box::new(one(parts)?)),
        Expr::Not(_) => Expr::Not(Box::new(one(parts)?)),
        Expr::Prime(_) => Expr::Prime(Box::new(one(parts)?)),

        // Re-read the argument slot exactly as `try_from_js` does: a tuple there
        // is the argument *list*, anything else is a single argument. So
        // substituting a tuple into `f(x)` makes it `f(x, y)`, as in JS.
        Expr::Apply(..) => {
            let (head, arg) = two(parts)?;
            let args = match arg {
                Expr::Seq(SeqKind::Tuple, xs) => xs,
                other => vec![other],
            };
            Expr::Apply(Box::new(head), args)
        }

        Expr::Relation { ops, .. } if parts.len() == ops.len() + 1 => Expr::Relation {
            operands: parts,
            ops: ops.clone(),
        },

        Expr::Matrix(_) => rebuild_matrix(parts)?,

        _ => return None,
    })
}

/// Re-read a matrix from its `[<dimension pair>, <tuple of row tuples>]`
/// operands, mirroring the `"matrix"` arm of `expr::serde::try_from_js`: the
/// declared dimensions win, and the body must be at least that big.
fn rebuild_matrix(parts: Vec<Expr>) -> Option<Expr> {
    let (size, body) = two(parts)?;
    let (rows, cols) = match components(&size)?.as_slice() {
        [r, c] => (as_count(r)?, as_count(c)?),
        _ => return None,
    };
    if rows.checked_mul(cols)? > 1_000_000 {
        return None;
    }
    let body = components(&body)?;
    let mut entries = Vec::with_capacity((rows * cols) as usize);
    for r in 0..rows as usize {
        let row = components(body.get(r)?)?;
        for c in 0..cols as usize {
            entries.push(row.get(c)?.clone());
        }
    }
    // The loops above push exactly `rows * cols` entries or bail via `?`.
    Mat::new(rows, cols, entries).map(Expr::Matrix)
}

/// The single `["apply", f, arg]` argument slot: several arguments ride in one
/// tuple, matching `expr::serde`.
fn apply_argument(args: &[Expr]) -> Expr {
    match args {
        [only] => only.clone(),
        many => tuple(many.to_vec()),
    }
}

fn tuple(xs: Vec<Expr>) -> Expr {
    Expr::Seq(SeqKind::Tuple, xs)
}

fn count(n: u32) -> Expr {
    Expr::Num(Number::Int(i64::from(n)))
}

fn as_count(e: &Expr) -> Option<u32> {
    match e {
        Expr::Num(Number::Int(i)) => u32::try_from(*i).ok(),
        _ => None,
    }
}

fn one(parts: Vec<Expr>) -> Option<Expr> {
    let [a]: [Expr; 1] = parts.try_into().ok()?;
    Some(a)
}

fn two(parts: Vec<Expr>) -> Option<(Expr, Expr)> {
    let [a, b]: [Expr; 2] = parts.try_into().ok()?;
    Some((a, b))
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

    fn txt(e: &Expr) -> String {
        to_text(e, &Default::default())
    }

    fn got(src: &str, path: &[usize]) -> String {
        txt(&get_component(&p(src), path).expect("component should exist"))
    }

    #[test]
    fn indexes_a_sequence_by_operand_position() {
        assert_eq!(got("x^2, y, z", &[0]), "x^2");
        assert_eq!(got("x^2, y, z", &[1]), "y");
        assert_eq!(got("(a,b,c,d)", &[3]), "d");
        assert!(get_component(&p("(a,b,c,d)"), &[4]).is_none());
    }

    /// The JS API accepts a path into nested sequences; a bare index is the
    /// one-element path and the empty path is the identity.
    #[test]
    fn walks_a_nested_path() {
        let src = "(a,(b0,b1), (c0,(c10,c11, c12), c2), d)";
        assert_eq!(got(src, &[1]), "(b0, b1)");
        assert_eq!(got(src, &[1, 0]), "b0");
        assert_eq!(got(src, &[2, 1, 2]), "c12");
        assert_eq!(got(src, &[]), txt(&p(src)));
    }

    /// Component access is generic over the JS operand list, not special-cased
    /// to sequences: `["+", "x", "y"]` has components `x` and `y`.
    #[test]
    fn indexes_ordinary_operators_too() {
        assert_eq!(got("x+y", &[0]), "x");
        assert_eq!(got("x/y", &[1]), "y");
        assert_eq!(got("x^2", &[1]), "2");
    }

    /// `Expr` nests associative operators as the parser built them, but the JS
    /// tree is flattened — `x+y+z` is `["+", "x", "y", "z"]` there, with three
    /// components, and would have only two (`x+y` and `z`) if this indexed the
    /// unflattened tree.
    #[test]
    fn indexes_the_flattened_tree_the_caller_sees() {
        assert_eq!(got("x+y+z", &[2]), "z");
        assert_eq!(got("a*b*c*d", &[3]), "d");
        assert_eq!(
            substitute_component(&p("x+y+z"), &[1], &p("q")).unwrap(),
            crate::expr::flatten(p("x+q+z"))
        );
    }

    /// A matrix follows its JS spelling
    /// `["matrix", ["tuple", rows, cols], ["tuple", <row-tuples>]]`, so an entry
    /// is reached through the body at `[1, row, col]` — not `[row, col]`.
    #[test]
    fn indexes_a_matrix_through_its_js_spelling() {
        let m = Expr::Matrix(
            Mat::new(2, 2, vec![p("x1"), p("x2"), p("x3"), p("x4")]).expect("test matrix shape"),
        );
        assert_eq!(txt(&get_component(&m, &[0]).unwrap()), "(2, 2)");
        assert_eq!(got_expr(&m, &[1, 0, 1]), "x2");
        assert_eq!(got_expr(&m, &[1, 1, 0]), "x3");
        assert!(get_component(&m, &[1, 2, 0]).is_none());
    }

    fn got_expr(e: &Expr, path: &[usize]) -> String {
        txt(&get_component(e, path).expect("component should exist"))
    }

    #[test]
    fn substitutes_at_a_bare_index() {
        let subbed = substitute_component(&p("x^2, y, z"), &[1], &p("q^2")).unwrap();
        assert_eq!(subbed, p("x^2, q^2, z"));
        assert!(substitute_component(&p("(a,b)"), &[2], &p("z")).is_none());
    }

    #[test]
    fn substitutes_along_a_nested_path() {
        let src = p("(a,(b0,b1), (c0,(c10,c11, c12), c2), d)");
        assert_eq!(
            substitute_component(&src, &[1, 0], &p("x")).unwrap(),
            p("(a,(x,b1), (c0,(c10,c11, c12), c2), d)")
        );
        assert_eq!(
            substitute_component(&src, &[2, 1, 2], &p("x")).unwrap(),
            p("(a,(b0,b1), (c0,(c10,c11, x), c2), d)")
        );
    }

    /// Substituting into a matrix has to survive the round trip through the
    /// `["tuple", <row-tuples>]` body — the rebuilt value must still be a
    /// `Matrix`, not the tuple-of-tuples it was decomposed into.
    #[test]
    fn substitutes_a_matrix_entry() {
        let m = Expr::Matrix(
            Mat::new(2, 2, vec![p("x1"), p("x2"), p("x3"), p("x4")]).expect("test matrix shape"),
        );
        let subbed = substitute_component(&m, &[1, 1, 0], &p("q")).unwrap();
        assert_eq!(
            subbed,
            Expr::Matrix(
                Mat::new(2, 2, vec![p("x1"), p("x2"), p("q"), p("x4")]).expect("test matrix shape")
            )
        );
    }

    /// A leaf has no components, and neither do the shapes whose JS spelling
    /// carries boolean flags. `None` rather than a panic — the wasm build
    /// aborts on panic.
    #[test]
    fn shapes_without_a_component_list_return_none() {
        assert!(get_component(&p("x"), &[0]).is_none());
        assert!(get_component(&p("3"), &[0]).is_none());
        assert!(get_component(&p("(1,2]"), &[0]).is_none()); // interval
        assert!(get_component(&p("x < y <= z"), &[0]).is_none()); // mixed chain
        assert!(substitute_component(&p("x"), &[0], &p("y")).is_none());
    }

    /// Every component list must agree with `expr::serde::to_js` operand for
    /// operand — that agreement is the whole contract, and this is what stops
    /// the two from drifting apart.
    #[test]
    fn component_lists_match_the_js_operand_lists() {
        use crate::expr::serde::to_js;
        let sources = [
            "x^2, y, z",
            "(a,(b0,b1),d)",
            "x+y+z",
            "x*y",
            "x/y",
            "x^2",
            "-x",
            "f(x)",
            "f(x,y,z)",
            "x = y",
            "x = y = z",
            "x < y",
            "a and b",
            "not a",
        ];
        for src in sources {
            let e = p(src);
            let js = to_js(&e);
            let arr = js
                .as_array()
                .unwrap_or_else(|| panic!("{src}: not an array"));
            let parts = components(&flat(&e)).unwrap_or_else(|| panic!("{src}: no components"));
            assert_eq!(parts.len(), arr.len() - 1, "{src}: operand count");
            for (i, part) in parts.iter().enumerate() {
                assert_eq!(to_js(part), arr[i + 1], "{src}: operand {i}");
            }
        }
    }
}
