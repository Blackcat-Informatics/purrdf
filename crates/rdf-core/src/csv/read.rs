// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reading: the W3C CSVW §8 parsing algorithm over bytes.
//!
//! [`Rows`] is the one engine. It reads a row ("read a row" and "read a quoted
//! value"), recognises a comment row by its content, parses a row into cells
//! ("parse a row"), checks the record length, checks each cell is UTF-8, trims
//! it ("conditionally trim a cell value") and drops skipped columns, and it
//! walks the table algorithm's three phases — skipped rows, header rows, data
//! rows — counting source row numbers as it goes. [`read_table`] collects its
//! whole output; [`Reader`] streams its data records.

use std::ops::Range;

use super::error::{CsvError, CsvErrorKind, CsvPosition};
use super::scan::StopSet;
use super::{Dialect, Encoding, Trim};
use crate::terminals::is_unicode_white_space;

/// The UTF-8 byte order mark, removed from the start of the input as the W3C
/// *Encoding* standard's "UTF-8 decode" removes it.
const BYTE_ORDER_MARK: &[u8] = b"\xEF\xBB\xBF";

/// One record's fields: text, with the end offset of each field.
///
/// Compares with `==` field by field, prints as its list of fields, and
/// iterates as `&str`.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct StringRecord {
    text: String,
    ends: Vec<usize>,
}

impl StringRecord {
    /// A record with no fields.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            text: String::new(),
            ends: Vec::new(),
        }
    }

    fn with_capacity(bytes: usize, fields: usize) -> Self {
        Self {
            text: String::with_capacity(bytes),
            ends: Vec::with_capacity(fields),
        }
    }

    /// The number of fields.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ends.len()
    }

    /// Whether the record has no fields.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ends.is_empty()
    }

    /// Field `index`, or `None` past the last field.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&str> {
        let end = *self.ends.get(index)?;
        let start = index
            .checked_sub(1)
            .map_or(0, |previous| self.ends[previous]);
        Some(&self.text[start..end])
    }

    /// The fields, in order.
    #[must_use]
    pub fn iter(&self) -> StringRecordIter<'_> {
        StringRecordIter {
            record: self,
            front: 0,
            back: self.len(),
        }
    }

    /// Append a field.
    pub fn push_field(&mut self, field: &str) {
        self.text.push_str(field);
        self.ends.push(self.text.len());
    }
}

impl std::fmt::Debug for StringRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("StringRecord")
            .field(&self.iter().collect::<Vec<_>>())
            .finish()
    }
}

impl<S: AsRef<str>> FromIterator<S> for StringRecord {
    fn from_iter<I: IntoIterator<Item = S>>(fields: I) -> Self {
        let mut record = Self::new();
        for field in fields {
            record.push_field(field.as_ref());
        }
        record
    }
}

impl<'r> IntoIterator for &'r StringRecord {
    type Item = &'r str;
    type IntoIter = StringRecordIter<'r>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for StringRecord {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
            .into_iter()
    }
}

/// The fields of a [`StringRecord`], in order.
#[derive(Clone, Debug)]
pub struct StringRecordIter<'r> {
    record: &'r StringRecord,
    front: usize,
    back: usize,
}

impl<'r> Iterator for StringRecordIter<'r> {
    type Item = &'r str;

    fn next(&mut self) -> Option<&'r str> {
        if self.front == self.back {
            return None;
        }
        let field = self.record.get(self.front);
        self.front += 1;
        field
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.back - self.front;
        (left, Some(left))
    }
}

impl DoubleEndedIterator for StringRecordIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            return None;
        }
        self.back -= 1;
        self.record.get(self.back)
    }
}

impl ExactSizeIterator for StringRecordIter<'_> {}

impl std::iter::FusedIterator for StringRecordIter<'_> {}

/// A table read by [`read_table`]: the CSVW §8 table algorithm's output.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    /// The comment rows (prefix removed) and the non-empty skipped rows, in
    /// source order — the embedded metadata's `rdfs:comment`.
    pub comments: Vec<String>,
    /// The header rows, skipped columns removed.
    pub header_rows: Vec<TableRow>,
    /// The data rows, skipped columns removed, blank rows dropped when the
    /// dialect skips them.
    pub rows: Vec<TableRow>,
}

/// One row of a [`Table`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TableRow {
    /// The row's 1-based source row number: every row read counts, skipped,
    /// comment, header and blank rows included.
    pub number: usize,
    /// The cell values.
    pub cells: Vec<String>,
}

impl TableRow {
    fn new(number: u64, record: &StringRecord) -> Self {
        Self {
            number: usize::try_from(number).unwrap_or(usize::MAX),
            cells: record.iter().map(str::to_owned).collect(),
        }
    }
}

/// Read a whole table by the CSVW §8 algorithm.
///
/// ```
/// use purrdf_core::csv::{Dialect, read_table};
///
/// let table = read_table(&Dialect::CSVW_CELLS, b"# made by hand\nname,age\nAda,36\n")
///     .expect("well-formed");
/// assert_eq!(table.comments, [" made by hand"]);
/// assert_eq!(table.header_rows[0].cells, ["name", "age"]);
/// assert_eq!((table.rows[0].number, &table.rows[0].cells[..]), (3, &["Ada".to_owned(), "36".to_owned()][..]));
/// ```
pub fn read_table(dialect: &Dialect<'_>, bytes: &[u8]) -> Result<Table, CsvError> {
    let mut rows = Rows::new(dialect, bytes);
    let mut table = Table::default();
    while let Some((number, event)) = rows.next_event()? {
        match event {
            Event::Comment(text) => table.comments.push(text),
            Event::Header(record) => table.header_rows.push(TableRow::new(number, &record)),
            Event::Data(record) => table.rows.push(TableRow::new(number, &record)),
        }
    }
    Ok(table)
}

/// A streaming reader of one table's records.
///
/// Skipped rows and comment rows are consumed and dropped; header rows are
/// consumed, and the first is [`headers`](Self::headers); [`records`](Self::records)
/// yields the data rows. After an error the reader yields nothing more.
#[derive(Debug)]
pub struct Reader<'a> {
    rows: Rows<'a>,
    header: Option<StringRecord>,
    preamble_read: bool,
    error: Option<CsvError>,
    done: bool,
}

impl<'a> Reader<'a> {
    /// A reader of `bytes` under `dialect`. A dialect the reader refuses is
    /// reported by the first read.
    #[must_use]
    pub fn new(dialect: Dialect<'_>, bytes: &'a [u8]) -> Self {
        Self {
            rows: Rows::new(&dialect, bytes),
            header: None,
            preamble_read: false,
            error: None,
            done: false,
        }
    }

    /// The first header row, reading the skipped and header rows if they have
    /// not been read yet. Empty when the dialect has no header rows or the
    /// input ends first.
    pub fn headers(&mut self) -> Result<&StringRecord, CsvError> {
        self.read_preamble()?;
        Ok(self.header.get_or_insert_with(StringRecord::new))
    }

    /// The data records, in order.
    pub fn records(&mut self) -> Records<'_, 'a> {
        Records { reader: self }
    }

    fn read_preamble(&mut self) -> Result<(), CsvError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        while !self.preamble_read {
            if !self.rows.in_preamble() {
                self.preamble_read = true;
                break;
            }
            match self.rows.next_event() {
                Ok(Some((_, Event::Header(record)))) => {
                    if self.header.is_none() {
                        self.header = Some(record);
                    }
                }
                Ok(Some(_)) => {}
                Ok(None) => self.preamble_read = true,
                Err(error) => {
                    self.error = Some(error.clone());
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    fn next_record(&mut self) -> Option<Result<StringRecord, CsvError>> {
        if self.done {
            return None;
        }
        if let Err(error) = self.read_preamble() {
            self.done = true;
            return Some(Err(error));
        }
        loop {
            match self.rows.next_event() {
                Ok(Some((_, Event::Data(record)))) => return Some(Ok(record)),
                Ok(Some(_)) => {}
                Ok(None) => {
                    self.done = true;
                    return None;
                }
                Err(error) => {
                    self.done = true;
                    self.error = Some(error.clone());
                    return Some(Err(error));
                }
            }
        }
    }
}

/// The data records of a [`Reader`].
#[derive(Debug)]
pub struct Records<'r, 'a> {
    reader: &'r mut Reader<'a>,
}

impl Iterator for Records<'_, '_> {
    type Item = Result<StringRecord, CsvError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.reader.next_record()
    }
}

impl std::iter::FusedIterator for Records<'_, '_> {}

/// What reading one row produced.
enum Event {
    /// A comment row's text after its prefix, or a skipped row's content.
    Comment(String),
    /// A header row.
    Header(StringRecord),
    /// A data row.
    Data(StringRecord),
}

/// Where a cell parse stands (CSVW §8 "parse a row").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CellState {
    /// The cell is empty and unquoted: a quote character opens it.
    Start,
    /// The cell holds unquoted text.
    Unquoted,
    /// Inside a quoted value: the delimiter is text.
    Quoted,
    /// After a closing quote: only the delimiter (or the row's end) may follow.
    AfterQuote,
}

/// The parsed cells of the current row.
#[derive(Debug, Default)]
struct Cells {
    text: String,
    /// Each cell's byte range in `text`.
    ranges: Vec<Range<usize>>,
    /// Where the current cell starts in `text`.
    open: usize,
}

impl Cells {
    fn clear(&mut self) {
        self.text.clear();
        self.ranges.clear();
        self.open = 0;
    }

    fn finish(&mut self) {
        self.ranges.push(self.open..self.text.len());
        self.open = self.text.len();
    }
}

/// The CSVW §8 engine over one input.
#[derive(Debug)]
struct Rows<'a> {
    input: &'a [u8],
    /// The longest UTF-8 prefix of `input`: every row that ends inside it is
    /// parsed from it, and the row holding the first byte past it is refused.
    text: &'a str,
    /// The next unread byte.
    pos: usize,
    delimiter: u8,
    quote: Option<u8>,
    /// Whether a doubled quote character stands for one.
    doubles: bool,
    /// The escape character distinct from the quote character.
    escape: Option<u8>,
    terminators: Vec<Box<[u8]>>,
    comment_prefix: Option<Box<[u8]>>,
    /// The effective trim: `trim`, with `start` added by `skip_initial_space`.
    trim: Trim,
    skip_columns: usize,
    skip_blank: bool,
    flexible: bool,
    /// Skipped rows still to read.
    skips_left: usize,
    /// Header rows still to read.
    headers_left: usize,
    /// Stops outside a quoted value while splitting rows.
    row_stops: StopSet,
    /// Stops inside a quoted value (splitting rows and parsing cells).
    quoted_stops: StopSet,
    /// Stops outside a quoted value while parsing cells.
    cell_stops: StopSet,
    /// The source row number of the last row read.
    last_row: u64,
    /// The first parsed record's field count.
    expected: Option<usize>,
    cells: Cells,
    /// A refused dialect, reported by every read.
    refused: Option<CsvError>,
}

impl<'a> Rows<'a> {
    fn new(dialect: &Dialect<'_>, input: &'a [u8]) -> Self {
        let refused = dialect
            .check()
            .err()
            .map(|fault| CsvError::new(fault.kind()));
        // UTF-8 is the only encoding: "UTF-8 decode" removes a leading BOM.
        let Encoding::Utf8 = dialect.encoding;
        let pos = if input.starts_with(BYTE_ORDER_MARK) {
            BYTE_ORDER_MARK.len()
        } else {
            0
        };
        // One validation of the whole input; the valid prefix is the text
        // every row is parsed from.
        let text = match std::str::from_utf8(input) {
            Ok(text) => text,
            Err(error) => std::str::from_utf8(&input[..error.valid_up_to()]).unwrap_or_default(),
        };
        let quote = dialect.quote_char;
        let escape = dialect.distinct_escape();
        let terminators: Vec<Box<[u8]>> = dialect
            .line_terminators
            .strings()
            .iter()
            .map(|terminator| Box::from(terminator.as_bytes()))
            .collect();
        let mut row_stop_bytes: Vec<u8> = quote.into_iter().chain(escape).collect();
        row_stop_bytes.extend(terminators.iter().filter_map(|t| t.first().copied()));
        let quoted_stop_bytes: Vec<u8> = quote.into_iter().chain(escape).collect();
        let cell_stop_bytes: Vec<u8> = std::iter::once(dialect.delimiter)
            .chain(quote)
            .chain(escape)
            .collect();
        Self {
            input,
            text,
            pos,
            delimiter: dialect.delimiter,
            quote,
            doubles: dialect.doubles_quote(),
            escape,
            terminators,
            comment_prefix: dialect.comment_prefix.map(|p| Box::from(p.as_bytes())),
            trim: match (dialect.trim, dialect.skip_initial_space) {
                (Trim::None, true) => Trim::Start,
                (Trim::End, true) => Trim::Both,
                (trim, _) => trim,
            },
            skip_columns: dialect.skip_columns,
            skip_blank: dialect.skip_blank_rows,
            flexible: dialect.flexible,
            skips_left: dialect.skip_rows,
            headers_left: dialect.header_row_count,
            row_stops: StopSet::new(&row_stop_bytes),
            quoted_stops: StopSet::new(&quoted_stop_bytes),
            cell_stops: StopSet::new(&cell_stop_bytes),
            last_row: 0,
            expected: None,
            cells: Cells::default(),
            refused,
        }
    }

    /// Whether skipped or header rows remain to be read.
    const fn in_preamble(&self) -> bool {
        self.skips_left > 0 || self.headers_left > 0
    }

    fn error(&self, kind: CsvErrorKind, row: u64, byte: usize) -> CsvError {
        match CsvPosition::locate(self.input, row, byte) {
            Ok(position) => CsvError::at(kind, position),
            Err(limit) => CsvError::new(limit),
        }
    }

    /// The next row's event and source row number, or `None` at the end.
    fn next_event(&mut self) -> Result<Option<(u64, Event)>, CsvError> {
        if let Some(refused) = &self.refused {
            return Err(refused.clone());
        }
        loop {
            if self.pos >= self.input.len() {
                return Ok(None);
            }
            let Some(number) = self.last_row.checked_add(1) else {
                let refused = CsvError::new(CsvErrorKind::SourceRowExhausted);
                self.refused = Some(refused.clone());
                return Err(refused);
            };
            let Some(content) = self.read_row()? else {
                return Ok(None);
            };
            self.last_row = number;
            let comment = self
                .comment_prefix
                .as_deref()
                .filter(|prefix| self.input[content.clone()].starts_with(prefix))
                .map(<[u8]>::len);
            if self.pos > self.text.len() {
                // §8 decodes the whole input before it reads a row, so the
                // row holding the first byte that is not UTF-8 is refused
                // before it is parsed.
                let (field, valid_up_to) = if comment.is_some() || self.skips_left > 0 {
                    (0, self.text.len() - content.start)
                } else {
                    self.locate_invalid(content.start)
                };
                return Err(self.error(
                    CsvErrorKind::NotUtf8 { field, valid_up_to },
                    number,
                    self.text.len(),
                ));
            }
            if self.skips_left > 0 {
                self.skips_left -= 1;
                // §8: a skipped row is a comment when it carries the prefix,
                // and its whole content is one otherwise, unless it is empty.
                let text = &self.text[content.start + comment.unwrap_or(0)..content.end];
                if comment.is_some() || !text.is_empty() {
                    return Ok(Some((number, Event::Comment(text.to_owned()))));
                }
                continue;
            }
            // A comment row, and under "skip blank rows" a blank row, is not
            // a header row: §8 defines the header row count as "The number of
            // header rows (following the skipped rows)", the comment prefix as
            // marking "that the row is a comment", and skip blank rows as
            // "whether to ignore wholly empty rows". (The algorithm's header
            // loop, read literally, would spend a header slot on a comment
            // row; the flag definitions are followed instead.)
            if let Some(prefix_len) = comment {
                let text = &self.text[content.start + prefix_len..content.end];
                return Ok(Some((number, Event::Comment(text.to_owned()))));
            }
            self.parse_row(content.clone(), number)?;
            if self.skip_blank && self.is_blank() {
                continue;
            }
            let header = self.headers_left > 0;
            let found = self.cells.ranges.len();
            match self.expected {
                None => self.expected = Some(found),
                Some(expected) if expected != found && !self.flexible => {
                    return Err(self.error(
                        CsvErrorKind::UnequalLengths { expected, found },
                        number,
                        content.start,
                    ));
                }
                Some(_) => {}
            }
            let record = self.record();
            if header {
                self.headers_left -= 1;
                return Ok(Some((number, Event::Header(record))));
            }
            return Ok(Some((number, Event::Data(record))));
        }
    }

    /// CSVW §8 "read a row": the byte range of the next row's content, the
    /// cursor moved past its terminator. `None` when no row remains.
    fn read_row(&mut self) -> Result<Option<Range<usize>>, CsvError> {
        let input = self.input;
        let end = input.len();
        if self.pos >= end {
            return Ok(None);
        }
        let start = self.pos;
        let mut at = start;
        loop {
            let Some(offset) = self.row_stops.find(&input[at..]) else {
                self.pos = end;
                return Ok(Some(start..end));
            };
            let hit = at + offset;
            let byte = input[hit];
            if Some(byte) == self.escape {
                // The escape character and the character after it are row
                // content; neither can end the row or open a quoted value.
                at = (hit + 2).min(end);
                continue;
            }
            if Some(byte) == self.quote {
                if self.doubles && input.get(hit + 1) == Some(&byte) {
                    // "the escape character followed by the quote character".
                    at = hit + 2;
                } else {
                    at = self.skip_quoted_value(hit)?;
                }
                continue;
            }
            if let Some(terminator) = self
                .terminators
                .iter()
                .find(|terminator| input[hit..].starts_with(terminator))
            {
                self.pos = hit + terminator.len();
                return Ok(Some(start..hit));
            }
            at = hit + 1;
        }
    }

    /// CSVW §8 "read a quoted value" from the opening quote at `open`: the
    /// offset just past its closing quote.
    fn skip_quoted_value(&self, open: usize) -> Result<usize, CsvError> {
        let input = self.input;
        let mut at = open + 1;
        loop {
            let Some(offset) = self.quoted_stops.find(&input[at..]) else {
                // The unclosed value runs to the end of the input, so a byte
                // that is not UTF-8 inside it is the first thing wrong.
                if self.text.len() < input.len() {
                    let (field, valid_up_to) = self.locate_invalid(self.pos);
                    return Err(self.error(
                        CsvErrorKind::NotUtf8 { field, valid_up_to },
                        self.last_row + 1,
                        self.text.len(),
                    ));
                }
                return Err(self.error(CsvErrorKind::UnterminatedQuote, self.last_row + 1, open));
            };
            let hit = at + offset;
            let byte = input[hit];
            if Some(byte) == self.escape {
                at = (hit + 2).min(input.len());
            } else if self.doubles && input.get(hit + 1) == Some(&byte) {
                at = hit + 2;
            } else {
                return Ok(hit + 1);
            }
        }
    }

    /// CSVW §8 "parse a row" over the row content `row`, into `self.cells`.
    /// Every byte of `row` is inside the valid UTF-8 prefix.
    fn parse_row(&mut self, row: Range<usize>, number: u64) -> Result<(), CsvError> {
        let text = self.text;
        let bytes = text.as_bytes();
        let end = row.end;
        let mut at = row.start;
        let mut state = CellState::Start;
        self.cells.clear();
        loop {
            let stops = if state == CellState::Quoted {
                &self.quoted_stops
            } else {
                &self.cell_stops
            };
            let hit = stops.find(&bytes[at..end]).map(|offset| at + offset);
            let run_end = hit.unwrap_or(end);
            if run_end > at {
                match state {
                    CellState::AfterQuote => {
                        return Err(self.error(CsvErrorKind::TextAfterClosingQuote, number, at));
                    }
                    CellState::Start => state = CellState::Unquoted,
                    CellState::Unquoted | CellState::Quoted => {}
                }
                // Every stop byte is ASCII, so `at` and `run_end` are
                // character boundaries.
                self.cells.text.push_str(&text[at..run_end]);
            }
            let Some(hit) = hit else {
                break;
            };
            let byte = bytes[hit];
            if Some(byte) == self.escape {
                match state {
                    CellState::AfterQuote => {
                        return Err(self.error(CsvErrorKind::TextAfterClosingQuote, number, hit));
                    }
                    CellState::Start => state = CellState::Unquoted,
                    CellState::Unquoted | CellState::Quoted => {}
                }
                // "append the character following the escape character"; with
                // nothing following, the escape character is kept rather than
                // silently dropped.
                if let Some(escaped) = text[hit + 1..end].chars().next() {
                    self.cells.text.push(escaped);
                    at = hit + 1 + escaped.len_utf8();
                } else {
                    self.cells.text.push(char::from(byte));
                    at = hit + 1;
                }
                continue;
            }
            if Some(byte) == self.quote {
                let doubled = self.doubles && hit + 1 < end && bytes[hit + 1] == byte;
                match state {
                    CellState::Start => {
                        state = CellState::Quoted;
                        at = hit + 1;
                    }
                    CellState::Unquoted | CellState::Quoted if doubled => {
                        self.cells.text.push(char::from(byte));
                        at = hit + 2;
                    }
                    CellState::Unquoted => {
                        return Err(self.error(CsvErrorKind::QuoteInUnquotedField, number, hit));
                    }
                    CellState::Quoted => {
                        state = CellState::AfterQuote;
                        at = hit + 1;
                    }
                    CellState::AfterQuote => {
                        return Err(self.error(CsvErrorKind::TextAfterClosingQuote, number, hit));
                    }
                }
                continue;
            }
            // The delimiter: it is only a stop outside a quoted value.
            debug_assert_eq!(byte, self.delimiter);
            self.cells.finish();
            state = CellState::Start;
            at = hit + 1;
        }
        if state == CellState::Quoted {
            return Err(self.error(CsvErrorKind::UnterminatedQuote, number, end));
        }
        self.cells.finish();
        Ok(())
    }

    /// Which cell of the row starting at `start` holds the first byte that is
    /// not UTF-8, and how many of that cell's bytes precede it — for the
    /// error only, so the row is split without refusing anything: a quote
    /// character opens a quoted value only at the start of a cell, closes it
    /// unless doubled, and is text anywhere else.
    fn locate_invalid(&self, start: usize) -> (usize, usize) {
        let bytes = &self.input[start..self.text.len()];
        let (mut field, mut length, mut quoted) = (0, 0, false);
        let mut at = 0;
        while at < bytes.len() {
            let byte = bytes[at];
            if Some(byte) == self.escape {
                at += 1;
                continue;
            }
            if Some(byte) == self.quote {
                if self.doubles && bytes.get(at + 1) == Some(&byte) && (quoted || length > 0) {
                    length += 1;
                    at += 2;
                    continue;
                }
                if quoted {
                    quoted = false;
                } else if length == 0 {
                    quoted = true;
                } else {
                    length += 1;
                }
            } else if byte == self.delimiter && !quoted {
                field += 1;
                length = 0;
            } else {
                length += 1;
            }
            at += 1;
        }
        (field, length)
    }

    /// Trim `text` as the dialect says.
    fn trim<'t>(&self, mut text: &'t str) -> &'t str {
        if self.trim.start() {
            text = text.trim_start_matches(is_unicode_white_space);
        }
        if self.trim.end() {
            text = text.trim_end_matches(is_unicode_white_space);
        }
        text
    }

    /// Whether every parsed cell is empty after trimming (CSVW "skip blank
    /// rows": "all of the values in the list of cell values are empty
    /// strings").
    fn is_blank(&self) -> bool {
        self.cells
            .ranges
            .iter()
            .all(|range| self.trim(&self.cells.text[range.clone()]).is_empty())
    }

    /// The parsed cells as a record: trimmed, and the skipped columns dropped.
    fn record(&self) -> StringRecord {
        let cells = &self.cells;
        let mut record = StringRecord::with_capacity(
            cells.text.len(),
            cells.ranges.len().saturating_sub(self.skip_columns),
        );
        for range in cells.ranges.iter().skip(self.skip_columns) {
            record.push_field(self.trim(&cells.text[range.clone()]));
        }
        record
    }
}

#[cfg(test)]
mod source_width_tests {
    use super::{CsvErrorKind, Dialect, Rows};

    #[test]
    fn source_rows_cross_u32_without_truncation() {
        let mut rows = Rows::new(&Dialect::RFC4180, b"value\n");
        rows.last_row = u64::from(u32::MAX);
        let (number, _) = rows.next_event().unwrap().unwrap();
        assert_eq!(number, u64::from(u32::MAX) + 1);
        assert!(rows.next_event().unwrap().is_none());
    }

    #[test]
    fn terminal_source_row_is_issued_once_and_then_refused_before_consumption() {
        let mut rows = Rows::new(&Dialect::RFC4180, b"value\nnext\n");
        rows.last_row = u64::MAX - 1;
        let (number, _) = rows.next_event().unwrap().unwrap();
        assert_eq!(number, u64::MAX);
        let stopped_at = rows.pos;
        for _ in 0..2 {
            assert_eq!(
                rows.next_event()
                    .err()
                    .expect("source row exhausted")
                    .kind(),
                &CsvErrorKind::SourceRowExhausted
            );
            assert_eq!(rows.pos, stopped_at);
        }
    }
}
