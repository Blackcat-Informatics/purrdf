// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Extended grapheme boundaries from UAX 29 revision 47, including Indic
//! conjuncts, pictographic joins and regional-indicator parity.

use super::{is_extended_pictographic, lookup_two_stage, tables};
use std::str::CharIndices;

fn property(c: char) -> u8 {
    lookup_two_stage(&tables::GRAPHEME_INDEX, &tables::GRAPHEME_BLOCKS, c)
}

/// Exact Indic conjunct consonant property from the generated data.
pub fn is_conjunct_consonant(c: char) -> bool {
    property(c) & tables::INCB_MASK == tables::INCB_CONSONANT
}

/// Qualification and presentation metadata from the finite emoji data set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EmojiStatus(u8);
impl EmojiStatus {
    /// Source data flags: full/minimal/unqualified/component/text/emoji style.
    pub const fn flags(self) -> u8 {
        self.0
    }
}

/// Exact recognition metadata; an absent result does not declare malformed text.
pub fn emoji_status(text: &str) -> Option<EmojiStatus> {
    tables::EMOJI_ATOMS
        .binary_search_by(|(atom, _)| atom.cmp(&text))
        .ok()
        .map(|at| EmojiStatus(tables::EMOJI_ATOMS[at].1))
}

/// Protect known atoms and structurally joined unrecognized pictographic EGCs.
/// `text` must be one extended grapheme, obtained from [`grapheme_bounds`].
pub fn is_emoji_grapheme(text: &str) -> bool {
    if text.is_ascii() {
        return false;
    }
    // EP/RI already protect every finite atom except keycaps, their standardized
    // variation spellings and skin-tone components. Only a keycap/variation
    // prefix or a singleton skin-tone component needs finite recognition;
    // keeping that lookup first retains their direct successful return.
    // The complete official inventory and variation sequences independently
    // verify this candidate class on native and WebAssembly targets.
    let mut scalars = text.chars();
    let candidate = match scalars.next() {
        Some('#' | '*' | '0'..='9') => {
            matches!(scalars.next(), Some('\u{20e3}' | '\u{fe0e}' | '\u{fe0f}'))
        }
        Some('\u{1f3fb}'..='\u{1f3ff}') => scalars.next().is_none(),
        _ => false,
    };
    (candidate && emoji_status(text).is_some())
        || text.chars().any(|c| {
            is_extended_pictographic(c)
                || property(c) & tables::GB_MASK == tables::GB_REGIONAL_INDICATOR
        })
}

/// An allocation-free iterator over `(UTF-8 byte offset, extended grapheme)`.
#[derive(Clone, Debug)]
pub struct GraphemeBounds<'a> {
    text: &'a str,
    cursor: CharIndices<'a>,
    pending: Option<(usize, char)>,
}

/// Extended graphemes, including degenerate leading-mark clusters.
pub fn grapheme_bounds(text: &str) -> GraphemeBounds<'_> {
    GraphemeBounds {
        text,
        cursor: text.char_indices(),
        pending: None,
    }
}

// UAX 29 has independent pictographic and Indic state machines.
#[allow(clippy::struct_excessive_bools)]
#[derive(Default)]
struct Context {
    previous: u8,
    regional: usize,
    pictographic: bool,
    after_joiner: bool,
    conjunct: bool,
    linked: bool,
}

impl Context {
    fn breaks_before(&self, c: char, raw: u8) -> bool {
        let current = raw & tables::GB_MASK;
        let previous = self.previous;
        if previous == tables::GB_CR && current == tables::GB_LF {
            return false;
        }
        if matches!(previous, tables::GB_CR | tables::GB_LF | tables::GB_CONTROL)
            || matches!(current, tables::GB_CR | tables::GB_LF | tables::GB_CONTROL)
        {
            return true;
        }
        if previous == tables::GB_L
            && matches!(
                current,
                tables::GB_L | tables::GB_V | tables::GB_LV | tables::GB_LVT
            )
            || matches!(previous, tables::GB_LV | tables::GB_V)
                && matches!(current, tables::GB_V | tables::GB_T)
            || matches!(previous, tables::GB_LVT | tables::GB_T) && current == tables::GB_T
        {
            return false;
        }
        if matches!(
            current,
            tables::GB_EXTEND | tables::GB_ZWJ | tables::GB_SPACINGMARK
        ) || previous == tables::GB_PREPEND
        {
            return false;
        }
        if raw & tables::INCB_MASK == tables::INCB_CONSONANT && self.conjunct && self.linked {
            return false;
        }
        if self.after_joiner && is_extended_pictographic(c) {
            return false;
        }
        !(previous == tables::GB_REGIONAL_INDICATOR
            && current == tables::GB_REGIONAL_INDICATOR
            && self.regional % 2 == 1)
    }

    fn push(&mut self, c: char, raw: u8) {
        let current = raw & tables::GB_MASK;
        self.after_joiner = current == tables::GB_ZWJ && self.pictographic;
        if is_extended_pictographic(c) {
            self.pictographic = true;
        } else if current != tables::GB_EXTEND {
            self.pictographic = false;
        }
        self.regional = if current == tables::GB_REGIONAL_INDICATOR {
            self.regional + 1
        } else {
            0
        };
        match raw & tables::INCB_MASK {
            tables::INCB_CONSONANT => {
                self.conjunct = true;
                self.linked = false;
            }
            tables::INCB_LINKER => self.linked |= self.conjunct,
            tables::INCB_EXTEND => {}
            _ => {
                self.conjunct = false;
                self.linked = false;
            }
        }
        self.previous = current;
    }
}

impl<'a> Iterator for GraphemeBounds<'a> {
    type Item = (usize, &'a str);
    fn next(&mut self) -> Option<Self::Item> {
        let (start, first) = self.pending.take().or_else(|| self.cursor.next())?;
        let mut context = Context {
            previous: tables::GB_OTHER,
            ..Context::default()
        };
        context.push(first, property(first));
        for (at, c) in self.cursor.by_ref() {
            let raw = property(c);
            if context.breaks_before(c, raw) {
                self.pending = Some((at, c));
                return Some((start, &self.text[start..at]));
            }
            context.push(c, raw);
        }
        Some((start, &self.text[start..]))
    }
}
