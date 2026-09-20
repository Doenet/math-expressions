//! Unit annotations: stripping and adding `["unit", …]` wrappers, port of
//! `me.remove_units` / `me.remove_scaling_units` / `me.add_unit`. The
//! `["unit", …]` node layout itself is owned by [`crate::normalize`] (the
//! scaling-unit engine); this module is the `me.*`-parity façade over it.

use crate::expr::map_children;
use crate::expr::Expr;
use crate::normalize::unit_body;

/// `me.remove_units`: strip unit annotations. With `scale_based_on_unit`, the
/// scaling units are applied (`50%` → `1/2`, `90 deg` → `pi/2`) via
/// [`crate::normalize::desugar_units`]; without it, the bare value is kept
/// (`50%` → `50`).
pub fn remove_units(e: &Expr, scale_based_on_unit: bool) -> Expr {
    if let Expr::OtherOp(name, args) = e {
        if name.name() == "unit" {
            if scale_based_on_unit {
                // `$` is an identity-scale marker (`scale: x => x`): *removing*
                // it yields the bare value. `desugar_units` deliberately keeps it
                // as a `$·v` factor so `$5 ≠ 5` for equality, which is the wrong
                // answer here — `me.remove_units` strips the unit. `%`/`deg` do
                // scale (`v/100`, `v·π/180`), which desugaring already gives.
                if let Some(("$", value)) = crate::normalize::scaling_unit_and_value(args) {
                    return remove_units(value, true);
                }
                return crate::normalize::desugar_units(e);
            }
            if let Some(body) = unit_body(args) {
                return remove_units(body, false);
            }
        }
    }
    map_children(e, |c| remove_units(c, scale_based_on_unit))
}

/// `me.remove_scaling_units`: drop only the *scaling* units (`%`, `deg`, `$`),
/// rewriting them into plain arithmetic. Identical to the equality-time
/// [`crate::normalize::desugar_units`] pass.
pub fn remove_scaling_units(e: &Expr) -> Expr {
    crate::normalize::desugar_units(e)
}

/// `me.add_unit`: wrap `e` in the given unit. `$` is a prefix unit
/// (`unit($, e)`); everything else is postfix (`unit(e, name)`).
pub fn add_unit(e: &Expr, unit: &str) -> Expr {
    let args = if unit == "$" {
        vec![Expr::sym("$"), e.clone()]
    } else {
        vec![e.clone(), Expr::sym(unit)]
    };
    Expr::OtherOp(crate::expr::sym::Sym::new("unit"), args)
}
