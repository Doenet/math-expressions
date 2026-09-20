//! Which of `pi`, `e` and `i` denote mathematical constants here, and which are
//! ordinary variable names.
//!
//! The three names are genuinely ambiguous. `2 π r` and `e^{iπ}` want them to be
//! constants; `(e, f)` coordinates, a matrix with entries `e`…`h`, and the
//! variable run `g, h, i` want them to be names. No heuristic settles that from
//! the expression alone — an earlier attempt to guess from context is what this
//! module replaces — so it is *declared*, per document.
//!
//! This restores a knob the JS library had and the Rust port dropped:
//! `createInstance({define_e, define_pi, define_i})` in `lib/mathjs.js`, read by
//! its variable listing, polynomial reader, evaluator, equality sampler,
//! simplifier and differentiator.
//!
//! # What the declaration does *not* do
//!
//! It does not change any comparator. Sorting is alphabetical over symbol names
//! whatever the policy says, because:
//!
//! - [`crate::normalize::order`] is what makes structural equality work —
//!   canonical trees are compared with `==`, which is sound only because
//!   operands are in one agreed order. A policy-dependent canonical order means
//!   two expressions declared differently compare unequal when identical, and
//!   any *persisted* canonical tree (DoenetML stores normalized ASTs in document
//!   state) silently stops matching when the document's declaration changes.
//! - `normalize::default_order` reproduces the pinned `alpha94` sort key, and
//!   that key treats all three as plain symbols under every setting of
//!   `define_*` — its own `sort_key` carries the unactioned TODO
//!   *"if string is a constant, return number with value?"*. Matching the oracle
//!   means matching that.
//!
//! The one exception is opt-in and display-only: [`ConstantPolicy::sort_constants_first`]
//! affects `normalize::present`, which runs *after* canonicalization, is
//! idempotent, and produces trees no comparator ever sees.
//!
//! # Scoping
//!
//! The policy lives in a thread-local (WASM is single-threaded, and the crate
//! already uses thread-local symbol interning and [`crate::resource_limits`]),
//! so no signatures change: deep call sites read [`current`].
//!
//! ```
//! use math_expressions::constant_policy::{self, ConstantPolicy};
//! // A problem about points (e, f) and (g, h): `e` is a coordinate, not Euler's.
//! let coords = ConstantPolicy { define_e: false, ..ConstantPolicy::default() };
//! constant_policy::with(coords, || {
//!     // parse()/simplify()/equals() here treat `e` as an ordinary variable
//! });
//! ```

use crate::expr::{Expr, MathConst};
use std::cell::Cell;

/// Which named constants are declared, plus the one display option that depends
/// on the answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstantPolicy {
    /// `pi` denotes π rather than a variable named `pi`.
    pub define_pi: bool,
    /// `e` denotes Euler's number rather than a variable named `e`. Turning this
    /// off also stops `e^x` folding to `exp(x)`.
    pub define_e: bool,
    /// `i` denotes the imaginary unit rather than a variable named `i`. Turning
    /// this off also stops `i·i` folding to `−1`.
    pub define_i: bool,
    /// **Display only.** Sort declared constants ahead of ordinary variables in
    /// `normalize::present`'s sums and products, so a term reads `2 π r` and
    /// `2 π i` rather than the alphabetical `2 π r` / `2 i π`. Off by default:
    /// the JS oracle sorts alphabetically, and the compat suite pins that
    /// (`slow_simplify` expects `3a+3b+3c+2d+2e+2f+…`, with `e` in alphabetical
    /// position). Turning it on is a deliberate divergence for readability.
    pub sort_constants_first: bool,
    /// Whether `0^0`, `(±∞)^0` and `NaN^0` are indeterminate (fold to `NaN`) or
    /// — with this **off** — fold to `1` like any other `x^0`, matching the JS
    /// library's non-strict `pow`. Not a constant declaration; it shares this
    /// module's thread-local and FFI plumbing because it is the same kind of
    /// document-level interpretation knob (`me.math.pow_strict`). Default `true`
    /// (strict): a bare `0^0` has no limit, so asserting one is the safer
    /// default.
    pub pow_strict: bool,
}

impl Default for ConstantPolicy {
    fn default() -> Self {
        Self {
            define_pi: true,
            define_e: true,
            define_i: true,
            sort_constants_first: false,
            pow_strict: true,
        }
    }
}

impl ConstantPolicy {
    /// No name is a constant: `pi`, `e` and `i` are ordinary variables.
    pub const ALL_VARIABLES: Self = Self {
        define_pi: false,
        define_e: false,
        define_i: false,
        sort_constants_first: false,
        pow_strict: true,
    };

    /// Is `name` declared to be a mathematical constant here?
    pub fn declares(&self, name: &str) -> bool {
        match name {
            "pi" => self.define_pi,
            "e" => self.define_e,
            "i" => self.define_i,
            _ => false,
        }
    }

    /// Does `name` sort ahead of ordinary variables in the display pass? Only a
    /// *declared* constant can, and only under [`Self::sort_constants_first`].
    pub(crate) fn sorts_first(&self, name: &str) -> bool {
        self.sort_constants_first && self.declares(name)
    }
}

thread_local! {
    static POLICY: Cell<ConstantPolicy> = const {
        Cell::new(ConstantPolicy {
            define_pi: true,
            define_e: true,
            define_i: true,
            sort_constants_first: false,
            pow_strict: true,
        })
    };
}

/// The policy in force on this thread.
pub fn current() -> ConstantPolicy {
    POLICY.with(|c| c.get())
}

/// Run `f` under `policy`, restoring the previous one afterwards (including on
/// unwind).
pub fn with<R>(policy: ConstantPolicy, f: impl FnOnce() -> R) -> R {
    struct Restore(ConstantPolicy);
    impl Drop for Restore {
        fn drop(&mut self) {
            POLICY.with(|c| c.set(self.0));
        }
    }
    let _restore = Restore(POLICY.with(|c| c.replace(policy)));
    f()
}

/// Replace the policy for this thread without a scope. For embedders that set
/// it once per call at an FFI boundary; prefer [`with`] inside the crate.
pub fn set(policy: ConstantPolicy) {
    POLICY.with(|c| c.set(policy));
}

// ---------------------------------------------------------------------------
// Leaf predicates — the single home for the "either spelling" question
// ---------------------------------------------------------------------------
//
// π, e and i each have two spellings: the `MathConst` variant and the `Sym` the
// parsers produce. Three modules used to carry their own private `is_e`/`is_pi`
// pair; they now all come from here, which is what lets the policy apply
// uniformly instead of at whichever call sites remembered to ask.

/// The bare name of a leaf that spells one of the three named constants, in
/// **either** form and **regardless of policy** — `Const(Pi)` and `Sym("pi")`
/// both answer `"pi"`.
///
/// This is the comparators' question: both spellings must sort in the same
/// place whether or not the name is declared, or the sort stops being total in
/// any useful sense. Semantics wants [`is_pi`]/[`is_e`]/[`is_i`] instead.
pub(crate) fn constant_spelling(e: &Expr) -> Option<String> {
    match e {
        Expr::Const(c) => c.symbol_name().map(str::to_string),
        Expr::Sym(s) => {
            let n = s.name();
            crate::expr::sym::CONSTANT_SYMBOLS
                .contains(&n.as_str())
                .then_some(n)
        }
        _ => None,
    }
}

/// Does this leaf denote the named constant, under the policy in force?
///
/// The `MathConst` variant always does: an expression built with `Const(E)`
/// asked for Euler's number explicitly, and no naming declaration takes that
/// back. The *symbol* `e` does only while `define_e` holds — which is exactly
/// the distinction that lets `(e, f)` be a point in a document that still
/// writes `Const(E)` where it means the constant.
fn is_const(e: &Expr, variant: MathConst, name: &str) -> bool {
    match e {
        Expr::Const(c) => *c == variant,
        Expr::Sym(s) => current().declares(name) && s.name() == name,
        _ => false,
    }
}

/// Is `e` π, in either spelling, under the policy in force?
pub fn is_pi(e: &Expr) -> bool {
    is_const(e, MathConst::Pi, "pi")
}

/// Is `e` Euler's number, in either spelling, under the policy in force?
pub fn is_e(e: &Expr) -> bool {
    is_const(e, MathConst::E, "e")
}

/// Is `e` the imaginary unit, in either spelling, under the policy in force?
pub fn is_i(e: &Expr) -> bool {
    is_const(e, MathConst::I, "i")
}
