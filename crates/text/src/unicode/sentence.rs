// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Default sentence boundaries from UAX 29 revision 47, on the workspace's
//! pinned Sentence_Break data, without locale tailoring or text normalization.

use super::{SOT, lookup_two_stage, next_borrowed_segment, tables};

/// The default sentence-boundary rule law, recorded with [`super::UNICODE_VERSION`]
/// and [`SENTENCE_DATA_DIGEST`] when segmentation contributes to an identity.
pub const SENTENCE_BOUNDARY_LAW: &str = "uax29-default-sentence/revision-47";

/// Content identity of the pinned Sentence_Break property input.
pub const SENTENCE_DATA_DIGEST: [u8; 32] = tables::SENTENCE_DATA_DIGEST;

#[inline]
fn property(c: char) -> u8 {
    lookup_two_stage(&tables::SENTENCE_INDEX, &tables::SENTENCE_BLOCKS, c)
}

#[inline]
const fn is_paragraph(value: u8) -> bool {
    matches!(value, tables::SB_SEP | tables::SB_CR | tables::SB_LF)
}

#[inline]
const fn is_ignored(value: u8) -> bool {
    matches!(value, tables::SB_EXTEND | tables::SB_FORMAT)
}

#[inline]
const fn is_terminal(value: u8) -> bool {
    matches!(value, tables::SB_ATERM | tables::SB_STERM)
}

/// One left-to-right pass, retaining only the rule-relevant left context.
#[derive(Clone, Debug)]
struct Boundaries<'a> {
    text: &'a str,
    start: usize,
    raw: u8,
    last: u8,
    before_last: u8,
    /// The suffix SATerm Close* Sp*, if present.
    terminal: Option<u8>,
    /// Whether the suffix has entered Sp*: Close no longer extends it.
    trailing_space: bool,
    /// The next SB8 decisive character, or the end of text. Every candidate
    /// at or before this offset has the same lowercase lookahead result.
    lookahead_end: usize,
    lookahead_lower: bool,
    #[cfg(test)]
    lookahead_reads: usize,
}

impl<'a> Boundaries<'a> {
    const fn new(text: &'a str) -> Self {
        Self {
            text,
            start: 0,
            raw: SOT,
            last: SOT,
            before_last: SOT,
            terminal: None,
            trailing_space: false,
            lookahead_end: 0,
            lookahead_lower: false,
            #[cfg(test)]
            lookahead_reads: 0,
        }
    }

    fn accept(&mut self, value: u8) {
        // SB5 ignores attached Extend/Format, except immediately after sot
        // or ParaSep. The raw predecessor preserves SB3 and SB4 precedence.
        let attached = is_ignored(value) && self.raw != SOT && !is_paragraph(self.raw);
        self.raw = value;
        if attached {
            return;
        }
        self.before_last = self.last;
        self.last = value;
        if is_terminal(value) {
            self.terminal = Some(value);
            self.trailing_space = false;
        } else if self.terminal.is_some() && value == tables::SB_SP {
            self.trailing_space = true;
        } else if !(self.terminal.is_some() && !self.trailing_space && value == tables::SB_CLOSE) {
            self.terminal = None;
            self.trailing_space = false;
        }
    }

    /// SB8's first Lower, or a character that forbids reaching Lower. The
    /// cached decisive offset moves monotonically; an arbitrarily long
    /// Close/Sp/other run is read at most once by this lookahead.
    fn lower_ahead(&mut self, at: usize) -> bool {
        if at <= self.lookahead_end {
            return self.lookahead_lower;
        }
        self.lookahead_end = self.text.len();
        self.lookahead_lower = false;
        for (offset, c) in self.text[at..].char_indices() {
            #[cfg(test)]
            {
                self.lookahead_reads += 1;
            }
            let value = property(c);
            if matches!(
                value,
                tables::SB_OLETTER | tables::SB_UPPER | tables::SB_LOWER
            ) || is_paragraph(value)
                || is_terminal(value)
            {
                self.lookahead_end = at + offset;
                self.lookahead_lower = value == tables::SB_LOWER;
                break;
            }
        }
        self.lookahead_lower
    }

    /// The rules in precedence order, at a byte boundary before `value`.
    fn breaks_before(&mut self, value: u8, at: usize) -> bool {
        // SB3 precedes the paragraph-separator break of SB4.
        if self.raw == tables::SB_CR && value == tables::SB_LF {
            return false;
        }
        if is_paragraph(self.raw) {
            return true;
        }
        // SB5: attach Format/Extend. The exceptions were decided above.
        if is_ignored(value) {
            return false;
        }
        // SB6: a decimal full stop directly before Numeric.
        if self.last == tables::SB_ATERM && value == tables::SB_NUMERIC {
            return false;
        }
        // SB7: a full stop between a letter and an uppercase letter.
        if self.last == tables::SB_ATERM
            && matches!(self.before_last, tables::SB_UPPER | tables::SB_LOWER)
            && value == tables::SB_UPPER
        {
            return false;
        }
        let Some(terminal) = self.terminal else {
            return false; // SB998.
        };
        if terminal == tables::SB_ATERM && self.lower_ahead(at) {
            return false; // SB8.
        }
        if value == tables::SB_SCONTINUE || is_terminal(value) {
            return false; // SB8a.
        }
        if (!self.trailing_space && value == tables::SB_CLOSE)
            || value == tables::SB_SP
            || is_paragraph(value)
        {
            return false; // SB9 and SB10.
        }
        // SB11. Its optional ParaSep is already covered by SB4.
        true
    }

    fn next_segment(&mut self) -> Option<(usize, &'a str)> {
        let start = self.start;
        // SB1 accepts the first scalar; SB2 ends the final nonempty segment.
        let (end, segment) = next_borrowed_segment(self.text, start, |c, at, _, first| {
            let value = property(c);
            if !first && self.breaks_before(value, at) {
                return true;
            }
            self.accept(value);
            false
        })?;
        self.start = end;
        Some((start, segment))
    }
}

/// Every segment between default sentence boundaries, borrowed verbatim and
/// in input order. Concatenating the segments reconstructs `text`, including
/// punctuation, whitespace, emoji-only segments and paragraph separators.
///
/// This is UAX 29's default language-independent law. It does not tailor
/// abbreviations for a locale or choose embedding-window sizes/overlap.
#[must_use]
pub fn sentence_bounds(text: &str) -> SentenceBounds<'_> {
    SentenceBounds(Boundaries::new(text))
}

/// Sentence segments and their UTF-8 byte offsets, keeping only segments that
/// contain an [`super::is_alphanumeric`] character, exactly as [`super::word_indices`].
/// Use [`sentence_bounds`] when every punctuation/emoji-only segment is needed.
#[must_use]
pub fn sentence_indices(text: &str) -> SentenceIndices<'_> {
    SentenceIndices(Boundaries::new(text))
}

/// The iterator [`sentence_bounds`] returns. No segment is copied or allocated.
#[derive(Clone, Debug)]
pub struct SentenceBounds<'a>(Boundaries<'a>);

impl<'a> Iterator for SentenceBounds<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_segment().map(|(_, segment)| segment)
    }
}

/// The iterator [`sentence_indices`] returns, over the same borrowed boundaries.
#[derive(Clone, Debug)]
pub struct SentenceIndices<'a>(Boundaries<'a>);

impl<'a> Iterator for SentenceIndices<'a> {
    type Item = (usize, &'a str);
    fn next(&mut self) -> Option<Self::Item> {
        super::next_alphanumeric_segment(|| self.0.next_segment())
    }
}

#[cfg(test)]
mod tests {
    use super::{Boundaries, sentence_bounds};

    #[test]
    fn lowercase_lookahead_does_not_rescan_long_closing_and_space_runs() {
        let run = format!("{}{}", ")".repeat(100_000), " ".repeat(100_000));
        for suffix in ["a", "A", "文", "!"] {
            let input = format!("A.{run}{suffix}");
            let mut boundaries = Boundaries::new(&input);
            let mut reconstructed = String::new();
            while let Some((_, segment)) = boundaries.next_segment() {
                reconstructed.push_str(segment);
            }
            assert_eq!(reconstructed, input);
            assert!(
                boundaries.lookahead_reads <= input.chars().count(),
                "SB8 must visit each lookahead scalar at most once"
            );
            if suffix == "a" {
                assert_eq!(sentence_bounds(&input).count(), 1);
            } else {
                // Close* and Sp* stay with the terminator; the final terminal
                // continues the sentence under SB8a rather than beginning one.
                assert_eq!(
                    sentence_bounds(&input).count(),
                    usize::from(suffix != "!") + 1
                );
            }
        }
    }
}
