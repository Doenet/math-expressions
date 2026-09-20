//! Traversal and n-ary flattening of the [`Expr`] tree.
//!
//! [`Expr::children`] is the single full-variant read-only traversal every
//! contains-X predicate is built on; [`map_children`] is its rebuilding
//! (structure-preserving map) counterpart; [`flatten`] merges nested
//! associative operators.

use super::Expr;

impl Expr {
    /// All immediate child expressions (empty for leaves). The single
    /// full-variant read-only traversal — predicates like [`Expr::any_subexpr`]
    /// and the crate's contains-X checks are built on it, so a new variant
    /// needs exactly one match arm here (the compiler enforces it).
    pub fn children(&self) -> Vec<&Expr> {
        match self {
            Expr::Num(_)
            | Expr::Sym(_)
            | Expr::Const(_)
            | Expr::Bool(_)
            | Expr::RootOf { .. }
            | Expr::Blank
            | Expr::Ldots => vec![],
            Expr::Add(xs)
            | Expr::Mul(xs)
            | Expr::And(xs)
            | Expr::Or(xs)
            | Expr::Union(xs)
            | Expr::Intersect(xs)
            | Expr::Seq(_, xs)
            | Expr::OtherOp(_, xs) => xs.iter().collect(),
            Expr::Apply(h, xs) => std::iter::once(&**h).chain(xs.iter()).collect(),
            Expr::Div(a, b) | Expr::Pow(a, b) | Expr::Index(a, b) => vec![a, b],
            Expr::Neg(x) | Expr::Not(x) | Expr::Prime(x) => vec![x],
            Expr::Interval { endpoints, .. } => vec![&endpoints.0, &endpoints.1],
            Expr::Relation { operands, .. } => operands.iter().collect(),
            Expr::Matrix(m) => m.entries().iter().collect(),
        }
    }

    /// Does `pred` hold for this expression or any subexpression?
    pub fn any_subexpr(&self, pred: &dyn Fn(&Expr) -> bool) -> bool {
        pred(self) || self.children().into_iter().any(|c| c.any_subexpr(pred))
    }
}

/// Flatten nested associative operators, porting flatten.js exactly:
/// a same-operator child is merged only when it has >= 2 operands
/// (JS: `operands[i].length > 2`), so unary `["+", x]` survives.
pub fn flatten(expr: Expr) -> Expr {
    fn flatten_args(args: Vec<Expr>) -> Vec<Expr> {
        args.into_iter().map(flatten).collect()
    }

    /// Merge same-variant children (matched by `same`) with >= 2 operands.
    fn merge(args: Vec<Expr>, same: fn(&Expr) -> Option<&Vec<Expr>>) -> Vec<Expr> {
        let mut result = Vec::with_capacity(args.len());
        for a in args {
            match same(&a) {
                Some(inner) if inner.len() >= 2 => {
                    if let Some(inner) = into_args(a) {
                        result.extend(inner);
                    }
                }
                _ => result.push(a),
            }
        }
        result
    }

    fn into_args(e: Expr) -> Option<Vec<Expr>> {
        match e {
            Expr::Add(v)
            | Expr::Mul(v)
            | Expr::And(v)
            | Expr::Or(v)
            | Expr::Union(v)
            | Expr::Intersect(v) => Some(v),
            _ => None,
        }
    }

    macro_rules! assoc {
        ($variant:ident, $args:expr) => {{
            let args = flatten_args($args);
            let args = merge(args, |e| match e {
                Expr::$variant(v) => Some(v),
                _ => None,
            });
            Expr::$variant(args)
        }};
    }

    match expr {
        Expr::Add(args) => assoc!(Add, args),
        Expr::Mul(args) => assoc!(Mul, args),
        Expr::And(args) => assoc!(And, args),
        Expr::Or(args) => assoc!(Or, args),
        Expr::Union(args) => assoc!(Union, args),
        Expr::Intersect(args) => assoc!(Intersect, args),

        // Non-associative nodes: recurse into children.
        Expr::Div(a, b) => Expr::Div(Box::new(flatten(*a)), Box::new(flatten(*b))),
        Expr::Pow(a, b) => Expr::Pow(Box::new(flatten(*a)), Box::new(flatten(*b))),
        Expr::Neg(a) => Expr::Neg(Box::new(flatten(*a))),
        Expr::Not(a) => Expr::Not(Box::new(flatten(*a))),
        Expr::Prime(a) => Expr::Prime(Box::new(flatten(*a))),
        Expr::Index(a, b) => Expr::Index(Box::new(flatten(*a)), Box::new(flatten(*b))),
        Expr::Apply(head, args) => Expr::Apply(Box::new(flatten(*head)), flatten_args(args)),
        Expr::Seq(kind, args) => Expr::Seq(kind, flatten_args(args)),
        Expr::Interval { endpoints, closed } => {
            let (a, b) = *endpoints;
            Expr::Interval {
                endpoints: Box::new((flatten(a), flatten(b))),
                closed,
            }
        }
        Expr::Relation { operands, ops } => Expr::Relation {
            operands: flatten_args(operands),
            ops,
        },
        Expr::Matrix(m) => Expr::Matrix(m.into_map(flatten)),
        Expr::OtherOp(op, args) => Expr::OtherOp(op, flatten_args(args)),

        // Leaves — spelled out (no catch-all) so that adding a new compound
        // variant is a compile error here rather than it silently being
        // treated as a leaf and never flattened.
        leaf @ (Expr::Num(_)
        | Expr::Sym(_)
        | Expr::Const(_)
        | Expr::Bool(_)
        | Expr::RootOf { .. }
        | Expr::Blank
        | Expr::Ldots) => leaf,
    }
}

/// Apply `f` to every immediate `Expr` child, rebuilding the node; leaves are
/// returned unchanged. The rebuilding counterpart of [`Expr::children`] and the
/// crate's generic structure-preserving tree map — shared by the syntactic
/// passes, `normalize::simplify`, and the `ops` rewrites. Generic over `FnMut`
/// so callers can thread state (e.g. a change flag).
pub(crate) fn map_children<F: FnMut(&Expr) -> Expr>(e: &Expr, mut f: F) -> Expr {
    match e {
        Expr::Num(_)
        | Expr::Sym(_)
        | Expr::Const(_)
        | Expr::Bool(_)
        | Expr::RootOf { .. }
        | Expr::Blank
        | Expr::Ldots => e.clone(),
        Expr::Add(xs) => Expr::Add(xs.iter().map(&mut f).collect()),
        Expr::Mul(xs) => Expr::Mul(xs.iter().map(&mut f).collect()),
        Expr::And(xs) => Expr::And(xs.iter().map(&mut f).collect()),
        Expr::Or(xs) => Expr::Or(xs.iter().map(&mut f).collect()),
        Expr::Union(xs) => Expr::Union(xs.iter().map(&mut f).collect()),
        Expr::Intersect(xs) => Expr::Intersect(xs.iter().map(&mut f).collect()),
        Expr::Div(a, b) => Expr::Div(Box::new(f(a)), Box::new(f(b))),
        Expr::Pow(a, b) => Expr::Pow(Box::new(f(a)), Box::new(f(b))),
        Expr::Index(a, b) => Expr::Index(Box::new(f(a)), Box::new(f(b))),
        Expr::Neg(x) => Expr::Neg(Box::new(f(x))),
        Expr::Not(x) => Expr::Not(Box::new(f(x))),
        Expr::Prime(x) => Expr::Prime(Box::new(f(x))),
        Expr::Apply(h, xs) => {
            let h = f(h);
            Expr::Apply(Box::new(h), xs.iter().map(&mut f).collect())
        }
        Expr::Seq(k, xs) => Expr::Seq(*k, xs.iter().map(&mut f).collect()),
        Expr::Interval { endpoints, closed } => Expr::Interval {
            endpoints: Box::new((f(&endpoints.0), f(&endpoints.1))),
            closed: *closed,
        },
        Expr::Relation { operands, ops } => Expr::Relation {
            operands: operands.iter().map(&mut f).collect(),
            ops: ops.clone(),
        },
        Expr::Matrix(m) => Expr::Matrix(m.map(&mut f)),
        Expr::OtherOp(name, xs) => Expr::OtherOp(*name, xs.iter().map(&mut f).collect()),
    }
}
