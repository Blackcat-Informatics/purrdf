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
//! `tests/golden/` directory.

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
    /// [`Mode::Regenerate`] exactly when `PURRDF_REGENERATE_GOLDEN` is `1`.
    pub fn from_env() -> Self {
        if std::env::var_os(REGENERATE_ENV).is_some_and(|value| value == "1") {
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
        }
    }
}

impl std::error::Error for GoldenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } | Self::Write { source, .. } => Some(source),
            Self::Differs { .. } => None,
        }
    }
}

/// Compare `actual` with the file at `path` byte for byte, or write it there
/// (creating parent directories) under [`Mode::Regenerate`].
pub fn check(path: &Path, actual: &str, mode: Mode) -> Result<(), GoldenError> {
    match mode {
        Mode::Regenerate => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|source| GoldenError::Write {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            std::fs::write(path, actual).map_err(|source| GoldenError::Write {
                path: path.to_path_buf(),
                source,
            })
        }
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
