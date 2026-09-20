//! Scaling units (`%`, `deg`, `$`): the single source of truth for the
//! `["unit", …]` node layout, plus the equality-time desugaring pass.
//!
//! [`is_scaling_unit_symbol`] and [`unit_body`] own the operand-layout
//! knowledge (which operand is the symbol, which is the value); the `me.*`
//! stripping façade in [`crate::ops`] (`remove_units` / `add_unit`) delegates
//! here rather than re-deriving it. [`desugar_units`] is the equality-time
//! analogue of JS `remove_scaling_units` combined with numerical unit removal.

use crate::expr::Expr;

/// The scaling-unit spellings the parsers emit (see `lib/expression/units.js`):
/// `%`, `deg` (LaTeX spelling `circ`), and the prefix `$`. This is the *symbol*
/// set — a superset of what [`desugar_units`] rewrites (`circ` is recognized as
/// a unit but has no numeric desugaring rule, so it is stripped but not scaled).
pub(crate) fn is_scaling_unit_symbol(e: &Expr) -> bool {
    matches!(e, Expr::Sym(s) if matches!(s.name().as_str(), "%" | "$" | "deg" | "circ"))
}

/// Decode a two-operand `["unit", …]` node into `(symbol, value)`. The parsers
/// emit prefix `$` as `[unit, value]` and postfix `%`/`deg` as `[value, unit]`
/// (mirroring `get_unit_value_of_tree` in lib/expression/units.js); this takes
/// whichever operand *is* a scaling-unit symbol, so either order decodes for
/// either spelling — matching what `ops::remove_units` has always accepted.
/// If both operands are unit symbols the first one is treated as the unit.
fn unit_parts(args: &[Expr]) -> Option<(&Expr, &Expr)> {
    match args {
        [a, b] if is_scaling_unit_symbol(a) => Some((a, b)),
        [a, b] if is_scaling_unit_symbol(b) => Some((b, a)),
        _ => None,
    }
}

/// The value operand of a `["unit", …]` node (the operand that is not the unit
/// symbol). The shared layout primitive behind `me.remove_units`.
pub(crate) fn unit_body(args: &[Expr]) -> Option<&Expr> {
    unit_parts(args).map(|(_, value)| value)
}

/// The value operand, but only for a unit [`desugar_units`] can actually
/// rewrite — everything [`unit_body`] accepts except the `circ` spelling.
///
/// Any pass that has to agree with `equals` must use *this* set, because
/// `equals` desugars first and simply leaves a `circ` node standing. Folding
/// `30 circ + 60 circ → 90 circ` while `equals` treats the nodes as opaque
/// produced a `simplify` result its own equality oracle rejected.
pub(crate) fn desugarable_unit_body(args: &[Expr]) -> Option<&Expr> {
    unit_value(args).map(|(_, value)| value)
}

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

impl Unit {
    /// The symbol as JS spells it in `all_units` — the string a consumer
    /// appends when it needs to name the unit rather than apply it.
    fn name(self) -> &'static str {
        match self {
            Unit::Dollar => "$",
            Unit::Percent => "%",
            Unit::Deg => "deg",
        }
    }
}

/// The `(unit symbol, value)` of a `["unit", …]` node — JS
/// `get_unit_value_of_tree` (lib/expression/units.js).
///
/// Restricted to the units JS's `all_units` lists, so `circ` answers `None`
/// here even though [`is_scaling_unit_symbol`] accepts it — JS never had a
/// `circ` entry. What a caller should *do* with that `None` is its own call:
/// JS destructures the null return and throws, which is not a behaviour worth
/// reproducing. (The LaTeX parser substitutes `\circ` → `deg`, so a `circ`
/// node only ever arrives hand-built.)
pub(crate) fn scaling_unit_and_value(args: &[Expr]) -> Option<(&'static str, &Expr)> {
    unit_value(args).map(|(unit, value)| (unit.name(), value))
}

/// Classify a `["unit", …]` node into its desugarable [`Unit`] and value.
/// `None` for a non-unit node or the `circ` spelling (recognized as a unit
/// symbol, but with no numeric scaling rule).
fn unit_value(args: &[Expr]) -> Option<(Unit, &Expr)> {
    let (symbol, value) = unit_parts(args)?;
    let Expr::Sym(s) = symbol else { return None };
    let unit = match s.name().as_str() {
        "$" => Unit::Dollar,
        "%" => Unit::Percent,
        "deg" => Unit::Deg,
        _ => return None,
    };
    Some((unit, value))
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
                    );
                }
                // An `OtherOp("unit", …)` that does not match a known unit
                // shape is left structurally intact (recurse into operands
                // via the shared traversal below).
                None => {}
            }
        }
    }
    crate::expr::map_children(e, desugar_units)
}

/// The `["unit", …]` node for `value` in `unit`, in the operand order the
/// parsers use: `$` is a prefix (`["unit","$",v]`), `%`/`deg` are postfix
/// (`["unit",v,"%"]`). Round-trips [`unit_parts`], which reads either order.
fn make_unit(unit: &Expr, value: Expr) -> Expr {
    let args = if matches!(unit, Expr::Sym(s) if s.name() == "$") {
        vec![unit.clone(), value]
    } else {
        vec![value, unit.clone()]
    };
    Expr::OtherOp(crate::Sym::new("unit"), args)
}

/// Read a term as a scaling-unit quantity: `(unit symbol, value)`.
///
/// Sees through the two shapes canonicalization produces around a unit — a
/// negation (`−(270 deg)`) and a product with scalar factors (`50% · 5`,
/// `$12/4`, which is `$12 · 4⁻¹`) — folding those factors into the value. A
/// product with *two* unit factors is not a scaling quantity (`$2 · $3` has no
/// meaning here) and returns `None`.
fn as_unit_quantity(e: &Expr) -> Option<(Expr, Expr)> {
    match e {
        // Gated on `unit_value`, not `unit_parts`: only a unit `desugar_units`
        // rewrites may be folded, or `simplify` and `equals` disagree — see
        // [`desugarable_unit_body`]. `circ` is a unit symbol with no scaling
        // rule, so it is opaque to both.
        Expr::OtherOp(name, args) if name.name() == "unit" => {
            let (unit, _) = unit_parts(args)?;
            let value = desugarable_unit_body(args)?;
            Some((unit.clone(), value.clone()))
        }
        Expr::Neg(x) => {
            let (unit, value) = as_unit_quantity(x)?;
            Some((unit, Expr::Neg(Box::new(value))))
        }
        Expr::Mul(factors) => {
            let mut found: Option<(Expr, Expr)> = None;
            let mut rest = Vec::new();
            for f in factors {
                match (as_unit_quantity(f), &found) {
                    // A second unit factor: not a single scaling quantity.
                    (Some(_), Some(_)) => return None,
                    (Some(uv), None) => found = Some(uv),
                    (None, _) => rest.push(f.clone()),
                }
            }
            let (unit, value) = found?;
            if rest.is_empty() {
                return Some((unit, value));
            }
            rest.push(value);
            Some((unit, Expr::Mul(rest)))
        }
        _ => None,
    }
}

/// Combine like scaling units and absorb scalar factors into a unit, so
/// `simplify` folds unit arithmetic the way `equals` already evaluates it:
/// `$3 + $2 → $5`, `50% · 5 → 250%`, `$12/4 → $3`, `360 deg − 270 deg → 90 deg`.
///
/// Two boundaries are deliberate, because crossing either would assert
/// something false:
/// - **unlike units never combine** (`$3 + 2 deg` is left alone) — there is no
///   conversion between these, only the shared *scaling* shape;
/// - **a unit never combines with a bare scalar** (`$3 + 2`), which is why the
///   grouping requires two terms of the *same* unit before it rewrites
///   anything.
///
/// Only `+` and `·` are folded. The result is canonical because every value it
/// builds goes back through the smart constructors.
pub(crate) fn fold_units(e: &Expr) -> Expr {
    let e = crate::expr::map_children(e, fold_units);
    // A scalar × unit (or a negated / divided one) absorbs the scalar into the
    // value: `3·50deg → 150deg`, `$50·3 → $150`, `x·$50·y/10 → $5xy`. The value
    // is canonicalized so `3·50` folds to `150`. (Sums of like units are handled
    // by the `Add` arm below.)
    if matches!(e, Expr::Mul(_) | Expr::Neg(_)) {
        if let Some((unit, value)) = as_unit_quantity(&e) {
            return make_unit(&unit, super::canonicalize(&value));
        }
    }
    match &e {
        Expr::Add(terms) => {
            // Group by unit symbol, preserving first-seen order.
            let mut groups: Vec<(Expr, Vec<Expr>)> = Vec::new();
            let mut others: Vec<Expr> = Vec::new();
            for t in terms {
                match as_unit_quantity(t) {
                    Some((unit, value)) => match groups.iter_mut().find(|(u, _)| *u == unit) {
                        Some((_, vs)) => vs.push(value),
                        None => groups.push((unit, vec![value])),
                    },
                    None => others.push(t.clone()),
                }
            }
            // Nothing to combine: return the node untouched rather than a
            // rebuilt-but-equal one, so a lone unit keeps its original form.
            if !groups.iter().any(|(_, vs)| vs.len() > 1) {
                return e;
            }
            let mut out = others;
            for (unit, values) in groups {
                let value = if values.len() == 1 {
                    values.into_iter().next().expect("len checked")
                } else {
                    super::add(values)
                };
                out.push(make_unit(&unit, value));
            }
            super::add(out)
        }
        Expr::Mul(_) => match as_unit_quantity(&e) {
            // `as_unit_quantity` already folded the scalar factors into the
            // value; rebuild only when it actually absorbed something.
            Some((unit, value)) => {
                let folded = make_unit(&unit, super::canonicalize(&value));
                if folded == e {
                    e
                } else {
                    folded
                }
            }
            None => e,
        },
        _ => e,
    }
}
