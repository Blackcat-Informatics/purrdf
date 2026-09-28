// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Checked-in golden files: the exact text a test produces, compared exactly.
//!
//! A golden holds the produced bytes verbatim — line endings and trailing
//! whitespace included — so a serializer's framing is pinned along with its
//! content. Nothing is normalized on either side: a CRLF golden compared with
//! LF output is a difference, and so is a missing final newline.
//! `PURRDF_REGENERATE_GOLDEN=1` rewrites each golden a test reaches from what
//! it produced instead of comparing; the diff is then the review. Any other
//! value of the variable, or its absence, compares.
//!
//! Tests call [`crate::assert_golden!`], which resolves the calling crate's
//! `tests/golden/` directory. Output that is bytes rather than text — a
//! container, a pack, a digest — goes through [`check_bytes`] and
//! [`crate::assert_golden_bytes!`] under the same rules, and a difference is
//! reported at its first offset.

use std::fmt;
use std::path::{Path, PathBuf};

/// The environment variable that switches comparison to regeneration.
pub const REGENERATE_ENV: &str = "PURRDF_REGENERATE_GOLDEN";

/// Whether a golden is compared against or rewritten.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Compare the produced text against the checked-in file.
    Compare,
    /// Write the produced text over the checked-in file.
    Regenerate,
}

impl Mode {
    /// [`Mode::Regenerate`] exactly when `PURRDF_REGENERATE_GOLDEN` is `1`,
    /// by [`crate::env_flag`]'s rule.
    pub fn from_env() -> Self {
        if crate::env_flag(REGENERATE_ENV) {
            Self::Regenerate
        } else {
            Self::Compare
        }
    }
}

/// Why a golden check did not pass.
#[derive(Debug)]
pub enum GoldenError {
    /// The golden could not be read (usually: it does not exist yet).
    Read {
        /// The golden file.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// The golden (or its directory) could not be written.
    Write {
        /// The file or directory being written.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// The produced text differs from the golden.
    Differs {
        /// The golden file.
        path: PathBuf,
        /// The golden's contents.
        expected: String,
        /// The produced text.
        actual: String,
    },
    /// The produced bytes differ from the golden.
    DiffersBytes {
        /// The golden file.
        path: PathBuf,
        /// The golden's contents.
        expected: Vec<u8>,
        /// The produced bytes.
        actual: Vec<u8>,
    },
}

/// The first offset at which `expected` and `actual` differ: the first
/// unequal byte, or the length of the shorter when one is a prefix of the
/// other.
fn first_difference(expected: &[u8], actual: &[u8]) -> usize {
    expected
        .iter()
        .zip(actual)
        .position(|(expected, actual)| expected != actual)
        .unwrap_or_else(|| expected.len().min(actual.len()))
}

/// The byte of `bytes` at `offset` as `0xHH`, or a note that there is none.
fn byte_at(bytes: &[u8], offset: usize) -> String {
    bytes.get(offset).map_or_else(
        || "none (the data ends)".to_owned(),
        |byte| format!("{byte:#04x}"),
    )
}

impl fmt::Display for GoldenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(
                f,
                "read golden {} ({source}); {REGENERATE_ENV}=1 writes it",
                path.display()
            ),
            Self::Write { path, source } => {
                write!(f, "write golden {}: {source}", path.display())
            }
            Self::Differs {
                path,
                expected,
                actual,
            } => write!(
                f,
                "output differs from golden {}; {REGENERATE_ENV}=1 rewrites it\n\
                 --- expected\n{expected}\n--- actual\n{actual}",
                path.display()
            ),
            Self::DiffersBytes {
                path,
                expected,
                actual,
            } => {
                let offset = first_difference(expected, actual);
                write!(
                    f,
                    "output differs from golden {} at byte {offset}: golden has {}, output has {} \
                     ({} golden bytes, {} produced); {REGENERATE_ENV}=1 rewrites it",
                    path.display(),
                    byte_at(expected, offset),
                    byte_at(actual, offset),
                    expected.len(),
                    actual.len(),
                )
            }
        }
    }
}

impl std::error::Error for GoldenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } | Self::Write { source, .. } => Some(source),
            Self::Differs { .. } | Self::DiffersBytes { .. } => None,
        }
    }
}

/// Compare `actual` with the file at `path` byte for byte, or write it there
/// (creating parent directories) under [`Mode::Regenerate`].
pub fn check(path: &Path, actual: &str, mode: Mode) -> Result<(), GoldenError> {
    match mode {
        Mode::Regenerate => regenerate(path, actual.as_bytes()),
        Mode::Compare => {
            let expected = std::fs::read_to_string(path).map_err(|source| GoldenError::Read {
                path: path.to_path_buf(),
                source,
            })?;
            if actual == expected {
                Ok(())
            } else {
                Err(GoldenError::Differs {
                    path: path.to_path_buf(),
                    expected,
                    actual: actual.to_owned(),
                })
            }
        }
    }
}

/// Compare `actual` with the file at `path` byte for byte, or write it there
/// (creating parent directories) under [`Mode::Regenerate`]: [`check`] for
/// output that is bytes rather than text, under the same rules, with a
/// difference reported at its first offset.
pub fn check_bytes(path: &Path, actual: &[u8], mode: Mode) -> Result<(), GoldenError> {
    match mode {
        Mode::Regenerate => regenerate(path, actual),
        Mode::Compare => {
            let expected = std::fs::read(path).map_err(|source| GoldenError::Read {
                path: path.to_path_buf(),
                source,
            })?;
            if actual == expected.as_slice() {
                Ok(())
            } else {
                Err(GoldenError::DiffersBytes {
                    path: path.to_path_buf(),
                    expected,
                    actual: actual.to_vec(),
                })
            }
        }
    }
}

/// Write `bytes` at `path`, creating its parent directories.
fn regenerate(path: &Path, bytes: &[u8]) -> Result<(), GoldenError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| GoldenError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(path, bytes).map_err(|source| GoldenError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Compare `actual` against `<manifest_dir>/tests/golden/<name>`, or rewrite
/// it when `PURRDF_REGENERATE_GOLDEN=1`; panics on any failure.
///
/// Prefer [`crate::assert_golden!`], which supplies the calling crate's
/// manifest directory.
#[track_caller]
pub fn assert_golden_in(manifest_dir: &Path, name: &str, actual: &str) {
    let path = manifest_dir.join("tests/golden").join(name);
    if let Err(error) = check(&path, actual, Mode::from_env()) {
        panic!("{error}");
    }
}

/// Compare the bytes `actual` against `<manifest_dir>/tests/golden/<name>`,
/// or rewrite it when `PURRDF_REGENERATE_GOLDEN=1`; panics on any failure.
///
/// Prefer [`crate::assert_golden_bytes!`], which supplies the calling crate's
/// manifest directory.
#[track_caller]
pub fn assert_golden_bytes_in(manifest_dir: &Path, name: &str, actual: &[u8]) {
    let path = manifest_dir.join("tests/golden").join(name);
    if let Err(error) = check_bytes(&path, actual, Mode::from_env()) {
        panic!("{error}");
    }
}
