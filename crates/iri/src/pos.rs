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
        match count_scalars(prefix, &mut purrdf_lex::control::Unmetered) {
            Ok(scalars) => checked_column(scalars),
            Err(purrdf_lex::control::ControlledError::Native(cause)) => Err(cause),
            Err(purrdf_lex::control::ControlledError::Control(cause)) => match cause {},
        }
    }
}

/// A source position cannot fit its explicitly bounded local column space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PositionError {
    /// The 1-based Unicode-scalar column exceeds `u32::MAX`.
    ColumnLimit,
    /// A forward source cursor was asked to revisit an earlier scalar boundary.
    BackwardsOffset,
}

impl purrdf_hash::error::StructuredError for PositionError {
    purrdf_hash::error_visit! {
        ColumnLimit => "iri-position-column-limit" {},
        BackwardsOffset => "iri-position-backwards-offset" {},
    }
}

impl core::fmt::Display for PositionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ColumnLimit => f.write_str("source column exceeds u32::MAX"),
            Self::BackwardsOffset => {
                f.write_str("forward source position cannot revisit an earlier offset")
            }
        }
    }
}

impl std::error::Error for PositionError {}

fn checked_column(scalars: usize) -> Result<u32, PositionError> {
    let scalars = u32::try_from(scalars).map_err(|_| PositionError::ColumnLimit)?;
    scalars.checked_add(1).ok_or(PositionError::ColumnLimit)
}

/// An allocation-free forward cursor over one borrowed source document.
///
/// Newlines and Unicode columns follow [`LineIndex`]: only LF starts a new
/// line, CR is an ordinary scalar, and offsets clamp down to a UTF-8 boundary.
/// Each source extent is traversed once across monotonic requests, including
/// repeated locations on one long line. No newline table or prefix rescans are
/// needed. The source remains borrowed for the cursor's complete lifetime.
///
/// ```
/// use purrdf_iri::PositionCursor;
/// use purrdf_lex::control::Unmetered;
/// let mut cursor = PositionCursor::new("é\r\n𝔸");
/// assert_eq!(cursor.advance(2, &mut Unmetered)?.column, 2);
/// assert_eq!(cursor.advance(8, &mut Unmetered)?.line, 2);
/// # Ok::<(), purrdf_lex::control::ControlledError<purrdf_iri::PositionError, core::convert::Infallible>>(())
/// ```
#[derive(Debug)]
pub struct PositionCursor<'a> {
    source: &'a str,
    at: usize,
    line: u64,
    scalars: usize,
}

impl<'a> PositionCursor<'a> {
    /// Start at byte zero, line one, column one, without reading the source.
    pub const fn new(source: &'a str) -> Self {
        Self {
            source,
            at: 0,
            line: 1,
            scalars: 0,
        }
    }

    /// The line reached by the original forward scan, including on refusal.
    pub const fn line(&self) -> u64 {
        self.line
    }

    /// Locate a nondecreasing byte offset through bounded native source scans.
    /// # Errors
    /// Preserves the caller's original progress failure; refuses a backwards
    /// request or a final column outside the same local space as `LineIndex`.
    pub fn advance<C: purrdf_lex::control::Progress + ?Sized>(
        &mut self,
        byte_offset: usize,
        progress: &mut C,
    ) -> Result<Position, purrdf_lex::control::ControlledError<PositionError, C::Error>> {
        use purrdf_lex::control::ControlledError;
        progress.charge(0).map_err(ControlledError::Control)?;
        let off = self.source.floor_char_boundary(byte_offset);
        if off < self.at {
            return Err(PositionError::BackwardsOffset.into());
        }
        while self.at < off {
            let mut end = if C::ENABLED {
                self.at.saturating_add(256).min(off)
            } else {
                off
            };
            while !self.source.is_char_boundary(end) {
                end -= 1;
            }
            let bytes = &self.source.as_bytes()[self.at..end];
            // One concrete bounded classifier step; the kernel may stop at an
            // early LF, so do not precharge the entire unconsumed suffix.
            progress.charge(1).map_err(ControlledError::Control)?;
            if let Some(newline) = purrdf_lex::scan::find_byte(bytes, b'\n') {
                // A completed preceding line contributes only its newline to
                // the requested position. It cannot overflow the final column.
                self.at += newline + 1;
                self.line += 1;
                self.scalars = 0;
            } else {
                let extent = &self.source[self.at..end];
                self.scalars += count_scalars(extent, progress)?;
                self.at = end;
            }
        }
        Ok(Position {
            line: self.line,
            column: checked_column(self.scalars).map_err(ControlledError::Native)?,
            byte_offset: off,
        })
    }
}

fn count_scalars<C: purrdf_lex::control::Progress + ?Sized>(
    text: &str,
    progress: &mut C,
) -> Result<usize, purrdf_lex::control::ControlledError<PositionError, C::Error>> {
    if !C::ENABLED {
        return Ok(text.chars().count());
    }
    let mut count = 0;
    for _ in text.chars() {
        progress
            .charge(1)
            .map_err(purrdf_lex::control::ControlledError::Control)?;
        count += 1;
    }
    Ok(count)
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
        let mut resident = purrdf_lex::allocation::Resident;
        let mut memory = purrdf_lex::allocation::Memory::new(&mut resident);
        Self::new_with_memory(src, &mut memory).expect("resident source positions")
    }

    /// Build the same newline table under original physical admission.
    ///
    /// # Errors
    /// Returns checked storage overflow, admission refusal or allocator refusal.
    pub fn new_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        src: &str,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<Self, purrdf_lex::allocation::StorageError> {
        let bytes = src.as_bytes();
        let mut line_starts = Vec::new();
        memory.push(&mut line_starts, 0)?;
        let mut at = 0;
        while let Some(offset) = purrdf_lex::scan::find_byte(&bytes[at..], b'\n') {
            at += offset + 1;
            memory.push(&mut line_starts, at)?;
        }
        Ok(Self { line_starts })
    }

    /// Destroy the newline table before releasing its original physical owner.
    ///
    /// # Errors
    /// Returns an accounting invariant or shrink refusal.
    pub fn release_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        self,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<(), purrdf_lex::allocation::StorageError> {
        memory.release_vec(self.line_starts)
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
        let source = format!("{}\r\n{}", "é𝔸x".repeat(512), "é𝔸x".repeat(512));
        let index = LineIndex::new(&source);
        let mut cursor = PositionCursor::new(&source);
        let mut work = 0u64;
        for off in 0..=source.len() + 1 {
            let position = cursor
                .advance(off, &mut |units| {
                    work += units;
                    Ok::<_, u8>(())
                })
                .unwrap();
            assert_eq!(position, index.locate(&source, off));
        }
        assert!(
            work <= 2 * source.len() as u64,
            "monotonic queries do not rescan source prefixes"
        );
        let mut stopped = PositionCursor::new(&source);
        assert!(matches!(
            stopped.advance(source.len(), &mut |_| Err(9u8)),
            Err(purrdf_lex::control::ControlledError::Control(9))
        ));
        assert!(matches!(
            cursor.advance(0, &mut purrdf_lex::control::Unmetered),
            Err(purrdf_lex::control::ControlledError::Native(
                PositionError::BackwardsOffset
            ))
        ));
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
