// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Writing: RFC 4180 §2 rules 6 and 7 under a [`Dialect`].
//!
//! One [`Quoter`] decides and writes every field for all three writers. A
//! field is quoted when the dialect always quotes, or when one scan finds a
//! byte that would otherwise be read as structure: the delimiter, the quote
//! character, CR, LF, a distinct escape character, or the first byte of a
//! line terminator. The same scan then continues from that byte, stopping only
//! at the quote and escape characters so each is doubled (or escaped), and
//! every run between stops is copied whole.

use std::io;

use super::error::{CsvError, CsvErrorKind};
use super::scan::StopSet;
use super::{Dialect, DialectFault, QuoteStyle};
use crate::sink::TextOut;

/// How many buffered bytes [`Writer`] holds before it writes them to its sink.
const WRITER_BUFFER: usize = 64 * 1024;

/// The most bytes a quoting stop set collects: delimiter, quote, escape, CR,
/// LF, and the first byte of up to eight terminators.
const MAX_QUOTING_BYTES: usize = 13;

/// Where written text goes: a byte buffer or a [`TextOut`].
trait Out {
    fn text(&mut self, text: &str);
    fn byte(&mut self, byte: u8);
}

impl Out for Vec<u8> {
    #[inline]
    fn text(&mut self, text: &str) {
        self.extend_from_slice(text.as_bytes());
    }

    #[inline]
    fn byte(&mut self, byte: u8) {
        self.push(byte);
    }
}

/// A [`TextOut`] as an [`Out`]. Every byte written through [`Out::byte`] is an
/// ASCII dialect byte, so it is one `char`.
struct TextOutput<'o, W: TextOut + ?Sized>(&'o mut W);

impl<W: TextOut + ?Sized> Out for TextOutput<'_, W> {
    #[inline]
    fn text(&mut self, text: &str) {
        self.0.push_str(text);
    }

    #[inline]
    fn byte(&mut self, byte: u8) {
        self.0.push(char::from(byte));
    }
}

/// A field [`Quoter::field`] could not write.
struct Unquotable;

/// The quoting law of one dialect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Quoter {
    delimiter: u8,
    quote: Option<u8>,
    /// Whether an inner quote character is written doubled.
    doubles: bool,
    /// The escape character distinct from the quote character.
    escape: Option<u8>,
    always: bool,
    /// The bytes that make a field quoted.
    quoting: StopSet,
    /// The bytes inside a quoted field that are written escaped.
    inner: StopSet,
}

impl Quoter {
    /// The quoting law of `dialect`, or why the dialect cannot be written.
    const fn new(dialect: &Dialect<'_>) -> Result<Self, DialectFault> {
        if let Err(fault) = dialect.check() {
            return Err(fault);
        }
        let always = matches!(dialect.quote_style, QuoteStyle::Always);
        if always && dialect.quote_char.is_none() {
            return Err(DialectFault::Invalid(
                "a dialect that always quotes needs a quote character",
            ));
        }
        let escape = dialect.distinct_escape();
        let mut quoting = [0_u8; MAX_QUOTING_BYTES];
        quoting[0] = dialect.delimiter;
        quoting[1] = b'\r';
        quoting[2] = b'\n';
        let mut count = 3;
        if let Some(quote) = dialect.quote_char {
            quoting[count] = quote;
            count += 1;
        }
        if let Some(escape) = escape {
            quoting[count] = escape;
            count += 1;
        }
        let terminators = dialect.line_terminators.strings();
        let mut i = 0;
        while i < terminators.len() {
            let bytes = terminators[i].as_bytes();
            if !bytes.is_empty() {
                if count == MAX_QUOTING_BYTES {
                    return Err(DialectFault::Invalid(
                        "a written dialect names at most eight line terminators",
                    ));
                }
                quoting[count] = bytes[0];
                count += 1;
            }
            i += 1;
        }
        let (quoting_bytes, _) = quoting.split_at(count);
        let mut inner = [0_u8; 2];
        let mut inner_count = if let Some(quote) = dialect.quote_char {
            inner[0] = quote;
            1
        } else {
            0
        };
        if let Some(escape) = escape {
            inner[inner_count] = escape;
            inner_count += 1;
        }
        let (inner_bytes, _) = inner.split_at(inner_count);
        Ok(Self {
            delimiter: dialect.delimiter,
            quote: dialect.quote_char,
            doubles: dialect.doubles_quote(),
            escape,
            always,
            quoting: StopSet::new(quoting_bytes),
            inner: StopSet::new(inner_bytes),
        })
    }

    /// Write `value` as one field, quoted when the rule says so or `force`.
    #[inline]
    fn field<O: Out>(&self, value: &str, force: bool, out: &mut O) -> Result<(), Unquotable> {
        let bytes = value.as_bytes();
        let first_special = self.quoting.find(bytes);
        if first_special.is_none() && !self.always && !force {
            out.text(value);
            return Ok(());
        }
        let Some(quote) = self.quote else {
            return Err(Unquotable);
        };
        out.byte(quote);
        let mut run_start = 0;
        let mut from = first_special.unwrap_or(bytes.len());
        while let Some(offset) = self.inner.find(&bytes[from..]) {
            let hit = from + offset;
            let byte = bytes[hit];
            out.text(&value[run_start..hit]);
            if byte == quote && self.doubles {
                out.byte(quote);
            } else if let Some(escape) = self.escape {
                out.byte(escape);
            } else {
                // A quote character inside a quoted field with neither
                // doubling nor an escape character cannot be written.
                return Err(Unquotable);
            }
            out.byte(byte);
            run_start = hit + 1;
            from = hit + 1;
        }
        out.text(&value[run_start..]);
        out.byte(quote);
        Ok(())
    }

    /// Write every field of one record (no terminator). A first field that
    /// begins with `comment_prefix` is quoted, so the record is not read back
    /// as a comment row. Returns the field count, or the index of the field
    /// that could not be written.
    fn fields<I, O>(
        &self,
        fields: I,
        comment_prefix: Option<&str>,
        out: &mut O,
    ) -> Result<usize, usize>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
        O: Out,
    {
        let mut count = 0;
        let mut only_empty = false;
        for field in fields {
            let field = field.as_ref();
            if count > 0 {
                out.byte(self.delimiter);
            }
            let force =
                count == 0 && comment_prefix.is_some_and(|prefix| field.starts_with(prefix));
            self.field(field, force, out).map_err(|Unquotable| count)?;
            only_empty = count == 0 && field.is_empty();
            count += 1;
        }
        if count == 1 && only_empty && !self.always {
            // One empty field would otherwise be an empty line, which a
            // reader skipping blank rows drops.
            let Some(quote) = self.quote else {
                return Err(0);
            };
            out.byte(quote);
            out.byte(quote);
        }
        Ok(count)
    }
}

/// A `const`-constructible, infallible field and record writer onto any
/// [`TextOut`].
///
/// ```
/// use purrdf_core::csv::{Dialect, FieldWriter};
///
/// const FIELDS: FieldWriter = FieldWriter::new(Dialect::SPARQL_RESULTS);
/// let mut out = String::new();
/// FIELDS.write_record(["x", "y"], &mut out);
/// FIELDS.write_field("a,b", &mut out);
/// FIELDS.write_delimiter(&mut out);
/// FIELDS.write_field("say \"hi\"", &mut out);
/// FIELDS.end_record(&mut out);
/// assert_eq!(out, "x,y\r\n\"a,b\",\"say \"\"hi\"\"\"\r\n");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldWriter {
    quoter: Quoter,
    comment_prefix: Option<&'static str>,
    terminator: &'static str,
}

impl FieldWriter {
    /// The writer of `dialect`.
    ///
    /// # Panics
    ///
    /// When the dialect cannot be written (it is refused by the reader, or has
    /// no quote character, or cannot double or escape a quote character
    /// inside a quoted field) — a compile-time error in a `const` item.
    #[must_use]
    pub const fn new(dialect: Dialect<'static>) -> Self {
        let Ok(quoter) = Quoter::new(&dialect) else {
            panic!("FieldWriter::new: the dialect cannot be written")
        };
        assert!(
            quoter.quote.is_some() && (quoter.doubles || quoter.escape.is_some()),
            "FieldWriter::new: an infallible writer needs a quote character it can double or escape"
        );
        Self {
            quoter,
            comment_prefix: dialect.comment_prefix,
            terminator: dialect.record_terminator,
        }
    }

    /// Write one field, quoted when the dialect's rule says so. A field is
    /// written as a record's first field only by [`write_record`](Self::write_record).
    pub fn write_field<W: TextOut + ?Sized>(&self, value: &str, out: &mut W) {
        if self
            .quoter
            .field(value, false, &mut TextOutput(out))
            .is_err()
        {
            unreachable!("FieldWriter::new admits only dialects that can quote every field");
        }
    }

    /// Write the delimiter between two fields.
    pub fn write_delimiter<W: TextOut + ?Sized>(&self, out: &mut W) {
        out.push(char::from(self.quoter.delimiter));
    }

    /// Write the record terminator.
    pub fn end_record<W: TextOut + ?Sized>(&self, out: &mut W) {
        out.push_str(self.terminator);
    }

    /// Write one whole record, terminator included. A record of no fields is
    /// the terminator alone; a record of one empty field is written `""`.
    pub fn write_record<I, W>(&self, fields: I, out: &mut W)
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
        W: TextOut + ?Sized,
    {
        if self
            .quoter
            .fields(fields, self.comment_prefix, &mut TextOutput(out))
            .is_err()
        {
            unreachable!("FieldWriter::new admits only dialects that can quote every field");
        }
        self.end_record(out);
    }
}

/// Append one record, terminator included, to `out`.
///
/// # Panics
///
/// When the dialect cannot be written (see [`FieldWriter::new`]), or a field
/// needs quoting that the dialect cannot express.
pub fn write_record_text(dialect: &Dialect<'_>, fields: &[&str], out: &mut String) {
    let quoter = match Quoter::new(dialect) {
        Ok(quoter) => quoter,
        Err(fault) => panic!("write_record_text: {}", fault.kind()),
    };
    if let Err(field) = quoter.fields(fields, dialect.comment_prefix, &mut TextOutput(out)) {
        panic!(
            "write_record_text: {}",
            CsvErrorKind::UnquotableField { field }
        );
    }
    out.push_str(dialect.record_terminator);
}

/// A buffered record writer onto an [`io::Write`] sink.
///
/// ```
/// use purrdf_core::csv::{Dialect, Writer};
///
/// let mut writer = Writer::new(Dialect::RFC4180, Vec::new());
/// writer.write_record(["id", "note"]).expect("in memory");
/// writer.write_record(["1", "a, b"]).expect("in memory");
/// let bytes = writer.into_inner().expect("in memory");
/// assert_eq!(bytes, b"id,note\n1,\"a, b\"\n");
/// ```
#[derive(Debug)]
pub struct Writer<W: io::Write> {
    sink: W,
    buffer: Vec<u8>,
    quoter: Result<Quoter, DialectFault>,
    comment_prefix: Option<Box<str>>,
    terminator: Box<[u8]>,
    flexible: bool,
    expected: Option<usize>,
}

impl<W: io::Write> Writer<W> {
    /// A writer of `dialect` records onto `sink`. A dialect the writer
    /// refuses is reported by the first write.
    pub fn new(dialect: Dialect<'_>, sink: W) -> Self {
        Self {
            sink,
            buffer: Vec::with_capacity(WRITER_BUFFER),
            quoter: Quoter::new(&dialect),
            comment_prefix: dialect.comment_prefix.map(Box::from),
            terminator: Box::from(dialect.record_terminator.as_bytes()),
            flexible: dialect.flexible,
            expected: None,
        }
    }

    /// Write one record, terminator included.
    ///
    /// Refused, with nothing written: a record of no fields
    /// ([`CsvErrorKind::EmptyRecord`]); a record whose length differs from
    /// the first record's in a dialect that is not flexible
    /// ([`CsvErrorKind::UnequalLengths`]); a field the dialect cannot quote
    /// ([`CsvErrorKind::UnquotableField`]).
    pub fn write_record<I, T>(&mut self, record: I) -> Result<(), CsvError>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let quoter = self.quoter.map_err(|fault| CsvError::new(fault.kind()))?;
        let mark = self.buffer.len();
        let result = quoter.fields(record, self.comment_prefix.as_deref(), &mut self.buffer);
        let count = match result {
            Ok(count) => count,
            Err(field) => {
                self.buffer.truncate(mark);
                return Err(CsvError::new(CsvErrorKind::UnquotableField { field }));
            }
        };
        if count == 0 {
            self.buffer.truncate(mark);
            return Err(CsvError::new(CsvErrorKind::EmptyRecord));
        }
        match self.expected {
            Some(expected) if expected != count && !self.flexible => {
                self.buffer.truncate(mark);
                return Err(CsvError::new(CsvErrorKind::UnequalLengths {
                    expected,
                    found: count,
                }));
            }
            Some(_) => {}
            None => self.expected = Some(count),
        }
        self.buffer.extend_from_slice(&self.terminator);
        if self.buffer.len() >= WRITER_BUFFER {
            self.drain()?;
        }
        Ok(())
    }

    /// Write the buffered bytes to the sink, without flushing it.
    fn drain(&mut self) -> Result<(), CsvError> {
        if !self.buffer.is_empty() {
            let result = self.sink.write_all(&self.buffer);
            self.buffer.clear();
            result.map_err(|error| CsvError::io(&error))?;
        }
        Ok(())
    }

    /// Write the buffered bytes to the sink and flush it.
    pub fn flush(&mut self) -> Result<(), CsvError> {
        self.drain()?;
        self.sink.flush().map_err(|error| CsvError::io(&error))
    }

    /// Flush, then give back the sink.
    pub fn into_inner(mut self) -> Result<W, CsvError> {
        self.flush()?;
        Ok(self.sink)
    }

    /// The sink, as it stands (buffered bytes not yet written to it).
    pub const fn get_ref(&self) -> &W {
        &self.sink
    }
}
