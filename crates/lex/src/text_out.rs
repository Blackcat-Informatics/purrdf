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
