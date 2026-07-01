// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Path helpers for the conformance harness.

use std::path::{Path, PathBuf};

/// The directory holding a manifest's case files (the manifest's parent).
#[must_use]
pub fn manifest_dir(manifest: &Path) -> PathBuf {
    manifest
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// Resolve a manifest-relative file name (extracted from a test-case IRI) against
/// the manifest's directory.
#[must_use]
pub fn resolve(manifest_dir: &Path, relative: &str) -> PathBuf {
    manifest_dir.join(relative)
}
