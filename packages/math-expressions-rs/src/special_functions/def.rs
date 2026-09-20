//! The [`FnDef`] schema: everything the crate can know about one named math
//! function. A contributor reads this file to learn what facets exist; the
//! family files under this module fill them in, and [`super::registry`]
//! registers the results.

use crate::expr::Expr;
use num_complex::Complex64;
use num_rational::BigRational;

/// Complex evaluation of a variadic function (the aggregates).
pub type EvalN = fn(&[Complex64]) -> Option<Complex64>;

/// Exact rational folding of an application whose arguments are all exact.
pub type FoldExact = fn(&[BigRational]) -> Option<BigRational>;

/// Everything the crate knows about one named math function.
///
/// Definitions spell out only the facets that apply and default the rest
/// with `..DEFAULTS`.
pub struct FnDef {
    /// Canonical spelling — what normalization rewrites *to* (`asin`, `log`).
    pub name: &'static str,
    /// Alternate spellings folded to `name` by [`canonical_name`](super::canonical_name)
    /// (the JS `function_normalizations` table: `arcsin`, `ln`, `cosec`, …).
    pub aliases: &'static [&'static str],
    /// Spellings this function contributes to the TEXT parser's default
    /// `applied_function_symbols` list.
    pub parse_text: &'static [&'static str],
    /// Spellings contributed to the LATEX parser's default list. May differ
    /// from `parse_text` (e.g. `re` in text vs `Re` in LaTeX).
    pub parse_latex: &'static [&'static str],
    /// Canonical name of the notated inverse (`sin` → `asin`) for the
    /// `f^(-1)(x)` → `af(x)` rewrite. Matched on `name` only — the rewrite
    /// sites run before alias renaming, and the historical tables never
    /// listed aliases here.
    pub inverse: Option<&'static str>,
    /// Spellings for which `f^n(x)` (n ≠ −1) rewrites to `(f(x))^n`. Listed
    /// explicitly (not derived from `aliases`) because the rewrite runs
    /// *before* name normalization in `canon_apply`, and the historical
    /// MOVE_EXPONENT_OUTSIDE set covered `ln` but not `cosec`.
    pub move_exponent_spellings: &'static [&'static str],
    /// The mathjs derivative-table entry, as a text template in the
    /// placeholder `x` (`None`: `diff` falls back to prime notation).
    pub derivative: Option<&'static str>,
    /// One antiderivative in the argument `u`, as an expression builder
    /// using the `normalize` smart constructors — exactly the shapes the
    /// integrator's elementary table historically produced. The caller
    /// handles the linear-inner-argument division.
    pub antiderivative: Option<fn(Expr) -> Expr>,
    /// Complex evaluation with one argument. `None` result: undefined on
    /// that input (e.g. `floor` of a non-real value). Matched on the
    /// canonical spelling only — evaluation runs on canonicalized trees,
    /// and the historical `known_function` list never held aliases.
    pub eval1: Option<fn(Complex64) -> Option<Complex64>>,
    /// Complex evaluation with two arguments (`atan2`, `mod`, …).
    pub eval2: Option<fn(Complex64, Complex64) -> Option<Complex64>>,
    /// Complex evaluation at *any* arity — the aggregates (`sum`, `mean`,
    /// `max`, …), which take as many arguments as they are given. Tried
    /// before `eval1`/`eval2`, so a function defines this or those, never
    /// both. Matched on the canonical spelling, like the other two.
    pub evaln: Option<EvalN>,
    /// Exact folding for `simplify`: the value of an application whose
    /// arguments are all exact rationals, *when that value is itself an exact
    /// rational*.
    ///
    /// Returning `None` — from the function, or from the facet being absent —
    /// leaves the application unfolded, and that is the whole design: it is
    /// what keeps `sqrt(2)`, `log10(3)` and `asin(1)` symbolic instead of
    /// collapsing them to a float. Contrast the legacy library, which folded
    /// through floating point and then tried to *recover* a fraction from the
    /// result; that is why its `log(1000, 10)` is `2.9999999999999996`.
    /// Working in `BigRational` throughout means the question "is this exactly
    /// 3?" is decided, not estimated.
    ///
    /// Arity is the implementation's business: it receives the whole argument
    /// list and returns `None` for a shape it does not handle.
    pub fold_exact: Option<FoldExact>,
    /// LaTeX control-word rendering, per spelling: `("asin", "arcsin")`
    /// renders the symbol `asin` as `\arcsin`. Spellings not listed fall
    /// back to `\operatorname{…}`. Per-spelling (like
    /// `move_exponent_spellings`) because faithful trees carry unnormalized
    /// names — `ln` renders `\ln` while `cosec` never had a control word.
    pub latex_commands: &'static [(&'static str, &'static str)],
    /// Override for the head of a rendered LaTeX application, where it
    /// differs from the symbol form (`log10` → `\log_{10}`, `re` → `\Re`).
    pub latex_head: Option<&'static str>,
    /// Precise-evaluation kernel (`certified_digits/` tier-0 obligations +
    /// optional MpFix tier-2 kernel). The tape's `Op::Call` id space is the
    /// order of kernel-bearing defs in [`ALL`](super::ALL)
    /// (`eval_numeric::certified_digits::kernels::registry`).
    pub kernel: Option<&'static crate::eval_numeric::certified_digits::kernels::FnKernel>,
}

/// The all-defaults definition, for `..DEFAULTS` in family files.
pub const DEFAULTS: FnDef = FnDef {
    name: "",
    aliases: &[],
    parse_text: &[],
    parse_latex: &[],
    inverse: None,
    move_exponent_spellings: &[],
    derivative: None,
    antiderivative: None,
    eval1: None,
    eval2: None,
    evaln: None,
    fold_exact: None,
    latex_commands: &[],
    latex_head: None,
    kernel: None,
};
