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
//! `tests/golden/` directory. A binary golden, or one whose regeneration is
//! switched by a variable of its own (a corpus that is rewritten as a whole,
//! say), goes through [`check_bytes`] and [`regenerating`]; the one rule — a
//! variable switches to regeneration exactly when it is `1` — is the same for
//! every variable, so no golden rewrites itself because a variable happens to
//! be set to `0` or to nothing.

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
        Self::from_env_var(REGENERATE_ENV)
    }

    /// [`Mode::Regenerate`] exactly when the variable `name` is `1`; see
    /// [`regenerating`].
    pub fn from_env_var(name: &str) -> Self {
        if regenerating(name) {
            Self::Regenerate
        } else {
            Self::Compare
        }
    }
}

/// Whether the regeneration switch `name` is on: the variable is set and its
/// value is exactly `1`.
///
/// The one reading of every regeneration variable in the workspace, so a
/// golden and a corpus agree on what turns regeneration on: `0`, an empty
/// value, `true` and an unset variable all compare.
pub fn regenerating(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| value == "1")
}

/// Why a golden check did not pass.
#[derive(Debug)]
pub enum GoldenError {
    /// The golden could not be read (usually: it does not exist yet).
    Read {
        /// The golden file.
        path: PathBuf,
        /// The variable that switches this golden to regeneration.
        env_var: &'static str,
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
        /// The variable that switches this golden to regeneration.
        env_var: &'static str,
        /// The golden's contents.
        expected: String,
        /// The produced text.
        actual: String,
    },
    /// The produced bytes differ from a binary golden.
    BytesDiffer {
        /// The golden file.
        path: PathBuf,
        /// The variable that switches this golden to regeneration.
        env_var: &'static str,
        /// The golden's length in bytes.
        expected_len: usize,
        /// The produced length in bytes.
        actual_len: usize,
        /// The offset of the first byte that differs; the shorter length when
        /// one is a prefix of the other.
        first_difference: usize,
    },
}

impl fmt::Display for GoldenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read {
                path,
                env_var,
                source,
            } => write!(
                f,
                "read golden {} ({source}); {env_var}=1 writes it",
                path.display()
            ),
            Self::Write { path, source } => {
                write!(f, "write golden {}: {source}", path.display())
            }
            Self::Differs {
                path,
                env_var,
                expected,
                actual,
            } => write!(
                f,
                "output differs from golden {}; {env_var}=1 rewrites it\n\
                 --- expected\n{expected}\n--- actual\n{actual}",
                path.display()
            ),
            Self::BytesDiffer {
                path,
                env_var,
                expected_len,
                actual_len,
                first_difference,
            } => write!(
                f,
                "output differs from golden {} at byte {first_difference} \
                 ({actual_len} bytes produced, {expected_len} expected); {env_var}=1 rewrites it",
                path.display()
            ),
        }
    }
}

impl std::error::Error for GoldenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } | Self::Write { source, .. } => Some(source),
            Self::Differs { .. } | Self::BytesDiffer { .. } => None,
        }
    }
}

/// Compare `actual` with the file at `path` byte for byte, or write it there
/// (creating parent directories) under [`Mode::Regenerate`].
pub fn check(path: &Path, actual: &str, mode: Mode) -> Result<(), GoldenError> {
    let Some(expected) = read_or_regenerate(path, actual.as_bytes(), mode, REGENERATE_ENV)? else {
        return Ok(());
    };
    match String::from_utf8(expected) {
        Ok(expected) if expected == actual => Ok(()),
        Ok(expected) => Err(GoldenError::Differs {
            path: path.to_path_buf(),
            env_var: REGENERATE_ENV,
            expected,
            actual: actual.to_owned(),
        }),
        Err(not_text) => Err(bytes_differ(
            path,
            REGENERATE_ENV,
            not_text.as_bytes(),
            actual.as_bytes(),
        )),
    }
}

/// Compare `actual` with the binary golden at `path` byte for byte, or write it
/// there (creating parent directories) when the variable `regenerate_env_var`
/// is `1` (see [`regenerating`]).
///
/// For a golden that is not text — an encoded artifact, a compressed stream —
/// and for one whose regeneration has a switch of its own. A difference
/// reports both lengths and the first offset that differs rather than the
/// bytes themselves.
pub fn check_bytes(
    path: &Path,
    actual: &[u8],
    regenerate_env_var: &'static str,
) -> Result<(), GoldenError> {
    let mode = Mode::from_env_var(regenerate_env_var);
    match read_or_regenerate(path, actual, mode, regenerate_env_var)? {
        Some(expected) if expected != actual => {
            Err(bytes_differ(path, regenerate_env_var, &expected, actual))
        }
        Some(_) | None => Ok(()),
    }
}

/// Under [`Mode::Regenerate`], write `actual` to `path` and return `None`;
/// under [`Mode::Compare`], return the golden's bytes.
fn read_or_regenerate(
    path: &Path,
    actual: &[u8],
    mode: Mode,
    env_var: &'static str,
) -> Result<Option<Vec<u8>>, GoldenError> {
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
            })?;
            Ok(None)
        }
        Mode::Compare => std::fs::read(path)
            .map(Some)
            .map_err(|source| GoldenError::Read {
                path: path.to_path_buf(),
                env_var,
                source,
            }),
    }
}

/// The [`GoldenError::BytesDiffer`] between `expected` and `actual`.
fn bytes_differ(path: &Path, env_var: &'static str, expected: &[u8], actual: &[u8]) -> GoldenError {
    let first_difference = expected
        .iter()
        .zip(actual)
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| expected.len().min(actual.len()));
    GoldenError::BytesDiffer {
        path: path.to_path_buf(),
        env_var,
        expected_len: expected.len(),
        actual_len: actual.len(),
        first_difference,
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
