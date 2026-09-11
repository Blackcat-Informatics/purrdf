// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Markdown dialect reader: lines to sections, units, and
//! concordance rows.
//!
//! Everything Markdown-specific in this crate is here — ATX headings,
//! the U+2042 movement marker, `N.` verses, blank-line paragraphs,
//! horizontal rules, the `## Concordance` table and the GFM escapes its
//! cells are read through — and nothing else in the crate reads a line.
//! See [`crate::dialect`] for the contract this reader satisfies.
//!
//! Two of the concordance's rules are containment rules rather than
//! line rules, and both are places where a wrong reading loses a row in
//! silence rather than refusing it: a table row is a citation row when
//! the concordance is in force at **any** depth above it
//! ([`Walker::in_concordance`]), and a row's cells are delimited by its
//! **unescaped** pipes only ([`cells`]).

use std::sync::Arc;

use super::{DefectiveRow, RawRow, RawSection, RawUnit, Reading};
use crate::model::RowDefect;

/// The byte order mark, `EF BB BF`: a statement about the encoding when
/// it opens a document, ordinary content anywhere else.
const BYTE_ORDER_MARK: char = '\u{feff}';

/// The heading that opens a concordance table, matched without regard
/// to case.
const CONCORDANCE: &str = "Concordance";

/// The cells a concordance data row states: the verse range, the canon
/// sources, the anchors.
const CONCORDANCE_CELLS: usize = 3;

/// Reads a Markdown document into the structure the law then works on.
pub(crate) fn read(text: &str) -> Reading {
    let mut w = Walker::default();
    for (start, end) in lines(text) {
        w.line(text, start, end);
    }
    w.close_unit();
    Reading {
        title: w.title,
        sections: w.sections,
        units: w.units,
        rows: w.rows,
        defective_rows: w.defective_rows,
    }
}

#[derive(Default)]
struct Walker {
    sections: Vec<RawSection>,
    /// Indices into `sections`: the headings in force.
    stack: Vec<usize>,
    /// The headings of `stack`, outermost first: the lineage every unit
    /// opened under the current stack carries. Rebuilt where the stack
    /// changes — which is only when a section opens — so a document of
    /// many units under few headings builds it once per heading rather
    /// than once per unit.
    lineage: Arc<[String]>,
    units: Vec<RawUnit>,
    open: Option<RawUnit>,
    rows: Vec<RawRow>,
    defective_rows: Vec<DefectiveRow>,
    title: Option<String>,
    /// How many table rows have run without a break since the last
    /// line that was not one: row 0 of a table is its header.
    table_row_index: usize,
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
        if !line.trim_start().starts_with('|') {
            // A table ended, so the next one starts its rows afresh.
            self.table_row_index = 0;
        }
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
            self.table_row(line, start, end);
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
            lineage: Arc::clone(&self.lineage),
        }
    }

    fn close_unit(&mut self) {
        if let Some(open) = self.open.take() {
            self.units.push(open);
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
        self.sections.push(RawSection {
            start,
            line_end: end,
            level,
            heading,
            movement,
        });
        self.stack.push(index);
        // The stack has just changed, and it changes nowhere else, so
        // this is the whole of when the lineage has to be rebuilt.
        self.lineage = self
            .stack
            .iter()
            .map(|&i| self.sections[i].heading.clone())
            .collect();
    }

    /// Whether a concordance section is **in force**: whether *any*
    /// section still open is the concordance heading, and not merely the
    /// innermost one.
    ///
    /// The law reads a row *inside* the concordance as a citation, and
    /// inside is containment. A concordance organised into
    /// subsections — one per volume, one per surveyor — holds its rows
    /// at a depth, and every one of them is still inside the concordance
    /// section's own span. Asking only which section is innermost is the
    /// reading that loses every one of those rows, and loses them in
    /// **silence**: a row read as nothing is neither a citation nor a
    /// defect, so no surface of the model reports it and the author
    /// never learns the table did not lift.
    ///
    /// The stack holds exactly the sections in force, so the containment
    /// question is answered by looking at all of it rather than at its
    /// top. A heading at or above the concordance's own level pops it
    /// ([`Walker::open_section`]), so a table after the concordance has
    /// closed still lifts nothing — which is the other half of the law,
    /// and the one this must not trade away.
    fn in_concordance(&self) -> bool {
        self.stack
            .iter()
            .any(|&i| self.sections[i].heading.eq_ignore_ascii_case(CONCORDANCE))
    }

    /// One `| a | b | c |` line inside a concordance section, at any
    /// depth under its heading.
    ///
    /// A row that states a verse range and three cells is a citation.
    /// A row that does not is either the table's own frame — its header
    /// line, or the `|---|---|---|` that separates the header from the
    /// body — or a defect, and the two are told apart without ever
    /// touching a row that would have lifted: the frame test is asked
    /// only of a row that could not be read as data. Everything else is
    /// reported, because a row that names verses and anchors and lifts
    /// nothing is exactly the failure a silent parser hides.
    ///
    /// A subsection of the concordance starts its own table, so its
    /// header line is the row at index 0 of that table: the run of table
    /// rows is counted from the last line that was not one, and a
    /// heading is not one.
    fn table_row(&mut self, line: &str, start: usize, end: usize) {
        if !self.in_concordance() {
            return;
        }
        let index = self.table_row_index;
        self.table_row_index += 1;
        let cells = cells(line);
        if cells.len() >= CONCORDANCE_CELLS
            && let Some((first, last)) = verse_range(&cells[0])
        {
            self.rows.push(RawRow {
                first,
                last,
                sources: backticked(&cells[1]),
                anchors: backticked(&cells[2]),
                start,
                end,
            });
            return;
        }
        if index == 0 || is_delimiter_row(&cells) {
            return;
        }
        let defect = if cells.len() < CONCORDANCE_CELLS {
            RowDefect::TooFewCells { found: cells.len() }
        } else {
            RowDefect::UnreadableVerseRange
        };
        self.defective_rows
            .push(DefectiveRow { start, end, defect });
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

/// The cells of a `| a | b | c |` row: the fields its **unescaped** `|`
/// characters delimit, each unescaped and then trimmed.
///
/// # The cell-escape law
///
/// A `\` escapes the character after it, and exactly two escapes are
/// recognized:
///
/// * `\|` is a literal `|` and is no delimiter at all — not between two
///   cells, and not as the row's own leading or trailing delimiter. It
///   is GFM's spelling of a pipe inside a table cell, so a row carrying
///   one is ordinary input rather than a hostile one.
/// * `\\` is a literal `\`, which is what keeps `\\|` a delimiter
///   standing after a literal backslash rather than an escaped pipe.
///
/// A `\` before anything else is content and keeps its backslash: `\n`
/// in a cell is the two characters `\` and `n`, and a `\` at the end of
/// the line escapes nothing and is content. Nothing else about a cell is
/// interpreted — this is the table's own escape, not Markdown's inline
/// grammar, and the cell's value is what the verse range and the
/// backticked names are then read from.
///
/// Splitting on every `|` regardless is the silent failure this exists
/// to close, and it is worse than a wrong answer: such a row still
/// states three cells, so it still reads as a citation and still lifts
/// onto its verse, and what it lifts is the wreck of two cells cut in
/// the wrong places. It is neither unmatched nor malformed, so nothing
/// anywhere reports it.
fn cells(line: &str) -> Vec<String> {
    let body = line.trim();
    // The row was recognized by its opening `|`, so that one is a
    // delimiter by construction and can be no escaped pipe.
    let body = body.strip_prefix('|').unwrap_or(body);
    let mut out: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut escaped = false;
    // Whether the last character read was a delimiter: how the row's
    // optional trailing `|` is told from an empty final cell.
    let mut closed = false;
    for c in body.chars() {
        closed = false;
        match (escaped, c) {
            (true, '|' | '\\') => cell.push(c),
            (true, other) => {
                cell.push('\\');
                cell.push(other);
            }
            (false, '\\') => {
                escaped = true;
                continue;
            }
            (false, '|') => {
                out.push(std::mem::take(&mut cell).trim().to_owned());
                closed = true;
            }
            (false, other) => cell.push(other),
        }
        escaped = false;
    }
    if escaped {
        cell.push('\\');
    }
    if !closed {
        out.push(cell.trim().to_owned());
    }
    out
}

/// `|---|---|---|`, `|:--|--:|`: the line that separates a table's
/// header from its body, in any of its alignment spellings.
fn is_delimiter_row(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let body = cell.trim_start_matches(':').trim_end_matches(':');
            !body.is_empty() && body.bytes().all(|b| b == b'-')
        })
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
    fn a_row_splits_at_its_unescaped_pipes_and_keeps_the_escaped_ones_as_content() {
        // `\|` is a pipe in a cell, and never a delimiter — between
        // cells, or as the row's own leading or trailing one.
        assert_eq!(
            cells(r"| 1 | `a\|b.ttl` | `x` |"),
            vec!["1".to_owned(), "`a|b.ttl`".to_owned(), "`x`".to_owned()]
        );
        assert_eq!(cells(r"| a \| b |"), vec!["a | b".to_owned()]);
        assert_eq!(
            cells(r"| a | b\|"),
            vec!["a".to_owned(), "b|".to_owned()],
            "an escaped pipe at the line's end is content, not the closing delimiter"
        );
        // `\\` is a backslash, so the pipe after it still delimits.
        assert_eq!(
            cells(r"| a\\ | b |"),
            vec![r"a\".to_owned(), "b".to_owned()]
        );
        // Any other backslash is content, backslash and all, and one at
        // the end of the line escapes nothing.
        assert_eq!(
            cells(r"| a\nb | c\ |"),
            vec![r"a\nb".to_owned(), r"c\".to_owned()]
        );
        assert_eq!(cells(r"| a | b\"), vec!["a".to_owned(), r"b\".to_owned()]);
        // And a row with no backslash in it splits exactly as before.
        assert_eq!(
            cells("| 2\u{2013}5 | `x` | `y` |"),
            vec!["2\u{2013}5".to_owned(), "`x`".to_owned(), "`y`".to_owned()]
        );
        assert_eq!(cells("|"), vec![String::new()]);
    }

    #[test]
    fn a_delimiter_row_is_frame_in_every_alignment_spelling_and_a_verse_row_is_not() {
        for row in ["|---|---|---|", "| :--- | ---: | :---: |", "|-|-|-|"] {
            assert!(is_delimiter_row(&cells(row)), "{row}");
        }
        for row in ["| 1 | `a` | `b` |", "| Verses | Canon source | Anchors |"] {
            assert!(!is_delimiter_row(&cells(row)), "{row}");
        }
    }
}
