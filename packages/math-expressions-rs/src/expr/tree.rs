//! Expression tree — the [`Expr`] enum, its notation sub-enums, and the basic
//! leaf constructors.
//!
//! One enum serves two layers: the *faithful* layer (parser output — flat
//! n-ary ops, but unsorted and unfolded) and the *canonical* layer (produced
//! by normalize(), phase 4). `Div` and `Neg` exist only in the faithful
//! layer; canonicalisation rewrites them. `OtherOp` lives in BOTH layers:
//! canonicalize preserves it, and canonical-layer code mints new ones (`pm`,
//! `derivative` nodes from diff, matrix ops like `det`/`rref`,
//! `discrete_infinite_set`) — do not assume an `OtherOp` arm is dead on
//! canonical trees.
//!
//! Read-only traversal ([`Expr::children`], [`Expr::any_subexpr`]) and n-ary
//! flattening ([`flatten`](super::flatten)) live in [`visit`](super::visit).

use crate::expr::matrix::Mat;
use crate::expr::sym::Sym;
use crate::num::Number;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    // Atomic leaves
    Num(Number),
    Sym(Sym),
    Const(MathConst),
    /// A boolean literal — `["and", true, false]` in the JS AST.
    ///
    /// Deliberately *not* a [`MathConst`]: every other `MathConst` serializes
    /// to a JSON string (`"pi"`) or a tagged object, whereas a boolean must
    /// serialize to a JSON boolean or it comes back as the symbol `"true"`.
    /// It is also not a mathematical constant.
    ///
    /// No parser produces this — there is no text or LaTeX spelling for a
    /// boolean literal — so it only ever enters a tree through
    /// [`serde::try_from_js`](super::serde::try_from_js) or by hand. The
    /// printers spell it `true`/`false`, which reads back as a symbol; that
    /// asymmetry is accepted, since the AST round-trip is the one DoenetML
    /// relies on.
    ///
    /// Interval closures and chained-inequality strictness are *not* booleans
    /// here — they are metadata on [`Expr::Interval`] / [`Expr::Relation`],
    /// which is what keeps their invariants structural. See `ops::components`.
    Bool(bool),
    /// The `index`-th root of the univariate polynomial with the given dense
    /// coefficients (low → high). A *leaf*: the
    /// coefficients are `Number`s, not subexpressions, so traversal and
    /// substitution treat it as an atom. Canonical invariant: primitive
    /// integer coefficients, positive leading coefficient, squarefree;
    /// `index` follows the canonical root order (real roots ascending, then
    /// conjugate pairs, negative imaginary part first). Text form
    /// `rootof(t^3 - t - 1, 2)`.
    RootOf {
        poly: Box<[Number]>,
        index: u32,
    },
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
    // lives in expr::serde.
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

    /// Row-major. The shape invariant `entries.len() == rows * cols` is carried
    /// by [`Mat`](crate::expr::Mat) itself, whose fields are private, so no
    /// tree can hold a mis-shaped matrix.
    Matrix(Mat),

    /// Escape hatch for the long tail of faithful-layer notation operators
    /// that only parsers and printers touch: angle, unit, pm, d,
    /// derivative_leibniz, forall, exists, implies, iff, arrows, perp,
    /// parallel, binom, vec, linesegment, ":", "|". Algorithms that care
    /// about an operator promote it to a dedicated variant; the tail stays
    /// generic by design.
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
    /// The `{"$":"None"}` special — DoenetML's "no value here". A sibling of the
    /// other `{"$":…}` specials in *shape* (they are the four JSON values that
    /// have no bare literal), which is why it lives here rather than as its own
    /// leaf: it serializes to a tagged object, exactly like `Inf`/`NaN`, not to
    /// a string or a JSON scalar. It is not a mathematical constant, but neither
    /// is `NaN`; the grouping is by serde shape, not by meaning.
    ///
    /// No parser produces it and it is non-numeric, so — like [`Expr::Bool`] —
    /// only the AST round-trip through [`serde`](super::serde) is faithful; the
    /// printers spell it `None` for display and it does not read back.
    None,
}

impl MathConst {
    /// The bare-string spelling this constant has in the JS tree, for the three
    /// that have one. `Inf`/`NegInf`/`NaN`/`None` serialize as tagged objects
    /// rather than strings and answer `None` here.
    ///
    /// This is the *spelling*, not a claim about meaning: whether the name it
    /// returns denotes the constant or an ordinary variable is
    /// [`crate::constant_policy`]'s question. Comparators want the spelling
    /// (so both forms of one constant sort together); semantics wants the
    /// policy.
    pub fn symbol_name(self) -> Option<&'static str> {
        match self {
            MathConst::Pi => Some("pi"),
            MathConst::E => Some("e"),
            MathConst::I => Some("i"),
            MathConst::Inf | MathConst::NegInf | MathConst::NaN | MathConst::None => None,
        }
    }
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
}
