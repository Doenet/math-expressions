//! P4: complex fixed point — pairs of `MpFix` with kernels composed from the
//! real ones (principal branches matching `eval_complex`). Composition adds
//! a few ulps per level; every helper takes generous internal guard bits and
//! the Ziv-style mantissa check in `mod.rs` validates delivered magnitude.

use super::fix::MpFix;
use super::kernels::{self, Budget};
use num_bigint::BigInt;
use num_traits::{Signed, Zero};

#[derive(Clone, Debug)]
pub struct CFix {
    pub re: MpFix,
    pub im: MpFix,
}

impl CFix {
    pub fn real(re: MpFix) -> CFix {
        let s = re.scale;
        CFix {
            re,
            im: MpFix::zero(s),
        }
    }

    pub fn i(scale: i32) -> CFix {
        CFix {
            re: MpFix::zero(scale),
            im: MpFix {
                mant: BigInt::from(1) << (-scale).max(0) as u32,
                scale,
            },
        }
    }

    pub fn zero(scale: i32) -> CFix {
        CFix {
            re: MpFix::zero(scale),
            im: MpFix::zero(scale),
        }
    }

    /// Coarsen both components to a common scale (≥ both).
    pub fn rescale(&self, s: i32) -> CFix {
        CFix {
            re: self.re.at_scale(s.max(self.re.scale)),
            im: self.im.at_scale(s.max(self.im.scale)),
        }
    }

    pub fn neg(&self) -> CFix {
        CFix {
            re: self.re.neg(),
            im: self.im.neg(),
        }
    }

    /// i·z = (−im, re).
    pub fn times_i(&self) -> CFix {
        CFix {
            re: self.im.neg(),
            im: self.re.clone(),
        }
    }

    pub fn is_real_within_ulp(&self) -> bool {
        // |im| ≤ 4 ulp at its scale.
        self.im.mant.abs() <= BigInt::from(4)
    }
}

pub fn cadd(items: &[&CFix], s: i32) -> CFix {
    let child = items
        .iter()
        .flat_map(|c| [c.re.scale, c.im.scale])
        .min()
        .unwrap_or(s);
    let mut re = BigInt::zero();
    let mut im = BigInt::zero();
    for c in items {
        re += &c.re.at_scale(child).mant;
        im += &c.im.at_scale(child).mant;
    }
    CFix {
        re: MpFix {
            mant: re,
            scale: child,
        },
        im: MpFix {
            mant: im,
            scale: child,
        },
    }
    .rescale(s)
}

pub fn cmul(a: &CFix, b: &CFix, s: i32) -> Option<CFix> {
    let rr = kernels::mul_fix(&a.re, &b.re, s - 2)?;
    let ii = kernels::mul_fix(&a.im, &b.im, s - 2)?;
    let ri = kernels::mul_fix(&a.re, &b.im, s - 2)?;
    let ir = kernels::mul_fix(&a.im, &b.re, s - 2)?;
    let cs = rr.scale.min(ii.scale).min(ri.scale).min(ir.scale);
    Some(
        CFix {
            re: MpFix {
                mant: &rr.at_scale(cs.max(rr.scale)).refine_exact(cs).mant
                    - &ii.at_scale(cs.max(ii.scale)).refine_exact(cs).mant,
                scale: cs,
            },
            im: MpFix {
                mant: &ri.at_scale(cs.max(ri.scale)).refine_exact(cs).mant
                    + &ir.at_scale(cs.max(ir.scale)).refine_exact(cs).mant,
                scale: cs,
            },
        }
        .rescale(s),
    )
}

/// |z|² at scale `s`.
fn norm_sq(z: &CFix, s: i32) -> Option<MpFix> {
    let r2 = kernels::mul_fix(&z.re, &z.re, s - 2)?;
    let i2 = kernels::mul_fix(&z.im, &z.im, s - 2)?;
    let cs = r2.scale.min(i2.scale);
    Some(
        MpFix {
            mant: &r2.at_scale(cs.max(r2.scale)).refine_exact(cs).mant
                + &i2.at_scale(cs.max(i2.scale)).refine_exact(cs).mant,
            scale: cs,
        }
        .rescale(s.max(cs)),
    )
}

pub fn cinv(z: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    let g = s - 16;
    let n2 = norm_sq(z, g)?;
    if n2.mant.is_zero() {
        return None;
    }
    let _ = budget;
    Some(CFix {
        re: kernels::div_fix(&z.re, &n2, s)?,
        im: kernels::div_fix(&z.im.neg(), &n2, s)?,
    })
}

pub fn cdiv(a: &CFix, b: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    let g = s - 8;
    let binv = cinv(b, g, budget)?;
    cmul(a, &binv, s)
}

pub fn cpowint(z: &CFix, k: i64, s: i32, budget: &mut Budget) -> Option<CFix> {
    if k == 0 {
        return Some(CFix::real(MpFix::from_number(
            &crate::num::Number::Int(1),
            s.min(0),
        )?));
    }
    if k < 0 {
        if k == i64::MIN {
            return None;
        }
        let pos = cpowint(z, -k, s - 16, budget)?;
        return cinv(&pos, s, budget);
    }
    // Guard the result magnitude before squaring: |z^k| ≈ 2^(k·msb(z)) bits, so
    // an astronomically large power would materialize a multi-gigabyte mantissa
    // → allocation failure (a hard abort under `panic = "abort"`). Refuse with
    // `None` (→ `Unknown`), matching `powint_fix` and `exp_fix`.
    if let Some(m) = z.re.msb().into_iter().chain(z.im.msb()).max() {
        let cap = 8 * i64::from(crate::resource_limits::current().max_eval_precision_bits);
        if (i128::from(k) * i128::from(m)).abs() > i128::from(cap) {
            return None;
        }
    }
    let steps = 64 - k.leading_zeros() as i32;
    let work = s - 2 * steps - 4;
    let mut acc: Option<CFix> = None;
    let mut sq = z.rescale(work.min(z.re.scale.min(z.im.scale)));
    let mut kk = k as u64;
    loop {
        if !budget.tick() {
            return None;
        }
        if kk & 1 == 1 {
            acc = Some(match acc {
                None => sq.clone(),
                Some(a) => cmul(&a, &sq, work)?,
            });
        }
        kk >>= 1;
        if kk == 0 {
            break;
        }
        sq = cmul(&sq, &sq, work)?;
    }
    Some(acc.unwrap().rescale(s))
}

/// Principal square root: the component with the larger |m ± re| computes
/// directly, the other through `im/(2·larger)` (cancellation-safe).
pub fn csqrt(z: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    let g = s - 16;
    if z.im.mant.is_zero() {
        if !z.re.mant.is_negative() {
            return Some(CFix::real(kernels::sqrt_fix(&z.re, s, budget)?));
        }
        let root = kernels::sqrt_fix(&z.re.neg(), s, budget)?;
        return Some(CFix {
            re: MpFix::zero(s),
            im: root,
        });
    }
    let n2 = norm_sq(z, 2 * (g - 4))?;
    let m = kernels::sqrt_fix(&n2, g - 4, budget)?; // |z| at g−4
    let cs = m.scale.min(z.re.scale);
    let m_c = m.at_scale(cs);
    let re_c = z.re.at_scale(cs);
    let clamp = |mant: BigInt| if mant.is_negative() { BigInt::zero() } else { mant };
    let plus = MpFix {
        mant: clamp(&m_c.mant + &re_c.mant),
        scale: cs,
    };
    let minus = MpFix {
        mant: clamp(&m_c.mant - &re_c.mant),
        scale: cs,
    };
    // sqrt((m+re)/2), sqrt((m−re)/2): halving = scale − 1 (exact).
    let half = |x: &MpFix| MpFix {
        mant: x.mant.clone(),
        scale: x.scale - 1,
    };
    let (big, small_is_re) = if plus.mant >= minus.mant {
        (half(&plus), false) // big = re component²
    } else {
        (half(&minus), true) // big = im component²
    };
    let big_arg = big.at_scale(2 * (g - 4).min(big.scale)).at_scale(2 * (g - 4));
    let big_root = kernels::sqrt_fix(&big_arg, g - 4, budget)?;
    if big_root.mant.is_zero() {
        return Some(CFix::zero(s));
    }
    // other = |im| / (2·big_root)
    let im_abs = MpFix {
        mant: z.im.mant.abs(),
        scale: z.im.scale,
    };
    let two_big = MpFix {
        mant: big_root.mant.clone(),
        scale: big_root.scale + 1,
    };
    let other = kernels::div_fix(&im_abs, &two_big, g - 4)?;
    let (re_out, im_mag) = if small_is_re {
        (other, big_root)
    } else {
        (big_root, other)
    };
    let im_out = if z.im.mant.is_negative() {
        im_mag.neg()
    } else {
        im_mag
    };
    Some(
        CFix {
            re: re_out,
            im: im_out,
        }
        .rescale(s),
    )
}

/// e^z = e^re·(cos im + i sin im).
pub fn cexp(z: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    let g = s - 16;
    let mag = kernels::exp_fix(&z.re, g, budget)?;
    let c = kernels::cos_fix(&z.im, g, budget)?;
    let sn = kernels::sin_fix(&z.im, g, budget)?;
    Some(
        CFix {
            re: kernels::mul_fix(&mag, &c, s - 2)?,
            im: kernels::mul_fix(&mag, &sn, s - 2)?,
        }
        .rescale(s),
    )
}

/// Principal log: (½·ln|z|², atan2(im, re)).
pub fn cln(z: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    let g = s - 16;
    let n2 = norm_sq(z, g - 8)?;
    if n2.mant.is_zero() {
        return None;
    }
    let ln_n2 = kernels::ln_fix(&n2, g, budget)?;
    let re_out = MpFix {
        mant: ln_n2.mant,
        scale: ln_n2.scale - 1, // ÷2 exact
    };
    let im_out = atan2_fix(&z.im, &z.re, g, budget)?;
    Some(
        CFix {
            re: re_out,
            im: im_out,
        }
        .rescale(s),
    )
}

/// atan2(y, x) with the standard quadrant conventions.
pub fn atan2_fix(y: &MpFix, x: &MpFix, s: i32, budget: &mut Budget) -> Option<MpFix> {
    let g = s - 8;
    if x.mant.is_zero() {
        if y.mant.is_zero() {
            return None;
        }
        let hp = kernels::const_half_pi(s);
        return Some(if y.mant.is_negative() { hp.neg() } else { hp });
    }
    let ratio = kernels::div_fix(y, x, g - 4)?;
    let base = kernels::atan_fix(&ratio, g, budget)?;
    if !x.mant.is_negative() {
        return Some(base.rescale(s.max(base.scale)));
    }
    // x < 0: add ±π. The negative real axis (y == 0, x < 0) takes the
    // principal +π branch — matching `atan2(+0, −x) = +π` and the
    // `eval_complex` (num-complex `.ln()`) reference — so `ln(-a) = ln a + iπ`,
    // not `−iπ`. Only a strictly negative y takes the −π branch.
    let pi = kernels::const_pi(g);
    let adjusted = if y.mant.is_negative() {
        MpFix {
            mant: &base.at_scale(g).mant - &pi.mant,
            scale: g,
        }
    } else {
        MpFix {
            mant: &base.at_scale(g).mant + &pi.mant,
            scale: g,
        }
    };
    Some(adjusted.rescale(s))
}

/// a^b = exp(b·ln a) (principal).
pub fn cpow(a: &CFix, b: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    let g = s - 16;
    let ln_a = cln(a, g - 8, budget)?;
    let prod = cmul(b, &ln_a, g)?;
    cexp(&prod, s, budget)
}

/// sin(a+bi) = sin a cosh b + i cos a sinh b.
pub fn csin(z: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    let g = s - 16;
    let (sa, ca) = (
        kernels::sin_fix(&z.re, g, budget)?,
        kernels::cos_fix(&z.re, g, budget)?,
    );
    let (shb, chb) = (
        kernels::sinh_fix(&z.im, g, budget)?,
        kernels::cosh_fix(&z.im, g, budget)?,
    );
    Some(
        CFix {
            re: kernels::mul_fix(&sa, &chb, s - 2)?,
            im: kernels::mul_fix(&ca, &shb, s - 2)?,
        }
        .rescale(s),
    )
}

pub fn ccos(z: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    let g = s - 16;
    let (sa, ca) = (
        kernels::sin_fix(&z.re, g, budget)?,
        kernels::cos_fix(&z.re, g, budget)?,
    );
    let (shb, chb) = (
        kernels::sinh_fix(&z.im, g, budget)?,
        kernels::cosh_fix(&z.im, g, budget)?,
    );
    Some(
        CFix {
            re: kernels::mul_fix(&ca, &chb, s - 2)?,
            im: kernels::mul_fix(&sa, &shb, s - 2)?.neg(),
        }
        .rescale(s),
    )
}

pub fn ctan(z: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    let g = s - 24;
    let sn = csin(z, g, budget)?;
    let cs = ccos(z, g, budget)?;
    cdiv(&sn, &cs, s, budget)
}

/// sinh(a+bi) = sinh a cos b + i cosh a sin b.
pub fn csinh(z: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    // sinh z = −i·sin(iz)
    let w = csin(&z.times_i(), s, budget)?;
    Some(w.times_i().neg())
}

pub fn ccosh(z: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    ccos(&z.times_i(), s, budget)
}

pub fn ctanh(z: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    let w = ctan(&z.times_i(), s, budget)?;
    Some(w.times_i().neg())
}

/// asin z = −i·ln(iz + √(1−z²)).
pub fn casin(z: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    let g = s - 24;
    let z2 = cmul(z, z, g - 8)?;
    let one = CFix::real(MpFix {
        mant: BigInt::from(1) << (-(g - 8)).max(0) as u32,
        scale: g - 8,
    });
    let one_minus = cadd(&[&one, &z2.neg()], g - 8);
    let root = csqrt(&one_minus, g, budget)?;
    let iz = z.times_i();
    let inner = cadd(&[&iz, &root], g);
    let ln = cln(&inner, g, budget)?;
    Some(ln.times_i().neg().rescale(s))
}

pub fn cacos(z: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    let g = s - 8;
    let asin = casin(z, g, budget)?;
    let hp = kernels::const_half_pi(g);
    let re = MpFix {
        mant: &hp.mant - &asin.re.at_scale(g).mant,
        scale: g,
    };
    Some(
        CFix {
            re,
            im: asin.im.neg(),
        }
        .rescale(s),
    )
}

/// atan z = (i/2)·(ln(1−iz) − ln(1+iz)).
pub fn catan(z: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    let g = s - 24;
    let iz = z.times_i();
    let one = CFix::real(MpFix {
        mant: BigInt::from(1) << (-g).max(0) as u32,
        scale: g,
    });
    let a = cadd(&[&one, &iz.neg()], g - 4);
    let b = cadd(&[&one, &iz], g - 4);
    let la = cln(&a, g, budget)?;
    let lb = cln(&b, g, budget)?;
    let diff = cadd(&[&la, &lb.neg()], g);
    let half_i = diff.times_i();
    Some(
        CFix {
            re: MpFix {
                mant: half_i.re.mant,
                scale: half_i.re.scale - 1,
            },
            im: MpFix {
                mant: half_i.im.mant,
                scale: half_i.im.scale - 1,
            },
        }
        .rescale(s),
    )
}

pub fn cabs(z: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    if z.im.mant.is_zero() {
        return Some(CFix::real(MpFix {
            mant: z.re.mant.abs(),
            scale: z.re.scale,
        }));
    }
    let n2 = norm_sq(z, 2 * s)?;
    Some(CFix::real(kernels::sqrt_fix(&n2, s, budget)?))
}

pub fn clog10(z: &CFix, s: i32, budget: &mut Budget) -> Option<CFix> {
    let g = s - 16;
    let ln = cln(z, g, budget)?;
    let ln10 = kernels::const_ln10(g - 8);
    Some(
        CFix {
            re: kernels::div_fix(&ln.re, &ln10, s)?,
            im: kernels::div_fix(&ln.im, &ln10, s)?,
        },
    )
}
