//! f64 numeric utilities replacing the parts of `me.math` (the re-exported
//! mathjs instance) that Doenet consumes — see the Doenet-usage survey in
//! tmp/PORTING_PLAN.md §17 notes. Everything here is plain double-precision
//! numerics, deliberately: the exact/symbolic counterparts (matrix algebra,
//! `RootOf` eigenvalues, certified precision) live in their own plans; this
//! module is the drop-in for what Doenet uses *numerically* today
//! (`mod`/`gcd`/`lcm`, statistics, `lusolve`, `eigs`). ODE solving (`dopri`)
//! is intentionally absent — see tmp/ODE_PLAN.md.
//!
//! All loops are bounded (fixed iteration caps, no wall-clock); failures are
//! `None`/NaN, never panics — this module is wasm-boundary-facing.

use num_complex::Complex64;

// ---- scalar utilities ----

/// mathjs `mod`: `x − y·floor(x/y)`, result has the sign of `y`; `y = 0`
/// returns `x` (mathjs convention).
pub fn math_mod(x: f64, y: f64) -> f64 {
    if y == 0.0 {
        return x;
    }
    x - y * (x / y).floor()
}

/// mathjs `gcd` for numbers: defined on integers only (NaN otherwise —
/// where mathjs throws, the wasm boundary reports NaN). `gcd(0,0) = 0`.
pub fn gcd_f64(x: f64, y: f64) -> f64 {
    if x.fract() != 0.0 || y.fract() != 0.0 || !x.is_finite() || !y.is_finite() {
        return f64::NAN;
    }
    let (mut a, mut b) = (x.abs(), y.abs());
    // Euclid on exactly-representable integers; f64 keeps exactness ≤ 2^53.
    while b > 0.0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

/// mathjs `lcm` on integers (NaN otherwise); `lcm(0, _) = 0`.
pub fn lcm_f64(x: f64, y: f64) -> f64 {
    let g = gcd_f64(x, y);
    if g.is_nan() {
        return f64::NAN;
    }
    if g == 0.0 {
        return 0.0;
    }
    (x / g * y).abs()
}

// ---- statistics (mathjs defaults) ----

pub fn mean(data: &[f64]) -> f64 {
    if data.is_empty() {
        return f64::NAN;
    }
    data.iter().sum::<f64>() / data.len() as f64
}

pub fn median(data: &[f64]) -> f64 {
    quantile_seq(data, 0.5)
}

/// mathjs `variance` default: unbiased (divide by n − 1).
pub fn variance(data: &[f64]) -> f64 {
    if data.len() < 2 {
        return f64::NAN;
    }
    let m = mean(data);
    data.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (data.len() - 1) as f64
}

pub fn std_dev(data: &[f64]) -> f64 {
    variance(data).sqrt()
}

/// mathjs `quantileSeq` with default linear interpolation:
/// `h = (n−1)p`, result `= a⌊h⌋ + (h − ⌊h⌋)(a⌊h⌋₊₁ − a⌊h⌋)` on sorted data.
pub fn quantile_seq(data: &[f64], prob: f64) -> f64 {
    if data.is_empty() || !(0.0..=1.0).contains(&prob) {
        return f64::NAN;
    }
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let h = (sorted.len() - 1) as f64 * prob;
    let lo = h.floor() as usize;
    let frac = h - h.floor();
    if lo + 1 >= sorted.len() || frac == 0.0 {
        return sorted[lo.min(sorted.len() - 1)];
    }
    sorted[lo] + frac * (sorted[lo + 1] - sorted[lo])
}

// ---- linear solve ----

/// Solve `A·x = b` (A row-major n×n) by Gaussian elimination with partial
/// pivoting — the mathjs `lusolve` replacement. `None` if the dimensions are
/// inconsistent or the matrix is numerically singular.
pub fn lusolve(a: &[f64], b: &[f64], n: usize) -> Option<Vec<f64>> {
    if a.len() != n * n || b.len() != n || n == 0 {
        return None;
    }
    let mut m: Vec<f64> = a.to_vec();
    let mut x: Vec<f64> = b.to_vec();
    for col in 0..n {
        // Partial pivot.
        let pivot_row = (col..n).max_by(|&i, &j| {
            m[i * n + col]
                .abs()
                .partial_cmp(&m[j * n + col].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        if m[pivot_row * n + col] == 0.0 {
            return None; // singular
        }
        if pivot_row != col {
            for k in 0..n {
                m.swap(col * n + k, pivot_row * n + k);
            }
            x.swap(col, pivot_row);
        }
        for row in col + 1..n {
            let f = m[row * n + col] / m[col * n + col];
            if f == 0.0 {
                continue;
            }
            for k in col..n {
                m[row * n + k] -= f * m[col * n + k];
            }
            x[row] -= f * x[col];
        }
    }
    // Back substitution.
    for col in (0..n).rev() {
        let mut s = x[col];
        for k in col + 1..n {
            s -= m[col * n + k] * x[k];
        }
        x[col] = s / m[col * n + col];
        if !x[col].is_finite() {
            return None;
        }
    }
    Some(x)
}

// ---- eigendecomposition ----

/// One eigenpair of a real matrix: possibly-complex value and vector.
pub struct EigenPair {
    pub value: Complex64,
    pub vector: Vec<Complex64>,
}

/// Numeric eigendecomposition of a real n×n matrix (row-major) — the mathjs
/// `eigs` replacement for Doenet's EigenDecomposition component.
///
/// Algorithm: Householder reduction to upper Hessenberg, then the shifted QR
/// algorithm in complex arithmetic (Wilkinson shift, Givens rotations,
/// deflation on negligible subdiagonals), then one eigenvector per value via
/// complex Gaussian elimination on `A − λI` (null-vector extraction). Complex
/// eigenvalues of real matrices appear in conjugate pairs; imaginary parts
/// below `1e-12·‖A‖` snap to real. Values sort by (re, im) for determinism.
///
/// `None` when QR fails to converge within the fixed iteration cap (rare;
/// mathjs `eigs` throws in the same situations).
pub fn eigs(a: &[f64], n: usize) -> Option<Vec<EigenPair>> {
    if a.len() != n * n || n == 0 {
        return None;
    }
    if !a.iter().all(|v| v.is_finite()) {
        return None;
    }
    let norm = a.iter().map(|v| v.abs()).fold(0.0f64, f64::max).max(1.0);

    // 1. Hessenberg reduction (Householder), real arithmetic.
    let mut h: Vec<f64> = a.to_vec();
    for k in 0..n.saturating_sub(2) {
        // Householder vector for column k, rows k+1..n.
        let mut alpha = 0.0f64;
        for i in k + 1..n {
            alpha += h[i * n + k] * h[i * n + k];
        }
        alpha = alpha.sqrt();
        if alpha == 0.0 {
            continue;
        }
        if h[(k + 1) * n + k] > 0.0 {
            alpha = -alpha;
        }
        let mut v = vec![0.0f64; n];
        v[k + 1] = h[(k + 1) * n + k] - alpha;
        for i in k + 2..n {
            v[i] = h[i * n + k];
        }
        let vnorm2: f64 = v.iter().map(|x| x * x).sum();
        if vnorm2 == 0.0 {
            continue;
        }
        // H ← (I − 2vvᵀ/‖v‖²) H (I − 2vvᵀ/‖v‖²)
        for j in 0..n {
            let dot: f64 = (k + 1..n).map(|i| v[i] * h[i * n + j]).sum();
            let f = 2.0 * dot / vnorm2;
            for i in k + 1..n {
                h[i * n + j] -= f * v[i];
            }
        }
        for i in 0..n {
            let dot: f64 = (k + 1..n).map(|j| v[j] * h[i * n + j]).sum();
            let f = 2.0 * dot / vnorm2;
            for j in k + 1..n {
                h[i * n + j] -= f * v[j];
            }
        }
    }

    // 2. Shifted QR on the Hessenberg matrix, complex arithmetic.
    let mut hc: Vec<Complex64> = h.iter().map(|&x| Complex64::new(x, 0.0)).collect();
    let mut values: Vec<Complex64> = Vec::with_capacity(n);
    let mut hi = n; // active block is hc[0..hi][0..hi]
    let eps = f64::EPSILON;
    let max_iters = 60 * n.max(4); // fixed budget: QR converges in ~2-3 per value
    let mut iters = 0;
    while hi > 0 {
        if hi == 1 {
            values.push(hc[0]);
            hi = 0;
            continue;
        }
        // Deflate: find negligible subdiagonal from the bottom.
        let mut lo = hi - 1;
        while lo > 0 {
            let s = hc[(lo - 1) * n + (lo - 1)].norm() + hc[lo * n + lo].norm();
            if hc[lo * n + (lo - 1)].norm() <= eps * s.max(norm * eps) {
                hc[lo * n + (lo - 1)] = Complex64::new(0.0, 0.0);
                break;
            }
            lo -= 1;
        }
        if lo == hi - 1 {
            // 1×1 block converged.
            values.push(hc[(hi - 1) * n + (hi - 1)]);
            hi -= 1;
            continue;
        }
        iters += 1;
        if iters > max_iters {
            return None;
        }
        // Wilkinson shift from the trailing 2×2 of the active block.
        let (a11, a12, a21, a22) = (
            hc[(hi - 2) * n + (hi - 2)],
            hc[(hi - 2) * n + (hi - 1)],
            hc[(hi - 1) * n + (hi - 2)],
            hc[(hi - 1) * n + (hi - 1)],
        );
        let tr = a11 + a22;
        let det = a11 * a22 - a12 * a21;
        let disc = (tr * tr - 4.0 * det).sqrt();
        let (r1, r2) = ((tr + disc) / 2.0, (tr - disc) / 2.0);
        let mu = if (r1 - a22).norm() < (r2 - a22).norm() {
            r1
        } else {
            r2
        };
        // Exceptional shift every 10 iterations to break cycles.
        let mu = if iters % 10 == 0 {
            mu + Complex64::new(hc[(hi - 1) * n + (hi - 2)].norm(), 0.0)
        } else {
            mu
        };

        // One QR step on rows/cols lo..hi via Givens rotations.
        let mut rotations: Vec<(usize, Complex64, Complex64)> = Vec::with_capacity(hi - lo);
        for i in lo..hi {
            hc[i * n + i] -= mu;
        }
        for i in lo..hi - 1 {
            let x = hc[i * n + i];
            let y = hc[(i + 1) * n + i];
            let r = (x.norm_sqr() + y.norm_sqr()).sqrt();
            if r == 0.0 {
                rotations.push((i, Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)));
                continue;
            }
            let c = x / r;
            let s = y / r;
            // Apply Gᵢᴴ on the left to rows i, i+1.
            for j in i..n {
                let hij = hc[i * n + j];
                let hi1j = hc[(i + 1) * n + j];
                hc[i * n + j] = c.conj() * hij + s.conj() * hi1j;
                hc[(i + 1) * n + j] = -s * hij + c * hi1j;
            }
            rotations.push((i, c, s));
        }
        // RQ: apply each Gᵢ on the right to columns i, i+1.
        for &(i, c, s) in &rotations {
            for r_ in 0..(i + 2).min(hi) {
                let hri = hc[r_ * n + i];
                let hri1 = hc[r_ * n + (i + 1)];
                hc[r_ * n + i] = hri * c + hri1 * s;
                hc[r_ * n + (i + 1)] = hri * (-s.conj()) + hri1 * c.conj();
            }
        }
        for i in lo..hi {
            hc[i * n + i] += mu;
        }
    }

    // 3. Cleanup: snap tiny imaginary parts, sort deterministically.
    let snap = 1e-12 * norm;
    for v in &mut values {
        if v.im.abs() <= snap {
            v.im = 0.0;
        }
        if v.re.abs() <= snap * 1e-3 {
            v.re = 0.0;
        }
    }
    values.sort_by(|p, q| {
        p.re.partial_cmp(&q.re)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(p.im.partial_cmp(&q.im).unwrap_or(std::cmp::Ordering::Equal))
    });

    // 4. Eigenvectors: null vector of A − λI by complex elimination.
    let pairs = values
        .into_iter()
        .map(|value| EigenPair {
            vector: null_vector(a, n, value),
            value,
        })
        .collect();
    Some(pairs)
}

/// A unit null vector of `A − λI` via complex Gaussian elimination with
/// partial pivoting: eliminate, take the column with the smallest pivot as
/// the free variable (= 1), back-substitute. Accurate for simple eigenvalues;
/// for (numerically) repeated ones it returns *a* vector in the eigenspace.
fn null_vector(a: &[f64], n: usize, lambda: Complex64) -> Vec<Complex64> {
    let mut m: Vec<Complex64> = a.iter().map(|&x| Complex64::new(x, 0.0)).collect();
    for i in 0..n {
        m[i * n + i] -= lambda;
    }
    let mut pivot_cols: Vec<usize> = Vec::with_capacity(n);
    let mut row = 0;
    for col in 0..n {
        // Pivot search in rows row..n.
        let Some(p) = (row..n).max_by(|&i, &j| {
            m[i * n + col]
                .norm()
                .partial_cmp(&m[j * n + col].norm())
                .unwrap_or(std::cmp::Ordering::Equal)
        }) else {
            break;
        };
        let tol = 1e-10
            * a.iter().map(|v| v.abs()).fold(0.0f64, f64::max).max(1.0)
            * (1.0 + lambda.norm());
        if m[p * n + col].norm() <= tol {
            continue; // free column
        }
        if p != row {
            for k in 0..n {
                m.swap(row * n + k, p * n + k);
            }
        }
        for r in row + 1..n {
            let f = m[r * n + col] / m[row * n + col];
            for k in col..n {
                let sub = f * m[row * n + k];
                m[r * n + k] -= sub;
            }
        }
        pivot_cols.push(col);
        row += 1;
        if row == n {
            break;
        }
    }
    // Free column: first column not used as a pivot (guaranteed since
    // A − λI is singular up to roundoff; fall back to the last column).
    let free = (0..n)
        .find(|c| !pivot_cols.contains(c))
        .unwrap_or(n - 1);
    let mut x = vec![Complex64::new(0.0, 0.0); n];
    x[free] = Complex64::new(1.0, 0.0);
    // Back-substitute pivot rows in reverse.
    for (r, &col) in pivot_cols.iter().enumerate().rev() {
        let mut s = Complex64::new(0.0, 0.0);
        for k in col + 1..n {
            s += m[r * n + k] * x[k];
        }
        x[col] = -s / m[r * n + col];
    }
    // Normalize to unit length with a real, positive leading component.
    let mag = x.iter().map(|v| v.norm_sqr()).sum::<f64>().sqrt();
    if mag > 0.0 {
        let lead = x
            .iter()
            .find(|v| v.norm() > 1e-12)
            .copied()
            .unwrap_or(Complex64::new(1.0, 0.0));
        let phase = lead / lead.norm();
        for v in &mut x {
            *v /= mag * phase;
        }
    }
    x
}
