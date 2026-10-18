// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native files-profile metadata admission failures.

use core::fmt;
use purrdf_hash::error::{ErrorValue, IoFailure};
use std::path::PathBuf;

/// A malformed files-profile entry or metadata graph.
#[derive(Debug)]
#[non_exhaustive]
pub enum FileError {
    /// The native GTS writer refused the files-profile header.
    Writer(crate::writer::WriterError),
    /// No entry was supplied to the v2 writer.
    EmptyEntries,
    /// An archive path is empty.
    EmptyPath,
    /// An absolute or drive-relative archive path was supplied.
    AbsolutePath {
        /// Original spelling.
        path: String,
    },
    /// An archive path contains a parent traversal.
    TraversalPath {
        /// Original spelling.
        path: String,
    },
    /// An archive path uses backslash separators.
    BackslashPath {
        /// Original spelling.
        path: String,
    },
    /// An archive path has an empty or current-directory component.
    ComponentPath {
        /// Original spelling.
        path: String,
    },
    /// Multiple authoring entries have the same path.
    DuplicateArchivePath {
        /// Original path.
        path: String,
    },
    /// An I/O operation failed, retaining the original native cause.
    Io {
        /// Original operation label.
        operation: &'static str,
        /// Original native path, when the operation has one.
        path: Option<PathBuf>,
        /// Original I/O error.
        cause: IoFailure,
    },
    /// A source does not match its declared content digest.
    BlobDigest {
        /// Original source kind label.
        kind: &'static str,
        /// Original native path for a filesystem source.
        path: Option<PathBuf>,
        /// Declared digest.
        expected: String,
        /// Computed digest.
        actual: String,
    },
    /// A streaming source changed its declared byte length.
    BlobSize {
        /// Declared source byte count.
        expected: u64,
        /// Observed source byte count.
        actual: u64,
    },
    /// A streaming source changed between hashing and output.
    BlobChangedDigest {
        /// Previously computed digest.
        expected: String,
        /// Digest observed during output.
        actual: String,
    },
    /// A non-file entry carries regular-file payload fields.
    EntryPayload {
        /// Original entry path.
        path: String,
        /// Original entry kind.
        kind: super::FileEntryKind,
        /// Original inline byte count, if supplied.
        data_size: Option<usize>,
        /// Original declared digest, if supplied.
        digest: Option<String>,
        /// Original declared byte count, if supplied.
        size: Option<u64>,
    },
    /// A link entry has no usable target.
    LinkTarget {
        /// Original entry path.
        path: String,
        /// Original entry kind.
        kind: super::FileEntryKind,
    },
    /// A device entry lacks required device numbers.
    Device {
        /// Original entry path.
        path: String,
        /// Original entry kind.
        kind: super::FileEntryKind,
        /// Original major number.
        major: Option<u64>,
        /// Original minor number.
        minor: Option<u64>,
    },
    /// Inline entry bytes disagree with its declared digest.
    EntryDigest {
        /// Original entry path.
        path: String,
        /// Declared digest.
        expected: String,
        /// Computed digest.
        actual: String,
    },
    /// Inline entry bytes disagree with its declared length.
    EntrySize {
        /// Original entry path.
        path: String,
        /// Declared byte count.
        expected: u64,
        /// Actual inline byte count.
        actual: u64,
    },
    /// A regular file has neither inline data nor a complete digest/size pair.
    MissingPayload {
        /// Original entry path.
        path: String,
        /// Original declared digest.
        digest: Option<String>,
        /// Original declared byte count.
        size: Option<u64>,
    },
    /// An unknown entry kind.
    Kind {
        /// Original files:type spelling.
        actual: String,
    },
    /// A required archive vocabulary term is absent.
    MissingVocabulary {
        /// Original required term label.
        term: &'static str,
    },
    /// Multiple entries declare the same path.
    DuplicatePath {
        /// Original path.
        path: String,
    },
    /// An xattr or pax node has no collected fields.
    MissingFields {
        /// Original metadata role.
        role: &'static str,
        /// Original node ID.
        node: usize,
    },
    /// A required metadata field is absent.
    MissingField {
        /// Original metadata role.
        role: &'static str,
        /// Original node ID.
        node: usize,
        /// Required field name.
        field: &'static str,
    },
    /// A numeric field is not representable in its declared integer type.
    Integer {
        /// Original field name.
        field: String,
        /// Original field spelling.
        actual: String,
        /// Original standard integer parser cause.
        cause: core::num::ParseIntError,
    },
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Writer(cause) => write!(f, "cannot author files v2 header: {cause}"),
            Self::EmptyEntries => f.write_str("files-profile v2 archive needs at least one entry"),
            Self::EmptyPath => f.write_str("empty archive path"),
            Self::AbsolutePath { path } => write!(
                f,
                "absolute or drive-relative path not allowed in archive: {path}"
            ),
            Self::TraversalPath { path } => {
                write!(f, "path traversal not allowed in archive: {path}")
            }
            Self::BackslashPath { path } => {
                write!(f, "backslash path separator not allowed in archive: {path}")
            }
            Self::ComponentPath { path } => write!(
                f,
                "empty or current-directory path component not allowed in archive: {path}"
            ),
            Self::DuplicateArchivePath { path } => write!(f, "duplicate archive path: {path}"),
            Self::Io {
                operation,
                path: Some(path),
                cause,
            } => write!(f, "{operation} {path:?}: {cause}"),
            Self::Io {
                operation,
                path: None,
                cause,
            } => write!(f, "{operation}: {cause}"),
            Self::BlobDigest {
                path: Some(path),
                expected,
                actual,
                ..
            } => write!(
                f,
                "digest mismatch for {path:?}: expected {expected}, read {actual}"
            ),
            Self::BlobDigest {
                kind,
                path: None,
                expected,
                actual,
            } => write!(
                f,
                "digest mismatch for {kind}: expected {expected}, read {actual}"
            ),
            Self::BlobSize { expected, actual } => write!(
                f,
                "blob source changed size: expected {expected}, read {actual}"
            ),
            Self::BlobChangedDigest { expected, actual } => write!(
                f,
                "blob source changed digest: expected {expected}, read {actual}"
            ),
            Self::EntryPayload { path, kind, .. } => write!(
                f,
                "{} entry {path} must not carry file bytes, digest, or size",
                kind.as_str()
            ),
            Self::LinkTarget { path, kind } => {
                write!(f, "{} entry {path} needs linkTarget", kind.as_str())
            }
            Self::Device { path, kind, .. } => write!(
                f,
                "{} entry {path} needs devMajor and devMinor",
                kind.as_str()
            ),
            Self::EntryDigest { path, .. } => write!(f, "digest mismatch for {path}"),
            Self::EntrySize { path, .. } => write!(f, "size mismatch for {path}"),
            Self::MissingPayload { path, .. } => {
                write!(f, "regular file entry {path} needs data or digest+size")
            }
            Self::Kind { actual } => write!(f, "unknown files:type value: {actual}"),
            Self::MissingVocabulary { term } => {
                write!(f, "not a files-profile archive: missing {term}")
            }
            Self::DuplicatePath { path } => write!(f, "duplicate files:path in archive: {path}"),
            Self::MissingFields { role, node } => {
                write!(f, "files:{role} node {node} has no fields")
            }
            Self::MissingField { role, node, field } => {
                write!(f, "files:{role} node {node} missing {field}")
            }
            Self::Integer {
                field,
                actual,
                cause,
            } => write!(f, "invalid files:{field} integer {actual:?}: {cause}"),
        }
    }
}

purrdf_lex::error_source!(FileError => {
    Self::Writer(cause) => cause,
    Self::Integer { cause, .. } => cause,
    Self::Io { cause, .. } => cause,
});

impl purrdf_hash::error::StructuredError for FileError {
    purrdf_hash::error_visit! {
        Writer(cause) => "gts-files-writer" { "cause" => Cause(cause) },
        EmptyEntries => "gts-files-empty-entries" {},
        EmptyPath => "gts-files-empty-path" {},
        AbsolutePath { path } => "gts-files-absolute-path" { "path" => Text(path) },
        TraversalPath { path } => "gts-files-traversal-path" { "path" => Text(path) },
        BackslashPath { path } => "gts-files-backslash-path" { "path" => Text(path) },
        ComponentPath { path } => "gts-files-component-path" { "path" => Text(path) },
        DuplicateArchivePath { path } => "gts-files-duplicate-archive-path" { "path" => Text(path) },
        Io { operation, path, cause } => "gts-files-io" {
            "operation" => Code(operation),
            "path" => Value(path.as_ref().map_or(ErrorValue::Absent, |path| ErrorValue::Bytes(path.as_os_str().as_encoded_bytes()))),
            "path_encoding" => Code("os-encoded-bytes"),
            "cause" => Cause(cause)
        },
        BlobDigest { kind, path, expected, actual } => "gts-files-blob-digest" {
            "kind" => Code(kind),
            "path" => Value(path.as_ref().map_or(ErrorValue::Absent, |path| ErrorValue::Bytes(path.as_os_str().as_encoded_bytes()))),
            "path_encoding" => Code("os-encoded-bytes"),
            "expected" => Text(expected), "actual" => Text(actual)
        },
        BlobSize { expected, actual } => "gts-files-blob-size" { "expected" => ByteCount(u128::from(*expected)), "actual" => ByteCount(u128::from(*actual)) },
        BlobChangedDigest { expected, actual } => "gts-files-blob-changed-digest" { "expected" => Text(expected), "actual" => Text(actual) },
        EntryPayload { path, kind, data_size, digest, size } => "gts-files-entry-payload" {
            "path" => Text(path), "kind" => Code(kind.as_str()),
            "data_size" => Value(data_size.map_or(ErrorValue::Absent, |value| ErrorValue::ByteCount(value as u128))),
            "digest" => Value(digest.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
            "size" => Value(size.map_or(ErrorValue::Absent, |value| ErrorValue::ByteCount(u128::from(value))))
        },
        LinkTarget { path, kind } => "gts-files-link-target" { "path" => Text(path), "kind" => Code(kind.as_str()) },
        Device { path, kind, major, minor } => "gts-files-device" {
            "path" => Text(path), "kind" => Code(kind.as_str()),
            "major" => Value(major.map_or(ErrorValue::Absent, |value| ErrorValue::Unsigned(u128::from(value)))),
            "minor" => Value(minor.map_or(ErrorValue::Absent, |value| ErrorValue::Unsigned(u128::from(value))))
        },
        EntryDigest { path, expected, actual } => "gts-files-entry-digest" { "path" => Text(path), "expected" => Text(expected), "actual" => Text(actual) },
        EntrySize { path, expected, actual } => "gts-files-entry-size" { "path" => Text(path), "expected" => ByteCount(u128::from(*expected)), "actual" => ByteCount(u128::from(*actual)) },
        MissingPayload { path, digest, size } => "gts-files-missing-payload" {
            "path" => Text(path), "digest" => Value(digest.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
            "size" => Value(size.map_or(ErrorValue::Absent, |value| ErrorValue::ByteCount(u128::from(value))))
        },
        Kind { actual } => "gts-files-kind" { "actual" => Text(actual) },
        MissingVocabulary { term } => "gts-files-missing-vocabulary" { "term" => Code(term) },
        DuplicatePath { path } => "gts-files-duplicate-path" { "path" => Text(path) },
        MissingFields { role, node } => "gts-files-missing-fields" { "role" => Code(role), "node" => Unsigned(*node as u128) },
        MissingField { role, node, field } => "gts-files-missing-field" { "role" => Code(role), "node" => Unsigned(*node as u128), "field" => Code(field) },
        Integer { field, actual, cause } => "gts-files-integer" { "field" => Code(field), "actual" => Text(actual), "cause" => Cause(cause) },
    }
}
