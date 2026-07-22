//! Expression tree (PORTING_PLAN.md §5).
//!
//! One enum serves two layers: the *faithful* layer (parser output — flat
//! n-ary ops, but unsorted and unfolded) and the *canonical* layer (produced
//! by normalize(), phase 4). `Div` and `Neg` exist only in the faithful
//! layer; canonicalisation rewrites them. `OtherOp` lives in BOTH layers:
//! canonicalize preserves it, and canonical-layer code mints new ones (`pm`,
//! `derivative` nodes from diff, matrix ops like `det`/`rref`,
//! `discrete_infinite_set`) — do not assume an `OtherOp` arm is dead on
//! canonical trees.

use crate::num::Number;
use crate::sym::Sym;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    // Atomic leaves
    Num(Number),
    Sym(Sym),
    Const(MathConst),
    /// The `index`-th root of the univariate polynomial with the given dense
    /// coefficients (low → high) — MATRIX_PLAN.md §2a. A *leaf*: the
    /// coefficients are `Number`s, not subexpressions, so traversal and
    /// substitution treat it as an atom. Canonical invariant: primitive
    /// integer coefficients, positive leading coefficient, squarefree;
    /// `index` follows the canonical root order (real roots ascending, then
    /// conjugate pairs, negative imaginary part first). Text form
    /// `rootof(t^3 - t - 1, 2)`.
    RootOf { poly: Box<[Number]>, index: u32 },
    /// Missing operand "＿" — a real variant, not a magic symbol.
    Blank,
    /// "..." inside lists — ["ldots"] in the JS AST.
    Ldots,

    // Algebraic core (n-ary ops always flat; sorted only in canonical layer)
    Add(Vec<Expr>),
    Mul(Vec<Expr>),
    /// Faithful layer only ("a/b" prints as written); canonicalised to
    /// Mul(a, Pow(b, -1)).
    Div(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    /// Faithful layer only; canonicalised to Mul(-1, x).
    Neg(Box<Expr>),

    // Boolean / set algebra (n-ary, flattened, same invariants as Add/Mul)
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Not(Box<Expr>),
    Union(Vec<Expr>),
    Intersect(Vec<Expr>),

    // Function application. The head is a full expression, NOT just a name:
    //   f'(x) → Apply(Prime(f), [x]);  sin^2(x) → Apply(Pow(sin, 2), [x])
    // Args are native (f(x,y) has two args); the JS single-arg-tuple encoding
    // lives in js_tree.
    Apply(Box<Expr>, Vec<Expr>),

    // Notation nodes (from the parsers)
    Prime(Box<Expr>),            // f'  — ["prime", f]
    Index(Box<Expr>, Box<Expr>), // x_i — ["_", x, i]

    // Sequences: one variant + kind, instead of five unrelated JS heads.
    Seq(SeqKind, Vec<Expr>),

    /// Closure is metadata, not subexpressions.
    Interval {
        endpoints: Box<(Expr, Expr)>,
        closed: (bool, bool),
    },

    /// Relations, chained: "x < y <= z" → operands [x, y, z], ops [Lt, Le].
    /// Invariant: operands.len() == ops.len() + 1.
    Relation {
        operands: Vec<Expr>,
        ops: Vec<RelOp>,
    },

    /// Row-major; invariant: entries.len() == rows * cols.
    Matrix {
        rows: u32,
        cols: u32,
        entries: Vec<Expr>,
    },

    /// Escape hatch for the long tail of faithful-layer notation operators
    /// that only parsers and printers touch: angle, unit, pm, d,
    /// derivative_leibniz, forall, exists, implies, iff, arrows, perp,
    /// parallel, binom, vec, linesegment, ":", "|". Algorithms that care
    /// about an operator promote it to a dedicated variant; the tail stays
    /// generic by design (see the §5 design discussion).
    OtherOp(Sym, Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeqKind {
    Tuple,
    Array,
    List,
    Set,
    Vector,
    AltVector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MathConst {
    Pi,
    E,
    I,
    Inf,
    NegInf,
    NaN,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    In,
    NotIn,
    Ni,
    NotNi,
    Subset,
    NotSubset,
    SubsetEq,
    NotSubsetEq,
    Superset,
    NotSuperset,
    SupersetEq,
    NotSupersetEq,
}

impl RelOp {
    /// The JS tree head for this operator (`["le", a, b]` etc.).
    pub fn js_name(self) -> &'static str {
        match self {
            RelOp::Eq => "=",
            RelOp::Ne => "ne",
            RelOp::Lt => "<",
            RelOp::Le => "le",
            RelOp::Gt => ">",
            RelOp::Ge => "ge",
            RelOp::In => "in",
            RelOp::NotIn => "notin",
            RelOp::Ni => "ni",
            RelOp::NotNi => "notni",
            RelOp::Subset => "subset",
            RelOp::NotSubset => "notsubset",
            RelOp::SubsetEq => "subseteq",
            RelOp::NotSubsetEq => "notsubseteq",
            RelOp::Superset => "superset",
            RelOp::NotSuperset => "notsuperset",
            RelOp::SupersetEq => "superseteq",
            RelOp::NotSupersetEq => "notsuperseteq",
        }
    }

    /// The logical negation of this relation (`not(a < b)` ⇔ `a ≥ b`), used by
    /// `simplify_logical` to push `not` through relations. Operand order is
    /// preserved (no swap): each operator maps to its complement.
    pub fn negate(self) -> RelOp {
        match self {
            RelOp::Eq => RelOp::Ne,
            RelOp::Ne => RelOp::Eq,
            RelOp::Lt => RelOp::Ge,
            RelOp::Ge => RelOp::Lt,
            RelOp::Le => RelOp::Gt,
            RelOp::Gt => RelOp::Le,
            RelOp::In => RelOp::NotIn,
            RelOp::NotIn => RelOp::In,
            RelOp::Ni => RelOp::NotNi,
            RelOp::NotNi => RelOp::Ni,
            RelOp::Subset => RelOp::NotSubset,
            RelOp::NotSubset => RelOp::Subset,
            RelOp::SubsetEq => RelOp::NotSubsetEq,
            RelOp::NotSubsetEq => RelOp::SubsetEq,
            RelOp::Superset => RelOp::NotSuperset,
            RelOp::NotSuperset => RelOp::Superset,
            RelOp::SupersetEq => RelOp::NotSupersetEq,
            RelOp::NotSupersetEq => RelOp::SupersetEq,
        }
    }
}

impl SeqKind {
    pub fn js_name(self) -> &'static str {
        match self {
            SeqKind::Tuple => "tuple",
            SeqKind::Array => "array",
            SeqKind::List => "list",
            SeqKind::Set => "set",
            SeqKind::Vector => "vector",
            SeqKind::AltVector => "altvector",
        }
    }
}

impl Expr {
    pub fn sym(name: &str) -> Expr {
        // The blank is a dedicated variant; never intern "＿" as a symbol.
        if name == "\u{ff3f}" {
            Expr::Blank
        } else {
            Expr::Sym(Sym::new(name))
        }
    }

    pub fn int(v: i64) -> Expr {
        Expr::Num(Number::Int(v))
    }

    /// All immediate child expressions (empty for leaves). The single
    /// full-variant read-only traversal — predicates like [`Expr::any_subexpr`]
    /// and the crate's contains-X checks are built on it, so a new variant
    /// needs exactly one match arm here (the compiler enforces it).
    pub fn children(&self) -> Vec<&Expr> {
        match self {
            Expr::Num(_)
            | Expr::Sym(_)
            | Expr::Const(_)
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
            Expr::Matrix { entries, .. } => entries.iter().collect(),
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
        Expr::Matrix {
            rows,
            cols,
            entries,
        } => Expr::Matrix {
            rows,
            cols,
            entries: flatten_args(entries),
        },
        Expr::OtherOp(op, args) => Expr::OtherOp(op, flatten_args(args)),

        // Leaves — spelled out (no catch-all) so that adding a new compound
        // variant is a compile error here rather than it silently being
        // treated as a leaf and never flattened.
        leaf @ (Expr::Num(_)
        | Expr::Sym(_)
        | Expr::Const(_)
        | Expr::RootOf { .. }
        | Expr::Blank
        | Expr::Ldots) => leaf,
    }
}
