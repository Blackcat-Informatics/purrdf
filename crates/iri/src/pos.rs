// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! Hosting this in the zero-dependency [`purrdf-iri`](crate) leaf lets every
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
    pub line: u32,
    /// 1-based column, counted in Unicode scalar values from the line start.
    pub column: u32,
    /// Byte offset into the source, clamped to a `char` boundary within `[0, len]`.
    pub byte_offset: usize,
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
    /// Build the newline table for `src` with a single linear scan.
    ///
    /// A plain byte loop (not `memchr`) is deliberate: this crate is a
    /// zero-dependency leaf, and the scan only runs when a diagnostic is being
    /// constructed, so it is never on a hot path.
    #[must_use]
    pub fn new(src: &str) -> Self {
        let mut line_starts = Vec::with_capacity(src.len() / 32 + 1);
        line_starts.push(0);
        for (i, &b) in src.as_bytes().iter().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    /// Resolve `byte_offset` to a 1-based [`Position`].
    ///
    /// An offset past the end of `src` is clamped to end-of-input, and an offset
    /// landing inside a multi-byte `char` is clamped down to the enclosing `char`
    /// boundary. This never panics, whatever offset a lexer hands it.
    #[must_use]
    pub fn locate(&self, src: &str, byte_offset: usize) -> Position {
        // Clamp to the source length, then down to a char boundary so the
        // column slice below can never split a multi-byte scalar value.
        let mut off = byte_offset.min(src.len());
        while off > 0 && !src.is_char_boundary(off) {
            off -= 1;
        }

        // The line containing `off` is the one with the greatest start <= off.
        // `line_starts[0] == 0 <= off`, so `count` is always >= 1.
        let count = self.line_starts.partition_point(|&start| start <= off);
        let line_start = self.line_starts[count - 1];
        let column = src[line_start..off].chars().count() + 1;

        Position {
            line: u32::try_from(count).unwrap_or(u32::MAX),
            column: u32::try_from(column).unwrap_or(u32::MAX),
            byte_offset: off,
        }
    }

    /// Resolve `byte_offset` to a `(line, column)` pair (both 1-based).
    ///
    /// A convenience over [`locate`](Self::locate) for callers that only need the
    /// line/column and not the clamped byte offset.
    #[must_use]
    pub fn line_col(&self, src: &str, byte_offset: usize) -> (u32, u32) {
        let p = self.locate(src, byte_offset);
        (p.line, p.column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

    #[test]
    fn empty_source_is_line_one_column_one() {
        let idx = LineIndex::new("");
        assert_eq!(idx.locate("", 0), Position { line: 1, column: 1, byte_offset: 0 });
        // An offset past the (empty) end clamps to EOF, still 1:1.
        assert_eq!(idx.locate("", 99), Position { line: 1, column: 1, byte_offset: 0 });
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
        assert_eq!(p, Position { line: 1, column: 1, byte_offset: 0 });
    }

    proptest! {
        // `locate` is monotonic in byte offset: a larger offset never resolves to
        // an earlier position (line, then column, then byte_offset ordering).
        #[test]
        fn locate_is_monotonic(src in ".{0,200}", a in 0usize..256, b in 0usize..256) {
            let idx = LineIndex::new(&src);
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            let pa = idx.locate(&src, lo);
            let pb = idx.locate(&src, hi);
            prop_assert!(pa <= pb, "locate({lo})={pa:?} must be <= locate({hi})={pb:?}");
        }

        // Never panics and always yields 1-based coordinates for any offset.
        #[test]
        fn locate_never_panics_and_is_one_based(src in ".{0,200}", off in 0usize..1024) {
            let idx = LineIndex::new(&src);
            let p = idx.locate(&src, off);
            prop_assert!(p.line >= 1);
            prop_assert!(p.column >= 1);
            prop_assert!(p.byte_offset <= src.len());
            prop_assert!(src.is_char_boundary(p.byte_offset));
        }
    }
}
