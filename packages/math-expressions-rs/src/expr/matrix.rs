//! The row-major matrix payload of [`Expr::Matrix`], with its shape invariant
//! enforced by construction rather than by convention.

use crate::expr::Expr;

/// A row-major matrix for which `entries.len() == rows * cols` always holds.
///
/// The fields are private and no constructor can produce a violation: the
/// validating ones ([`Mat::new`], [`Mat::from_rows`]) reject a bad entry count,
/// and the generating ones ([`Mat::generate`], [`Mat::map`], [`Mat::zip_map`])
/// walk the index space and so produce exactly `rows * cols` entries. Readers
/// may therefore index `entries()[r * cols + c]` for `r < rows`, `c < cols`
/// without a bounds check of their own — [`Mat::get`] is there for when the
/// indices themselves are untrusted.
///
/// This was three sibling fields on the enum variant with the invariant stated
/// only in a doc comment. Dozens of readers across `matrix`, `ops`, `normalize`
/// and `print` indexed on faith, and `from_ast` accepts arbitrary JSON, so a
/// mismatched entry count would have indexed out of bounds — an uncatchable
/// abort in wasm, which builds with `panic = "abort"`. Every construction path
/// happened to be correct, so nothing was reachable; making the fields private
/// turns that coincidence into a guarantee the compiler keeps.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Mat {
    rows: u32,
    cols: u32,
    entries: Vec<Expr>,
}

impl Mat {
    /// A matrix from row-major `entries`, or `None` if the count does not match
    /// `rows * cols`. This is the constructor for untrusted shapes — the JSON
    /// codec and anything else reconstructing a matrix from outside data.
    pub fn new(rows: u32, cols: u32, entries: Vec<Expr>) -> Option<Mat> {
        let want = (rows as usize).checked_mul(cols as usize)?;
        (entries.len() == want).then_some(Mat {
            rows,
            cols,
            entries,
        })
    }

    /// A matrix built by generating every cell in row-major order. Infallible:
    /// it produces exactly `rows * cols` entries by construction, which is why
    /// internal builders (the identity, products, entrywise rewrites) should
    /// prefer it over [`Mat::new`] and its `Option`.
    pub fn generate(rows: u32, cols: u32, mut f: impl FnMut(u32, u32) -> Expr) -> Mat {
        let n = (rows as usize).saturating_mul(cols as usize);
        // Cap the speculative reservation: `rows`/`cols` are `u32`, so a bogus
        // pair asks for an allocation that would abort the process outright.
        // Growth reallocates instead, which real sizes never reach anyway.
        let mut entries = Vec::with_capacity(n.min(1 << 16));
        for r in 0..rows {
            for c in 0..cols {
                entries.push(f(r, c));
            }
        }
        Mat {
            rows,
            cols,
            entries,
        }
    }

    /// A matrix from a row-of-rows nesting, or `None` if the rows are ragged.
    /// Empty input is the 0×0 matrix.
    pub fn from_rows(rows: Vec<Vec<Expr>>) -> Option<Mat> {
        let n_rows = u32::try_from(rows.len()).ok()?;
        let n_cols = u32::try_from(rows.first().map_or(0, Vec::len)).ok()?;
        if rows.iter().any(|r| r.len() != n_cols as usize) {
            return None;
        }
        Some(Mat {
            rows: n_rows,
            cols: n_cols,
            entries: rows.into_iter().flatten().collect(),
        })
    }

    pub fn rows(&self) -> u32 {
        self.rows
    }

    pub fn cols(&self) -> u32 {
        self.cols
    }

    /// The entries in row-major order; `entries().len() == rows * cols`.
    pub fn entries(&self) -> &[Expr] {
        &self.entries
    }

    pub fn into_entries(self) -> Vec<Expr> {
        self.entries
    }

    /// The entry at `(r, c)`, or `None` if either index is out of range. Use
    /// this when the indices come from outside; a loop over `0..rows()` can
    /// index [`Mat::entries`] directly.
    pub fn get(&self, r: u32, c: u32) -> Option<&Expr> {
        if r >= self.rows || c >= self.cols {
            return None;
        }
        self.entries
            .get(r as usize * self.cols as usize + c as usize)
    }

    pub fn is_square(&self) -> bool {
        self.rows == self.cols
    }

    /// Entrywise rewrite. Shape-preserving, so it cannot break the invariant.
    pub fn map(&self, f: impl FnMut(&Expr) -> Expr) -> Mat {
        Mat {
            rows: self.rows,
            cols: self.cols,
            entries: self.entries.iter().map(f).collect(),
        }
    }

    /// Entrywise rewrite that consumes the matrix, for the by-value traversals
    /// (`flatten`) that would otherwise clone every entry. Shape-preserving.
    pub fn into_map(self, f: impl FnMut(Expr) -> Expr) -> Mat {
        Mat {
            rows: self.rows,
            cols: self.cols,
            entries: self.entries.into_iter().map(f).collect(),
        }
    }

    /// Move the entries out, leaving an empty 0×0 matrix.
    ///
    /// For [`tear_down`](crate::expr::tear_down), which drains a tree
    /// iteratively so that dropping a deep one cannot overflow the wasm stack.
    /// Resetting the dimensions alongside is what keeps this from being a hole
    /// in the invariant — 0×0 with no entries is a legal shape, so even the
    /// drained husk is well-formed.
    pub fn take_entries(&mut self) -> Vec<Expr> {
        self.rows = 0;
        self.cols = 0;
        std::mem::take(&mut self.entries)
    }

    /// Entrywise combination of two matrices, or `None` if their shapes differ.
    pub fn zip_map(&self, other: &Mat, mut f: impl FnMut(&Expr, &Expr) -> Expr) -> Option<Mat> {
        if self.rows != other.rows || self.cols != other.cols {
            return None;
        }
        Some(Mat {
            rows: self.rows,
            cols: self.cols,
            entries: self
                .entries
                .iter()
                .zip(&other.entries)
                .map(|(a, b)| f(a, b))
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_a_mismatched_entry_count() {
        assert!(Mat::new(2, 2, vec![Expr::int(1); 4]).is_some());
        assert!(Mat::new(2, 2, vec![Expr::int(1); 3]).is_none());
        assert!(Mat::new(2, 2, vec![]).is_none());
        // The overflow guard: `rows * cols` past `usize` has no valid count.
        assert!(Mat::new(u32::MAX, u32::MAX, vec![]).is_none());
    }

    #[test]
    fn generate_and_map_preserve_the_invariant() {
        let m = Mat::generate(3, 4, |r, c| Expr::int(i64::from(r * 10 + c)));
        assert_eq!(m.entries().len(), 12);
        assert_eq!(m.get(2, 3), Some(&Expr::int(23)));
        assert_eq!(m.map(|_| Expr::int(0)).entries().len(), 12);
    }

    #[test]
    fn get_is_bounds_checked_on_both_axes() {
        let m = Mat::generate(2, 3, |_, _| Expr::int(1));
        assert!(m.get(1, 2).is_some());
        assert!(m.get(2, 0).is_none());
        // Out of range on the column axis alone must not wrap into the next
        // row, which a bare `r * cols + c` index would happily do.
        assert!(m.get(0, 3).is_none());
    }

    #[test]
    fn from_rows_rejects_ragged_input() {
        assert_eq!(
            Mat::from_rows(vec![vec![Expr::int(1), Expr::int(2)], vec![Expr::int(3)]]),
            None
        );
        let m = Mat::from_rows(vec![vec![Expr::int(1), Expr::int(2)]]).unwrap();
        assert_eq!((m.rows(), m.cols()), (1, 2));
    }

    #[test]
    fn zip_map_rejects_a_shape_mismatch() {
        let a = Mat::generate(2, 2, |_, _| Expr::int(1));
        let b = Mat::generate(2, 3, |_, _| Expr::int(1));
        assert!(a.zip_map(&b, |x, _| x.clone()).is_none());
        assert!(a.zip_map(&a, |x, _| x.clone()).is_some());
    }
}
