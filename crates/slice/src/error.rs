// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Error type for the purrdf-slice crate.

/// All errors that can arise from slice discovery, manifest parsing, artifact
/// inventory, and content-addressed digest operations.
#[derive(Debug)]
pub enum SliceError {
    /// An I/O error reading from the filesystem.
    Io(std::io::Error),
    /// An RDF parse error (Turtle or other format).
    Parse(String),
    /// The manifest.ttl is structurally invalid (missing required field, etc.).
    InvalidManifest(String),
    /// A path within a slice violates the safety rules (absolute, `..`, etc.).
    InvalidPath(String),
    /// A digest computed at discovery time does not match a stored expectation.
    DigestMismatch { expected: String, actual: String },
}

impl std::fmt::Display for SliceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Parse(msg) => write!(f, "RDF parse error: {msg}"),
            Self::InvalidManifest(msg) => write!(f, "invalid manifest: {msg}"),
            Self::InvalidPath(msg) => write!(f, "invalid path: {msg}"),
            Self::DigestMismatch { expected, actual } => {
                write!(f, "digest mismatch: expected {expected}, got {actual}")
            }
        }
    }
}

impl std::error::Error for SliceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SliceError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
