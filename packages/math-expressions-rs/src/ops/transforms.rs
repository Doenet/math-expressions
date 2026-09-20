//! Structural rewrites that map one faithful tree to another: symbol
//! substitution, subscript ⇄ flat-name conversion, tuple/vector reinterpretation,
//! interval coercion, and function-name canonicalization.

use crate::expr::map_children;
use crate::expr::Expr;
use std::collections::HashMap;

/// Simultaneously replace each `Sym(name)` with `subs[name]`. Substitution is
/// one-pass and simultaneous — a replacement is not itself re-substituted, so
/// `{x: y, y: x}` swaps — and does NOT simplify (`x^2` with `x → 2` gives
/// `2^2`, not `4`), matching `me.substitute`. Recurses into every subexpression,
/// including function arguments.
pub fn substitute(e: &Expr, subs: &HashMap<String, Expr>) -> Expr {
    match e {
        Expr::Sym(s) => match subs.get(&s.name()) {
            Some(rep) => rep.clone(),
            None => e.clone(),
        },
        _ => map_children(e, |c| substitute(c, subs)),
    }
}

/// Collapse simple subscripts into flat symbol names, port of
/// `me.subscripts_to_strings`: `x_1` (`Index(x, 1)`) → the symbol `x_1`.
/// Both halves must be a bare symbol or a number (`2_2` and `3_y` collapse just
/// as `x_2` does); anything else is left structural.
pub fn subscripts_to_strings(e: &Expr) -> Expr {
    subscripts_to_strings_with(e, false)
}

/// [`subscripts_to_strings`] with legacy's `force` flag: a subscript one of
/// whose halves is a compound expression is collapsed too, by spelling the
/// whole node in text notation (`(x^3)_2`). Off by default because the result
/// is a symbol whose *name* is punctuation — readable, but no longer a tree
/// anything can compute with.
pub fn subscripts_to_strings_with(e: &Expr, force: bool) -> Expr {
    if let Expr::Index(base, idx) = e {
        if let (Some(b), Some(sfx)) = (flat_name(base), flat_name(idx)) {
            return Expr::sym(&format!("{}_{}", b, sfx));
        }
        if force {
            return Expr::sym(&crate::to_text(e, &crate::print::TextOpts::default()));
        }
    }
    map_children(e, |c| subscripts_to_strings_with(c, force))
}

/// The flat spelling of a subscript half, or `None` if it is compound.
fn flat_name(e: &Expr) -> Option<String> {
    match e {
        Expr::Sym(s) => Some(s.name()),
        Expr::Num(n) => n.terminating_decimal(),
        _ => None,
    }
}

/// Inverse of [`subscripts_to_strings`]: a symbol containing `_` splits at the
/// first underscore into `Index(base, index)`, with a numeric half parsed back
/// to a number (`x_1` → `Index(x, 1)`, `2_2` → `Index(2, 2)`, `y_a` →
/// `Index(y, a)`).
pub fn strings_to_subscripts(e: &Expr) -> Expr {
    if let Expr::Sym(s) = e {
        let name = s.name();
        if let Some(pos) = name.find('_') {
            let (base, sfx) = (&name[..pos], &name[pos + 1..]);
            if !base.is_empty() && !sfx.is_empty() {
                return Expr::Index(Box::new(sym_or_number(base)), Box::new(sym_or_number(sfx)));
            }
        }
        return e.clone();
    }
    map_children(e, strings_to_subscripts)
}

/// A subscript half read back from its flat spelling: an integer if it spells
/// one, otherwise a symbol.
fn sym_or_number(s: &str) -> Expr {
    match s.parse::<i64>() {
        Ok(n) => Expr::int(n),
        Err(_) => Expr::sym(s),
    }
}

/// Convert 2-element tuples/arrays into interval notation, port of
/// `me.to_intervals`: `(1,2)` → the open interval, `[1,2]` → the closed one
/// (half-open forms already parse as intervals). Recurses everywhere; other
/// shapes are untouched.
pub fn to_intervals(e: &Expr) -> Expr {
    use crate::expr::SeqKind;
    if let Expr::Seq(kind, xs) = e {
        if xs.len() == 2 && matches!(kind, SeqKind::Tuple | SeqKind::Array) {
            let closed = matches!(kind, SeqKind::Array);
            return Expr::Interval {
                endpoints: Box::new((to_intervals(&xs[0]), to_intervals(&xs[1]))),
                closed: (closed, closed),
            };
        }
    }
    map_children(e, to_intervals)
}

/// `me.normalize_function_names`: fold alternate function spellings to their
/// canonical form (`arcsin` → `asin`, `ln` → `log`, …) via the function
/// registry's alias map. Only bare-symbol heads are rewritten.
///
/// `exp(x)` folds to `e^x`, the two being spellings of one thing. This has to
/// happen *here* rather than only in [`normalize_syntactic`], because callers
/// normalize names before simplifying and simplification takes the two
/// spellings in different directions: `e^(-t)` becomes `1/e^t` (a negative
/// exponent is a reciprocal) while `exp(-t)` stays applied. Fold first and
/// `-5e^{-t}`, `-5\exp(-t)`, `-5/e^t` and `-5/\exp(t)` all reach one tree,
/// which is what lets any of them match any of the others under
/// `symbolicEquality`.
///
/// JS folded the other way (`e^x` → `exp(x)`) and got no such agreement — its
/// simplifier left the reciprocal and the applied form apart, so only the
/// exactly-matching spelling scored. The direction is chosen for the engine
/// that has to live with it: this one canonicalizes powers, so powers are where
/// the spellings meet.
///
/// `sqrt(x)` deliberately does *not* fold to `x^(1/2)` here, unlike JS: the two
/// stay distinct canonical trees and `equals` reconciles them (see
/// `equality.rs`, `sqrt_and_half_power_are_equal`).
pub fn normalize_function_names(e: &Expr) -> Expr {
    fn rename_head(h: &Expr) -> Expr {
        match h {
            Expr::Sym(s) => match crate::special_functions::canonical_name(&s.name()) {
                Some(canon) => Expr::sym(canon),
                None => h.clone(),
            },
            Expr::Pow(b, x) => Expr::Pow(Box::new(rename_head(b)), x.clone()),
            Expr::Prime(x) => Expr::Prime(Box::new(rename_head(x))),
            other => other.clone(),
        }
    }
    if let Expr::Apply(head, args) = e {
        // `exp(x)` is a spelling of `e^x`, and folding the two together is what
        // lets a student's `-5e^{-t}` match an author's `-5\exp(-t)`.
        if args.len() == 1 {
            if let Expr::Sym(s) = head.as_ref() {
                if s.name() == "exp" {
                    return Expr::Pow(
                        Box::new(Expr::sym("e")),
                        Box::new(normalize_function_names(&args[0])),
                    );
                }
            }
        }
        return Expr::Apply(
            Box::new(rename_head(head)),
            args.iter().map(normalize_function_names).collect(),
        );
    }
    map_children(e, normalize_function_names)
}

/// `me.tuples_to_vectors`: reinterpret tuple sequences as vectors.
pub fn tuples_to_vectors(e: &Expr) -> Expr {
    use crate::expr::SeqKind;
    if let Expr::Seq(SeqKind::Tuple, xs) = e {
        return Expr::Seq(SeqKind::Vector, xs.iter().map(tuples_to_vectors).collect());
    }
    map_children(e, tuples_to_vectors)
}

/// `me.altvectors_to_vectors`: reinterpret `⟨…⟩` alt-vectors as vectors.
pub fn altvectors_to_vectors(e: &Expr) -> Expr {
    use crate::expr::SeqKind;
    if let Expr::Seq(SeqKind::AltVector, xs) = e {
        return Expr::Seq(
            SeqKind::Vector,
            xs.iter().map(altvectors_to_vectors).collect(),
        );
    }
    map_children(e, altvectors_to_vectors)
}
