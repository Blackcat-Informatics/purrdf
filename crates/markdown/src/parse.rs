// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The structural pass: lines to sections, units, and concordance rows,
//! then the oversize split. Pure over the text and the profile.

use crate::Profile;

/// The byte order mark, `EF BB BF`: a statement about the encoding when
/// it opens a document, ordinary content anywhere else.
const BYTE_ORDER_MARK: char = '\u{feff}';

/// A heading or movement section.
#[derive(Clone, Debug)]
pub(crate) struct Section {
    /// First byte of the heading line.
    pub(crate) start: usize,
    /// One past the last byte of the section's content.
    pub(crate) end: usize,
    /// One past the last byte of the heading line (no newline).
    pub(crate) line_end: usize,
    pub(crate) level: u32,
    pub(crate) ordinal: u64,
    pub(crate) parent: Option<usize>,
    pub(crate) heading: String,
    pub(crate) movement: bool,
}

/// A verse, a paragraph, or one piece of an oversize unit.
#[derive(Clone, Debug)]
pub(crate) struct Unit {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) section: Option<usize>,
    pub(crate) verse: Option<u64>,
    pub(crate) lineage: Vec<String>,
    /// The index of the previous piece, for a split unit.
    pub(crate) continues: Option<usize>,
    pub(crate) ordinal: u64,
}

/// One concordance row: the verses it covers, the canon sources, the
/// anchors.
#[derive(Clone, Debug)]
pub(crate) struct Citation {
    pub(crate) first: u64,
    pub(crate) last: u64,
    pub(crate) sources: Vec<String>,
    pub(crate) anchors: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Structure {
    pub(crate) title: Option<String>,
    pub(crate) sections: Vec<Section>,
    pub(crate) units: Vec<Unit>,
    pub(crate) citations: Vec<Citation>,
}

/// A unit before the oversize split.
#[derive(Clone, Debug)]
struct RawUnit {
    start: usize,
    end: usize,
    section: Option<usize>,
    verse: Option<u64>,
    lineage: Vec<String>,
}

#[derive(Default)]
struct Walker {
    sections: Vec<Section>,
    /// Indices into `sections`: the headings in force.
    stack: Vec<usize>,
    raw: Vec<RawUnit>,
    open: Option<RawUnit>,
    citations: Vec<Citation>,
    title: Option<String>,
}

pub(crate) fn parse(text: &str, profile: &Profile) -> Structure {
    let mut w = Walker::default();
    for (start, end) in lines(text) {
        w.line(text, start, end);
    }
    w.close_unit();
    let mut sections = w.sections;
    close_sections(&mut sections, text.len());
    let units = split_all(text, &w.raw, profile);
    Structure {
        title: w.title,
        sections,
        units,
        citations: w.citations,
    }
}

/// Byte ranges of each line, excluding the terminating newline.
///
/// A byte order mark at the very start of the document is not part of
/// the first line: the mark is a statement about the encoding, so a
/// heading or a verse that opens the document is read as if it began
/// the line, and a document that carries one slices into the same
/// structure as the one that does not. It is skipped for structure
/// only. Every span still counts the document's own bytes, so the mark
/// falls before the first span and no unit's literal is ever anything
/// but the verbatim bytes of its span. At any other offset the mark is
/// ordinary content and stays inside the unit that holds it.
fn lines(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = if text.starts_with(BYTE_ORDER_MARK) {
        BYTE_ORDER_MARK.len_utf8()
    } else {
        0
    };
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            out.push((start, i));
            start = i + 1;
        }
    }
    if start < text.len() {
        out.push((start, text.len()));
    }
    out
}

impl Walker {
    /// One line, read for what it opens or continues.
    ///
    /// Recognition never looks at a line's trailing whitespace: a
    /// heading's title, a movement's name, a rule, and a blank line are
    /// all read from the trimmed text, and a verse number is read from
    /// the line's opening digits. A document written with CRLF endings
    /// therefore slices into the structure its LF twin slices into —
    /// the `\r` a line ends with is never part of what is recognized.
    ///
    /// It is trimmed for recognition only. Every span still counts the
    /// document's own bytes, so a `\r` inside a unit's span stays in
    /// that unit's verbatim literal, where a reader who returns to the
    /// bytes at the span will find it.
    fn line(&mut self, text: &str, start: usize, end: usize) {
        let line = &text[start..end];
        if let Some((level, heading)) = atx_heading(line) {
            self.close_unit();
            self.open_section(start, end, level, heading, false);
        } else if let Some(name) = movement(line) {
            self.close_unit();
            // A movement sits one level under the nearest heading, so
            // consecutive movements are siblings, never nested.
            let level = self
                .stack
                .iter()
                .rev()
                .find(|&&i| !self.sections[i].movement)
                .map_or(1, |&i| self.sections[i].level + 1);
            self.open_section(start, end, level, name, true);
        } else if line.trim().is_empty() || is_rule(line) {
            self.close_unit();
        } else if line.trim_start().starts_with('|') {
            self.close_unit();
            self.table_row(line);
        } else if let Some(number) = verse_number(line) {
            self.close_unit();
            self.open = Some(self.raw_unit(start, end, Some(number)));
        } else if let Some(open) = self.open.as_mut() {
            open.end = end;
        } else {
            self.open = Some(self.raw_unit(start, end, None));
        }
    }

    fn raw_unit(&self, start: usize, end: usize, verse: Option<u64>) -> RawUnit {
        RawUnit {
            start,
            end,
            section: self.stack.last().copied(),
            verse,
            lineage: self
                .stack
                .iter()
                .map(|&i| self.sections[i].heading.clone())
                .collect(),
        }
    }

    fn close_unit(&mut self) {
        if let Some(open) = self.open.take() {
            self.raw.push(open);
        }
    }

    fn open_section(
        &mut self,
        start: usize,
        end: usize,
        level: u32,
        heading: String,
        movement: bool,
    ) {
        while self
            .stack
            .last()
            .is_some_and(|&i| self.sections[i].level >= level)
        {
            self.stack.pop();
        }
        if self.title.is_none() && !movement {
            self.title = Some(heading.clone());
        }
        let index = self.sections.len();
        self.sections.push(Section {
            start,
            end,
            line_end: end,
            level,
            ordinal: index as u64,
            parent: self.stack.last().copied(),
            heading,
            movement,
        });
        self.stack.push(index);
    }

    fn in_concordance(&self) -> bool {
        self.stack
            .last()
            .is_some_and(|&i| self.sections[i].heading.eq_ignore_ascii_case("Concordance"))
    }

    fn table_row(&mut self, line: &str) {
        if !self.in_concordance() {
            return;
        }
        let cells = cells(line);
        if cells.len() < 3 {
            return;
        }
        let Some((first, last)) = verse_range(&cells[0]) else {
            return;
        };
        self.citations.push(Citation {
            first,
            last,
            sources: backticked(&cells[1]),
            anchors: backticked(&cells[2]),
        });
    }
}

/// `# Heading` through `###### Heading`.
fn atx_heading(line: &str) -> Option<(u32, String)> {
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &line[hashes..];
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let heading = rest.trim().trim_end_matches('#').trim();
    Some((u32::try_from(hashes).ok()?, heading.to_owned()))
}

/// `⁂ *name*`: a movement marker, a section below the nearest heading.
fn movement(line: &str) -> Option<String> {
    let rest = line.strip_prefix('\u{2042}')?;
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let name = rest.trim();
    let name = name
        .strip_prefix('*')
        .and_then(|n| n.strip_suffix('*'))
        .or_else(|| name.strip_prefix('_').and_then(|n| n.strip_suffix('_')))
        .unwrap_or(name);
    Some(name.trim().to_owned())
}

fn is_rule(line: &str) -> bool {
    let t = line.trim();
    t.len() >= 3 && (t.bytes().all(|b| b == b'-') || t.bytes().all(|b| b == b'*'))
}

/// `12. text`: a numbered verse line.
///
/// A verse number is a `u64`, and that is the dialect's bound, not an
/// accident of the parse. A run of digits that overflows it is not a
/// verse number at all: the line is ordinary prose and becomes a
/// paragraph, carrying no verse. Nothing is truncated and nothing
/// wraps, so a number a consumer reads back is the number the document
/// wrote.
fn verse_number(line: &str) -> Option<u64> {
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let rest = &line[digits..];
    if !rest.starts_with(". ") && !rest.starts_with(".\t") {
        return None;
    }
    line[..digits].parse().ok()
}

/// The cells of a `| a | b | c |` row, trimmed.
fn cells(line: &str) -> Vec<String> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    t.split('|').map(|c| c.trim().to_owned()).collect()
}

/// `2–5` (en dash), `2-5`, or `4`.
fn verse_range(cell: &str) -> Option<(u64, u64)> {
    let (a, b) = cell
        .split_once('\u{2013}')
        .or_else(|| cell.split_once('-'))
        .unwrap_or((cell, cell));
    let first: u64 = a.trim().parse().ok()?;
    let last: u64 = b.trim().parse().ok()?;
    (first <= last).then_some((first, last))
}

/// Every backticked name in a cell, in order; prose is dropped.
fn backticked(cell: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = cell;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else { break };
        let name = after[..close].trim();
        if !name.is_empty() {
            out.push(name.to_owned());
        }
        rest = &after[close + 1..];
    }
    out
}

/// A section ends where the next section at its level or above begins.
fn close_sections(sections: &mut [Section], len: usize) {
    for i in 0..sections.len() {
        let level = sections[i].level;
        let end = sections[i + 1..]
            .iter()
            .find(|s| s.level <= level)
            .map_or(len, |s| s.start);
        sections[i].end = end;
    }
}

fn split_all(text: &str, raw: &[RawUnit], profile: &Profile) -> Vec<Unit> {
    let mut units = Vec::new();
    for r in raw {
        let mut previous = None;
        for (start, end) in split_spans(text, r.start, r.end, profile) {
            let index = units.len();
            units.push(Unit {
                start,
                end,
                section: r.section,
                verse: r.verse,
                lineage: r.lineage.clone(),
                continues: previous,
                ordinal: index as u64,
            });
            previous = Some(index);
        }
    }
    units
}

/// The split law: pieces of at most `max_bytes`, cut at the last
/// newline at or before the bound, else at the last scalar boundary; a
/// continuation reaches back `overlap` bytes, snapped backward to a
/// newline, never before the unit's start.
///
/// The bound holds with no exception, and the seam is what makes it
/// hold. [`slice_markdown`](crate::slice_markdown) refuses a
/// `max_bytes` under [`MIN_MAX_BYTES`](crate::MIN_MAX_BYTES) before a
/// byte of the document is read, so a unit only ever arrives here under
/// a bound of four or more. The widest UTF-8 scalar is four bytes, so
/// the scalar opening a piece ends at or before `ps + 4`, which is at
/// or before `ps + max_bytes` — the bound itself. The fallback's
/// `ceil_boundary(ps + 1)` therefore cannot climb past the bound, the
/// newline branch cuts at an offset the bound already covers, and every
/// cut is at or before the bound. The same fact is what puts `bytes[cut]`
/// in range: a piece is only cut when the unit runs past the bound, so
/// the cut is under the unit's end and under the text's length.
fn split_spans(text: &str, start: usize, end: usize, profile: &Profile) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut pieces = Vec::new();
    let mut ps = start;
    while ps < end {
        if end - ps <= profile.max_bytes {
            pieces.push((ps, end));
            break;
        }
        let bound = ps + profile.max_bytes;
        let cut = match bytes[ps..=bound].iter().rposition(|b| *b == b'\n') {
            Some(i) if i > 0 => ps + i,
            _ => floor_boundary(text, bound).max(ceil_boundary(text, ps + 1)),
        };
        debug_assert!(
            cut <= bound,
            "a cut is never past the bound: the widest scalar is four bytes and the bound is at least four"
        );
        pieces.push((ps, cut));
        let resume = if bytes[cut] == b'\n' { cut + 1 } else { cut };
        ps = overlap_start(text, ps, cut, profile.overlap).unwrap_or(resume);
    }
    pieces
}

/// The continuation start: `cut - overlap` snapped backward to the
/// byte after a newline (else a scalar boundary), if that is still
/// inside the piece; otherwise none, and the caller resumes at the cut.
fn overlap_start(text: &str, ps: usize, cut: usize, overlap: usize) -> Option<usize> {
    if overlap == 0 {
        return None;
    }
    let candidate = cut.checked_sub(overlap)?;
    if candidate <= ps {
        return None;
    }
    let snapped = text.as_bytes()[ps..candidate]
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or_else(|| floor_boundary(text, candidate), |i| ps + i + 1);
    (snapped > ps).then_some(snapped)
}

fn floor_boundary(text: &str, mut i: usize) -> usize {
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn ceil_boundary(text: &str, mut i: usize) -> usize {
    while i < text.len() && !text.is_char_boundary(i) {
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_heading_a_movement_and_a_verse_are_recognized() {
        assert_eq!(
            atx_heading("## Concordance"),
            Some((2, "Concordance".to_owned()))
        );
        assert_eq!(atx_heading("#not"), None);
        assert_eq!(
            movement("\u{2042} *the foundation*"),
            Some("the foundation".to_owned())
        );
        assert_eq!(verse_number("12. Hear the first thing"), Some(12));
        assert_eq!(verse_number("12.Hear"), None);
        assert!(is_rule("---"));
    }

    #[test]
    fn a_concordance_cell_yields_its_range_and_its_backticked_names() {
        assert_eq!(verse_range("2\u{2013}5"), Some((2, 5)));
        assert_eq!(verse_range("4"), Some((4, 4)));
        assert_eq!(verse_range("Verses"), None);
        assert_eq!(
            backticked("`a`, `b` (prose); `c`"),
            vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
        );
        assert_eq!(cells("| 2–5 | `x` | `y` |").len(), 3);
    }

    #[test]
    fn the_split_never_cuts_inside_a_scalar_and_always_progresses() {
        let text = "\u{2042}".repeat(10);
        let mut profile = Profile::new(
            "test",
            1,
            crate::Vocabulary::under("urn:test:").expect("a vocabulary"),
        );
        profile.max_bytes = 4;
        profile.overlap = 0;
        let pieces = split_spans(&text, 0, text.len(), &profile);
        assert!(pieces.iter().all(|&(s, e)| text.is_char_boundary(s)
            && text.is_char_boundary(e)
            && e > s
            && e - s <= 4));
        assert_eq!(pieces.last(), Some(&(27, 30)));
    }
}
