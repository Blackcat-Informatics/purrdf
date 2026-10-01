// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-text position math: byte offset → 1-based line/column.
//!
//! The parsers in the query stack ([`purrdf-sparql-algebra`], [`purrdf-shex`],
//! the native RDF text codecs, and this crate's own IRI grammar) all report a
//! failure as a **byte offset** into the source they were handed. That is the
//! right thing to carry on the *happy* path — a byte offset is a single `usize`
//! with no scanning cost. Turning it into a human-facing `line:column` (and a
//! SARIF `region`) is a *resolution* step, and this module owns it.
//!
//! # Why resolution, not instrumentation
//!
//! A [`LineIndex`] is built with a single linear scan and answers positions in
//! `O(log n)` via binary search over a newline table. Crucially it is built
//! **lazily, on the error path only** — a successful parse never constructs one,
//! so line/column fidelity costs nothing unless a diagnostic is actually being
//! produced. This is what lets the codecs gain source-traced diagnostics without
//! threading a live line counter through their hot loops (which would regress the
//! parse-throughput baseline).
//!
//! Hosting this in the [`purrdf-iri`](crate) foundation, which takes no
//! third-party dependency, lets every
//! parser above it (`sparql-algebra`, `shex`, `rdf`) share one tested primitive
//! with no new dependency edge and no cycle.
//!
//! Columns count **Unicode scalar values** (code points) from the line start,
//! matching SARIF `region` column semantics; both line and column are 1-based.
//!
//! [`purrdf-sparql-algebra`]: https://docs.rs/purrdf-sparql-algebra
//! [`purrdf-shex`]: https://docs.rs/purrdf-shex

/// A resolved 1-based source position.
///
/// [`byte_offset`](Self::byte_offset) is retained alongside line/column because
/// SARIF `region.byteOffset` wants it and because it is the join key back to the
/// originating token span.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Position {
    /// 1-based line number.
    pub line: u64,
    /// 1-based column, counted in Unicode scalar values from the line start.
    pub column: u32,
    /// Byte offset into the source, clamped to a `char` boundary within `[0, len]`.
    pub byte_offset: usize,
}

impl Position {
    /// The 1-based column after a line prefix, counted in Unicode scalar values.
    ///
    /// # Errors
    /// Refuses a column exceeding `u32::MAX`.
    pub fn column_after(prefix: &str) -> Result<u32, PositionError> {
        checked_column(prefix.chars().count())
    }
}

/// A source position cannot fit its explicitly bounded local column space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PositionError {
    /// The 1-based Unicode-scalar column exceeds `u32::MAX`.
    ColumnLimit,
}

impl core::fmt::Display for PositionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ColumnLimit => f.write_str("source column exceeds u32::MAX"),
        }
    }
}

impl std::error::Error for PositionError {}

fn checked_column(scalars: usize) -> Result<u32, PositionError> {
    let scalars = u32::try_from(scalars).map_err(|_| PositionError::ColumnLimit)?;
    scalars.checked_add(1).ok_or(PositionError::ColumnLimit)
}

/// A newline table over one source document.
///
/// Built once with [`LineIndex::new`] (a single byte scan); resolves any byte
/// offset to a [`Position`] with [`LineIndex::locate`]. Intended to be
/// constructed on the error path only — see the [module docs](self).
#[derive(Clone, Debug)]
pub struct LineIndex {
    /// Byte offset of the first byte of each line. Always begins with `0`, so it
    /// is non-empty and strictly increasing.
    line_starts: Vec<usize>,
}

impl LineIndex {
    /// Build the newline table for `src` with a single linear scan, each line
    /// feed found by [`purrdf_lex::scan::find_byte`].
    #[must_use]
    pub fn new(src: &str) -> Self {
        let bytes = src.as_bytes();
        let mut line_starts = Vec::with_capacity(bytes.len() / 32 + 1);
        line_starts.push(0);
        let mut at = 0;
        while let Some(offset) = purrdf_lex::scan::find_byte(&bytes[at..], b'\n') {
            at += offset + 1;
            line_starts.push(at);
        }
        Self { line_starts }
    }

    /// Resolve `byte_offset` to a 1-based [`Position`].
    ///
    /// An offset past the end of `src` is clamped to end-of-input, and an offset
    /// landing inside a multi-byte `char` is clamped down to the enclosing `char`
    /// boundary. Offsets never cause a panic. Use [`Self::try_locate`] for
    /// sources whose columns may exceed the bounded local column space.
    ///
    /// # Panics
    /// Panics if the source column exceeds `u32::MAX`.
    #[must_use]
    pub fn locate(&self, src: &str, byte_offset: usize) -> Position {
        self.try_locate(src, byte_offset)
            .expect("source fits the bounded local column space")
    }

    /// Resolve a position, refusing a column outside its bounded local space.
    /// Offsets past the source or inside a scalar clamp as in [`Self::locate`].
    ///
    /// # Errors
    /// Returns [`PositionError::ColumnLimit`] instead of saturating a long column.
    pub fn try_locate(&self, src: &str, byte_offset: usize) -> Result<Position, PositionError> {
        // Clamp to the source length, then down to a char boundary so the
        // column slice below can never split a multi-byte scalar value.
        let off = src.floor_char_boundary(byte_offset);

        // The line containing `off` is the one with the greatest start <= off.
        // `line_starts[0] == 0 <= off`, so `count` is always >= 1.
        let count = self.line_starts.partition_point(|&start| start <= off);
        let line_start = self.line_starts[count - 1];
        let column = Position::column_after(&src[line_start..off])?;

        Ok(Position {
            line: count as u64,
            column,
            byte_offset: off,
        })
    }

    /// Resolve `byte_offset` to a `(line, column)` pair (both 1-based).
    ///
    /// A convenience over [`locate`](Self::locate) for callers that only need the
    /// line/column and not the clamped byte offset.
    #[must_use]
    pub fn line_col(&self, src: &str, byte_offset: usize) -> (u64, u32) {
        let p = self.locate(src, byte_offset);
        (p.line, p.column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_column_limit_refuses_instead_of_saturating() {
        assert_eq!(checked_column(u32::MAX as usize - 1), Ok(u32::MAX));
        assert_eq!(
            checked_column(u32::MAX as usize),
            Err(PositionError::ColumnLimit)
        );
    }
    use purrdf_testkit::prop::prelude::*;

    #[test]
    fn empty_source_is_line_one_column_one() {
        let idx = LineIndex::new("");
        assert_eq!(
            idx.locate("", 0),
            Position {
                line: 1,
                column: 1,
                byte_offset: 0
            }
        );
        // An offset past the (empty) end clamps to EOF, still 1:1.
        assert_eq!(
            idx.locate("", 99),
            Position {
                line: 1,
                column: 1,
                byte_offset: 0
            }
        );
    }

    #[test]
    fn single_line_columns_advance() {
        let src = "abcde";
        let idx = LineIndex::new(src);
        assert_eq!(idx.line_col(src, 0), (1, 1));
        assert_eq!(idx.line_col(src, 3), (1, 4));
        // Offset at len() is EOF: column one past the last character.
        assert_eq!(idx.line_col(src, 5), (1, 6));
    }

    #[test]
    fn newlines_advance_lines_and_reset_columns() {
        let src = "ab\ncd\n\nef";
        let idx = LineIndex::new(src);
        assert_eq!(idx.line_col(src, 0), (1, 1)); // 'a'
        assert_eq!(idx.line_col(src, 1), (1, 2)); // 'b'
        assert_eq!(idx.line_col(src, 3), (2, 1)); // 'c' (byte 3, after "ab\n")
        assert_eq!(idx.line_col(src, 4), (2, 2)); // 'd'
        assert_eq!(idx.line_col(src, 6), (3, 1)); // empty line (byte 6, after "cd\n")
        assert_eq!(idx.line_col(src, 7), (4, 1)); // 'e'
    }

    #[test]
    fn columns_count_scalar_values_not_bytes() {
        // "é" is 2 bytes (U+00E9), "𝔸" is 4 bytes (U+1D538). Columns count chars.
        let src = "é𝔸x";
        let idx = LineIndex::new(src);
        assert_eq!(idx.line_col(src, 0), (1, 1)); // before 'é'
        assert_eq!(idx.line_col(src, 2), (1, 2)); // before '𝔸' (after 2-byte 'é')
        assert_eq!(idx.line_col(src, 6), (1, 3)); // before 'x' (after 4-byte '𝔸')
    }

    #[test]
    fn offset_inside_multibyte_char_clamps_down() {
        let src = "é"; // bytes [0xC3, 0xA9]
        let idx = LineIndex::new(src);
        // Byte 1 is mid-scalar; clamp down to the boundary at 0.
        let p = idx.locate(src, 1);
        assert_eq!(
            p,
            Position {
                line: 1,
                column: 1,
                byte_offset: 0
            }
        );
    }

    prop_test! {
        // `locate` is monotonic in byte offset: a larger offset never resolves to
        // an earlier position (line, then column, then byte_offset ordering).
        #[test]
        fn locate_is_monotonic(src in prop::string::regex(".{0,200}"), a in 0usize..256, b in 0usize..256) {
            let idx = LineIndex::new(&src);
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            let pa = idx.locate(&src, lo);
            let pb = idx.locate(&src, hi);
            prop_assert!(pa <= pb, "locate({lo})={pa:?} must be <= locate({hi})={pb:?}");
        }

        // Never panics and always yields 1-based coordinates for any offset.
        #[test]
        fn locate_never_panics_and_is_one_based(src in prop::string::regex(".{0,200}"), off in 0usize..1024) {
            let idx = LineIndex::new(&src);
            let p = idx.locate(&src, off);
            prop_assert!(p.line >= 1);
            prop_assert!(p.column >= 1);
            prop_assert!(p.byte_offset <= src.len());
            prop_assert!(src.is_char_boundary(p.byte_offset));
        }
    }
}
