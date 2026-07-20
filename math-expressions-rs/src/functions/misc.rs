//! The long tail: complex-part accessors, combinatorics, rounding, matrix
//! reducers, and other applied symbols with no family of their own. `re`/`im`
//! illustrate per-parser spellings: lowercase in text, capitalized in LaTeX.

use super::{real_only, FnDef, DEFAULTS};
use num_complex::Complex64;

pub const MOD: FnDef = FnDef {
    name: "mod",
    parse_text: &["mod"],
    eval2: Some(|a, b| Some(Complex64::new(a.re.rem_euclid(b.re), 0.0))),
    ..DEFAULTS
};

pub const ERF: FnDef = FnDef {
    name: "erf",
    parse_text: &["erf"],
    parse_latex: &["erf"],
    latex_commands: &[("erf", "erf")],
    ..DEFAULTS
};

pub const ARG: FnDef = FnDef {
    name: "arg",
    parse_text: &["arg"],
    parse_latex: &["arg"],
    eval1: Some(|z| Some(Complex64::new(z.arg(), 0.0))),
    latex_commands: &[("arg", "arg")],
    ..DEFAULTS
};

pub const CONJ: FnDef = FnDef {
    name: "conj",
    parse_text: &["conj"],
    parse_latex: &["conj"],
    eval1: Some(|z| Some(z.conj())),
    ..DEFAULTS
};

pub const RE: FnDef = FnDef {
    name: "re",
    parse_text: &["re"],
    parse_latex: &["Re"],
    eval1: Some(|z| Some(Complex64::new(z.re, 0.0))),
    latex_commands: &[("Re", "Re")],
    latex_head: Some("\\Re"),
    ..DEFAULTS
};

pub const IM: FnDef = FnDef {
    name: "im",
    parse_text: &["im"],
    parse_latex: &["Im"],
    eval1: Some(|z| Some(Complex64::new(z.im, 0.0))),
    latex_commands: &[("Im", "Im")],
    latex_head: Some("\\Im"),
    ..DEFAULTS
};

pub const DET: FnDef = FnDef {
    name: "det",
    parse_text: &["det"],
    parse_latex: &["det"],
    latex_commands: &[("det", "det")],
    ..DEFAULTS
};

pub const TRACE: FnDef = FnDef {
    name: "trace",
    parse_text: &["trace"],
    parse_latex: &["trace"],
    eval1: Some(Some),
    ..DEFAULTS
};

pub const NPR: FnDef = FnDef {
    name: "nPr",
    parse_text: &["nPr"],
    parse_latex: &["nPr"],
    eval2: Some(|n, r| combinatorial(n, r, true)),
    ..DEFAULTS
};

pub const NCR: FnDef = FnDef {
    name: "nCr",
    parse_text: &["nCr"],
    parse_latex: &["nCr"],
    eval2: Some(|n, r| combinatorial(n, r, false)),
    ..DEFAULTS
};

pub const FLOOR: FnDef = FnDef {
    name: "floor",
    parse_text: &["floor"],
    parse_latex: &["floor"],
    eval1: Some(|z| real_only(z, f64::floor)),
    ..DEFAULTS
};

pub const CEIL: FnDef = FnDef {
    name: "ceil",
    parse_text: &["ceil"],
    parse_latex: &["ceil"],
    eval1: Some(|z| real_only(z, f64::ceil)),
    ..DEFAULTS
};

pub const ROUND: FnDef = FnDef {
    name: "round",
    parse_text: &["round"],
    parse_latex: &["round"],
    eval1: Some(|z| real_only(z, f64::round)),
    ..DEFAULTS
};

pub const ROOTOF: FnDef = FnDef {
    name: "rootof",
    parse_text: &["rootof"],
    ..DEFAULTS
};

/// Not a parseable applied symbol — `n!` postfix notation produces it — but
/// a real function for evaluation: `n! = Γ(n+1)`, as a complex function so
/// identities like `(n+1)·n! = (n+1)!` hold at sampled points.
pub const FACTORIAL: FnDef = FnDef {
    name: "factorial",
    eval1: Some(|z| Some(gamma(z + 1.0))),
    ..DEFAULTS
};

/// Complex gamma function via the Lanczos approximation (g = 7, 9
/// coefficients), with the reflection formula for the left half-plane.
/// Accurate to ~1e-13, so the recurrence `Γ(z+1) = z·Γ(z)` holds well within
/// the equality tolerance — which is what lets `(n+1)·n! = (n+1)!` and
/// `n/n! = 1/(n-1)!` pass.
fn gamma(z: Complex64) -> Complex64 {
    const G: f64 = 7.0;
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    let pi = std::f64::consts::PI;
    if z.re < 0.5 {
        // Reflection: Γ(z)·Γ(1-z) = π / sin(πz).
        Complex64::new(pi, 0.0)
            / ((Complex64::new(pi, 0.0) * z).sin() * gamma(Complex64::new(1.0, 0.0) - z))
    } else {
        let z = z - 1.0;
        let mut x = Complex64::new(C[0], 0.0);
        for (i, &c) in C.iter().enumerate().skip(1) {
            x += c / (z + i as f64);
        }
        let t = z + (G + 0.5);
        let sqrt_2pi = (2.0 * pi).sqrt();
        Complex64::new(sqrt_2pi, 0.0) * t.powc(z + 0.5) * (-t).exp() * x
    }
}

/// `nCr`/`nPr` on non-negative integer arguments.
fn combinatorial(n: Complex64, r: Complex64, ordered: bool) -> Option<Complex64> {
    let is_int = |z: Complex64| z.im.abs() < 1e-9 && (z.re.round() - z.re).abs() < 1e-9;
    if !is_int(n) || !is_int(r) {
        return None;
    }
    let (n, r) = (n.re.round() as i64, r.re.round() as i64);
    // The r-length product loop must stay bounded on any input; past ~10^4
    // the f64 result is astronomically large/imprecise anyway.
    if n < 0 || r < 0 || r > n || r > 10_000 {
        return None;
    }
    // P(n,r) = n·(n-1)···(n-r+1); C(n,r) = P(n,r)/r!.
    let mut num = 1.0f64;
    for k in 0..r {
        num *= (n - k) as f64;
    }
    if ordered {
        return Some(Complex64::new(num, 0.0));
    }
    let mut den = 1.0f64;
    for k in 1..=r {
        den *= k as f64;
    }
    Some(Complex64::new(num / den, 0.0))
}
