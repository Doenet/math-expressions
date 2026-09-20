//! Symbol interning.
//!
//! Symbols are u32 indices into a thread-local interner; comparison and
//! hashing are O(1). WASM is single-threaded, so thread_local is free.

use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sym(u32);

/// The symbol names that *can* denote mathematical constants rather than free
/// variables (the parsers emit `pi`/`e`/`i` as plain symbols, matching the JS
/// convention). Whether a given one *does* is a per-document declaration —
/// see [`crate::constant_policy`] — so this is the candidate set, not the
/// answer. Ask [`is_constant_symbol`] for the answer.
pub const CONSTANT_SYMBOLS: &[&str] = &["pi", "e", "i"];

/// Is `name` a mathematical constant under the policy in force? Single source of
/// truth — the evaluator's sampling filter, the ∞/NaN fold guard, and
/// `evaluate_to_constant`'s closedness check must all agree.
pub fn is_constant_symbol(name: &str) -> bool {
    crate::constant_policy::current().declares(name)
}

/// The named constants mathjs put in scope, which the JS library evaluated
/// through — so `1E-300` (uppercase `E`, not scientific notation unless the
/// parser is asked) came out as `1·e − 300`, a number, and `SQRT2` as 1.414.
///
/// Deliberately *not* [`CONSTANT_SYMBOLS`]: these are not constants of the
/// language. The parsers do not emit them, they sort and print as the ordinary
/// variables they are, and [`crate::ops::variables`] still lists them — which
/// matters, because the equality sampler binds every variable it lists, and a
/// binding takes precedence over this table wherever one exists. The table
/// applies only where a *closed* expression is being reduced to a number and
/// there is no binding to be had, which is exactly the reach mathjs's scope had.
pub fn mathjs_constant(name: &str) -> Option<f64> {
    Some(match name {
        "E" => std::f64::consts::E,
        "PI" => std::f64::consts::PI,
        "LN2" => std::f64::consts::LN_2,
        "LN10" => std::f64::consts::LN_10,
        "LOG2E" => std::f64::consts::LOG2_E,
        "LOG10E" => std::f64::consts::LOG10_E,
        "SQRT1_2" => std::f64::consts::FRAC_1_SQRT_2,
        "SQRT2" => std::f64::consts::SQRT_2,
        "Infinity" => f64::INFINITY,
        "NaN" => f64::NAN,
        _ => return None,
    })
}

/// The number of distinct symbol names interned so far — a memory gauge for the
/// long-lived worker (DoenetML issue #83, item 8). The interner is append-only:
/// a `Sym` is a raw index into it, so names are never evicted while any `Sym`
/// could still reference them. True eviction needs generational or ref-counted
/// symbols (a redesign); this exposes the growth so it can be measured first.
pub fn interner_len() -> usize {
    INTERNER.with(|i| i.borrow().names.len())
}

thread_local! {
    static INTERNER: RefCell<Interner> = RefCell::new(Interner::default());
}

#[derive(Default)]
struct Interner {
    by_name: HashMap<String, u32>,
    names: Vec<String>,
}

impl Sym {
    pub fn new(name: &str) -> Sym {
        INTERNER.with(|i| {
            let mut i = i.borrow_mut();
            if let Some(&id) = i.by_name.get(name) {
                return Sym(id);
            }
            let id = i.names.len() as u32;
            i.names.push(name.to_string());
            i.by_name.insert(name.to_string(), id);
            Sym(id)
        })
    }

    /// The symbol's name. Returns an owned String because the interner is
    /// thread-local; symbol-heavy code paths should compare `Sym`s directly.
    pub fn name(self) -> String {
        INTERNER.with(|i| i.borrow().names[self.0 as usize].clone())
    }
}

impl std::fmt::Display for Sym {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
