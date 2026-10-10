// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! HTML character references, independently implemented from the WHATWG
//! reference submachine. This does not parse tags, change quotes, normalize
//! line endings, or recursively decode its output.
//!
//! [`resolve`] implements the specified recovery and reports every reference
//! error. [`resolve_strict`] makes acceptance atomic: an error returns only
//! diagnostics, never a partially decoded value. Diagnostics cover references
//! alone; literal controls and malformed HTML outside a reference are preserved.
//!
//! Named data is generated from the pinned WHATWG machine-readable table. Its
//! separate BSD-3-Clause notice and provenance live under `data/html/`.

use std::borrow::Cow;
use std::fmt;
use std::ops::Range;

mod entities;

/// The pinned HTML Standard revision defining the reference submachine.
pub const SPEC_SNAPSHOT: &str = "a5e15011a00ddefd648c29e4d27734f3e7ff821f";

/// Version of the resolver's semantics, independent of its table layout.
pub const REFERENCE_LAW: &str = "whatwg-reference-submachine/v1";

/// BLAKE3-256 of the exact pinned `entities.json` bytes, independent of layout.
pub const NAMED_TABLE_BLAKE3: [u8; 32] = entities::DATA_BLAKE3;

/// BLAKE3-256 of the numeric C1 recovery table's pinned JSON bytes.
pub const NUMERIC_TABLE_BLAKE3: [u8; 32] = entities::NUMERIC_BLAKE3;

/// Number of named spellings, including the 106 semicolonless legacy names.
pub const NAMED_REFERENCE_COUNT: usize = entities::ENTRIES.len();

/// How an ampersand is interpreted. All other text remains literal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Mode {
    /// No decoding, no diagnostics, and an identity source mapping.
    #[default]
    Plain,
    /// HTML text/RCDATA character-reference rules.
    Text,
    /// HTML attribute rules, including ambiguous legacy-name suppression.
    Attribute,
}

/// One complete decoded value and its original UTF-8 contributors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decoded<'a> {
    /// Borrowed when no reference was replaced; owned otherwise.
    pub text: Cow<'a, str>,
    /// One original UTF-8 range per output scalar, in output order.
    ///
    /// Empty means the identity mapping: `text` is the unmodified input, and
    /// callers needing ranges can enumerate its `char_indices()`. Otherwise
    /// every output scalar has an entry, including unchanged scalars. A
    /// replacement's scalar or two scalars contribute the entire reference
    /// spelling; unchanged scalars contribute only their own UTF-8 bytes.
    pub sources: Vec<Range<usize>>,
}

/// Recovery output plus all reference errors in encounter order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolution<'a> {
    /// Complete output, including the standard's recovery replacements.
    pub decoded: Decoded<'a>,
    /// All errors; a numeric reference can produce two, e.g. `&#0`.
    pub diagnostics: Vec<Diagnostic>,
}

/// The eight parse errors of the HTML character-reference submachine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// An unrecognized alphanumeric name followed by a semicolon.
    UnknownNamedCharacterReference,
    /// No digits after `&#`, `&#x`, or `&#X`.
    AbsenceOfDigitsInNumericCharacterReference,
    /// A recognized reference without a terminating semicolon.
    MissingSemicolonAfterCharacterReference,
    /// Numeric zero, recovered to U+FFFD.
    NullCharacterReference,
    /// A numeric value above U+10FFFF, recovered to U+FFFD.
    CharacterReferenceOutsideUnicodeRange,
    /// A numeric surrogate, recovered to U+FFFD.
    SurrogateCharacterReference,
    /// A Unicode noncharacter, preserved.
    NoncharacterCharacterReference,
    /// A disallowed referenced control, preserved or mapped by the C1 table.
    ControlCharacterReference,
}

impl ErrorKind {
    /// The WHATWG diagnostic code, suitable for machine-readable reports.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnknownNamedCharacterReference => "unknown-named-character-reference",
            Self::AbsenceOfDigitsInNumericCharacterReference => {
                "absence-of-digits-in-numeric-character-reference"
            }
            Self::MissingSemicolonAfterCharacterReference => {
                "missing-semicolon-after-character-reference"
            }
            Self::NullCharacterReference => "null-character-reference",
            Self::CharacterReferenceOutsideUnicodeRange => {
                "character-reference-outside-unicode-range"
            }
            Self::SurrogateCharacterReference => "surrogate-character-reference",
            Self::NoncharacterCharacterReference => "noncharacter-character-reference",
            Self::ControlCharacterReference => "control-character-reference",
        }
    }
}

/// A reference error and the complete consumed reference spelling's range.
///
/// The range includes a consumed semicolon; a missing-digit error covers only
/// the consumed introducer. Ranges always index the original UTF-8 input.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    /// Standard error classification.
    pub kind: ErrorKind,
    /// Original input bytes responsible for the diagnostic.
    pub source: Range<usize>,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at bytes {}..{}",
            self.kind.code(),
            self.source.start,
            self.source.end
        )
    }
}

impl std::error::Error for Diagnostic {}

// Names occupy one 16,641-byte ASCII arena. Entries have no pointers or
// target-width fields, and binary prefix narrowing examines at most 32 bytes.
struct Entry {
    first: char,
    second: char,
    start: u16,
    len: u8,
}

impl Entry {
    fn byte(&self, depth: usize) -> u8 {
        if depth < usize::from(self.len) {
            entities::NAMES[usize::from(self.start) + depth]
        } else {
            0
        }
    }
}

/// Resolve each reference exactly once, preserving all non-reference content.
///
/// The ordinary path uses the shared native ampersand scanner and returns a
/// borrowed value with no source-vector allocation. Lookup memory is constant;
/// numeric runs of arbitrary length use a capped accumulator and are consumed
/// completely. All offsets are byte positions in the original input.
#[must_use]
pub fn resolve(input: &str, mode: Mode) -> Resolution<'_> {
    let mut result = Resolution {
        decoded: Decoded {
            text: Cow::Borrowed(input),
            sources: Vec::new(),
        },
        diagnostics: Vec::new(),
    };
    if mode == Mode::Plain {
        return result;
    }
    let mut cursor = 0;
    let mut copied = 0;
    while let Some(relative) = crate::scan::find_byte(&input.as_bytes()[cursor..], b'&') {
        let start = cursor + relative;
        let (end, replacement) = reference(input, start, mode, &mut |diagnostic| {
            result.diagnostics.push(diagnostic);
        });
        if let Some((first, second)) = replacement {
            if matches!(result.decoded.text, Cow::Borrowed(_)) {
                result.decoded.text = Cow::Owned(String::with_capacity(input.len()));
            }
            append_original(&mut result.decoded, input, copied..start);
            let output = result.decoded.text.to_mut();
            output.push(first);
            result.decoded.sources.push(start..end);
            if second != '\0' {
                output.push(second);
                result.decoded.sources.push(start..end);
            }
            copied = end;
        }
        cursor = end;
    }
    if matches!(result.decoded.text, Cow::Owned(_)) {
        append_original(&mut result.decoded, input, copied..input.len());
    }
    result
}

/// Resolve references, refusing the entire value if any reference is erroneous.
///
/// # Errors
/// Returns every character-reference diagnostic, without decoded output.
pub fn resolve_strict(input: &str, mode: Mode) -> Result<Decoded<'_>, Vec<Diagnostic>> {
    let result = resolve(input, mode);
    if result.diagnostics.is_empty() {
        Ok(result.decoded)
    } else {
        Err(result.diagnostics)
    }
}

/// Exact counts produced by the existing reference reader, without allocation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResolutionLayout {
    /// Exact UTF-8 length of the decoded text.
    pub text_bytes: usize,
    /// Number of decoded scalar source ranges.
    pub sources: usize,
    /// Number of reference-resolution diagnostics.
    pub diagnostics: usize,
    /// Whether reference resolution replaces any source text.
    pub replaced: bool,
}

/// A caller destination was too small or a checked count overflowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolveIntoError {
    /// A caller-supplied destination has insufficient capacity.
    Capacity,
    /// The exact destination layout exceeds addressable allocation size.
    LayoutOverflow,
}

fn visit_resolution(
    input: &str,
    mode: Mode,
    mut scalar: impl FnMut(char, Range<usize>),
    mut diagnostic: impl FnMut(Diagnostic),
) -> bool {
    if mode == Mode::Plain {
        for (at, c) in input.char_indices() {
            scalar(c, at..at + c.len_utf8());
        }
        return false;
    }
    let mut cursor = 0;
    let mut copied = 0;
    let mut replaced = false;
    while let Some(relative) = crate::scan::find_byte(&input.as_bytes()[cursor..], b'&') {
        let start = cursor + relative;
        let (end, replacement) = reference(input, start, mode, &mut diagnostic);
        if let Some((first, second)) = replacement {
            for (relative, c) in input[copied..start].char_indices() {
                let at = copied + relative;
                scalar(c, at..at + c.len_utf8());
            }
            scalar(first, start..end);
            if second != '\0' {
                scalar(second, start..end);
            }
            copied = end;
            replaced = true;
        }
        cursor = end;
    }
    for (relative, c) in input[copied..].char_indices() {
        let at = copied + relative;
        scalar(c, at..at + c.len_utf8());
    }
    replaced
}

/// Inspect actual decoded bytes, scalar ranges and diagnostics at the native home.
pub fn resolution_layout(input: &str, mode: Mode) -> Result<ResolutionLayout, ResolveIntoError> {
    let mut text_bytes = Some(0usize);
    let mut sources = Some(0usize);
    let mut diagnostics = Some(0usize);
    let replaced = visit_resolution(
        input,
        mode,
        |c, _| {
            text_bytes = text_bytes.and_then(|n| n.checked_add(c.len_utf8()));
            sources = sources.and_then(|n| n.checked_add(1));
        },
        |_| {
            diagnostics = diagnostics.and_then(|n| n.checked_add(1));
        },
    );
    Ok(ResolutionLayout {
        text_bytes: if replaced {
            text_bytes.ok_or(ResolveIntoError::LayoutOverflow)?
        } else {
            0
        },
        sources: if replaced {
            sources.ok_or(ResolveIntoError::LayoutOverflow)?
        } else {
            0
        },
        diagnostics: diagnostics.ok_or(ResolveIntoError::LayoutOverflow)?,
        replaced,
    })
}

/// Fill caller-preallocated storage through the same reference reader.
/// The result moves those buffers, so their caller keeps the acquired grants.
pub fn resolve_preallocated(
    input: &str,
    mode: Mode,
    layout: ResolutionLayout,
    mut text: String,
    mut sources: Vec<Range<usize>>,
    mut diagnostics: Vec<Diagnostic>,
) -> Result<Resolution<'_>, ResolveIntoError> {
    if text.capacity() < layout.text_bytes
        || sources.capacity() < layout.sources
        || diagnostics.capacity() < layout.diagnostics
    {
        return Err(ResolveIntoError::Capacity);
    }
    text.clear();
    sources.clear();
    diagnostics.clear();
    let failed = core::cell::Cell::new(false);
    let replaced = visit_resolution(
        input,
        mode,
        |c, source| {
            if !layout.replaced {
                return;
            }
            if text
                .len()
                .checked_add(c.len_utf8())
                .is_none_or(|n| n > layout.text_bytes)
                || sources.len() == layout.sources
            {
                failed.set(true);
                return;
            }
            text.push(c);
            sources.push(source);
        },
        |diagnostic| {
            if diagnostics.len() == layout.diagnostics {
                failed.set(true);
                return;
            }
            diagnostics.push(diagnostic);
        },
    );
    if failed.get()
        || replaced != layout.replaced
        || text.len() != layout.text_bytes
        || sources.len() != layout.sources
        || diagnostics.len() != layout.diagnostics
    {
        return Err(ResolveIntoError::Capacity);
    }
    Ok(Resolution {
        decoded: Decoded {
            text: if replaced {
                Cow::Owned(text)
            } else {
                Cow::Borrowed(input)
            },
            sources,
        },
        diagnostics,
    })
}

fn append_original(decoded: &mut Decoded<'_>, input: &str, range: Range<usize>) {
    let original = &input[range.clone()];
    decoded.text.to_mut().push_str(original);
    decoded
        .sources
        .extend(original.char_indices().map(|(offset, c)| {
            let start = range.start + offset;
            start..start + c.len_utf8()
        }));
}

fn reference(
    input: &str,
    start: usize,
    mode: Mode,
    diagnostics: &mut impl FnMut(Diagnostic),
) -> (usize, Option<(char, char)>) {
    let bytes = input.as_bytes();
    let after = start + 1;
    if bytes.get(after) == Some(&b'#') {
        return numeric(bytes, start, diagnostics);
    }
    if !bytes.get(after).is_some_and(u8::is_ascii_alphanumeric) {
        return (after, None);
    }
    if let Some((len, entry)) = named(&bytes[after..]) {
        let end = after + len;
        if bytes[end - 1] != b';' {
            if mode == Mode::Attribute
                && bytes
                    .get(end)
                    .is_some_and(|byte| *byte == b'=' || byte.is_ascii_alphanumeric())
            {
                return (end, None);
            }
            diagnostics(Diagnostic {
                kind: ErrorKind::MissingSemicolonAfterCharacterReference,
                source: start..end,
            });
        }
        return (end, Some((entry.first, entry.second)));
    }
    let mut end = after;
    while bytes.get(end).is_some_and(u8::is_ascii_alphanumeric) {
        end += 1;
    }
    if bytes.get(end) == Some(&b';') {
        end += 1;
        diagnostics(Diagnostic {
            kind: ErrorKind::UnknownNamedCharacterReference,
            source: start..end,
        });
    }
    (end, None)
}

fn named(bytes: &[u8]) -> Option<(usize, &'static Entry)> {
    let mut candidates = entities::ENTRIES;
    let mut matched = None;
    for (depth, &byte) in bytes.iter().take(entities::MAX_NAME_LEN).enumerate() {
        let low = candidates.partition_point(|entry| entry.byte(depth) < byte);
        candidates = &candidates[low..];
        let high = candidates.partition_point(|entry| entry.byte(depth) == byte);
        candidates = &candidates[..high];
        let Some(first) = candidates.first() else {
            break;
        };
        if usize::from(first.len) == depth + 1 {
            matched = Some((depth + 1, first));
        }
        if byte == b';' {
            break;
        }
    }
    matched
}

fn numeric(
    bytes: &[u8],
    start: usize,
    diagnostics: &mut impl FnMut(Diagnostic),
) -> (usize, Option<(char, char)>) {
    let mut end = start + 2;
    let hexadecimal = matches!(bytes.get(end), Some(b'x' | b'X'));
    let radix = if hexadecimal { 16 } else { 10 };
    if hexadecimal {
        end += 1;
    }
    let first_digit = end;
    let mut value = 0_u32;
    while let Some(digit) = bytes
        .get(end)
        .and_then(|&byte| purrdf_hash::hex::nibble(byte))
        .filter(|&digit| u32::from(digit) < radix)
    {
        // Values above Unicode's ceiling are indistinguishable for recovery.
        // The cap keeps even an unbounded digit run within u32 arithmetic.
        value = (value * radix + u32::from(digit)).min(0x11_0000);
        end += 1;
    }
    if end == first_digit {
        diagnostics(Diagnostic {
            kind: ErrorKind::AbsenceOfDigitsInNumericCharacterReference,
            source: start..end,
        });
        return (end, None);
    }
    if bytes.get(end) == Some(&b';') {
        end += 1;
    } else {
        diagnostics(Diagnostic {
            kind: ErrorKind::MissingSemicolonAfterCharacterReference,
            source: start..end,
        });
    }
    let kind = match value {
        0 => Some(ErrorKind::NullCharacterReference),
        0x11_0000.. => Some(ErrorKind::CharacterReferenceOutsideUnicodeRange),
        0xD800..=0xDFFF => Some(ErrorKind::SurrogateCharacterReference),
        0xFDD0..=0xFDEF => Some(ErrorKind::NoncharacterCharacterReference),
        _ if value & 0xFFFF >= 0xFFFE => Some(ErrorKind::NoncharacterCharacterReference),
        1..=8 | 0xB | 0xD..=0x1F | 0x7F..=0x9F => Some(ErrorKind::ControlCharacterReference),
        _ => None,
    };
    if let Some(kind) = kind {
        diagnostics(Diagnostic {
            kind,
            source: start..end,
        });
        value = match kind {
            ErrorKind::NullCharacterReference
            | ErrorKind::CharacterReferenceOutsideUnicodeRange
            | ErrorKind::SurrogateCharacterReference => 0xFFFD,
            ErrorKind::ControlCharacterReference => c1_replacement(value),
            _ => value,
        };
    }
    (
        end,
        Some((
            char::from_u32(value).expect("recovered Unicode scalar"),
            '\0',
        )),
    )
}

// Numeric-reference recovery values, HTML Standard's end-state table.
const fn c1_replacement(value: u32) -> u32 {
    match value {
        0x80..=0x9F => entities::C1_REPLACEMENTS[(value - 0x80) as usize],
        _ => value,
    }
}
