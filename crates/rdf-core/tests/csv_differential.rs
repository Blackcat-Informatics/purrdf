// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Replays every record of `csv_differential_vectors.txt` — answers recorded
//! from a third-party CSV engine (see the file's header) — through the reader and the
//! writer, and holds every disagreement to a named class resolved by the text
//! of W3C CSVW or RFC 4180.
//!
//! # The vector format (from the file's own header)
//!
//! A body record is `case@index`, the input bytes, and the answer. A reader
//! answer is the records in order, each `R` then `<byte length>:<bytes>` per
//! field then `/`, followed by the first error if any: `E:utf8:<record>:<field>:
//! <valid bytes>` or `E:len:<record>:<expected>:<found>` (`<record>` counts
//! from 0, a header row included). A writer answer is the bytes of one written
//! record. The eleven historical `digest` records hash answers over 200,000
//! further generated inputs per dialect. Their generator was not preserved;
//! these rows are provenance only and are not counted as replayed evidence.
//!
//! # PROVENANCE-csv: disagreement classes
//!
//! Each class is an input feature on which the recorded engine's answer
//! contradicts the specification text, so the replacement follows the text:
//!
//! * `lone-cr` — the recorded readers ended a row at a CARRIAGE RETURN not
//!   followed by LINE FEED. The presets read the CSVW default line terminators
//!   `["\r\n", "\n"]` (*Metadata Vocabulary for Tabular Data* §5.9
//!   `lineTerminators`; RFC 4180 §2 rule 1 names CRLF), so a lone CR is cell
//!   text.
//! * `empty-line` — the recorded readers dropped an empty line. RFC 4180 §2's
//!   `record = field *(COMMA field)` with `non-escaped = *TEXTDATA` makes an
//!   empty line a record of one empty field, and CSVW §8 "parse a row" of an
//!   empty row gives one empty cell, dropped only under "skip blank rows".
//! * `sentinel` — the recorded CSVW cell parser was configured with the byte
//!   `0xFF` as its record terminator (a byte no UTF-8 text holds). It is not a
//!   CSVW line terminator; the replacement reads such an input as one row and
//!   refuses it as not UTF-8.
//! * `strict-quote` — CSVW §8 "parse a row": a quote character in a cell that
//!   is not empty "raise[s] an error", and after a closing quote "If the
//!   remaining string does not start with the delimiter, raise an error"; RFC
//!   4180 §2 rule 5 ("double quotes may not appear inside the fields") and the
//!   `escaped` production (a closing `DQUOTE`). The recorded engine accepted
//!   all three. Where this is the only class, the records before the refused
//!   row must equal the recorded ones.
//! * `quoted-row` — CSVW §8 "read a row": "if the string starts with the quote
//!   character, append the quoted value obtained by reading a quoted value",
//!   wherever in the row the quote stands, so a line terminator inside it is
//!   row content. The recorded engine opened a quoted value only at a field
//!   start.
//! * `doubled-quote` — CSVW §8 "parse a row": "If the string starts with the
//!   escape character followed by the quote character, append the quote
//!   character to the current cell value", in an unquoted cell too; the
//!   recorded engine kept both quotes there.
//! * `escape-outside-quotes` — CSVW §8 "parse a row" applies a distinct escape
//!   character ("append the character following the escape character")
//!   inside and outside quoted values; the recorded engine only inside.
//! * `decode-first` — CSVW §8 decodes the input ("Read the file using the
//!   encoding") before it reads a row, so in one row a byte that is not UTF-8
//!   is reported before an unequal field count; the recorded engine reported
//!   the count first.
//!
//! The configuration classes (`lone-cr`, `sentinel`) are also confirmed by
//! replaying the vector under the recorded configuration — CR added to the
//! line terminators, or the input split at `0xFF` — which must then agree or
//! leave only a specification class. Every class count is frozen below, so a
//! behaviour change moves a count and fails.

use std::collections::BTreeMap;

use purrdf_core::csv::{
    CsvError, CsvErrorKind, Dialect, LineTerminators, QuoteStyle, Reader, StringRecord, Writer,
};
use purrdf_testkit::vectors::{VectorFile, decode_bytes, encode_bytes, sha256_hex};

const VECTORS: &str = include_str!("csv_differential_vectors.txt");

/// No line terminators: the whole input is one row (the recorded CSVW cell
/// parser read one logical record at a time).
const NO_TERMINATORS: &[&str] = &[];
/// The recorded non-CSVW readers' terminators: CR as well as CRLF and LF.
const WITH_LONE_CR: &[&str] = &["\r\n", "\n", "\r"];

/// Every contributing input is present in the tracked vector file. This
/// catches an unrelated answer change even when a disagreement class remains
/// present and its count is unchanged.
const REPLAYED_ANSWER_DIGESTS: &[(&str, &str)] = &[
    (
        "read-csvw-comma",
        "0b99ca72b4e8625f81f295ac68cf1fa1f0022cdd13eb9fdd372b85eedeebcf1f",
    ),
    (
        "read-csvw-escape",
        "718ecbf82d2f726eeceb8b7a0a1a7c63f61cb905fec263a6a053241e436ff06c",
    ),
    (
        "read-csvw-semicolon",
        "9f1d336c920432a22c3d14fd31b41f28ef109ee460069e26b189e44f9b75f873",
    ),
    (
        "read-csvw-tab",
        "0032e81929fdd82398b2c41b7f1048429db333692bb84be3dbb3ce71763c2659",
    ),
    (
        "read-csvw-unquoted",
        "cefe326480fe045e65db167417c7ab33867553ac975a9090674149f792957bbe",
    ),
    (
        "read-lpg-csv",
        "2aacc6d9dfb258610e15a0361eec62d167fdef5094d457670fff8c37ab253723",
    ),
    (
        "read-rfc4180",
        "1baab4c5853c210bfc62806beba7de52eb58fcb5d004ee39319fc3fedde0fb7e",
    ),
    (
        "read-sssom-tsv",
        "bf5f716d5ea51a79379d224b01fb605ded8c1ede93214e1009d6073273508649",
    ),
    (
        "write-always-lf",
        "ddd1cb22c681765d3068084a212923971a9dfef17c87cb2b9e138ffc4b029b4e",
    ),
    (
        "write-necessary-crlf",
        "4d899528fb2f13e976a88c8df17ba97326d46484bc34ff0735a722ccfc9e976d",
    ),
    (
        "write-necessary-lf",
        "469cc9aa594ae08546c1007743568fe1a4bae207b5006ae75b3e75267aa4857b",
    ),
];

/// One reader answer, in the file's encoding.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Answer {
    records: Vec<Vec<u8>>,
    error: Option<String>,
}

impl Answer {
    fn encode(&self) -> String {
        let mut out = Vec::new();
        for record in &self.records {
            out.extend_from_slice(record);
        }
        if let Some(error) = &self.error {
            out.extend_from_slice(error.as_bytes());
        }
        encode_bytes(&out)
    }

    /// Split an encoded answer back into records and error.
    fn parse(encoded: &str) -> Self {
        let bytes = decode_bytes(encoded).expect("a well-formed answer field");
        let mut answer = Self::default();
        let mut at = 0;
        while at < bytes.len() {
            if bytes[at] == b'E' {
                answer.error = Some(String::from_utf8_lossy(&bytes[at..]).into_owned());
                break;
            }
            assert_eq!(bytes[at], b'R', "{encoded}");
            let start = at;
            at += 1;
            while bytes[at] != b'/' {
                let colon = at + bytes[at..].iter().position(|&b| b == b':').expect("length");
                let length: usize = std::str::from_utf8(&bytes[at..colon])
                    .expect("ASCII length")
                    .parse()
                    .expect("decimal length");
                at = colon + 1 + length;
            }
            at += 1;
            answer.records.push(bytes[start..at].to_vec());
        }
        answer
    }

    fn quote_error(&self) -> bool {
        self.error.as_deref().is_some_and(|error| {
            error.starts_with("E:quote-in-unquoted")
                || error.starts_with("E:after-quote")
                || error.starts_with("E:unterminated")
        })
    }

    fn utf8_error(&self) -> bool {
        self.error
            .as_deref()
            .is_some_and(|error| error.starts_with("E:utf8"))
    }
}

fn record_answer(record: &StringRecord) -> Vec<u8> {
    let mut out = vec![b'R'];
    for field in record {
        out.extend_from_slice(field.len().to_string().as_bytes());
        out.push(b':');
        out.extend_from_slice(field.as_bytes());
    }
    out.push(b'/');
    out
}

fn error_answer(error: &CsvError) -> String {
    let record = error.position().map_or(0, |position| position.row - 1);
    match error.kind() {
        CsvErrorKind::NotUtf8 { field, valid_up_to } => {
            format!("E:utf8:{record}:{field}:{valid_up_to}")
        }
        CsvErrorKind::UnequalLengths { expected, found } => {
            format!("E:len:{record}:{expected}:{found}")
        }
        CsvErrorKind::UnterminatedQuote => format!("E:unterminated:{record}"),
        CsvErrorKind::QuoteInUnquotedField => format!("E:quote-in-unquoted:{record}"),
        CsvErrorKind::TextAfterClosingQuote => format!("E:after-quote:{record}"),
        other => panic!("no reader vector can raise {other:?}"),
    }
}

/// Read `input` and render the answer: the header row first when the dialect
/// consumes one.
fn read(dialect: Dialect<'_>, input: &[u8]) -> Answer {
    let headers = dialect.header_row_count > 0;
    let mut reader = Reader::new(dialect, input);
    let mut answer = Answer::default();
    if headers {
        match reader.headers() {
            Ok(header) => answer.records.push(record_answer(header)),
            Err(error) => {
                answer.error = Some(error_answer(&error));
                return answer;
            }
        }
    }
    for record in reader.records() {
        match record {
            Ok(record) => answer.records.push(record_answer(&record)),
            Err(error) => {
                answer.error = Some(error_answer(&error));
                break;
            }
        }
    }
    answer
}

/// The recorded CSVW cell configuration: each `0xFF`-terminated piece is one
/// logical record (empty pieces dropped), parsed as one row.
fn read_split_at_sentinel(dialect: Dialect<'_>, input: &[u8]) -> Answer {
    let input = input
        .strip_prefix(b"\xEF\xBB\xBF".as_slice())
        .unwrap_or(input);
    let mut answer = Answer::default();
    for piece in input
        .split(|&b| b == 0xFF)
        .filter(|piece| !piece.is_empty())
    {
        let offset = answer.records.len();
        let mut piece_answer = read(dialect, piece);
        answer.records.append(&mut piece_answer.records);
        if let Some(error) = piece_answer.error {
            // Re-number the error's record from this piece's position.
            let mut parts: Vec<String> = error.split(':').map(str::to_owned).collect();
            let record: usize = parts[2].parse().expect("record");
            parts[2] = (record + offset).to_string();
            answer.error = Some(parts.join(":"));
            return answer;
        }
    }
    answer
}

/// The dialect each reader case replays under, as the call sites use it.
fn reader_dialect(case: &str) -> Option<Dialect<'static>> {
    let cells = |delimiter, quote_char, double_quote, escape| Dialect {
        line_terminators: LineTerminators::Strings(NO_TERMINATORS),
        comment_prefix: None,
        header_row_count: 0,
        delimiter,
        quote_char,
        double_quote,
        escape,
        ..Dialect::CSVW_CELLS
    };
    Some(match case {
        "read-sssom-tsv" => Dialect::SSSOM_TSV,
        "read-lpg-csv" => Dialect::LPG_CSV,
        "read-rfc4180" => Dialect::RFC4180,
        "read-csvw-comma" => cells(b',', Some(b'"'), true, None),
        "read-csvw-escape" => cells(b',', Some(b'"'), false, Some(b'\\')),
        "read-csvw-unquoted" => cells(b',', None, true, None),
        "read-csvw-semicolon" => cells(b';', Some(b'"'), true, None),
        "read-csvw-tab" => cells(b'\t', Some(b'"'), true, None),
        _ => return None,
    })
}

/// Split one written record back into fields — a local RFC 4180 field split,
/// independent of the reader under test.
fn split_written(record: &str, delimiter: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut chars = record.chars().peekable();
    let mut at_start = true;
    let mut quoted = false;
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
        } else if at_start && c == '"' {
            quoted = true;
            at_start = false;
        } else if c == delimiter {
            fields.push(std::mem::take(&mut field));
            at_start = true;
        } else {
            field.push(c);
            at_start = false;
        }
    }
    fields.push(field);
    fields
}

/// Replay one writer vector: the fields are recovered from the recorded bytes
/// and checked against the (lossily decoded) input, then written again.
fn replay_writer(case: &str, index: usize, input: &[u8], frozen: &str) -> String {
    let (quote_style, record_terminator) = match case {
        "write-necessary-lf" => (QuoteStyle::Necessary, "\n"),
        "write-necessary-crlf" => (QuoteStyle::Necessary, "\r\n"),
        "write-always-lf" => (QuoteStyle::Always, "\n"),
        other => panic!("unknown writer case {other}"),
    };
    let recorded = String::from_utf8(decode_bytes(frozen).expect("answer")).expect("UTF-8");
    let body = recorded
        .strip_suffix(record_terminator)
        .expect("a record ends in its terminator");
    let fields = split_written(body, ',');
    // The recorder wrote the lossy decoding of the input, split into fields,
    // except that every 97th input was written as one empty field.
    if index.is_multiple_of(97) {
        assert_eq!(fields, [""], "{case}@{index}");
    } else {
        assert_eq!(
            fields.concat(),
            String::from_utf8_lossy(input),
            "{case}@{index}"
        );
    }
    let dialect = Dialect {
        quote_style,
        record_terminator,
        ..Dialect::RFC4180
    };
    let mut writer = Writer::new(dialect, Vec::new());
    writer.write_record(&fields).expect("in memory");
    encode_bytes(&writer.into_inner().expect("in memory"))
}

/// A CARRIAGE RETURN not followed by LINE FEED.
fn has_lone_cr(input: &[u8]) -> bool {
    input
        .iter()
        .enumerate()
        .any(|(i, &b)| b == b'\r' && input.get(i + 1) != Some(&b'\n'))
}

/// An empty line as the recorded readers split lines — at CRLF, LF or a lone
/// CR: a line break at the start (after a BOM) or right after another.
fn has_empty_line(input: &[u8]) -> bool {
    let input = input
        .strip_prefix(b"\xEF\xBB\xBF".as_slice())
        .unwrap_or(input);
    let mut at = 0;
    let mut line_start = true;
    while at < input.len() {
        let width = match (input[at], input.get(at + 1)) {
            (b'\r', Some(b'\n')) => 2,
            (b'\r' | b'\n', _) => 1,
            _ => 0,
        };
        if width == 0 {
            line_start = false;
            at += 1;
        } else {
            if line_start {
                return true;
            }
            line_start = true;
            at += width;
        }
    }
    false
}

/// `""` after a byte that is not a quote, a delimiter or a line break: inside
/// an unquoted cell that already holds text.
fn has_doubled_quote_mid_cell(input: &[u8], delimiter: u8) -> bool {
    input.windows(3).any(|w| {
        w[1] == b'"' && w[2] == b'"' && !matches!(w[0], b'"' | b'\r' | b'\n') && w[0] != delimiter
    })
}

/// Classify one reader disagreement.
fn classify(case: &str, input: &[u8], old: &Answer, new: &Answer) -> Vec<&'static str> {
    let csvw = case.starts_with("read-csvw");
    let dialect = reader_dialect(case).expect("a reader case");
    let mut classes = Vec::new();
    if !csvw && has_lone_cr(input) {
        classes.push("lone-cr");
    }
    if !csvw && has_empty_line(input) {
        classes.push("empty-line");
    }
    if csvw && input.contains(&0xFF) {
        classes.push("sentinel");
    }
    if new.quote_error() {
        classes.push("strict-quote");
    }
    let first_invalid = std::str::from_utf8(input)
        .err()
        .map_or(input.len(), |error| error.valid_up_to());
    if dialect.quote_char.is_some()
        && new.utf8_error()
        && old.records.len() > new.records.len()
        && input[..first_invalid].contains(&b'"')
    {
        classes.push("quoted-row");
    }
    if dialect.quote_char.is_some() && has_doubled_quote_mid_cell(input, dialect.delimiter) {
        classes.push("doubled-quote");
    }
    if dialect.escape.is_some() && input.contains(&b'\\') {
        classes.push("escape-outside-quotes");
    }
    if new.utf8_error()
        && old
            .error
            .as_deref()
            .is_some_and(|error| error.starts_with("E:len"))
        && old.records == new.records
    {
        classes.push("decode-first");
    }
    classes
}

/// The recorded configuration's answer, where CSVW can express it.
fn recorded_configuration(case: &str, input: &[u8]) -> Answer {
    let dialect = reader_dialect(case).expect("a reader case");
    if case.starts_with("read-csvw") {
        read_split_at_sentinel(dialect, input)
    } else {
        read(
            Dialect {
                line_terminators: LineTerminators::Strings(WITH_LONE_CR),
                ..dialect
            },
            input,
        )
    }
}

/// Whether a class names a configuration difference rather than a
/// specification clause.
fn is_configuration(class: &str) -> bool {
    matches!(class, "lone-cr" | "sentinel" | "empty-line")
}

/// How many vectors of each case fall in each class set. A behaviour change
/// that moves a vector between classes, or makes one agree or disagree, fails
/// here and must be explained.
const FROZEN_CLASS_COUNTS: &[(&str, &str, usize)] = &[
    ("read-csvw-comma", "doubled-quote", 1),
    ("read-csvw-comma", "sentinel", 144),
    ("read-csvw-comma", "sentinel+doubled-quote", 16),
    ("read-csvw-comma", "sentinel+quoted-row", 62),
    ("read-csvw-comma", "sentinel+quoted-row+doubled-quote", 4),
    ("read-csvw-comma", "strict-quote", 21),
    ("read-csvw-comma", "strict-quote+doubled-quote", 1),
    ("read-csvw-escape", "escape-outside-quotes", 15),
    ("read-csvw-escape", "sentinel", 27),
    ("read-csvw-escape", "sentinel+doubled-quote", 3),
    (
        "read-csvw-escape",
        "sentinel+doubled-quote+escape-outside-quotes",
        14,
    ),
    ("read-csvw-escape", "sentinel+escape-outside-quotes", 148),
    ("read-csvw-escape", "sentinel+quoted-row", 10),
    (
        "read-csvw-escape",
        "sentinel+quoted-row+doubled-quote+escape-outside-quotes",
        4,
    ),
    (
        "read-csvw-escape",
        "sentinel+quoted-row+escape-outside-quotes",
        53,
    ),
    ("read-csvw-escape", "strict-quote", 11),
    (
        "read-csvw-escape",
        "strict-quote+doubled-quote+escape-outside-quotes",
        2,
    ),
    ("read-csvw-escape", "strict-quote+escape-outside-quotes", 9),
    ("read-csvw-semicolon", "doubled-quote", 1),
    ("read-csvw-semicolon", "sentinel", 143),
    ("read-csvw-semicolon", "sentinel+doubled-quote", 18),
    ("read-csvw-semicolon", "sentinel+quoted-row", 63),
    (
        "read-csvw-semicolon",
        "sentinel+quoted-row+doubled-quote",
        5,
    ),
    ("read-csvw-semicolon", "strict-quote", 21),
    ("read-csvw-semicolon", "strict-quote+doubled-quote", 1),
    ("read-csvw-tab", "doubled-quote", 1),
    ("read-csvw-tab", "sentinel", 144),
    ("read-csvw-tab", "sentinel+doubled-quote", 16),
    ("read-csvw-tab", "sentinel+quoted-row", 63),
    ("read-csvw-tab", "sentinel+quoted-row+doubled-quote", 4),
    ("read-csvw-tab", "strict-quote", 21),
    ("read-csvw-tab", "strict-quote+doubled-quote", 1),
    ("read-csvw-unquoted", "sentinel", 240),
    ("read-lpg-csv", "decode-first", 1),
    ("read-lpg-csv", "doubled-quote", 1),
    ("read-lpg-csv", "doubled-quote+decode-first", 1),
    ("read-lpg-csv", "empty-line", 6),
    ("read-lpg-csv", "empty-line+strict-quote", 2),
    ("read-lpg-csv", "lone-cr", 31),
    ("read-lpg-csv", "lone-cr+decode-first", 19),
    ("read-lpg-csv", "lone-cr+doubled-quote", 1),
    ("read-lpg-csv", "lone-cr+doubled-quote+decode-first", 2),
    ("read-lpg-csv", "lone-cr+empty-line", 53),
    ("read-lpg-csv", "lone-cr+empty-line+decode-first", 8),
    ("read-lpg-csv", "lone-cr+empty-line+doubled-quote", 5),
    (
        "read-lpg-csv",
        "lone-cr+empty-line+doubled-quote+decode-first",
        1,
    ),
    ("read-lpg-csv", "lone-cr+empty-line+quoted-row", 19),
    ("read-lpg-csv", "lone-cr+empty-line+strict-quote", 6),
    (
        "read-lpg-csv",
        "lone-cr+empty-line+strict-quote+doubled-quote",
        1,
    ),
    ("read-lpg-csv", "lone-cr+quoted-row", 27),
    ("read-lpg-csv", "lone-cr+quoted-row+doubled-quote", 4),
    ("read-lpg-csv", "lone-cr+strict-quote", 11),
    ("read-lpg-csv", "lone-cr+strict-quote+doubled-quote", 1),
    ("read-lpg-csv", "strict-quote", 7),
    ("read-rfc4180", "decode-first", 1),
    ("read-rfc4180", "doubled-quote", 1),
    ("read-rfc4180", "doubled-quote+decode-first", 1),
    ("read-rfc4180", "empty-line", 6),
    ("read-rfc4180", "empty-line+strict-quote", 2),
    ("read-rfc4180", "lone-cr", 31),
    ("read-rfc4180", "lone-cr+decode-first", 19),
    ("read-rfc4180", "lone-cr+doubled-quote", 1),
    ("read-rfc4180", "lone-cr+doubled-quote+decode-first", 2),
    ("read-rfc4180", "lone-cr+empty-line", 53),
    ("read-rfc4180", "lone-cr+empty-line+decode-first", 8),
    ("read-rfc4180", "lone-cr+empty-line+doubled-quote", 5),
    (
        "read-rfc4180",
        "lone-cr+empty-line+doubled-quote+decode-first",
        1,
    ),
    ("read-rfc4180", "lone-cr+empty-line+quoted-row", 19),
    ("read-rfc4180", "lone-cr+empty-line+strict-quote", 6),
    (
        "read-rfc4180",
        "lone-cr+empty-line+strict-quote+doubled-quote",
        1,
    ),
    ("read-rfc4180", "lone-cr+quoted-row", 27),
    ("read-rfc4180", "lone-cr+quoted-row+doubled-quote", 4),
    ("read-rfc4180", "lone-cr+strict-quote", 11),
    ("read-rfc4180", "lone-cr+strict-quote+doubled-quote", 1),
    ("read-rfc4180", "strict-quote", 7),
    ("read-sssom-tsv", "doubled-quote", 1),
    ("read-sssom-tsv", "empty-line", 6),
    ("read-sssom-tsv", "empty-line+strict-quote", 2),
    ("read-sssom-tsv", "lone-cr", 32),
    ("read-sssom-tsv", "lone-cr+doubled-quote", 2),
    ("read-sssom-tsv", "lone-cr+empty-line", 53),
    ("read-sssom-tsv", "lone-cr+empty-line+doubled-quote", 6),
    ("read-sssom-tsv", "lone-cr+empty-line+quoted-row", 21),
    (
        "read-sssom-tsv",
        "lone-cr+empty-line+quoted-row+doubled-quote",
        1,
    ),
    ("read-sssom-tsv", "lone-cr+empty-line+strict-quote", 7),
    (
        "read-sssom-tsv",
        "lone-cr+empty-line+strict-quote+doubled-quote",
        1,
    ),
    ("read-sssom-tsv", "lone-cr+quoted-row", 31),
    ("read-sssom-tsv", "lone-cr+quoted-row+doubled-quote", 4),
    ("read-sssom-tsv", "lone-cr+strict-quote", 11),
    ("read-sssom-tsv", "lone-cr+strict-quote+doubled-quote", 1),
    ("read-sssom-tsv", "quoted-row", 1),
    ("read-sssom-tsv", "strict-quote", 7),
    ("read-sssom-tsv", "strict-quote+doubled-quote", 1),
];

#[test]
fn every_frozen_vector_agrees_or_falls_in_a_documented_class() {
    let file = VectorFile::parse(VECTORS).expect("the vector file verifies against its header");
    let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut agreed = 0_usize;
    let mut unexplained = Vec::new();
    let mut replayed_answers: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut historical_digests = 0;
    for record in file.records() {
        let Some((case, index)) = record.fields[0].split_once('@') else {
            assert_eq!(record.fields[0], "digest");
            historical_digests += 1;
            continue;
        };
        let index: usize = index.parse().expect("input index");
        let input = decode_bytes(record.fields[1]).expect("input bytes");
        let frozen = record.fields[2];
        if case.starts_with("write-") {
            let replayed = replay_writer(case, index, &input, frozen);
            assert_eq!(replayed, frozen, "{case}@{index}: the writer disagrees");
            append_replayed_answer(&mut replayed_answers, case, &replayed);
            agreed += 1;
            continue;
        }
        let dialect = reader_dialect(case).expect("a known reader case");
        let new = read(dialect, &input);
        append_replayed_answer(&mut replayed_answers, case, &new.encode());
        if new.encode() == frozen {
            agreed += 1;
            continue;
        }
        let old = Answer::parse(frozen);
        let classes = classify(case, &input, &old, &new);
        let explained = if classes.is_empty() {
            false
        } else if classes == ["strict-quote"] {
            // Refused where the recorded engine read on: every record before
            // the refusal must agree.
            old.records.starts_with(&new.records)
        } else if classes.iter().all(|class| is_configuration(class)) {
            // Only configuration classes: under the recorded configuration the
            // answers agree, or what is left is a specification class (or the
            // empty-line class, which no dialect flag reproduces).
            let configured = recorded_configuration(case, &input);
            configured.encode() == frozen
                || classes.contains(&"empty-line")
                || classify(case, &input, &old, &configured)
                    .iter()
                    .any(|class| !is_configuration(class))
        } else {
            true
        };
        if !explained {
            unexplained.push(format!(
                "{case}@{index} {classes:?}\n  input:    {}\n  recorded: {frozen}\n  replayed: {}",
                record.fields[1],
                new.encode()
            ));
        }
        *counts
            .entry((case.to_owned(), classes.join("+")))
            .or_default() += 1;
    }
    assert!(
        unexplained.is_empty(),
        "{} disagreements fall in no documented class:\n{}",
        unexplained.len(),
        unexplained.join("\n")
    );
    assert_eq!(
        historical_digests, 11,
        "historical digests are provenance only"
    );
    let answer_digests: Vec<_> = replayed_answers
        .iter()
        .map(|(case, answers)| (case.as_str(), sha256_hex(answers)))
        .collect();
    let frozen_answer_digests: Vec<_> = REPLAYED_ANSWER_DIGESTS
        .iter()
        .map(|&(case, digest)| (case, digest.to_owned()))
        .collect();
    assert_eq!(answer_digests, frozen_answer_digests);
    let frozen: BTreeMap<(String, String), usize> = FROZEN_CLASS_COUNTS
        .iter()
        .map(|&(case, classes, count)| ((case.to_owned(), classes.to_owned()), count))
        .collect();
    if counts != frozen {
        let listing: Vec<String> = counts
            .iter()
            .map(|((case, classes), count)| format!("    ({case:?}, {classes:?}, {count}),"))
            .collect();
        panic!(
            "the class counts moved ({agreed} vectors agree); the replay now finds:\n{}",
            listing.join("\n")
        );
    }
    assert_eq!(agreed + counts.values().sum::<usize>(), 5_500);
}

fn append_replayed_answer(answers: &mut BTreeMap<String, Vec<u8>>, case: &str, encoded: &str) {
    let stream = answers.entry(case.to_owned()).or_default();
    stream.extend_from_slice(&(encoded.len() as u64).to_le_bytes());
    stream.extend_from_slice(encoded.as_bytes());
}

/// Generate valid records from fields, then spell their expected answer from
/// the RFC 4180 / CSVW length-prefixed answer format rather than using the
/// reader or a read/write round trip as the oracle.
#[test]
fn deterministic_spec_records_have_independent_expected_fields() {
    const CASES: &[&str] = &[
        "read-sssom-tsv",
        "read-lpg-csv",
        "read-rfc4180",
        "read-csvw-comma",
        "read-csvw-escape",
        "read-csvw-unquoted",
        "read-csvw-semicolon",
        "read-csvw-tab",
    ];
    for &case in CASES {
        let dialect = reader_dialect(case).expect("declared case");
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        for index in 0..512 {
            // Fixed LCG with all inputs derived here: unlike the 200,000
            // historical digest inputs, these cases can be replayed.
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let first = format!("a{:x}", state & 0xffff);
            let second = if index % 3 == 0 { "é" } else { "plain" };
            let third = if index % 5 == 0 && dialect.quote_char.is_some() {
                format!("x{}y", char::from(dialect.delimiter))
            } else {
                format!("z{}", (state >> 16) & 0xff)
            };
            let fields = [first.as_str(), second, third.as_str()];
            let mut wire = Vec::new();
            let mut expected = Vec::from(b"R".as_slice());
            for (field_index, field) in fields.iter().enumerate() {
                if field_index > 0 {
                    wire.push(dialect.delimiter);
                }
                let needs_quote = field.as_bytes().contains(&dialect.delimiter);
                if needs_quote {
                    wire.push(
                        dialect
                            .quote_char
                            .expect("quoted field has a quote character"),
                    );
                }
                wire.extend_from_slice(field.as_bytes());
                if needs_quote {
                    wire.push(
                        dialect
                            .quote_char
                            .expect("quoted field has a quote character"),
                    );
                }
                expected.extend_from_slice(field.len().to_string().as_bytes());
                expected.push(b':');
                expected.extend_from_slice(field.as_bytes());
            }
            expected.push(b'/');
            if !case.starts_with("read-csvw") {
                wire.extend_from_slice(if index % 2 == 0 { b"\r\n" } else { b"\n" });
            }
            let actual = read(dialect, &wire);
            assert_eq!(
                actual.encode(),
                encode_bytes(&expected),
                "{case} generated case {index}, wire {}",
                encode_bytes(&wire)
            );
        }
    }
}

#[test]
fn a_classified_lone_cr_does_not_excuse_corruption_in_another_field() {
    // The recorded csv engine splits at lone CR. RFC 4180 does not, so the
    // first cell must contain it verbatim, while the second cell must still
    // be exactly `safe`. A classifier-only check would also accept `evil`.
    let actual = read(Dialect::RFC4180, b"a\rb,safe\r\n");
    assert_eq!(actual.encode(), encode_bytes(b"R3:a\rb4:safe/"));
    assert_ne!(actual.encode(), encode_bytes(b"R3:a\rb4:evil/"));
}
