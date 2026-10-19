// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original tar framing, metadata and I/O failures.

use crate::{codec::CodecError, files::FileError};
use core::fmt;
use purrdf_hash::error::{ErrorValue, IoFailure};

/// A tar import/export failure with its original native context.
#[derive(Debug)]
#[non_exhaustive]
pub enum TarError {
    /// Input transport decoding failed.
    Transport(crate::transport::TransportError),
    /// A member requires an import option that was not enabled.
    Refused {
        /// Original member path.
        path: String,
        /// Original member kind.
        kind: crate::files::FileEntryKind,
        /// Required command option.
        option: &'static str,
    },
    /// Socket import is unsupported.
    ImportSocket {
        /// Original member path.
        path: String,
    },
    /// An imported link lacks its target.
    ImportLinkTarget {
        /// Original member path.
        path: String,
    },
    /// Unsigned Unix seconds do not fit the formatter's signed input.
    MtimeConversion {
        /// Original seconds.
        seconds: u64,
        /// Original conversion refusal.
        cause: core::num::TryFromIntError,
    },
    /// An instant is outside the timestamp writer's grammar.
    MtimeFormat {
        /// Original seconds.
        seconds: u64,
        /// Original native formatter failure.
        cause: purrdf_xsd::rfc3339::FormatError,
    },
    /// Files-profile metadata was refused.
    Files(FileError),
    /// A stream operation failed.
    Io {
        /// Original operation label.
        operation: &'static str,
        /// Archive member path, when applicable.
        path: Option<String>,
        /// Original standard I/O error and native source.
        cause: IoFailure,
    },
    /// A transform could not encode the tar stream.
    Codec {
        /// Original operation label.
        operation: &'static str,
        /// Original codec error.
        cause: CodecError,
    },
    /// Sockets cannot be emitted in tar.
    Socket {
        /// Original member path.
        path: String,
    },
    /// A regular file has no digest.
    MissingDigest {
        /// Original member path.
        path: String,
    },
    /// A regular file's inline blob is absent.
    MissingBlob {
        /// Original member path.
        path: String,
        /// Original digest label.
        digest: String,
    },
    /// A regular file's blob could not be decoded.
    Blob {
        /// Original member path.
        path: String,
        /// Original codec error.
        cause: CodecError,
    },
    /// A link has no usable target.
    LinkTarget {
        /// Original member kind label.
        kind: String,
        /// Original member path.
        path: String,
    },
    /// A modified timestamp could not be parsed.
    Mtime {
        /// Original timestamp spelling.
        actual: String,
        /// Original RFC 3339 parser error.
        cause: purrdf_xsd::rfc3339::ParseError,
    },
    /// The parsed timestamp precedes the epoch.
    NegativeMtime {
        /// Original timestamp spelling.
        actual: String,
        /// Original signed Unix seconds.
        seconds: i64,
    },
    /// An unsigned number does not fit an octal field.
    NumericField {
        /// Original integer.
        actual: u64,
        /// Actual field byte width, including its terminator.
        width: usize,
    },
    /// A member's size field is malformed.
    InvalidSize {
        /// Original header byte offset.
        offset: u64,
        /// Original field bytes.
        actual: Vec<u8>,
    },
    /// A member body cannot fit within the available archive.
    BodyOverrun {
        /// Original body byte offset.
        offset: usize,
        /// Original declared body length.
        size: u64,
        /// Original archive byte count.
        available: usize,
    },
    /// An entry type is unsupported.
    EntryType {
        /// Original type byte.
        actual: u8,
        /// Original reported block number.
        block: u128,
    },
    /// No supported entry was found.
    EmptyArchive,
    /// Offset arithmetic would overflow u64.
    OffsetOverflow {
        /// Original starting byte offset.
        offset: u64,
        /// Body/header bytes being added.
        size: u64,
        /// Padding bytes being added.
        padding: u64,
    },
    /// A body cannot be indexed on this platform.
    BodyTooLarge {
        /// Original body byte count.
        size: u64,
    },
    /// The member path is empty.
    EmptyPath,
    /// The checksum field is malformed.
    InvalidChecksum {
        /// Original header byte offset.
        offset: u64,
        /// Original checksum field bytes.
        actual: Vec<u8>,
    },
    /// The checksum disagrees with the header.
    Checksum {
        /// Original header byte offset.
        offset: u64,
        /// Computed checksum.
        actual: u64,
        /// Declared checksum.
        expected: u64,
    },
    /// A pax record has no length delimiter.
    PaxLengthMissing {
        /// Original byte offset in the pax body.
        offset: usize,
        /// Number of bytes remaining.
        remaining: usize,
    },
    /// A pax length is not UTF-8.
    PaxLengthUtf8 {
        /// Original pax record byte offset.
        offset: usize,
        /// Original rejected length bytes.
        actual: Vec<u8>,
        /// Original UTF-8 decoder error.
        cause: core::str::Utf8Error,
    },
    /// A pax length is not representable as a platform-sized integer.
    PaxLengthInteger {
        /// Original length spelling.
        actual: String,
        /// Original integer parser error.
        cause: core::num::ParseIntError,
    },
    /// A pax record length is outside its body's bounds.
    PaxOverrun {
        /// Original byte offset in the pax body.
        offset: usize,
        /// Declared record length.
        length: usize,
        /// Original pax body byte count.
        available: usize,
    },
    /// A pax record is not UTF-8.
    PaxRecordUtf8 {
        /// Original record byte offset.
        offset: usize,
        /// Original rejected record bytes.
        actual: Vec<u8>,
        /// Original UTF-8 decoder error.
        cause: core::str::Utf8Error,
    },
    /// A pax record lacks its required newline.
    PaxNewline {
        /// Original record byte offset.
        offset: usize,
        /// Original record text.
        actual: String,
    },
    /// A pax record lacks its key/value delimiter.
    PaxEquals {
        /// Original record byte offset.
        offset: usize,
        /// Original record text, excluding its newline.
        actual: String,
    },
}

impl TarError {
    pub(crate) fn io(operation: &'static str, path: Option<&str>, cause: std::io::Error) -> Self {
        Self::Io {
            operation,
            path: path.map(str::to_owned),
            cause: cause.into(),
        }
    }
}

impl fmt::Display for TarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(cause) => write!(f, "decode tar input: {cause}"),
            Self::Refused { path, kind, option } => {
                write!(f, "refusing {} entry {path}: use {option}", kind.as_str())
            }
            Self::ImportSocket { path } => write!(f, "unsupported tar socket entry {path}"),
            Self::ImportLinkTarget { path } => write!(f, "link entry {path} has no link target"),
            Self::MtimeConversion { seconds, .. } => {
                write!(f, "invalid tar mtime {seconds}: out of range")
            }
            Self::MtimeFormat { seconds, cause } => {
                write!(f, "invalid tar mtime {seconds}: {cause}")
            }
            Self::Files(cause) => cause.fmt(f),
            Self::Io {
                operation,
                path: Some(path),
                cause,
            } => write!(f, "{operation} for {path}: {cause}"),
            Self::Io {
                operation,
                path: None,
                cause,
            } => write!(f, "{operation}: {cause}"),
            Self::Codec { operation, cause } => write!(f, "{operation}: {cause}"),
            Self::Socket { path } => write!(f, "tar cannot encode socket entry {path}"),
            Self::MissingDigest { path } => write!(f, "file entry {path} has no digest"),
            Self::MissingBlob { path, digest } => {
                write!(f, "missing inline blob for {path}: {digest}")
            }
            Self::Blob { path, cause } => write!(f, "decode blob for {path}: {cause}"),
            Self::LinkTarget { kind, path } => write!(f, "{kind} entry {path} has no link target"),
            Self::Mtime { actual, cause } => write!(f, "parse mtime {actual}: {cause}"),
            Self::NegativeMtime { actual, .. } => write!(f, "negative tar mtime: {actual}"),
            Self::NumericField { actual, width } => write!(
                f,
                "tar numeric value {actual} does not fit {width}-byte field"
            ),
            Self::InvalidSize { offset, .. } => write!(
                f,
                "tar entry at block {} has invalid size",
                offset / super::BLOCK as u64
            ),
            Self::BodyOverrun { .. } => f.write_str("tar member body overruns archive"),
            Self::EntryType { actual, block } => write!(
                f,
                "unsupported tar entry type {:?} at block {block}",
                char::from(*actual)
            ),
            Self::EmptyArchive => f.write_str("tar archive contains no entries"),
            Self::OffsetOverflow { .. } => f.write_str("tar offset overflow"),
            Self::BodyTooLarge { size } => write!(f, "tar member body too large: {size}"),
            Self::EmptyPath => f.write_str("empty tar path"),
            Self::InvalidChecksum { offset, .. } => write!(
                f,
                "tar entry at block {} has invalid checksum",
                offset / super::BLOCK as u64
            ),
            Self::Checksum {
                offset,
                actual,
                expected,
            } => write!(
                f,
                "tar entry at block {} has checksum {actual:o}, expected {expected:o}",
                offset / super::BLOCK as u64
            ),
            Self::PaxLengthMissing { .. } => f.write_str("malformed pax record length"),
            Self::PaxLengthUtf8 { .. } => f.write_str("pax record length is not UTF-8"),
            Self::PaxLengthInteger { actual, .. } => {
                write!(f, "invalid pax record length: {actual}")
            }
            Self::PaxOverrun { .. } => f.write_str("pax record overruns body"),
            Self::PaxRecordUtf8 { .. } => f.write_str("pax record is not UTF-8"),
            Self::PaxNewline { .. } => f.write_str("pax record missing newline"),
            Self::PaxEquals { .. } => f.write_str("pax record missing '='"),
        }
    }
}

purrdf_lex::error_source!(TarError => {
    Self::Transport(cause) => cause,
    Self::MtimeConversion { cause, .. } => cause,
    Self::MtimeFormat { cause, .. } => cause,
    Self::Files(cause) => cause,
    Self::Io { cause, .. } => cause,
    Self::Codec { cause, .. } => cause,
    Self::Blob { cause, .. } => cause,
    Self::Mtime { cause, .. } => cause,
    Self::PaxLengthUtf8 { cause, .. } => cause,
    Self::PaxLengthInteger { cause, .. } => cause,
    Self::PaxRecordUtf8 { cause, .. } => cause,
});

impl purrdf_hash::error::StructuredError for TarError {
    purrdf_hash::error_visit! {
        Transport(cause) => "gts-tar-transport" { "cause" => Cause(cause) },
        Refused { path, kind, option } => "gts-tar-refused" { "path" => Text(path), "kind" => Code(kind.as_str()), "option" => Code(option) },
        ImportSocket { path } => "gts-tar-import-socket" { "path" => Text(path) },
        ImportLinkTarget { path } => "gts-tar-import-link-target" { "path" => Text(path) },
        MtimeConversion { seconds, cause } => "gts-tar-mtime-conversion" { "seconds" => Unsigned(u128::from(*seconds)), "cause" => Cause(cause) },
        MtimeFormat { seconds, cause } => "gts-tar-mtime-format" { "seconds" => Unsigned(u128::from(*seconds)), "cause" => Cause(cause) },
        Files(cause) => "gts-tar-files" { "cause" => Cause(cause) },
        Io { operation, path, cause } => "gts-tar-io" { "operation" => Code(operation), "path" => Value(path.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)), "cause" => Cause(cause) },
        Codec { operation, cause } => "gts-tar-codec" { "operation" => Code(operation), "cause" => Cause(cause) },
        Socket { path } => "gts-tar-socket" { "path" => Text(path) },
        MissingDigest { path } => "gts-tar-missing-digest" { "path" => Text(path) },
        MissingBlob { path, digest } => "gts-tar-missing-blob" { "path" => Text(path), "digest" => Text(digest) },
        Blob { path, cause } => "gts-tar-blob" { "path" => Text(path), "cause" => Cause(cause) },
        LinkTarget { kind, path } => "gts-tar-link-target" { "kind" => Code(kind), "path" => Text(path) },
        Mtime { actual, cause } => "gts-tar-mtime" { "actual" => Text(actual), "cause" => Cause(cause) },
        NegativeMtime { actual, seconds } => "gts-tar-negative-mtime" { "actual" => Text(actual), "seconds" => Signed(i128::from(*seconds)) },
        NumericField { actual, width } => "gts-tar-numeric-field" { "actual" => Unsigned(u128::from(*actual)), "width" => ByteCount(*width as u128) },
        InvalidSize { offset, actual } => "gts-tar-invalid-size" { "offset" => ByteCount(u128::from(*offset)), "actual" => Bytes(actual) },
        BodyOverrun { offset, size, available } => "gts-tar-body-overrun" { "offset" => ByteCount(*offset as u128), "size" => ByteCount(u128::from(*size)), "available" => ByteCount(*available as u128) },
        EntryType { actual, block } => "gts-tar-entry-type" { "actual" => Unsigned(u128::from(*actual)), "block" => Unsigned(*block) },
        EmptyArchive => "gts-tar-empty-archive" {},
        OffsetOverflow { offset, size, padding } => "gts-tar-offset-overflow" { "offset" => ByteCount(u128::from(*offset)), "size" => ByteCount(u128::from(*size)), "padding" => ByteCount(u128::from(*padding)) },
        BodyTooLarge { size } => "gts-tar-body-too-large" { "size" => ByteCount(u128::from(*size)), "maximum" => ByteCount(usize::MAX as u128) },
        EmptyPath => "gts-tar-empty-path" {},
        InvalidChecksum { offset, actual } => "gts-tar-invalid-checksum" { "offset" => ByteCount(u128::from(*offset)), "actual" => Bytes(actual) },
        Checksum { offset, actual, expected } => "gts-tar-checksum" { "offset" => ByteCount(u128::from(*offset)), "actual" => Unsigned(u128::from(*actual)), "expected" => Unsigned(u128::from(*expected)) },
        PaxLengthMissing { offset, remaining } => "gts-tar-pax-length-missing" { "offset" => ByteCount(*offset as u128), "remaining" => ByteCount(*remaining as u128) },
        PaxLengthUtf8 { offset, actual, cause } => "gts-tar-pax-length-utf8" { "offset" => ByteCount(*offset as u128), "actual" => Bytes(actual), "cause" => Cause(cause) },
        PaxLengthInteger { actual, cause } => "gts-tar-pax-length-integer" { "actual" => Text(actual), "cause" => Cause(cause) },
        PaxOverrun { offset, length, available } => "gts-tar-pax-overrun" { "offset" => ByteCount(*offset as u128), "length" => ByteCount(*length as u128), "available" => ByteCount(*available as u128) },
        PaxRecordUtf8 { offset, actual, cause } => "gts-tar-pax-record-utf8" { "offset" => ByteCount(*offset as u128), "actual" => Bytes(actual), "cause" => Cause(cause) },
        PaxNewline { offset, actual } => "gts-tar-pax-newline" { "offset" => ByteCount(*offset as u128), "actual" => Text(actual) },
        PaxEquals { offset, actual } => "gts-tar-pax-equals" { "offset" => ByteCount(*offset as u128), "actual" => Text(actual) },
    }
}
