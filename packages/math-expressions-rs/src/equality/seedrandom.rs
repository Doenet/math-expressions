//! A port of David Bau's `seedrandom` — the ARC4-based PRNG the JS library
//! seeds its sampling stages with.
//!
//! # Why this exists rather than a better generator
//!
//! The numerical equality stage does not *measure* agreement, it *searches* for
//! a region of it, and gives up after a fixed budget. For a response that sits
//! near the tolerance boundary — which is exactly what a
//! `allowedErrorInNumbers` award produces — whether the search succeeds is
//! decided by which points the generator happens to offer. Two correct
//! implementations with different generators will therefore disagree on those
//! responses, and no amount of care in the surrounding algorithm removes that.
//!
//! Measured on the pair from DoenetML's `errorInNumbers > don't ignore
//! exponents`: one response agrees at ~64 % of scale-10 draws and another at
//! ~3 %. Grading them the same way as the JS library means drawing the same
//! numbers, in the same order, from the same seed — so this reproduces
//! `seedrandom("real_seed")` and `seedrandom("complex_seed")` exactly, quirks
//! included.
//!
//! This is not a general-purpose RNG and must not be used as one. It is a
//! compatibility surface: its only contract is that its output stream is
//! identical to the JS library's.

/// ARC4 state, seeded and drop-256'd exactly as `seedrandom` does.
pub(super) struct SeedRandom {
    s: [u8; 256],
    i: usize,
    j: usize,
}

impl SeedRandom {
    /// `seedrandom(seed)` with no options: the seed string is folded into a key
    /// by `mixkey`, the key schedules ARC4, and the first 256 outputs are
    /// discarded (RC4-drop[256]).
    pub(super) fn new(seed: &str) -> SeedRandom {
        let key = mixkey(seed);
        let mut s: [u8; 256] = std::array::from_fn(|i| i as u8);
        // JS: `if (!keylen) { key = [keylen++]; }` — an empty key is `[0]`.
        let key: Vec<u8> = if key.is_empty() { vec![0] } else { key };
        let keylen = key.len();
        let mut j = 0usize;
        for i in 0..256 {
            let t = s[i];
            j = (j + key[i % keylen] as usize + t as usize) & 255;
            s[i] = s[j];
            s[j] = t;
        }
        // The key-schedule's `j` is a local in the JS and is *not* carried into
        // the generator; `me.i` and `me.j` are both still 0 when the drop runs.
        let mut me = SeedRandom { s, i: 0, j: 0 };
        me.g(256);
        me
    }

    /// The next `count` ARC4 outputs concatenated base-256, as `f64` — the JS
    /// `arc4.g`. The accumulator is a double there too, so a large `count`
    /// loses precision identically; only `count <= 6` is used for a value.
    fn g(&mut self, count: usize) -> f64 {
        let mut r = 0.0f64;
        let (mut i, mut j) = (self.i, self.j);
        for _ in 0..count {
            i = (i + 1) & 255;
            let t = self.s[i];
            j = (j + t as usize) & 255;
            let sj = self.s[j];
            self.s[i] = sj;
            self.s[j] = t;
            // Read *after* both writes, and index by the new `s[i]` plus `t`.
            r = r * 256.0 + self.s[(sj as usize + t as usize) & 255] as f64;
        }
        self.i = i;
        self.j = j;
        r
    }

    /// A double in `[0, 1)` with randomness in every mantissa bit — the JS
    /// `prng`. Transcribed rather than simplified: the loop bounds are what
    /// decide the low-order bits, and those bits move sample points.
    pub(super) fn next_f64(&mut self) -> f64 {
        const WIDTH: f64 = 256.0;
        const CHUNKS: usize = 6;
        const SIGNIFICANCE: f64 = 4_503_599_627_370_496.0; // 2^52
        const OVERFLOW: f64 = 9_007_199_254_740_992.0; // 2^53
        let mut n = self.g(CHUNKS);
        let mut d = WIDTH.powi(CHUNKS as i32); // 2^48
        let mut x = 0.0f64;
        while n < SIGNIFICANCE {
            n = (n + x) * WIDTH;
            d *= WIDTH;
            x = self.g(1);
        }
        while n >= OVERFLOW {
            n /= 2.0;
            d /= 2.0;
            // JS `x >>>= 1` — an unsigned shift, and `x` is a byte here.
            x = ((x as u32) >> 1) as f64;
        }
        (n + x) / d
    }
}

/// `mixkey(seed, [])`: fold the seed's UTF-16 code units into key bytes.
///
/// The JS reads `key[mask & j]` before writing it, so on a fresh key every read
/// is `undefined`; `undefined * 19` is `NaN`, and `smear ^= NaN` leaves `smear`
/// alone (both sides go through ToInt32, and ToInt32(NaN) is 0). That is why a
/// seed shorter than 256 units reduces to "take the low byte of each unit" —
/// but the smear is transcribed anyway so a longer seed still matches.
fn mixkey(seed: &str) -> Vec<u8> {
    let mut key: Vec<Option<u8>> = Vec::new();
    let mut smear: i32 = 0;
    for (j, unit) in seed.encode_utf16().enumerate() {
        let idx = j & 255;
        if idx >= key.len() {
            key.resize(idx + 1, None);
        }
        if let Some(v) = key[idx] {
            smear ^= v as i32 * 19;
        }
        key[idx] = Some(((smear as i64 + unit as i64) & 255) as u8);
    }
    // No holes are possible: indices are filled contiguously from 0, so a key
    // of length n has every slot below n written.
    key.into_iter().map(|o| o.unwrap_or(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::SeedRandom;

    /// Pinned against `node -e 'const s=require("seedrandom"); const r=s("complex_seed");
    /// for (let i=0;i<5;i++) console.log(r());'` — the exact stream the JS
    /// library samples with. If this drifts, grading drifts with it.
    #[test]
    fn matches_the_js_stream() {
        let mut r = SeedRandom::new("complex_seed");
        let got: Vec<f64> = (0..5).map(|_| r.next_f64()).collect();
        let want = [
            0.7350592960700851,
            0.13420310042705125,
            0.7506966892780972,
            0.35117780285752676,
            0.2128596810857395,
        ];
        for (g, w) in got.iter().zip(want.iter()) {
            assert_eq!(g, w, "stream diverged: {got:?}");
        }
    }

    #[test]
    fn real_seed_stream() {
        let mut r = SeedRandom::new("real_seed");
        let got: Vec<f64> = (0..3).map(|_| r.next_f64()).collect();
        let want = [
            0.16979478014439325,
            0.38553251359084145,
            0.07403646540305946,
        ];
        for (g, w) in got.iter().zip(want.iter()) {
            assert_eq!(g, w, "stream diverged: {got:?}");
        }
    }
}
