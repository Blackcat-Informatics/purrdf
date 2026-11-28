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

/// Recognize the atomic boundaries of pictographic and regional-indicator EGCs.
/// This boundary predicate does not grant their attached controls protection.
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
    // verify this candidate class in the native Unicode conformance suite.
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

/// Scalar protection inside an admitted atomic emoji grapheme. The existing
/// atomic predicate supplies admission; complete elements and joins (UTS 51
/// revision 29, ED-14a, ED-15a and ED-16) then protect individual controls.
/// Unrelated controls cannot borrow a neighboring emoji base's protection.
pub(crate) fn emoji_scalars(text: &str) -> EmojiScalars<'_> {
    EmojiScalars {
        text,
        admitted: is_emoji_grapheme(text),
        cursor: text.char_indices(),
        protected_end: 0,
        element_end: None,
    }
}

#[derive(Debug)]
pub(crate) struct EmojiScalars<'a> {
    text: &'a str,
    admitted: bool,
    cursor: CharIndices<'a>,
    protected_end: usize,
    element_end: Option<usize>,
}

impl Iterator for EmojiScalars<'_> {
    type Item = (usize, char, bool);

    fn next(&mut self) -> Option<Self::Item> {
        let (at, c) = self.cursor.next()?;
        if !self.admitted {
            return Some((at, c, false));
        }
        if at < self.protected_end {
            return Some((at, c, true));
        }
        if c == '\u{200d}' && self.element_end == Some(at) {
            let start = at + c.len_utf8();
            if let Some((end, true)) = emoji_element_end(&self.text[start..]) {
                self.protected_end = start + end;
                self.element_end = Some(self.protected_end);
                return Some((at, c, true));
            }
        }
        if let Some((end, joins)) = emoji_element_end(&self.text[at..]) {
            self.protected_end = at + end;
            self.element_end = joins.then_some(self.protected_end);
            return Some((at, c, true));
        }
        self.element_end = None;
        Some((at, c, false))
    }
}

/// The protected atom beginning here, together with its eligibility as a ZWJ
/// endpoint. A tag suffix must be syntactically complete. Pictographic bases
/// also admit joins absent from the finite RGI inventory. Core lookahead is
/// bounded; each tag run is inspected at most twice (once from its left joiner
/// and once from its base), so even an
/// unterminated suffix takes linear work and allocates no storage.
fn emoji_element_end(text: &str) -> Option<(usize, bool)> {
    let mut scalars = text.char_indices();
    let (_, first) = scalars.next()?;
    let mut end = first.len_utf8();
    if is_extended_pictographic(first) {
        // UAX 29 reserves pictographic boundaries for future emoji. Those
        // unassigned/non-emoji bases stay atomic, but only an admitted emoji
        // character can protect attached controls or a structural join.
        emoji_status(&text[..end])?;
    } else if property(first) & tables::GB_MASK != tables::GB_REGIONAL_INDICATOR
        && !matches!(first, '\u{1f3fb}'..='\u{1f3ff}' | '#' | '*' | '0'..='9')
    {
        return None;
    }
    // RI, modifiers and keycap bases also have Emoji=Yes (ED-3), so they are
    // core characters (ED-15) where the admitted EGC contains them. Component
    // status alone does not invalidate a complete join or tag suffix.
    let mut joins = true;
    if let Some((at, variation @ ('\u{fe0e}' | '\u{fe0f}'))) = scalars.clone().next() {
        end = at + variation.len_utf8();
        // A text presentation is preserved verbatim, but ED-15a does not
        // make it an emoji ZWJ or tag base. Presentation eligibility is
        // structural, independent of the finite variation inventory.
        joins = variation == '\u{fe0f}';
        scalars.next();
    }
    if matches!(first, '#' | '*' | '0'..='9')
        && let Some((at, '\u{20e3}')) = scalars.clone().next()
    {
        end = at + '\u{20e3}'.len_utf8();
        return emoji_status(&text[..end]).map(|_| (end, true));
    }
    if let Some((at, modifier @ '\u{1f3fb}'..='\u{1f3ff}')) = scalars.next() {
        let candidate = at + modifier.len_utf8();
        if emoji_status(&text[..candidate]).is_some() {
            end = candidate;
        }
    }
    let mut tags = text[end..].char_indices();
    if joins
        && matches!(tags.clone().next(), Some((_, '\u{e0020}'..='\u{e007e}')))
        && let Some((at, '\u{e007f}')) =
            tags.find(|&(_, c)| !matches!(c, '\u{e0020}'..='\u{e007e}'))
    {
        end += at + '\u{e007f}'.len_utf8();
    }
    Some((end, joins))
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
