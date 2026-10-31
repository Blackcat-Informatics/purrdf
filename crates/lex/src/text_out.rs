// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! [`TextOut`], the infallible append target every text writer in the
//! workspace emits into.
//!
//! It lives in the lexical layer because the writers that define the spelling
//! of a term — [`crate::iri_escape`], [`crate::literal_escape`] and
//! [`crate::term_syntax`] — live here, and every serializer above them
//! (N-Triples, Turtle, the canonical N-Quads writer, the SPARQL results
//! writers) hands them its own sink. `purrdf_core::sink` re-exports the trait
//! and implements it for its bounded, draining `TextSink`, so one emitter
//! serves both an in-memory `String` and a streamed document.

use core::fmt;

/// An infallible append target for text fragments.
///
/// Implemented here by [`String`], and by `purrdf_core::sink::TextSink`.
/// Emitters take this as a GENERIC bound and monomorphize, so shared writer
/// code costs no indirect call and a caller that already holds a whole
/// document — a sort comparator rendering a key into reusable scratch, say —
/// keeps using a plain `String` at full speed.
///
/// That is the division of labour that makes one emitter serve both
/// spellings. The object-safe boundary sits ABOVE these writers, at the codec
/// seam, where dispatch happens once per document rather than once per
/// fragment; below it, everything is generic.
///
/// Note the absence of any read-back, rewind, or length accessor. An emitter
/// written against this trait cannot inspect or retract what it has emitted,
/// which is what makes it safe to point at a sink that has already drained.
///
/// ```
/// use purrdf_lex::text_out::TextOut;
///
/// fn greet<W: TextOut + ?Sized>(out: &mut W) {
///     out.push_str("hello");
///     out.push('!');
/// }
///
/// let mut text = String::new();
/// greet(&mut text);
/// assert_eq!(text, "hello!");
/// assert!(!TextOut::failed(&text));
/// ```
pub trait TextOut: fmt::Write {
    /// Maximum raw byte extent one native classifier may examine before polling.
    /// Resident output keeps the original unbounded classifier fast path.
    fn scan_limit(&self) -> usize {
        usize::MAX
    }

    /// Admit a native scan before it examines that byte extent.
    fn checkpoint(&mut self, _bytes: usize) -> bool {
        !self.failed()
    }
    /// Append `text`.
    fn push_str(&mut self, text: &str);

    /// Append one character.
    fn push(&mut self, ch: char);

    /// Whether the destination has failed and is discarding further fragments.
    ///
    /// Emitters poll this at their innermost loop so a dead drain costs one
    /// fragment of formatting rather than a whole document. Always `false` for
    /// an in-memory target, which cannot fail.
    fn failed(&self) -> bool {
        false
    }
}

/// A transient writer borrowing the original native work and storage account.
/// Its destination keeps its existing ownership and allocation policy.
pub struct ProgressText<'o, 'm, 's, W: ?Sized, S: crate::allocation::Admission + ?Sized> {
    output: &'o mut W,
    memory: &'m mut crate::allocation::Memory<'s, S>,
    failure: Option<crate::allocation::StorageError>,
}

impl<W: ?Sized, S: crate::allocation::Admission + ?Sized> fmt::Debug
    for ProgressText<'_, '_, '_, W, S>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProgressText")
            .finish_non_exhaustive()
    }
}

impl<W: TextOut + ?Sized, S: crate::allocation::Admission + ?Sized> fmt::Write
    for ProgressText<'_, '_, '_, W, S>
{
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.push_str(text);
        if self.failed() {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

impl<W: TextOut + ?Sized, S: crate::allocation::Admission + ?Sized> TextOut
    for ProgressText<'_, '_, '_, W, S>
{
    fn scan_limit(&self) -> usize {
        if self.memory.admission().progress_enabled() {
            256
        } else {
            self.output.scan_limit()
        }
    }
    fn checkpoint(&mut self, bytes: usize) -> bool {
        if self.failed() {
            return false;
        }
        match u64::try_from(bytes)
            .map_err(|_| crate::allocation::StorageError::SizeOverflow)
            .and_then(|bytes| self.memory.admission_mut().checkpoint(bytes))
        {
            Ok(()) => true,
            Err(cause) => {
                self.failure = Some(cause);
                false
            }
        }
    }
    fn push_str(&mut self, text: &str) {
        if self.failed() {
            return;
        }
        if !self.memory.admission().progress_enabled() {
            self.output.push_str(text);
            return;
        }
        let mut at = 0;
        while at < text.len() {
            let mut end = at.saturating_add(256).min(text.len());
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            if !self.checkpoint(end - at) {
                return;
            }
            self.output.push_str(&text[at..end]);
            at = end;
        }
    }
    fn push(&mut self, character: char) {
        if self.checkpoint(character.len_utf8()) {
            self.output.push(character);
        }
    }
    fn failed(&self) -> bool {
        self.failure.is_some() || self.output.failed()
    }
}

/// Execute a native term writer under its original physical/progress account.
/// The writer stops through its existing failed-destination protocol; the first
/// typed inline refusal is restored before returning to the caller.
///
/// # Errors
/// Returns the first original native checkpoint refusal, never a formatting copy.
pub fn with_memory_progress<W: TextOut + ?Sized, S: crate::allocation::Admission + ?Sized>(
    output: &mut W,
    memory: &mut crate::allocation::Memory<'_, S>,
    body: impl FnOnce(&mut ProgressText<'_, '_, '_, W, S>),
) -> Result<(), crate::allocation::StorageError> {
    let mut writer = ProgressText {
        output,
        memory,
        failure: None,
    };
    if !writer.failed() {
        body(&mut writer);
    }
    match writer.failure {
        Some(cause) => Err(cause),
        None => Ok(()),
    }
}

impl TextOut for String {
    #[inline]
    fn push_str(&mut self, text: &str) {
        Self::push_str(self, text);
    }

    #[inline]
    fn push(&mut self, ch: char) {
        Self::push(self, ch);
    }
}
