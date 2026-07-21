//! Scaling-unit desugaring (`%`, `deg`, `$`) — the equality-time analogue of
//! JS `remove_scaling_units` combined with numerical unit removal.

use crate::expr::{Expr, MathConst};

/// The three scaling units from lib/expression/units.js.
enum Unit {
    /// `$` — a `prefix` unit that only marks its value (`scale: x => x`), so it
    /// survives desugaring as a free factor.
    Dollar,
    /// `%` — `only_scales`, `scale: x => x / 100`.
    Percent,
    /// `deg` — `only_scales`, `scale: x => x * pi / 180`.
    Deg,
}

/// Match the `["unit", …]` operand layout the parsers emit: prefix `$` is
/// `[unit, value]`; postfix `%`/`deg` is `[value, unit]` (mirrors
/// `get_unit_value_of_tree` in lib/expression/units.js).
fn unit_value(args: &[Expr]) -> Option<(Unit, &Expr)> {
    if args.len() != 2 {
        return None;
    }
    if let Expr::Sym(s) = &args[0] {
        if s.name() == "$" {
            return Some((Unit::Dollar, &args[1]));
        }
    }
    if let Expr::Sym(s) = &args[1] {
        match s.name().as_str() {
            "%" => return Some((Unit::Percent, &args[0])),
            "deg" => return Some((Unit::Deg, &args[0])),
            _ => {}
        }
    }
    None
}

/// Rewrite scaling-unit nodes into plain arithmetic. This is the equality-time
/// analogue of JS `remove_scaling_units` (lib/expression/simplify.js) combined
/// with numerical unit removal:
///
/// - `n %`   → `n / 100`
/// - `n deg` → `n * pi / 180`
/// - `$ n`   → `$ * n`  (the `$` becomes an ordinary factor)
///
/// Making `$` a plain multiplication by the symbol `$` is what preserves the JS
/// semantics with no special-casing downstream: the like-term folding in
/// [`add`](super::add) then gives `$3 + $2 → $5`, while the numerical stage
/// samples `$` as a free variable, so `$5` never equals a bare `5`. It is
/// applied only in the full [`equals`](crate::equals) path — never in
/// `equalsViaSyntax` — so `50%` and `1/2` stay *syntactically* distinct even
/// though they are numerically equal.
pub fn desugar_units(e: &Expr) -> Expr {
    match e {
        Expr::OtherOp(name, args) if name.name() == "unit" => match unit_value(args) {
            Some((Unit::Dollar, v)) => Expr::Mul(vec![Expr::sym("$"), desugar_units(v)]),
            Some((Unit::Percent, v)) => {
                Expr::Div(Box::new(desugar_units(v)), Box::new(Expr::int(100)))
            }
            Some((Unit::Deg, v)) => Expr::Div(
                Box::new(Expr::Mul(vec![
                    desugar_units(v),
                    Expr::Const(MathConst::Pi),
                ])),
                Box::new(Expr::int(180)),
            ),
            // An `OtherOp("unit", …)` that does not match a known unit shape is
            // left structurally intact (recurse into its operands).
            None => Expr::OtherOp(*name, args.iter().map(desugar_units).collect()),
        },

        Expr::Num(_)
        | Expr::Sym(_)
        | Expr::Const(_)
        | Expr::RootOf { .. }
        | Expr::Blank
        | Expr::Ldots => e.clone(),

        Expr::Add(xs) => Expr::Add(xs.iter().map(desugar_units).collect()),
        Expr::Mul(xs) => Expr::Mul(xs.iter().map(desugar_units).collect()),
        Expr::And(xs) => Expr::And(xs.iter().map(desugar_units).collect()),
        Expr::Or(xs) => Expr::Or(xs.iter().map(desugar_units).collect()),
        Expr::Union(xs) => Expr::Union(xs.iter().map(desugar_units).collect()),
        Expr::Intersect(xs) => Expr::Intersect(xs.iter().map(desugar_units).collect()),

        Expr::Div(a, b) => Expr::Div(Box::new(desugar_units(a)), Box::new(desugar_units(b))),
        Expr::Pow(a, b) => Expr::Pow(Box::new(desugar_units(a)), Box::new(desugar_units(b))),
        Expr::Index(a, b) => Expr::Index(Box::new(desugar_units(a)), Box::new(desugar_units(b))),
        Expr::Neg(x) => Expr::Neg(Box::new(desugar_units(x))),
        Expr::Not(x) => Expr::Not(Box::new(desugar_units(x))),
        Expr::Prime(x) => Expr::Prime(Box::new(desugar_units(x))),

        Expr::Apply(h, xs) => Expr::Apply(
            Box::new(desugar_units(h)),
            xs.iter().map(desugar_units).collect(),
        ),
        Expr::Seq(k, xs) => Expr::Seq(*k, xs.iter().map(desugar_units).collect()),
        Expr::Interval { endpoints, closed } => Expr::Interval {
            endpoints: Box::new((desugar_units(&endpoints.0), desugar_units(&endpoints.1))),
            closed: *closed,
        },
        Expr::Relation { operands, ops } => Expr::Relation {
            operands: operands.iter().map(desugar_units).collect(),
            ops: ops.clone(),
        },
        Expr::Matrix {
            rows,
            cols,
            entries,
        } => Expr::Matrix {
            rows: *rows,
            cols: *cols,
            entries: entries.iter().map(desugar_units).collect(),
        },
        Expr::OtherOp(name, args) => Expr::OtherOp(*name, args.iter().map(desugar_units).collect()),
    }
}
