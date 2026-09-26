// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Frozen differential vectors: record an implementation's answers once, then
//! replay them against its replacement forever.
//!
//! When a first-party implementation replaces a third-party crate, the claim
//! "it answers exactly as the old one did" has to stay falsifiable after the
//! old crate is gone. A recorder test, run while the old crate is still a
//! dependency, writes the old crate's answers over a chosen input set into a
//! vector file; a permanent test replays that file against the replacement.
//! Only answers are taken from the old crate, never its code.
//!
//! # The file format
//!
//! A vector file is UTF-8 text in two parts:
//!
//! * **Header** — every line starting with `#`. Free-form comment lines state
//!   where the inputs and answers came from. Lines of the form
//!   `# <key>: <value>` are fields; two are required: `vector-count` (the
//!   number of body lines) and `body-sha256` (the lowercase hex SHA-256 of the
//!   body).
//! * **Body** — every other line, one record per line: tab-separated fields,
//!   each written in the field encoding below. The body the digest covers is
//!   those lines in file order, each terminated by `\n`, so it can be
//!   reproduced with `grep -v '^#' <file> | sha256sum`.
//!
//! Editing any record without regenerating the header fails [`VectorFile::parse`],
//! which is what makes the file tamper-evident: a disagreement between the
//! vectors and the replacement is a defect in the replacement, never a reason
//! to edit a vector.
//!
//! # The field encoding
//!
//! A field is its text, except that `\` is written `\\`, the empty field is
//! written as a lone `\0`, and a byte below `0x21` (space, tab, newline and
//! every other control), `#` and DEL are written `\xHH` with two lowercase
//! hex digits. Everything else — printable ASCII and non-ASCII UTF-8 — is
//! verbatim. [`encode_bytes`] additionally writes every byte of `0x80` and
//! above as `\xHH`, so arbitrary byte strings, valid UTF-8 or not, fit a
//! line. The encoding is the one `crates/iri/tests/langtag_differential_vectors.txt`
//! uses, extended with the `#` escape so no record can be mistaken for a
//! header line.

use std::fmt::{self, Write as _};

use sha2::{Digest as _, Sha256};

/// The header field naming the number of body records.
pub const COUNT_KEY: &str = "vector-count";
/// The header field carrying the body's SHA-256.
pub const DIGEST_KEY: &str = "body-sha256";
/// The encoding of the empty field.
pub const EMPTY_FIELD: &str = "\\0";

/// Why a vector file or field was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorError {
    /// A required header field is absent.
    MissingHeader(&'static str),
    /// A header field appears more than once.
    DuplicateHeader(String),
    /// `vector-count` is not a number.
    MalformedCount(String),
    /// `vector-count` disagrees with the body.
    CountMismatch {
        /// The header's count.
        declared: usize,
        /// The number of body lines.
        actual: usize,
    },
    /// `body-sha256` disagrees with the body.
    DigestMismatch {
        /// The header's digest.
        declared: String,
        /// The digest of the body as it stands.
        actual: String,
    },
    /// A field holds a malformed escape.
    MalformedField {
        /// The encoded field.
        field: String,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// A record handed to the [`Recorder`] has no fields.
    EmptyRecord,
    /// A header key or value cannot be written on one header line.
    MalformedHeader(String),
}

impl fmt::Display for VectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHeader(key) => write!(f, "the vector header must declare `{key}`"),
            Self::DuplicateHeader(key) => write!(f, "the vector header declares `{key}` twice"),
            Self::MalformedCount(value) => {
                write!(f, "`{COUNT_KEY}` must be a decimal count, found {value:?}")
            }
            Self::CountMismatch { declared, actual } => write!(
                f,
                "`{COUNT_KEY}` declares {declared} records but the body holds {actual}"
            ),
            Self::DigestMismatch { declared, actual } => write!(
                f,
                "`{DIGEST_KEY}` declares {declared} but the body hashes to {actual}: a record \
                 was edited, or the vectors were regenerated without their header"
            ),
            Self::MalformedField { field, reason } => {
                write!(f, "malformed vector field {field:?}: {reason}")
            }
            Self::EmptyRecord => f.write_str("a vector record needs at least one field"),
            Self::MalformedHeader(text) => {
                write!(f, "header text {text:?} does not fit one header line")
            }
        }
    }
}

impl std::error::Error for VectorError {}

/// One body record: its 1-based line number and its encoded fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record<'a> {
    /// The 1-based line number in the file.
    pub line: usize,
    /// The record's fields, still encoded.
    pub fields: Vec<&'a str>,
}

/// A parsed, digest-verified vector file.
#[derive(Debug, Clone)]
pub struct VectorFile<'a> {
    headers: Vec<(&'a str, &'a str)>,
    records: Vec<Record<'a>>,
}

impl<'a> VectorFile<'a> {
    /// Parse `text` and verify its `vector-count` and `body-sha256` against
    /// its body.
    pub fn parse(text: &'a str) -> Result<Self, VectorError> {
        let mut headers: Vec<(&'a str, &'a str)> = Vec::new();
        let mut records = Vec::new();
        let mut body = String::with_capacity(text.len());
        for (index, line) in text.lines().enumerate() {
            if let Some(comment) = line.strip_prefix('#') {
                if let Some((key, value)) = comment
                    .strip_prefix(' ')
                    .and_then(|field| field.split_once(": "))
                    .filter(|(key, _)| is_header_key(key))
                {
                    if headers.iter().any(|(seen, _)| *seen == key) {
                        return Err(VectorError::DuplicateHeader(key.to_owned()));
                    }
                    headers.push((key, value));
                }
                continue;
            }
            body.push_str(line);
            body.push('\n');
            records.push(Record {
                line: index + 1,
                fields: line.split('\t').collect(),
            });
        }
        let file = Self { headers, records };
        let declared_count = file
            .header(COUNT_KEY)
            .ok_or(VectorError::MissingHeader(COUNT_KEY))?;
        let declared_count: usize = declared_count
            .parse()
            .map_err(|_| VectorError::MalformedCount(declared_count.to_owned()))?;
        if declared_count != file.records.len() {
            return Err(VectorError::CountMismatch {
                declared: declared_count,
                actual: file.records.len(),
            });
        }
        let declared_digest = file
            .header(DIGEST_KEY)
            .ok_or(VectorError::MissingHeader(DIGEST_KEY))?;
        let actual_digest = sha256_hex(body.as_bytes());
        if declared_digest != actual_digest {
            return Err(VectorError::DigestMismatch {
                declared: declared_digest.to_owned(),
                actual: actual_digest,
            });
        }
        Ok(file)
    }

    /// The value of the header field `key`.
    pub fn header(&self, key: &str) -> Option<&'a str> {
        self.headers
            .iter()
            .find_map(|(seen, value)| (*seen == key).then_some(*value))
    }

    /// The body records, in file order.
    pub fn records(&self) -> &[Record<'a>] {
        &self.records
    }

    /// Replay every record: the first `inputs` fields go to `answer`, whose
    /// returned fields must equal the record's remaining fields exactly.
    ///
    /// Returns the number of records replayed, or a [`ReplayMismatch`] naming
    /// the first record that differed and counting the rest. Both sides are
    /// compared encoded, so `answer` encodes what it computes with
    /// [`encode_str`] or [`encode_bytes`].
    pub fn replay<F>(&self, inputs: usize, mut answer: F) -> Result<usize, ReplayMismatch>
    where
        F: FnMut(&[&str]) -> Vec<String>,
    {
        let mut first: Option<FirstMismatch> = None;
        let mut mismatches = 0usize;
        for record in &self.records {
            if record.fields.len() < inputs {
                mismatches += 1;
                first.get_or_insert_with(|| FirstMismatch {
                    line: record.line,
                    inputs: record
                        .fields
                        .iter()
                        .map(|&field| field.to_owned())
                        .collect(),
                    expected: Vec::new(),
                    actual: vec![format!(
                        "<the record has {} fields, fewer than the {inputs} inputs>",
                        record.fields.len()
                    )],
                });
                continue;
            }
            let (given, expected) = record.fields.split_at(inputs);
            let actual = answer(given);
            if actual
                .iter()
                .map(String::as_str)
                .ne(expected.iter().copied())
            {
                mismatches += 1;
                first.get_or_insert_with(|| FirstMismatch {
                    line: record.line,
                    inputs: given.iter().map(|&field| field.to_owned()).collect(),
                    expected: expected.iter().map(|&field| field.to_owned()).collect(),
                    actual,
                });
            }
        }
        match first {
            None => Ok(self.records.len()),
            Some(first) => Err(ReplayMismatch {
                first,
                mismatches,
                records: self.records.len(),
            }),
        }
    }
}

/// The first record a replay disagreed with.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FirstMismatch {
    line: usize,
    inputs: Vec<String>,
    expected: Vec<String>,
    actual: Vec<String>,
}

/// A replay that disagreed with at least one record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayMismatch {
    first: FirstMismatch,
    mismatches: usize,
    records: usize,
}

impl ReplayMismatch {
    /// The 1-based line of the first differing record.
    pub fn first_line(&self) -> usize {
        self.first.line
    }

    /// How many records differed.
    pub fn mismatches(&self) -> usize {
        self.mismatches
    }
}

impl fmt::Display for ReplayMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} of {} frozen vectors disagree; the first is shown.\n  \
             line {}: inputs {:?}\n      frozen answer:  {:?}\n      replayed answer: {:?}\n\
             A disagreement is a defect in the implementation under test. \
             Do not edit the vectors to match it.",
            self.mismatches,
            self.records,
            self.first.line,
            self.first.inputs,
            self.first.expected,
            self.first.actual,
        )
    }
}

impl std::error::Error for ReplayMismatch {}

/// Builds a vector file: comment lines, extra header fields, and records.
#[derive(Debug, Clone, Default)]
pub struct Recorder {
    comments: Vec<String>,
    headers: Vec<(String, String)>,
    body: String,
    count: usize,
}

impl Recorder {
    /// An empty recorder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a free-form header comment line (without its leading `# `).
    pub fn comment(&mut self, line: &str) -> Result<&mut Self, VectorError> {
        if line.contains(['\n', '\r']) {
            return Err(VectorError::MalformedHeader(line.to_owned()));
        }
        self.comments.push(line.to_owned());
        Ok(self)
    }

    /// Append a `# <key>: <value>` header field. `vector-count` and
    /// `body-sha256` are written by [`Recorder::render`] and refused here.
    pub fn header(&mut self, key: &str, value: &str) -> Result<&mut Self, VectorError> {
        if !is_header_key(key) || key == COUNT_KEY || key == DIGEST_KEY {
            return Err(VectorError::MalformedHeader(key.to_owned()));
        }
        if value.contains(['\n', '\r']) {
            return Err(VectorError::MalformedHeader(value.to_owned()));
        }
        if self.headers.iter().any(|(seen, _)| seen == key) {
            return Err(VectorError::DuplicateHeader(key.to_owned()));
        }
        self.headers.push((key.to_owned(), value.to_owned()));
        Ok(self)
    }

    /// Append one record of already-encoded fields.
    pub fn record<S: AsRef<str>>(&mut self, fields: &[S]) -> Result<&mut Self, VectorError> {
        if fields.is_empty() {
            return Err(VectorError::EmptyRecord);
        }
        for field in fields {
            let field = field.as_ref();
            if decode(field, Encoding::Text).is_err() {
                decode(field, Encoding::Bytes)?;
            }
        }
        for (index, field) in fields.iter().enumerate() {
            if index > 0 {
                self.body.push('\t');
            }
            self.body.push_str(field.as_ref());
        }
        self.body.push('\n');
        self.count += 1;
        Ok(self)
    }

    /// The finished file: comments, the extra headers, `vector-count`,
    /// `body-sha256`, then the body.
    pub fn render(&self) -> String {
        let mut text = String::with_capacity(self.body.len() + 256);
        for comment in &self.comments {
            if comment.is_empty() {
                text.push_str("#\n");
            } else {
                let _ = writeln!(text, "# {comment}");
            }
        }
        for (key, value) in &self.headers {
            let _ = writeln!(text, "# {key}: {value}");
        }
        let _ = writeln!(text, "# {COUNT_KEY}: {}", self.count);
        let _ = writeln!(text, "# {DIGEST_KEY}: {}", sha256_hex(self.body.as_bytes()));
        text.push_str(&self.body);
        text
    }
}

/// A header key is a non-empty run of lowercase ASCII letters, digits and `-`.
fn is_header_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// Whether `byte` is written as `\xHH` in a text field.
const fn escaped_in_text(byte: u8) -> bool {
    byte < 0x21 || byte == b'#' || byte == 0x7f
}

/// Encode a text field.
pub fn encode_str(text: &str) -> String {
    if text.is_empty() {
        return EMPTY_FIELD.to_owned();
    }
    let mut encoded = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\\' => encoded.push_str("\\\\"),
            ascii if ascii.is_ascii() && escaped_in_text(ascii as u8) => {
                let _ = write!(encoded, "\\x{:02x}", ascii as u8);
            }
            other => encoded.push(other),
        }
    }
    encoded
}

/// Encode a byte-string field: [`encode_str`]'s rules, with every byte of
/// `0x80` and above also written `\xHH`.
pub fn encode_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return EMPTY_FIELD.to_owned();
    }
    let mut encoded = String::with_capacity(bytes.len());
    for &byte in bytes {
        match byte {
            b'\\' => encoded.push_str("\\\\"),
            byte if escaped_in_text(byte) || byte >= 0x80 => {
                let _ = write!(encoded, "\\x{byte:02x}");
            }
            byte => encoded.push(char::from(byte)),
        }
    }
    encoded
}

/// Decode a byte-string field.
pub fn decode_bytes(field: &str) -> Result<Vec<u8>, VectorError> {
    decode(field, Encoding::Bytes)
}

/// Decode a text field. A `\xHH` escape above `0x7f` is refused: text fields
/// write non-ASCII verbatim.
pub fn decode_str(field: &str) -> Result<String, VectorError> {
    let bytes = decode(field, Encoding::Text)?;
    String::from_utf8(bytes).map_err(|_| VectorError::MalformedField {
        field: field.to_owned(),
        reason: "the decoded field is not UTF-8",
    })
}

/// Which of the two field encodings a field is decoded under.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Encoding {
    /// [`encode_str`]: non-ASCII verbatim.
    Text,
    /// [`encode_bytes`]: every byte of `0x80` and above escaped.
    Bytes,
}

/// Decode a field. A byte its encoding escapes but the field carries raw, or
/// one it writes raw but the field escapes, is refused, so every value has
/// exactly one spelling.
fn decode(field: &str, encoding: Encoding) -> Result<Vec<u8>, VectorError> {
    if field == EMPTY_FIELD {
        return Ok(Vec::new());
    }
    let malformed = |reason| VectorError::MalformedField {
        field: field.to_owned(),
        reason,
    };
    if field.is_empty() {
        return Err(malformed("the empty field is written `\\0`"));
    }
    let bytes = field.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte != b'\\' {
            if escaped_in_text(byte) {
                return Err(malformed(
                    "a space, control, `#` or DEL byte must be `\\xHH`",
                ));
            }
            if byte >= 0x80 && encoding == Encoding::Bytes {
                return Err(malformed("a byte-string field writes non-ASCII as `\\xHH`"));
            }
            decoded.push(byte);
            index += 1;
            continue;
        }
        match bytes.get(index + 1) {
            Some(b'\\') => {
                decoded.push(b'\\');
                index += 2;
            }
            Some(b'x') => {
                let digits = bytes
                    .get(index + 2..index + 4)
                    .ok_or_else(|| malformed("`\\x` needs two hex digits"))?;
                let high = hex_value(digits[0])
                    .ok_or_else(|| malformed("`\\x` needs lowercase hex digits"))?;
                let low = hex_value(digits[1])
                    .ok_or_else(|| malformed("`\\x` needs lowercase hex digits"))?;
                let value = (high << 4) | low;
                if value >= 0x80 && encoding == Encoding::Text {
                    return Err(malformed(
                        "a text field writes non-ASCII verbatim, not `\\xHH`",
                    ));
                }
                if value < 0x80 && !escaped_in_text(value) {
                    return Err(malformed(
                        "`\\xHH` spells a byte the encoding writes verbatim",
                    ));
                }
                decoded.push(value);
                index += 4;
            }
            Some(b'0') => return Err(malformed("`\\0` is only valid as the whole field")),
            _ => return Err(malformed("unknown escape")),
        }
    }
    Ok(decoded)
}

const fn hex_value(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    }
}

/// The lowercase hex SHA-256 of `data`.
pub fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}
