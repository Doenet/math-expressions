//! Scaling-unit desugaring (`%`, `deg`, `$`) — the equality-time analogue of
//! JS `remove_scaling_units` combined with numerical unit removal.

use crate::expr::Expr;

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
    // One variant-specific rewrite; everything else is the blessed traversal
    // (`map_children`), so new `Expr` variants need no edit here.
    if let Expr::OtherOp(name, args) = e {
        if name.name() == "unit" {
            match unit_value(args) {
                Some((Unit::Dollar, v)) => {
                    return Expr::Mul(vec![Expr::sym("$"), desugar_units(v)])
                }
                Some((Unit::Percent, v)) => {
                    return Expr::Div(Box::new(desugar_units(v)), Box::new(Expr::int(100)))
                }
                Some((Unit::Deg, v)) => {
                    return Expr::Div(
                        Box::new(Expr::Mul(vec![
                            desugar_units(v),
                            // `Sym`, not `Const(Pi)`: the canonical spelling
                            // of π (matches the parsers; keeps `==`/tolerance
                            // paths on one representation).
                            Expr::sym("pi"),
                        ])),
                        Box::new(Expr::int(180)),
                    )
                }
                // An `OtherOp("unit", …)` that does not match a known unit
                // shape is left structurally intact (recurse into operands
                // via the shared traversal below).
                None => {}
            }
        }
    }
    crate::norm::syntactic::map_children(e, desugar_units)
}
