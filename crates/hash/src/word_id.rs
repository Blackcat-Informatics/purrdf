// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Checked, reversible word references over [`Digest32`].
//!
//! Full `b3w1: ` forms contain 22 payload words and one check word. Abbreviated
//! `b3p1: ` forms contain one through 21 payload words and their own check.
//! A prefix is only a validated bit selector: it never supplies a scope or
//! resolves content. The immutable version-one alphabet and complete law are
//! specified in `docs/design/purrdf-word-identities.md`.
//!
//! Scalar caller-buffer operations allocate nothing and publish no bytes on
//! refusal. Owned output allocates once, fallibly. Batch callbacks borrow one
//! reusable output buffer, preserving input order and each original refusal.

use core::fmt;
use std::collections::TryReserveError;

use crate::hex::Digest32;

mod data;

/// Dictionary words, in their frozen ASCII lexical order.
pub const WORD_COUNT: usize = 4096;
/// Number of payload words in a complete identity.
pub const FULL_PAYLOAD_WORDS: usize = 22;
/// Maximum canonical text length, including header and check word.
pub const MAX_ENCODED_LEN: usize = 6 + 23 * data::MAX_WORD_LEN + 22;

/// A precise lexical, capacity or invocation refusal. Positions are byte offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Unsupported or malformed version/header, at its first differing byte.
    Header {
        /// Byte offset.
        offset: usize,
    },
    /// Noncanonical byte, empty word or separator.
    Syntax {
        /// Byte offset.
        offset: usize,
    },
    /// A lowercase word is outside the immutable alphabet.
    UnknownWord {
        /// Start byte offset.
        offset: usize,
        /// Zero-based word position.
        word: usize,
    },
    /// Wrong number of displayed words for the declared form.
    WordCount {
        /// End byte offset, or first excess word.
        offset: usize,
        /// Observed word count, stopping at the first excess word.
        actual: usize,
    },
    /// The final full-form payload word has nonzero low eight bits.
    Padding {
        /// Start byte offset of the final payload word.
        offset: usize,
    },
    /// The displayed check differs from the count-first field recurrence.
    Check {
        /// Start byte offset of the check word.
        offset: usize,
        /// Required dictionary index.
        expected: u16,
        /// Supplied dictionary index.
        actual: u16,
    },
    /// No output byte was written: supply at least the required capacity.
    Capacity {
        /// Required bytes.
        needed: usize,
        /// Supplied bytes.
        available: usize,
    },
    /// Payload count must be one through 22; 22 selects the full form.
    PayloadCount {
        /// Supplied payload count.
        actual: usize,
    },
    /// A caller requested a complete digest from an abbreviated selector.
    ExpectedFull,
}

impl crate::error::StructuredError for Error {
    crate::error_visit! {
        Header { offset } => "word-id-header" { "offset" => ByteCount(*offset as u128) },
        Syntax { offset } => "word-id-syntax" { "offset" => ByteCount(*offset as u128) },
        UnknownWord { offset, word } => "word-id-unknown-word" { "offset" => ByteCount(*offset as u128), "word" => Unsigned(*word as u128) },
        WordCount { offset, actual } => "word-id-word-count" { "offset" => ByteCount(*offset as u128), "actual" => Unsigned(*actual as u128) },
        Padding { offset } => "word-id-padding" { "offset" => ByteCount(*offset as u128) },
        Check { offset, expected, actual } => "word-id-check" { "offset" => ByteCount(*offset as u128), "expected" => Unsigned(u128::from(*expected)), "actual" => Unsigned(u128::from(*actual)) },
        Capacity { needed, available } => "word-id-capacity" { "needed" => ByteCount(*needed as u128), "available" => ByteCount(*available as u128) },
        PayloadCount { actual } => "word-id-payload-count" { "actual" => Unsigned(*actual as u128) },
        ExpectedFull => "word-id-expected-full" {},
    }
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Header { offset } => write!(
                out,
                "unsupported or malformed word ID header at byte {offset}"
            ),
            Self::Syntax { offset } => write!(
                out,
                "noncanonical word ID byte or separator at byte {offset}"
            ),
            Self::UnknownWord { offset, word } => {
                write!(out, "unknown dictionary word {word} at byte {offset}")
            }
            Self::WordCount { offset, actual } => write!(
                out,
                "word count reached {actual} at byte {offset}; full forms require 23, prefixes two through 22"
            ),
            Self::Padding { offset } => {
                write!(out, "nonzero full word ID padding at byte {offset}")
            }
            Self::Check {
                offset,
                expected,
                actual,
            } => write!(
                out,
                "word ID check mismatch at byte {offset}: expected index {expected}, received {actual}"
            ),
            Self::Capacity { needed, available } => write!(
                out,
                "word ID output requires {needed} bytes, received {available}"
            ),
            Self::PayloadCount { actual } => write!(
                out,
                "word ID payload count {actual} is outside one through 22"
            ),
            Self::ExpectedFull => {
                out.write_str("a full word ID is required; a prefix cannot reconstruct its digest")
            }
        }
    }
}
impl core::error::Error for Error {}

/// Owned output refusal, retaining the original allocator cause.
#[derive(Debug)]
pub struct AllocationError(pub TryReserveError);
impl fmt::Display for AllocationError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(out)
    }
}
impl core::error::Error for AllocationError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(&self.0)
    }
}
impl crate::error::StructuredError for AllocationError {
    fn visit<'a>(&'a self, visitor: &mut dyn crate::error::ErrorVisitor<'a>) {
        visitor.variant("word-id-allocation");
        visitor.value(Some("cause"), crate::error::ErrorValue::Cause(&self.0));
    }
}

/// Validated leading bits, with zeroed undisplayed bits. No dataset binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Prefix {
    lower: Digest32,
    words: u8,
}
impl Prefix {
    /// Number of retained digest bits (12 through 252).
    pub const fn bit_len(self) -> usize {
        self.words as usize * 12
    }
    /// Zero-filled lower bound suitable for a later digest-index range lookup.
    pub const fn lower_bound(self) -> Digest32 {
        self.lower
    }
    /// Compare only the displayed bits; uniqueness is a dataset owner's job.
    pub fn matches(self, digest: Digest32) -> bool {
        mask(digest, self.words as usize) == self.lower
    }
}

/// A complete content identity or a validated, unresolved bit selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reference {
    /// All 256 original digest bits.
    Full(Digest32),
    /// Checked leading bits, meaningful only under an external scope.
    Prefix(Prefix),
}

fn mask(digest: Digest32, words: usize) -> Digest32 {
    let mut bytes = digest.into_bytes();
    let bits = words * 12;
    let whole = bits / 8;
    let tail = bits % 8;
    if tail != 0 {
        bytes[whole] &= 0xff << (8 - tail);
    }
    bytes[whole + usize::from(tail != 0)..].fill(0);
    Digest32::new(bytes)
}

impl Reference {
    /// Select a checked prefix, or the ordinary full form at 22 payload words.
    pub fn from_digest(digest: Digest32, payload_words: usize) -> Result<Self, Error> {
        match payload_words {
            1..=21 => Ok(Self::Prefix(Prefix {
                lower: mask(digest, payload_words),
                words: payload_words as u8,
            })),
            22 => Ok(Self::Full(digest)),
            actual => Err(Error::PayloadCount { actual }),
        }
    }
    /// Number of payload words; the check contributes one additional word.
    pub const fn payload_words(self) -> usize {
        match self {
            Self::Full(_) => 22,
            Self::Prefix(prefix) => prefix.words as usize,
        }
    }
    /// Exact canonical byte length, including header, spaces and check word.
    pub fn encoded_len(self) -> usize {
        Prepared::new(self).len
    }
    /// Render transactionally into caller-owned bytes. The unused tail is unchanged.
    pub fn encode(self, output: &mut [u8]) -> Result<usize, Error> {
        let prepared = Prepared::new(self);
        if output.len() < prepared.len {
            return Err(Error::Capacity {
                needed: prepared.len,
                available: output.len(),
            });
        }
        let mut position = 0;
        prepared.parts(|part| {
            output[position..position + part.len()].copy_from_slice(part.as_bytes());
            position += part.len();
        });
        Ok(position)
    }
    /// Allocate exactly one owned output, retaining an original reserve failure.
    pub fn to_text(self) -> Result<String, AllocationError> {
        let prepared = Prepared::new(self);
        let mut output = String::new();
        output
            .try_reserve_exact(prepared.len)
            .map_err(AllocationError)?;
        prepared.parts(|part| output.push_str(part));
        Ok(output)
    }
}

// Multiplication by x modulo x^12 + x^6 + x^4 + x + 1.
const fn step(state: u16, digit: u16) -> u16 {
    let value = state ^ digit;
    ((value << 1) ^ if value & 0x800 != 0 { 0x053 } else { 0 }) & 0xfff
}

struct Prepared {
    indices: [u16; 23],
    payload: usize,
    full: bool,
    len: usize,
}
impl Prepared {
    fn new(reference: Reference) -> Self {
        let payload = reference.payload_words();
        let (digest, full) = match reference {
            Reference::Full(digest) => (digest, true),
            Reference::Prefix(prefix) => (prefix.lower, false),
        };
        let bytes = digest.as_bytes();
        let mut indices = [0_u16; 23];
        for (group, triple) in bytes[..30].as_chunks::<3>().0.iter().enumerate() {
            indices[group * 2] = (u16::from(triple[0]) << 4) | (u16::from(triple[1]) >> 4);
            indices[group * 2 + 1] = (u16::from(triple[1] & 15) << 8) | u16::from(triple[2]);
        }
        indices[20] = (u16::from(bytes[30]) << 4) | (u16::from(bytes[31]) >> 4);
        indices[21] = u16::from(bytes[31] & 15) << 8;
        let mut check = step(if full { 0xb31 } else { 0xb32 }, payload as u16);
        let mut len = 6 + payload;
        for index in &indices[..payload] {
            check = step(check, *index);
            len += word(*index).len();
        }
        indices[payload] = check;
        len += word(check).len();
        Self {
            indices,
            payload,
            full,
            len,
        }
    }
    fn parts(&self, mut emit: impl FnMut(&'static str)) {
        emit(if self.full { "b3w1: " } else { "b3p1: " });
        for (position, index) in self.indices[..=self.payload].iter().enumerate() {
            if position != 0 {
                emit(" ");
            }
            emit(word(*index));
        }
    }
}

/// Borrow the immutable dictionary entry for a 12-bit index.
pub fn dictionary_word(index: u16) -> Option<&'static str> {
    (usize::from(index) < WORD_COUNT).then(|| word(index))
}
/// Resolve complete dictionary bytes; a hash or prefix alone never admits a word.
pub fn dictionary_index(bytes: &[u8]) -> Option<u16> {
    data::lookup(bytes)
}
fn word(index: u16) -> &'static str {
    data::word(index)
}

/// Parse a complete strict canonical form, validating its check before returning.
pub fn decode(input: &[u8]) -> Result<Reference, Error> {
    let full = input.get(2) == Some(&b'w');
    let header = if full { b"b3w1: " } else { b"b3p1: " };
    if let Some(offset) = header
        .iter()
        .enumerate()
        .find_map(|(offset, byte)| (input.get(offset) != Some(byte)).then_some(offset))
    {
        return Err(Error::Header { offset });
    }
    let mut indices = [0_u16; 23];
    let mut offsets = [0_usize; 23];
    let mut count = 0;
    let mut cursor = 6;
    while cursor < input.len() {
        let start = cursor;
        if count == 23 {
            return Err(Error::WordCount {
                offset: start,
                actual: count + 1,
            });
        }
        while cursor < input.len() && input[cursor] != b' ' {
            if !input[cursor].is_ascii_lowercase() {
                return Err(Error::Syntax { offset: cursor });
            }
            cursor += 1;
        }
        if cursor == start {
            return Err(Error::Syntax { offset: cursor });
        }
        indices[count] = data::lookup(&input[start..cursor]).ok_or(Error::UnknownWord {
            offset: start,
            word: count,
        })?;
        offsets[count] = start;
        count += 1;
        if cursor < input.len() {
            cursor += 1;
            if cursor == input.len() {
                return Err(Error::Syntax { offset: cursor - 1 });
            }
        }
    }
    if (full && count != 23) || (!full && !(2..=22).contains(&count)) {
        return Err(Error::WordCount {
            offset: cursor,
            actual: count,
        });
    }
    let payload = count - 1;
    if full && indices[21] & 255 != 0 {
        return Err(Error::Padding {
            offset: offsets[21],
        });
    }
    let mut check = step(if full { 0xb31 } else { 0xb32 }, payload as u16);
    for index in &indices[..payload] {
        check = step(check, *index);
    }
    if check != indices[payload] {
        return Err(Error::Check {
            offset: offsets[payload],
            expected: check,
            actual: indices[payload],
        });
    }
    let mut bytes = [0; 32];
    let mut bits = 0_u32;
    let mut available = 0;
    let mut position = 0;
    for index in &indices[..payload] {
        bits = (bits << 12) | u32::from(*index);
        available += 12;
        while available >= 8 {
            available -= 8;
            if position < 32 {
                bytes[position] = (bits >> available) as u8;
                position += 1;
            }
        }
        bits &= (1 << available) - 1;
    }
    if available != 0 && position < 32 {
        bytes[position] = (bits << (8 - available)) as u8;
    }
    let digest = Digest32::new(bytes);
    Ok(if full {
        Reference::Full(digest)
    } else {
        Reference::Prefix(Prefix {
            lower: digest,
            words: payload as u8,
        })
    })
}

/// Decode a full form into caller storage, leaving it unchanged on any refusal.
pub fn decode_full(input: &[u8], output: &mut [u8; 32]) -> Result<(), Error> {
    let Reference::Full(digest) = decode(input)? else {
        return Err(Error::ExpectedFull);
    };
    *output = digest.into_bytes();
    Ok(())
}

/// A per-record refusal retaining its original codec error and zero-based index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatchError {
    /// Zero-based input record index.
    pub record: usize,
    /// Original codec refusal, whose offset is relative to this record.
    pub cause: Error,
}
impl fmt::Display for BatchError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "record {}: {}", self.record, self.cause)
    }
}
impl core::error::Error for BatchError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(&self.cause)
    }
}
impl crate::error::StructuredError for BatchError {
    fn visit<'a>(&'a self, visitor: &mut dyn crate::error::ErrorVisitor<'a>) {
        visitor.variant("word-id-batch");
        visitor.value(
            Some("record"),
            crate::error::ErrorValue::Unsigned(self.record as u128),
        );
        visitor.value(Some("cause"), crate::error::ErrorValue::Cause(&self.cause));
    }
}

/// Decode in input order without allocating results or losing per-record failures.
pub fn decode_batch<'a>(
    inputs: impl IntoIterator<Item = &'a [u8]>,
) -> impl Iterator<Item = Result<Reference, BatchError>> {
    inputs
        .into_iter()
        .enumerate()
        .map(|(record, input)| decode(input).map_err(|cause| BatchError { record, cause }))
}

/// Render in input order through one reusable caller buffer. Each borrowed result
/// expires before the next record; a callback failure stops before the next input.
pub fn encode_batch<E>(
    inputs: impl IntoIterator<Item = Reference>,
    buffer: &mut [u8],
    mut emit: impl FnMut(usize, Result<&[u8], BatchError>) -> Result<(), E>,
) -> Result<(), E> {
    for (record, reference) in inputs.into_iter().enumerate() {
        let result = reference
            .encode(buffer)
            .map(|len| &buffer[..len])
            .map_err(|cause| BatchError { record, cause });
        emit(record, result)?;
    }
    Ok(())
}
