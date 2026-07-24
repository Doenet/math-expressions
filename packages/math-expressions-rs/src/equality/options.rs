//! Options mirroring the JS `equals` parameters.

/// Options mirroring the JS `equals` parameters: the
/// tolerances, coercion flags, `allowed_error_in_numbers` fuzzy matching, and
/// the `real_only` sampling mode used by [`super::equals_via_real`].
#[derive(Debug, Clone)]
pub struct EqOptions {
    pub relative_tolerance: f64,
    pub absolute_tolerance: f64,
    pub tolerance_for_zero: f64,
    /// Accept number literals that differ by this error (grading option):
    /// `3.14` matches `pi` when the allowed error covers the gap. `0` (the
    /// default) means exact-number semantics.
    pub allowed_error_in_numbers: f64,
    /// Whether the allowed number error also applies inside exponents
    /// (default: exponents must match exactly).
    pub include_error_in_number_exponents: bool,
    /// Interpret `allowed_error_in_numbers` as an absolute error instead of
    /// relative to the numbers' magnitude.
    pub allowed_error_is_absolute: bool,
    pub allow_blanks: bool,
    pub coerce_tuples_arrays: bool,
    pub coerce_vectors: bool,
    /// Number of random complex sample points for the numerical stage.
    pub num_samples: usize,
    /// Sample only real points in the numerical stage (the `equals_via_real`
    /// mode). Off by default — full `equals` samples the complex plane.
    pub real_only: bool,
}

impl Default for EqOptions {
    fn default() -> Self {
        EqOptions {
            relative_tolerance: 1e-12,
            absolute_tolerance: 0.0,
            tolerance_for_zero: 1e-15,
            allowed_error_in_numbers: 0.0,
            include_error_in_number_exponents: false,
            allowed_error_is_absolute: false,
            allow_blanks: false,
            coerce_tuples_arrays: true,
            coerce_vectors: true,
            num_samples: 20,
            real_only: false,
        }
    }
}
