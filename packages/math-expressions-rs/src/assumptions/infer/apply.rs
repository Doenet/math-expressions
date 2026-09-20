//! Function domain facts: what `f(x)` inherits from `x`.
//!
//! A port of the `functions` table at the top of `element_of_sets.js`, which
//! is indexed *from* a property of the argument *to* a property of the result
//! — `functions.C.R = ["abs", "arg"]` reads "the modulus of a complex number
//! is real". [`domain_facts`] is that table transposed: one arm per result
//! property, listing the (argument property, function) pairs that establish
//! it. A few rules the JS table does not have are appended afterwards; each
//! says why it is kept.

use super::super::facts::Facts;
use super::super::Assumptions;
use super::facts;
use crate::expr::Expr;

pub(super) fn apply_facts(head: &Expr, args: &[Expr], a: &Assumptions) -> Facts {
    let (Expr::Sym(f), [arg]) = (head, args) else {
        return Facts::unknown();
    };
    let name = f.name();
    let af = facts(arg, a);
    let mut out = domain_facts(name.as_str(), &af);

    // Strengthenings beyond the JS table, kept because our engine reaches them
    // and the JS answer is merely `undefined` (never a contradicting `false`).
    match name.as_str() {
        // |z| ≠ 0 iff z ≠ 0 holds in any field; JS demands complex-ness first.
        "abs" if af.nonzero == Some(true) => out.nonzero = Some(true),
        // `tan` is absent from every JS list, but a real argument still gives
        // a real value wherever it is defined.
        "tan" if af.real == Some(true) => {
            out.real = Some(true);
            out.complex = Some(true);
        }
        // log of a nonzero is complex; JS asks for complex-ness of the
        // argument, which `x ≠ 0` alone does not give.
        "log" | "ln" if af.nonzero == Some(true) => out.complex = Some(true),
        _ => {}
    }
    out
}

/// The JS `functions` table, read result-property first. Every entry is
/// "argument is P ⇒ result is Q"; nothing here ever concludes a `false`.
fn domain_facts(f: &str, af: &Facts) -> Facts {
    let complex = af.complex == Some(true);
    let real = af.real == Some(true);
    // `nonzeroC`: nonzero *and* complex, the JS guard spelled out.
    let nonzero_complex = complex && af.nonzero == Some(true);
    let nonneg = af.nonneg == Some(true);
    let positive = af.positive == Some(true);

    let mut out = Facts::unknown();
    // functions.R.Z
    if real && f == "sign" {
        out.integer = Some(true);
    }
    // functions.C.R, functions.R.R, functions.nonneg.R, functions.pos.R
    if complex && matches!(f, "abs" | "arg")
        || real && REAL_TO_REAL.contains(&f)
        || nonneg && NONNEG_TO_REAL.contains(&f)
        || positive && (NONNEG_TO_REAL.contains(&f) || matches!(f, "log" | "ln" | "log10"))
    {
        out.real = Some(true);
    }
    // functions.C.C — the only source of complex-ness in the table.
    if complex && COMPLEX_TO_COMPLEX.contains(&f) {
        out.complex = Some(true);
    }
    // functions.C.nonzero, functions.nonzeroC.nonzero, functions.pos.nonzero
    if complex && f == "exp" || nonzero_complex && f == "abs" || positive && POS_TO_POS.contains(&f)
    {
        out.nonzero = Some(true);
    }
    // functions.C.nonneg, functions.R.nonneg, functions.nonneg.nonneg,
    // functions.pos.nonneg
    if complex && f == "abs"
        || real && matches!(f, "abs" | "exp" | "arg")
        || nonneg && NONNEG_TO_NONNEG.contains(&f)
        || positive && POS_TO_POS.contains(&f)
    {
        out.nonneg = Some(true);
    }
    // functions.R.pos, functions.nonzeroC.pos, functions.pos.pos
    if real && f == "exp" || nonzero_complex && f == "abs" || positive && POS_TO_POS.contains(&f) {
        out.positive = Some(true);
    }
    // Realness is what every sign predicate is gated on, so a definite sign
    // that arrived without it (`|z|` of a complex `z`) implies it.
    if out.nonneg == Some(true) || out.positive == Some(true) {
        out.real = Some(true);
    }
    if out.real == Some(true) {
        out.complex = Some(true);
    }
    out
}

/// `functions.R.R`.
const REAL_TO_REAL: [&str; 9] = [
    "abs", "arg", "exp", "sign", "cos", "cosh", "sin", "sinh", "erf",
];
/// `functions.nonneg.nonneg`.
const NONNEG_TO_NONNEG: [&str; 5] = ["abs", "exp", "arg", "sqrt", "erf"];
/// `functions.nonneg.R` — the two lists above, unioned as JS does.
const NONNEG_TO_REAL: [&str; 10] = [
    "abs", "arg", "exp", "sign", "cos", "cosh", "sin", "sinh", "erf", "sqrt",
];
/// `functions.pos.pos`, which JS also uses for `pos.nonneg` and `pos.nonzero`.
const POS_TO_POS: [&str; 4] = ["abs", "exp", "sqrt", "erf"];
/// `functions.C.C`.
const COMPLEX_TO_COMPLEX: [&str; 13] = [
    "abs", "arg", "exp", "sign", "cos", "cosh", "sin", "sinh", "erf", "sqrt", "log", "ln", "log10",
];
