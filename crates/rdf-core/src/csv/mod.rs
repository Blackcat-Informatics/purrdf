// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! CSV and TSV on the W3C CSVW dialect model: one reader, one writer, every
//! dialect a value.
//!
//! # The dialect
//!
//! A [`Dialect`] is a plain struct with one public field per flag of the W3C
//! *Model for Tabular Data and Metadata on the Web* §8 ("Parsing Tabular
//! Data") — comment prefix, delimiter, encoding, escape character, header row
//! count, line terminators, quote character, skip blank rows, skip columns,
//! skip rows, trim — with the *Metadata Vocabulary for Tabular Data* §5.9
//! property names (`doubleQuote`, `skipInitialSpace`) kept where the
//! vocabulary splits a flag in two, plus three fields the reading algorithm
//! does not name: whether records may differ in length ([`Dialect::flexible`],
//! RFC 4180 §2 rule 4), and how records are written ([`Dialect::quote_style`],
//! [`Dialect::record_terminator`]). Five presets cover the workspace's
//! formats; a caller builds any other dialect with struct-update syntax from
//! one of them.
//!
//! # Reading
//!
//! Reading follows §8 in its two phases. *Read a row* splits the input into
//! rows at a line terminator outside a quoted value; *parse a row* splits a row
//! into cells at the delimiter and removes quoting and escaping. Where the
//! algorithm says "raise an error", this reader refuses with a typed
//! [`CsvError`] that carries the source row, the line and column, and the byte
//! offset: a quote character inside an unquoted non-empty cell, text after a
//! closing quote, and — where the algorithm has no end-of-input step, and
//! RFC 4180's `escaped` production requires a closing `DQUOTE` — an unclosed
//! quoted value. [`read_table`] runs the whole §8 table algorithm (skipped
//! rows, comments, header rows, blank rows, skipped columns); [`Reader`]
//! streams the data records of the same algorithm.
//!
//! Input is UTF-8 (the only [`Encoding`]); a leading byte order mark is removed
//! as the W3C *Encoding* standard's "UTF-8 decode" does. Bytes that are not
//! UTF-8 are refused ([`CsvErrorKind::NotUtf8`]) rather than replaced.
//!
//! # Writing
//!
//! [`Writer`] streams records to an [`std::io::Write`] sink; [`FieldWriter`]
//! writes fields and records to any [`TextOut`](crate::sink::TextOut) and is
//! `const`-constructible; [`write_record_text`] appends one record to a
//! `String`. All three share one quoting rule, RFC 4180 §2 rules 6 and 7.
//!
//! # The field scanner
//!
//! Every hot loop finds the next byte of a small run-time set — the delimiter,
//! the quote and escape characters, the first bytes of the line terminators —
//! with one kernel, compiled explicitly for SSE2, AVX2 (selected at run time),
//! NEON and WebAssembly `simd128`, and portably everywhere else.

mod arch;
mod error;
mod read;
mod scan;
mod write;

pub use error::{CsvError, CsvErrorKind, CsvPosition};
pub use read::{Reader, Records, StringRecord, StringRecordIter, Table, TableRow, read_table};
pub use write::{FieldWriter, Writer, write_record_text};

/// The line terminators CSVW reads by default: CRLF, then LF (*Metadata
/// Vocabulary for Tabular Data* §5.9, `lineTerminators`).
pub const DEFAULT_LINE_TERMINATORS: &[&str] = &["\r\n", "\n"];

/// A character encoding a table may be read in.
///
/// Only UTF-8 is available; [`Encoding::from_label`] is the W3C *Encoding*
/// standard's "get an encoding" restricted to the labels of UTF-8, so every
/// other label is answered `None` and a caller reports it as unavailable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Encoding {
    /// UTF-8. A leading byte order mark is removed, as "UTF-8 decode" does.
    Utf8,
}

/// The labels the *Encoding* standard maps to UTF-8.
const UTF8_LABELS: [&str; 6] = [
    "unicode-1-1-utf-8",
    "unicode11utf8",
    "unicode20utf8",
    "utf-8",
    "utf8",
    "x-unicode20utf8",
];

impl Encoding {
    /// The encoding a CSVW `encoding` label names, or `None` when the label
    /// names an encoding this engine does not decode (or no encoding at all).
    ///
    /// Leading and trailing ASCII whitespace is removed and the label is
    /// compared ASCII case-insensitively, as "get an encoding" specifies.
    ///
    /// ```
    /// use purrdf_core::csv::Encoding;
    ///
    /// assert_eq!(Encoding::from_label("utf-8"), Some(Encoding::Utf8));
    /// assert_eq!(Encoding::from_label(" UTF8\t"), Some(Encoding::Utf8));
    /// assert_eq!(Encoding::from_label("windows-1252"), None);
    /// ```
    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        // "ASCII whitespace" (WHATWG Infra, which "get an encoding" cites):
        // U+0009 TAB, U+000A LF, U+000C FF, U+000D CR and U+0020 SPACE.
        let label = label.trim_matches(|c| matches!(c, '\t' | '\n' | '\u{C}' | '\r' | ' '));
        UTF8_LABELS
            .iter()
            .any(|known| known.eq_ignore_ascii_case(label))
            .then_some(Self::Utf8)
    }

    /// The encoding's name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
        }
    }
}

/// The strings that end a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LineTerminators<'a> {
    /// Any of these strings, tried in the order given at each position (so a
    /// longer terminator listed first wins over its own prefix listed later).
    ///
    /// An empty list means no row ends before the input does. An empty string
    /// in the list is refused with [`CsvErrorKind::EmptyLineTerminator`]: it
    /// would end a row at every position without consuming anything.
    Strings(&'a [&'a str]),
}

impl<'a> LineTerminators<'a> {
    /// The terminator strings, in order.
    #[must_use]
    pub const fn strings(self) -> &'a [&'a str] {
        match self {
            Self::Strings(strings) => strings,
        }
    }
}

/// Which whitespace is removed from each cell value (the CSVW `trim` flag).
///
/// The whitespace is Unicode `White_Space`,
/// [`is_unicode_white_space`](crate::terminals::is_unicode_white_space).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Trim {
    /// Nothing is removed (`trim: false`).
    None,
    /// Leading whitespace is removed (`trim: "start"`).
    Start,
    /// Trailing whitespace is removed (`trim: "end"`).
    End,
    /// Both (`trim: true`).
    Both,
}

impl Trim {
    pub(crate) const fn start(self) -> bool {
        matches!(self, Self::Start | Self::Both)
    }

    pub(crate) const fn end(self) -> bool {
        matches!(self, Self::End | Self::Both)
    }
}

/// When a written field is enclosed in quote characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QuoteStyle {
    /// Only when the field holds the delimiter, the quote character, CR, LF,
    /// a distinct escape character or the first byte of a line terminator
    /// (RFC 4180 §2 rule 6), when it is the first field and begins with the
    /// comment prefix, or when it is the only field of its record and is empty
    /// (so the record cannot be read back as a blank line).
    Necessary,
    /// Always.
    Always,
}

/// A CSV dialect: the CSVW parsing flags plus how records are written.
///
/// Build one from a preset with struct-update syntax:
///
/// ```
/// use purrdf_core::csv::{Dialect, Reader};
///
/// let semicolons = Dialect { delimiter: b';', ..Dialect::RFC4180 };
/// let mut reader = Reader::new(semicolons, b"a;b\n1;2\n");
/// let rows: Vec<Vec<String>> = reader
///     .records()
///     .map(|record| record.map(|r| r.iter().map(str::to_owned).collect()))
///     .collect::<Result<_, _>>()
///     .expect("well-formed");
/// assert_eq!(rows, [["a", "b"], ["1", "2"]]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "one public field per CSVW dialect flag, spelled as the metadata vocabulary spells it"
)]
pub struct Dialect<'a> {
    /// The character encoding (CSVW `encoding`).
    pub encoding: Encoding,
    /// The strings that end a row (CSVW `lineTerminators`).
    pub line_terminators: LineTerminators<'a>,
    /// A row whose content begins with this string is a comment (CSVW
    /// `commentPrefix`); `None` makes no row a comment.
    pub comment_prefix: Option<&'a str>,
    /// How many rows after the skipped rows are header rows (CSVW `header`,
    /// `headerRowCount`).
    pub header_row_count: usize,
    /// How many rows at the start are skipped (CSVW `skipRows`).
    pub skip_rows: usize,
    /// How many cells at the start of each row are skipped (CSVW
    /// `skipColumns`).
    pub skip_columns: usize,
    /// Whether a data row whose cells are all empty is dropped (CSVW
    /// `skipBlankRows`).
    pub skip_blank_rows: bool,
    /// The byte between cells (CSVW `delimiter`). Must be ASCII.
    pub delimiter: u8,
    /// The byte around a quoted cell (CSVW `quoteChar`); `None` disables
    /// quoting. Must be ASCII.
    pub quote_char: Option<u8>,
    /// Whether two quote characters inside a quoted cell stand for one (CSVW
    /// `doubleQuote`: the escape character is the quote character).
    pub double_quote: bool,
    /// An escape character distinct from the quote character (CSVW
    /// `doubleQuote: false` makes it `\`): it and the character after it
    /// stand for that character, inside or outside quotes. Must be ASCII.
    pub escape: Option<u8>,
    /// Whether leading whitespace is removed from each cell (CSVW
    /// `skipInitialSpace`, which §5.9 defines as setting the trim flag to
    /// `start`); applied together with [`trim`](Self::trim).
    pub skip_initial_space: bool,
    /// Which whitespace is removed from each cell (CSVW `trim`).
    pub trim: Trim,
    /// Whether records may differ in field count. When `false`, a record whose
    /// field count differs from the first record's is refused with
    /// [`CsvErrorKind::UnequalLengths`], on reading and on writing.
    pub flexible: bool,
    /// When a written field is quoted.
    pub quote_style: QuoteStyle,
    /// The bytes that end each written record.
    pub record_terminator: &'a str,
}

impl Dialect<'static> {
    /// RFC 4180 CSV: comma, `"` with doubling, rows ended by CRLF or LF, every
    /// record the same length, no header consumption — [`Reader::records`]
    /// yields the header row as its first record. Written records end in LF
    /// and fields are quoted only when necessary.
    pub const RFC4180: Self = Self {
        encoding: Encoding::Utf8,
        line_terminators: LineTerminators::Strings(DEFAULT_LINE_TERMINATORS),
        comment_prefix: None,
        header_row_count: 0,
        skip_rows: 0,
        skip_columns: 0,
        skip_blank_rows: false,
        delimiter: b',',
        quote_char: Some(b'"'),
        double_quote: true,
        escape: None,
        skip_initial_space: false,
        trim: Trim::None,
        flexible: false,
        quote_style: QuoteStyle::Necessary,
        record_terminator: "\n",
    };

    /// The labelled-property-graph CSV artifacts: [`RFC4180`](Self::RFC4180)
    /// with one header row, which [`Reader::headers`] returns and
    /// [`Reader::records`] does not yield.
    pub const LPG_CSV: Self = Self {
        header_row_count: 1,
        ..Self::RFC4180
    };

    /// The SSSOM mapping table body: tab-delimited, one header row, records
    /// may differ in length (a short row leaves its trailing columns absent).
    pub const SSSOM_TSV: Self = Self {
        delimiter: b'\t',
        header_row_count: 1,
        flexible: true,
        ..Self::RFC4180
    };

    /// W3C SPARQL 1.1 Query Results CSV: RFC 4180 fields — `"`, `,`, LF and CR
    /// force quoting and an inner `"` is doubled — in records ending in CRLF,
    /// under one header row of variable names.
    pub const SPARQL_RESULTS: Self = Self {
        header_row_count: 1,
        record_terminator: "\r\n",
        ..Self::RFC4180
    };

    /// The CSVW default dialect description (*Metadata Vocabulary for Tabular
    /// Data* §5.9): UTF-8, CRLF or LF, `"` with doubling, comment prefix `#`,
    /// one header row, comma, no skipping, no trimming; records may differ in
    /// length. Written records quote every field and end in LF.
    pub const CSVW_CELLS: Self = Self {
        comment_prefix: Some("#"),
        header_row_count: 1,
        flexible: true,
        quote_style: QuoteStyle::Always,
        ..Self::RFC4180
    };
}

impl Dialect<'_> {
    /// The escape character distinct from the quote character, if any.
    const fn distinct_escape(&self) -> Option<u8> {
        match (self.escape, self.quote_char) {
            (Some(escape), Some(quote)) if escape == quote => None,
            (escape, _) => escape,
        }
    }

    /// Whether a doubled quote character stands for one: `double_quote`, or an
    /// escape character equal to the quote character.
    const fn doubles_quote(&self) -> bool {
        match (self.quote_char, self.escape) {
            (None, _) => false,
            (Some(quote), Some(escape)) if quote == escape => true,
            _ => self.double_quote,
        }
    }

    /// The first refusal this dialect's own fields earn, if any.
    const fn check(&self) -> Result<(), DialectFault> {
        if !self.delimiter.is_ascii() {
            return Err(DialectFault::Invalid("the delimiter must be an ASCII byte"));
        }
        if let Some(quote) = self.quote_char {
            if !quote.is_ascii() {
                return Err(DialectFault::Invalid(
                    "the quote character must be an ASCII byte",
                ));
            }
            if quote == self.delimiter {
                return Err(DialectFault::Invalid(
                    "the quote character must differ from the delimiter",
                ));
            }
        }
        if let Some(escape) = self.escape {
            if !escape.is_ascii() {
                return Err(DialectFault::Invalid(
                    "the escape character must be an ASCII byte",
                ));
            }
            if escape == self.delimiter {
                return Err(DialectFault::Invalid(
                    "the escape character must differ from the delimiter",
                ));
            }
        }
        let terminators = self.line_terminators.strings();
        let mut i = 0;
        while i < terminators.len() {
            if terminators[i].is_empty() {
                return Err(DialectFault::EmptyLineTerminator);
            }
            i += 1;
        }
        Ok(())
    }
}

/// Why a dialect is refused, in a form a `const fn` can return (no
/// destructor); [`DialectFault::kind`] is the public error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DialectFault {
    EmptyLineTerminator,
    Invalid(&'static str),
}

impl DialectFault {
    const fn kind(self) -> CsvErrorKind {
        match self {
            Self::EmptyLineTerminator => CsvErrorKind::EmptyLineTerminator,
            Self::Invalid(reason) => CsvErrorKind::InvalidDialect { reason },
        }
    }
}

#[cfg(test)]
mod tests;
