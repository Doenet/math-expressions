//! [`Facts`] — what is known about one (sub)expression, as eight three-valued
//! predicates, plus the constant-leaf fact tables.

use super::MaybeBool;
use crate::num::Number;
use num_complex::Complex64;

/// What is known about one (sub)expression. Every field is three-valued.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Facts {
    pub(super) integer: MaybeBool,
    pub(super) real: MaybeBool,
    pub(super) complex: MaybeBool,
    pub(super) nonzero: MaybeBool,
    pub(super) nonneg: MaybeBool,
    pub(super) positive: MaybeBool,
    pub(super) negative: MaybeBool,
    pub(super) nonpos: MaybeBool,
}

impl Facts {
    /// Everything unknown.
    pub(super) fn unknown() -> Facts {
        Facts::default()
    }

    /// Facts of an exact number.
    pub(super) fn of_number(n: &Number) -> Facts {
        let v = n.to_f64();
        if v.is_nan() {
            return Facts::unknown();
        }
        let is_int = match n {
            Number::Int(_) => true,
            Number::NegZero => true,
            Number::Rat(..) => false,
            Number::Big(_) => n.magnitude_log10().is_some() && is_big_int(n),
            Number::Float(_) => v.fract() == 0.0,
        };
        Facts {
            integer: Some(is_int),
            real: Some(true),
            complex: Some(true),
            nonzero: Some(v != 0.0),
            nonneg: Some(v >= 0.0),
            positive: Some(v > 0.0),
            negative: Some(v < 0.0),
            nonpos: Some(v <= 0.0),
        }
    }

    /// A real, positive, non-integer constant (`pi`, `e`).
    pub(super) fn positive_transcendental() -> Facts {
        Facts {
            integer: Some(false),
            real: Some(true),
            complex: Some(true),
            nonzero: Some(true),
            nonneg: Some(true),
            positive: Some(true),
            negative: Some(false),
            nonpos: Some(false),
        }
    }

    /// The imaginary unit: complex, not real; sign predicates are all false
    /// (JS reports F, not undefined, for `i`).
    pub(super) fn imaginary_unit() -> Facts {
        Facts {
            integer: Some(false),
            real: Some(false),
            complex: Some(true),
            nonzero: Some(true),
            nonneg: Some(false),
            positive: Some(false),
            negative: Some(false),
            nonpos: Some(false),
        }
    }

    /// Every predicate false — JS's "other operators don't return numbers"
    /// verdict, reached by tuples, sets, intervals and the rest of the
    /// non-numeric heads.
    pub(super) fn non_numeric() -> Facts {
        Facts {
            integer: Some(false),
            real: Some(false),
            complex: Some(false),
            nonzero: Some(false),
            nonneg: Some(false),
            positive: Some(false),
            negative: Some(false),
            nonpos: Some(false),
        }
    }

    /// A relation whose sides are constant, so it decides to a boolean
    /// (`5 = 3`). JS folds it to `false`, which is neither a number nor a
    /// mathjs complex, so every predicate reads false — except `is_nonzero`,
    /// whose test is `c.re !== undefined && …` and therefore gives up.
    pub(super) fn boolean() -> Facts {
        Facts {
            nonzero: None,
            ..Facts::non_numeric()
        }
    }

    /// `±∞`: JS deliberately calls infinity nonzero, but `Number.isFinite`
    /// fails, so it is neither real nor complex and — every sign predicate
    /// short-circuiting on `is_real` — unsigned in all four directions.
    pub(super) fn infinite() -> Facts {
        Facts {
            nonzero: Some(true),
            ..Facts::non_numeric()
        }
    }

    /// `NaN`: as [`Facts::infinite`], except that JS answers `undefined` for
    /// `is_nonzero(NaN)` rather than committing.
    pub(super) fn nan() -> Facts {
        Facts {
            nonzero: None,
            ..Facts::non_numeric()
        }
    }

    /// Facts of a subexpression that evaluated numerically to `z` — the port
    /// of JS's `evaluate_to_constant` short-circuit, which every predicate
    /// consults before looking at structure. The infinity test comes first
    /// because a `0`-times-`∞` product leaves `NaN` in the *other* component
    /// (`-2.2/(5-5)` evaluates to `-∞ + NaN·i`), and JS — which simplifies the
    /// tree to a bare `-Infinity` before asking — reads that as an infinity.
    pub(super) fn of_constant(z: Complex64) -> Facts {
        if z.re.is_infinite() || z.im.is_infinite() {
            return Facts::infinite();
        }
        if z.re.is_nan() || z.im.is_nan() {
            return Facts::nan();
        }
        if z.im != 0.0 {
            // A genuine complex value: not a JS `number`, so `is_real` is
            // false and every sign predicate follows it.
            return Facts {
                complex: Some(true),
                nonzero: Some(true),
                ..Facts::non_numeric()
            };
        }
        Facts::of_f64(z.re)
    }

    /// Facts of a finite real value.
    pub(super) fn of_f64(v: f64) -> Facts {
        Facts {
            integer: Some(v.fract() == 0.0),
            real: Some(true),
            complex: Some(true),
            nonzero: Some(v != 0.0),
            nonneg: Some(v >= 0.0),
            positive: Some(v > 0.0),
            negative: Some(v < 0.0),
            nonpos: Some(v <= 0.0),
        }
    }

    /// Both facts hold at once — the meet for an `and` of assumptions. Keep
    /// every definite value; where the two branches give *conflicting* definite
    /// answers the premises are contradictory, and the sound three-valued
    /// answer under an inconsistent premise is to decline (unknown) rather than
    /// pick a side. The legacy engine returns the first branch's answer
    /// (`left || right`) and so reports `is_real(x)` true for
    /// `x ∈ R and x ∉ R`; we report unknown.
    ///
    /// That is one of **two** places the answers differ on
    /// `slow_assumptions.spec.ts` → `logical combinations` (this one is its
    /// spec:7357 assertion). The other is `combine::mul`, which never carries a
    /// `real: Some(false)` operand through a product, so `is_positive(x·y)`
    /// with `x ∉ R` comes back unknown where legacy says false — see
    /// `active-plans/COMPAT_TEST_FAILURE_SUMMARY.md`. Both leave this engine
    /// *incomplete* relative to legacy, never unsound.
    pub(super) fn and_meet(&self, other: &Facts) -> Facts {
        fn meet(a: MaybeBool, b: MaybeBool) -> MaybeBool {
            match (a, b) {
                (Some(x), Some(y)) => (x == y).then_some(x),
                (Some(x), None) | (None, Some(x)) => Some(x),
                (None, None) => None,
            }
        }
        Facts {
            integer: meet(self.integer, other.integer),
            real: meet(self.real, other.real),
            complex: meet(self.complex, other.complex),
            nonzero: meet(self.nonzero, other.nonzero),
            nonneg: meet(self.nonneg, other.nonneg),
            positive: meet(self.positive, other.positive),
            negative: meet(self.negative, other.negative),
            nonpos: meet(self.nonpos, other.nonpos),
        }
    }

    /// Either fact holds — the join for an `or` of assumptions. A value is
    /// definite only when both branches agree on it: `is_p` is known for
    /// `A ∨ B` exactly when every model of A and every model of B share it
    /// (`x < 0 or x > 0` ⇒ nonzero, because both disjuncts are). Branches are
    /// expected already [`normalize`](Self::normalize)d, so the join compares
    /// the answers a caller would see, as the legacy engine does.
    pub(super) fn or_join(&self, other: &Facts) -> Facts {
        fn join(a: MaybeBool, b: MaybeBool) -> MaybeBool {
            match (a, b) {
                (Some(x), Some(y)) if x == y => Some(x),
                _ => None,
            }
        }
        Facts {
            integer: join(self.integer, other.integer),
            real: join(self.real, other.real),
            complex: join(self.complex, other.complex),
            nonzero: join(self.nonzero, other.nonzero),
            nonneg: join(self.nonneg, other.nonneg),
            positive: join(self.positive, other.positive),
            negative: join(self.negative, other.negative),
            nonpos: join(self.nonpos, other.nonpos),
        }
    }

    /// Restore the invariants JS gets for free from the way it *defines* the
    /// sign predicates: `is_negative` is `is_real && !is_nonnegative` and
    /// `is_nonpositive` is `is_real && !is_positive`, and all four short-
    /// circuit on `is_real` (`if (!is_real) return is_real`). So a non-real
    /// value has every sign predicate false, a value of unknown realness has
    /// all four unknown, and otherwise only two of the four are independent —
    /// so a rule can derive `nonneg` alone and get `negative` for free.
    pub(super) fn normalize(&mut self) {
        match self.real {
            Some(true) => {
                self.negative = self.nonneg.map(|b| !b);
                self.nonpos = self.positive.map(|b| !b);
            }
            Some(false) => {
                // Not real ⇒ not an integer either (JS `x notin R` ⇒
                // `is_integer` false).
                self.integer = Some(false);
                self.nonneg = Some(false);
                self.positive = Some(false);
                self.negative = Some(false);
                self.nonpos = Some(false);
            }
            None => {
                self.nonneg = None;
                self.positive = None;
                self.negative = None;
                self.nonpos = None;
            }
        }
    }
}

fn is_big_int(n: &Number) -> bool {
    matches!(n, Number::Big(b) if matches!(&**b, crate::num::BigNumber::Int(_)))
}
