//! `MpFix`: arbitrary-precision fixed point — `value ≈ mant · 2^scale`, with
//! the producer guaranteeing `|true − value| ≤ 2^scale` (one ulp at the
//! stored scale). This ±1-ulp bound is the *delivered* contract of a finished
//! kernel result: multi-level composites (e.g. the complex kernels) accumulate
//! a few ulps internally, then discharge them against the extra guard bits each
//! path carries (`bits ≥ need + 8`), so what a caller observes is still ±1 ulp
//! at the returned scale — it is not a per-elementary-operation guarantee.
//!
//! Contract: `rescale` coarsens with rounding; refining a *representation*
//! (left shift) is only meaningful for exactly-known values and is what
//! `refine_exact` is for.

use super::DecimalFormat;
use crate::num::{BigNumber, Number};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Signed, ToPrimitive, Zero};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MpFix {
    pub mant: BigInt,
    pub scale: i32,
}

/// `n · 2^sh`, rounding (half away from zero) when `sh < 0`.
pub(crate) fn shift_round(n: &BigInt, sh: i32) -> BigInt {
    use std::cmp::Ordering;
    match sh.cmp(&0) {
        Ordering::Equal => n.clone(),
        Ordering::Greater => n << sh as u32,
        Ordering::Less => {
            let k = (-sh) as u32;
            let half = BigInt::from(1) << (k - 1);
            if n.is_negative() {
                -((-n + half) >> k)
            } else {
                (n + half) >> k
            }
        }
    }
}

/// Rounded division `round(a/b)`, half away from zero.
pub(crate) fn div_round(a: &BigInt, b: &BigInt) -> BigInt {
    let neg = a.is_negative() != b.is_negative();
    let (aa, bb) = (a.abs(), b.abs());
    let q = (aa * 2i32 + &bb).div_floor(&(bb * 2i32));
    if neg {
        -q
    } else {
        q
    }
}

impl MpFix {
    pub fn zero(scale: i32) -> MpFix {
        MpFix {
            mant: BigInt::zero(),
            scale,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.mant.is_zero()
    }

    /// Does the ±1-ulp interval around this value exclude zero? The
    /// arbitrary-precision contract is "true value within one ulp of `mant`",
    /// so `|mant| > 1` proves the value is nonzero. This is THE certified-
    /// nonzero test — use it instead of re-encoding the magic comparison.
    pub fn excludes_zero(&self) -> bool {
        // |mant| > 1 ⟺ |mant| ≥ 2 ⟺ magnitude has more than one bit.
        self.mant.magnitude().bits() > 1
    }

    /// Bit position of the most significant bit: value ∈ [2^(msb−1), 2^msb).
    /// `None` for zero.
    pub fn msb(&self) -> Option<i64> {
        if self.mant.is_zero() {
            None
        } else {
            Some(self.mant.bits() as i64 + i64::from(self.scale))
        }
    }

    /// Coarsen to `new_scale ≥ self.scale`, rounding. Refining is a
    /// programming error (the stored precision cannot be invented).
    pub fn rescale(&self, new_scale: i32) -> MpFix {
        debug_assert!(new_scale >= self.scale, "rescale must coarsen");
        MpFix {
            mant: shift_round(&self.mant, self.scale - new_scale),
            scale: new_scale,
        }
    }

    /// Change representation to a finer scale (exact left shift). Only valid
    /// when `self` is exactly known (constants, exact conversions).
    pub fn refine_exact(&self, new_scale: i32) -> MpFix {
        debug_assert!(new_scale <= self.scale);
        MpFix {
            mant: &self.mant << (self.scale - new_scale) as u32,
            scale: new_scale,
        }
    }

    /// Either direction: coarsen with rounding or refine the representation.
    pub fn at_scale(&self, new_scale: i32) -> MpFix {
        if new_scale >= self.scale {
            self.rescale(new_scale)
        } else {
            self.refine_exact(new_scale)
        }
    }

    /// Exact `Number` rounded to `scale` (error ≤ ½ ulp). `None` for
    /// non-finite floats.
    pub fn from_number(n: &Number, scale: i32) -> Option<MpFix> {
        let (p, q): (BigInt, BigInt) = match n {
            Number::Int(i) => (BigInt::from(*i), BigInt::from(1)),
            Number::Rat(a, b) => (BigInt::from(*a), BigInt::from(*b)),
            Number::Big(b) => match &**b {
                BigNumber::Int(i) => (i.clone(), BigInt::from(1)),
                BigNumber::Rat(r) => (r.numer().clone(), r.denom().clone()),
            },
            Number::Float(f) => {
                let v = f.get();
                if !v.is_finite() {
                    return None;
                }
                let r = num_rational::BigRational::from_float(v)?;
                (r.numer().clone(), r.denom().clone())
            }
        };
        // round(p·2^(−scale) / q)
        let mant = if scale <= 0 {
            div_round(&(p << (-scale) as u32), &q)
        } else {
            div_round(&p, &(q << scale as u32))
        };
        Some(MpFix { mant, scale })
    }

    /// Exact conversion of a finite f64 (dyadic, so no rounding at a fine
    /// enough scale; rounded if `scale` is coarser than the value).
    pub fn from_f64(v: f64, scale: i32) -> Option<MpFix> {
        MpFix::from_number(&Number::Float(crate::num::F64::new(v)), scale)
    }

    pub fn to_f64(&self) -> f64 {
        // mant may exceed f64 range; go through a magnitude-safe split.
        let bits = self.mant.bits() as i64;
        if bits <= 900 {
            self.mant.to_f64().unwrap_or(f64::NAN) * 2f64.powi(self.scale)
        } else {
            let drop = (bits - 64) as i32;
            let top = shift_round(&self.mant, -drop);
            top.to_f64().unwrap_or(f64::NAN) * 2f64.powi(self.scale.saturating_add(drop))
        }
    }

    pub fn neg(&self) -> MpFix {
        MpFix {
            mant: -&self.mant,
            scale: self.scale,
        }
    }

    /// First `sig_digits` significant decimal digits in normalized scientific
    /// form, e.g. `"1.4142135623e0"`. Back-compat wrapper over
    /// [`Self::to_decimal_string_fmt`]. The last digit may be off by one ulp of
    /// the underlying binary value (standard for non-correctly-rounded
    /// output). Zero renders `"0"`.
    pub fn to_decimal_string(&self, sig_digits: usize) -> String {
        self.to_decimal_string_fmt(sig_digits, DecimalFormat::Scientific)
    }

    /// First `sig_digits` significant decimal digits, rendered per `format`
    /// (see [`DecimalFormat`]). The last digit may be off by one ulp. Zero
    /// renders `"0"`.
    pub fn to_decimal_string_fmt(&self, sig_digits: usize, format: DecimalFormat) -> String {
        if self.mant.is_zero() {
            return "0".to_string();
        }
        let neg = self.mant.is_negative();
        let guard = 3usize;
        let m10 = sig_digits + guard;
        let mut big = self.mant.abs() * BigInt::from(10u32).pow(m10 as u32);
        big = shift_round(&big, self.scale);
        if big.is_zero() {
            return "0".to_string();
        }
        let s = big.to_string();
        let dec_exp = s.len() as i64 - 1 - m10 as i64;
        // Round the digit string to sig_digits.
        let digits: Vec<u8> = s.bytes().map(|b| b - b'0').collect();
        let mut kept: Vec<u8> = digits[..sig_digits.min(digits.len())].to_vec();
        let round_up = digits.get(sig_digits).is_some_and(|&d| d >= 5);
        let mut dec_exp = dec_exp;
        if round_up {
            let mut i = kept.len();
            loop {
                if i == 0 {
                    kept.insert(0, 1);
                    kept.pop();
                    dec_exp += 1;
                    break;
                }
                i -= 1;
                if kept[i] == 9 {
                    kept[i] = 0;
                } else {
                    kept[i] += 1;
                    break;
                }
            }
        }
        match format {
            DecimalFormat::Scientific => fmt_scientific(neg, &kept, dec_exp),
            DecimalFormat::Plain => fmt_plain(neg, &kept, dec_exp),
        }
    }
}

/// Normalized scientific form: `<d0>.<rest>e<exp>`.
fn fmt_scientific(neg: bool, kept: &[u8], dec_exp: i64) -> String {
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    out.push((b'0' + kept[0]) as char);
    if kept.len() > 1 {
        out.push('.');
        for &d in &kept[1..] {
            out.push((b'0' + d) as char);
        }
    }
    out.push('e');
    out.push_str(&dec_exp.to_string());
    out
}

/// Plain decimal expansion. `kept` are the significant digits; the first has
/// place value `10^dec_exp`. Whole numbers get no decimal point; values below 1
/// get a leading `"0."` and the right number of leading zeros. No scientific
/// notation regardless of magnitude.
fn fmt_plain(neg: bool, kept: &[u8], dec_exp: i64) -> String {
    let n = kept.len() as i64;
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    let ch = |d: u8| (b'0' + d) as char;
    if dec_exp >= 0 {
        let int_len = dec_exp + 1; // digits left of the point
        if n <= int_len {
            // all significant digits are integer places; pad with zeros
            for &d in kept {
                out.push(ch(d));
            }
            for _ in 0..(int_len - n) {
                out.push('0');
            }
        } else {
            for &d in &kept[..int_len as usize] {
                out.push(ch(d));
            }
            out.push('.');
            for &d in &kept[int_len as usize..] {
                out.push(ch(d));
            }
        }
    } else {
        out.push_str("0.");
        for _ in 0..(-dec_exp - 1) {
            out.push('0');
        }
        for &d in kept {
            out.push(ch(d));
        }
    }
    out
}
