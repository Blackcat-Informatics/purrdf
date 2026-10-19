// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native filesystem and extraction refusals, before human presentation.

use super::FileEntryKind;
use core::fmt;
use purrdf_hash::error::{ErrorValue, IoFailure};
use std::path::PathBuf;

/// A filesystem input or extraction policy refused the original request.
#[derive(Debug)]
#[non_exhaustive]
pub enum FsRefusal {
    /// A native path cannot be represented by the archive's UTF-8 spelling.
    NonUtf8Path {
        /// Original native path.
        path: PathBuf,
    },
    /// A filesystem source is a symlink.
    Symlink {
        /// Original native path.
        path: PathBuf,
    },
    /// A source has no usable UTF-8 file name.
    InvalidSourceName {
        /// Original native path.
        path: PathBuf,
    },
    /// A source is outside its declared base directory.
    OutsideSource {
        /// Original source.
        path: PathBuf,
        /// Declared base.
        base: PathBuf,
    },
    /// The source is neither a file nor a directory.
    UnsupportedSource {
        /// Original native path.
        path: PathBuf,
    },
    /// A comparison file is outside the directory being compared.
    OutsideDirectory {
        /// Original file.
        path: PathBuf,
        /// Declared directory.
        base: PathBuf,
    },
    /// An existing ancestor redirects an extraction outside the destination.
    DestinationEscape {
        /// Original archive path.
        path: String,
        /// Canonical destination.
        root: PathBuf,
        /// Canonical existing ancestor.
        ancestor: PathBuf,
    },
    /// An output target has no parent.
    MissingParent {
        /// Original target.
        path: PathBuf,
    },
    /// An output target has no file name.
    MissingFileName {
        /// Original target.
        path: PathBuf,
    },
    /// All exclusive temporary-name attempts collided.
    TempAttempts {
        /// Original output target.
        path: PathBuf,
        /// Attempts made.
        attempts: usize,
    },
    /// Replacing this target would follow an existing symlink.
    WriteThroughSymlink {
        /// Original archive path.
        archive_path: String,
        /// Original output target.
        path: PathBuf,
    },
    /// Creating a node would replace an existing directory.
    ReplaceDirectory {
        /// Original archive path.
        archive_path: String,
        /// Original output target.
        path: PathBuf,
    },
    /// A link target would escape the extraction root.
    SymlinkEscape {
        /// Original archive path.
        path: String,
        /// Original target spelling.
        target: String,
    },
    /// A link target uses backslash separators.
    SymlinkBackslash {
        /// Original archive path.
        path: String,
        /// Original target spelling.
        target: String,
    },
    /// A link target resolves to the extraction root itself.
    SymlinkRoot {
        /// Original archive path.
        path: String,
        /// Original target spelling.
        target: String,
    },
    /// A non-link entry reached link materialization.
    NotLink {
        /// Original archive path.
        path: String,
        /// Original entry kind.
        kind: FileEntryKind,
    },
    /// This portable core does not create the requested native node kind.
    UnsupportedExtraction {
        /// Original archive path.
        path: String,
        /// Original entry kind.
        kind: FileEntryKind,
    },
    /// This portable core does not restore native ownership.
    UnsupportedOwner {
        /// Original archive path.
        path: String,
        /// Original user ID.
        uid: Option<u64>,
        /// Original group ID.
        gid: Option<u64>,
    },
    /// A regular file has no declared digest.
    MissingDigest {
        /// Original archive path.
        path: String,
    },
    /// A declared inline blob is absent.
    MissingBlob {
        /// Original archive path.
        path: String,
        /// Original digest label.
        digest: String,
    },
    /// An inline blob's native decoder refused its payload.
    BlobDecode {
        /// Original archive path.
        path: String,
        /// Original native decoder failure.
        cause: crate::codec::CodecError,
    },
    /// Extracted bytes disagree with the declared digest.
    Integrity {
        /// Original archive path.
        path: String,
        /// Declared digest label.
        digest: String,
        /// Computed digest label.
        actual: String,
    },
    /// The requested node kind requires an option that was not enabled.
    Refused {
        /// Original archive path.
        path: String,
        /// Original entry kind.
        kind: FileEntryKind,
        /// Required command option.
        option: &'static str,
    },
    /// The filesystem modification instant precedes the epoch.
    DateTimeBeforeEpoch {
        /// Original native source path.
        path: PathBuf,
        /// Original standard-library refusal.
        cause: std::time::SystemTimeError,
    },
    /// Unsigned filesystem seconds cannot fit the signed formatter input.
    DateTimeRange {
        /// Original native source path.
        path: PathBuf,
        /// Original unsigned seconds.
        seconds: u64,
        /// Original integer conversion error.
        cause: core::num::TryFromIntError,
    },
    /// The instant cannot be written in the profile's timestamp grammar.
    DateTimeFormat {
        /// Original native source path.
        path: PathBuf,
        /// Original unsigned seconds.
        seconds: u64,
        /// Original native formatter failure.
        cause: purrdf_xsd::rfc3339::FormatError,
    },
    /// An archived timestamp cannot be parsed.
    ParseModified {
        /// Original spelling.
        actual: String,
        /// Original native parser failure.
        cause: purrdf_xsd::rfc3339::ParseError,
    },
    /// The parsed instant cannot be represented by the host.
    MtimeHost {
        /// Original spelling.
        actual: String,
        /// Parsed whole Unix seconds.
        seconds: i64,
        /// Parsed nanoseconds.
        nanos: u32,
    },
    /// The comparison directory does not exist.
    MissingDirectory {
        /// Original native directory.
        path: PathBuf,
    },
    /// Native hard-link creation failed for this source and target.
    HardLinkIo {
        /// Original output target.
        target: PathBuf,
        /// Original source target.
        source: PathBuf,
        /// Original I/O failure.
        cause: IoFailure,
    },
}

impl fmt::Display for FsRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonUtf8Path { path } => write!(f, "non-UTF-8 path component in {path:?}"),
            Self::Symlink { path } => write!(f, "symlink not supported: {}", path.display()),
            Self::InvalidSourceName { path } => write!(f, "invalid source name: {path:?}"),
            Self::OutsideSource { path, .. } => write!(f, "path outside source: {path:?}"),
            Self::UnsupportedSource { path } => write!(f, "unsupported source type: {path:?}"),
            Self::OutsideDirectory { path, .. } => write!(f, "path outside directory: {path:?}"),
            Self::DestinationEscape { path, .. } => write!(f, "path escapes destination: {path}"),
            Self::MissingParent { path } => write!(f, "missing parent for {path:?}"),
            Self::MissingFileName { path } => write!(f, "missing file name for {path:?}"),
            Self::TempAttempts { path, .. } => {
                write!(f, "create temp file for {path:?}: too many attempts")
            }
            Self::WriteThroughSymlink { archive_path, path } => write!(
                f,
                "refusing to write through symlink for {archive_path}: {path:?}"
            ),
            Self::ReplaceDirectory { archive_path, path } => write!(
                f,
                "refusing to replace directory for {archive_path}: {path:?}"
            ),
            Self::SymlinkEscape { path, target } => {
                write!(f, "symlink target escapes destination for {path}: {target}")
            }
            Self::SymlinkBackslash { path, target } => write!(
                f,
                "backslash symlink target not allowed for {path}: {target}"
            ),
            Self::SymlinkRoot { path, target } => write!(
                f,
                "symlink target resolves to destination root for {path}: {target}"
            ),
            Self::NotLink { path, .. } => write!(f, "{path} is not a link entry"),
            Self::UnsupportedExtraction { path, kind } => write!(
                f,
                "{} extraction is host-specific and is not performed by the portable core: {path}",
                kind.as_str()
            ),
            Self::UnsupportedOwner { path, .. } => write!(
                f,
                "ownership restoration is host-specific and is not performed by the portable core: {path}"
            ),
            Self::MissingDigest { path } => write!(f, "missing digest for {path}"),
            Self::MissingBlob { path, digest } => {
                write!(f, "missing inline blob for {path}: {digest}")
            }
            Self::BlobDecode { path, cause } => {
                write!(f, "decode inline blob for {path}: {cause:?}")
            }
            Self::Integrity { path, digest, .. } => {
                write!(f, "integrity failure for {path}: {digest}")
            }
            Self::Refused { path, kind, option } => {
                write!(f, "refusing {} entry {path}: use {option}", kind.as_str())
            }
            Self::DateTimeBeforeEpoch { path, cause } => {
                write!(f, "datetime {path:?}: mtime before unix epoch: {cause}")
            }
            Self::DateTimeRange { path, seconds, .. } => {
                write!(
                    f,
                    "datetime {path:?}: invalid mtime timestamp {seconds}: out of range"
                )
            }
            Self::DateTimeFormat {
                path,
                seconds,
                cause,
            } => {
                write!(
                    f,
                    "datetime {path:?}: invalid mtime timestamp {seconds}: {cause}"
                )
            }
            Self::ParseModified { actual, cause } => write!(f, "parse datetime {actual}: {cause}"),
            Self::MtimeHost { actual, .. } => {
                write!(f, "mtime {actual} is not representable on this host")
            }
            Self::MissingDirectory { path } => {
                write!(f, "diff destination does not exist: {path:?}")
            }
            Self::HardLinkIo {
                target,
                source,
                cause,
            } => write!(f, "create hardlink {target:?} -> {source:?}: {cause}"),
        }
    }
}

purrdf_lex::error_source!(FsRefusal => {
    Self::BlobDecode { cause, .. } => cause,
    Self::DateTimeBeforeEpoch { cause, .. } => cause,
    Self::DateTimeRange { cause, .. } => cause,
    Self::DateTimeFormat { cause, .. } => cause,
    Self::ParseModified { cause, .. } => cause,
    Self::HardLinkIo { cause, .. } => cause,
});

impl purrdf_hash::error::StructuredError for FsRefusal {
    purrdf_hash::error_visit! {
        NonUtf8Path { path } => "gts-files-non-utf8-path" { "path" => Bytes(path.as_os_str().as_encoded_bytes()) },
        Symlink { path } => "gts-files-source-symlink" { "path" => Bytes(path.as_os_str().as_encoded_bytes()) },
        InvalidSourceName { path } => "gts-files-invalid-source-name" { "path" => Bytes(path.as_os_str().as_encoded_bytes()) },
        OutsideSource { path, base } => "gts-files-outside-source" { "path" => Bytes(path.as_os_str().as_encoded_bytes()), "base" => Bytes(base.as_os_str().as_encoded_bytes()) },
        UnsupportedSource { path } => "gts-files-unsupported-source" { "path" => Bytes(path.as_os_str().as_encoded_bytes()) },
        OutsideDirectory { path, base } => "gts-files-outside-directory" { "path" => Bytes(path.as_os_str().as_encoded_bytes()), "base" => Bytes(base.as_os_str().as_encoded_bytes()) },
        DestinationEscape { path, root, ancestor } => "gts-files-destination-escape" { "path" => Text(path), "root" => Bytes(root.as_os_str().as_encoded_bytes()), "ancestor" => Bytes(ancestor.as_os_str().as_encoded_bytes()) },
        MissingParent { path } => "gts-files-missing-parent" { "path" => Bytes(path.as_os_str().as_encoded_bytes()) },
        MissingFileName { path } => "gts-files-missing-file-name" { "path" => Bytes(path.as_os_str().as_encoded_bytes()) },
        TempAttempts { path, attempts } => "gts-files-temp-attempts" { "path" => Bytes(path.as_os_str().as_encoded_bytes()), "attempts" => Unsigned(*attempts as u128) },
        WriteThroughSymlink { archive_path, path } => "gts-files-write-through-symlink" { "archive_path" => Text(archive_path), "path" => Bytes(path.as_os_str().as_encoded_bytes()) },
        ReplaceDirectory { archive_path, path } => "gts-files-replace-directory" { "archive_path" => Text(archive_path), "path" => Bytes(path.as_os_str().as_encoded_bytes()) },
        SymlinkEscape { path, target } => "gts-files-symlink-escape" { "path" => Text(path), "target" => Text(target) },
        SymlinkBackslash { path, target } => "gts-files-symlink-backslash" { "path" => Text(path), "target" => Text(target) },
        SymlinkRoot { path, target } => "gts-files-symlink-root" { "path" => Text(path), "target" => Text(target) },
        NotLink { path, kind } => "gts-files-not-link" { "path" => Text(path), "kind" => Code(kind.as_str()) },
        UnsupportedExtraction { path, kind } => "gts-files-unsupported-extraction" { "path" => Text(path), "kind" => Code(kind.as_str()) },
        UnsupportedOwner { path, uid, gid } => "gts-files-unsupported-owner" {
            "path" => Text(path), "uid" => Value(uid.map_or(ErrorValue::Absent, |value| ErrorValue::Unsigned(u128::from(value)))),
            "gid" => Value(gid.map_or(ErrorValue::Absent, |value| ErrorValue::Unsigned(u128::from(value))))
        },
        MissingDigest { path } => "gts-files-missing-digest" { "path" => Text(path) },
        MissingBlob { path, digest } => "gts-files-missing-blob" { "path" => Text(path), "digest" => Text(digest) },
        BlobDecode { path, cause } => "gts-files-blob-decode" { "path" => Text(path), "cause" => Cause(cause) },
        Integrity { path, digest, actual } => "gts-files-integrity" { "path" => Text(path), "digest" => Text(digest), "actual" => Text(actual) },
        Refused { path, kind, option } => "gts-files-refused" { "path" => Text(path), "kind" => Code(kind.as_str()), "option" => Code(option) },
        DateTimeBeforeEpoch { path, cause } => "gts-files-time-before-epoch" { "path" => Bytes(path.as_os_str().as_encoded_bytes()), "cause" => Cause(cause) },
        DateTimeRange { path, seconds, cause } => "gts-files-time-range" { "path" => Bytes(path.as_os_str().as_encoded_bytes()), "seconds" => Unsigned(u128::from(*seconds)), "cause" => Cause(cause) },
        DateTimeFormat { path, seconds, cause } => "gts-files-time-format" { "path" => Bytes(path.as_os_str().as_encoded_bytes()), "seconds" => Unsigned(u128::from(*seconds)), "cause" => Cause(cause) },
        ParseModified { actual, cause } => "gts-files-parse-modified" { "actual" => Text(actual), "cause" => Cause(cause) },
        MtimeHost { actual, seconds, nanos } => "gts-files-mtime-host" { "actual" => Text(actual), "seconds" => Signed(i128::from(*seconds)), "nanoseconds" => Unsigned(u128::from(*nanos)) },
        MissingDirectory { path } => "gts-files-missing-directory" { "path" => Bytes(path.as_os_str().as_encoded_bytes()) },
        HardLinkIo { target, source, cause } => "gts-files-hardlink-io" { "target" => Bytes(target.as_os_str().as_encoded_bytes()), "source" => Bytes(source.as_os_str().as_encoded_bytes()), "cause" => Cause(cause) },
    }
}
