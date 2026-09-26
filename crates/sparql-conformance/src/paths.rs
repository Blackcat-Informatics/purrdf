// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

/// The file name that makes a file under `suite/` a live conformance case.
pub const SUITE_MANIFEST_NAME: &str = "manifest.ttl";

/// One `manifest.ttl` discovered under a suite root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiteManifest {
    /// The path relative to the root, components joined by `/` whatever the
    /// host's separator: the case's name, and its sort key.
    pub relative: String,
    /// The root joined with the relative path: the path the case loads.
    pub path: PathBuf,
}

/// Every `manifest.ttl` under `root`, at any depth below it, sorted by
/// [`SuiteManifest::relative`] as bytes.
///
/// This is the discovery rule of the `sparql_conformance` test target: one case
/// per returned manifest, in the returned order. A regular file counts only if
/// it is named exactly [`SUITE_MANIFEST_NAME`] and sits in a subdirectory of
/// `root`; a `manifest.ttl` directly in `root` is not a case. Symbolic links are
/// neither followed into nor counted, so a link cannot make one manifest run
/// twice. The order is the byte order of the `/`-joined relative path, so
/// `a-b/manifest.ttl` sorts before `a/b/manifest.ttl` (`-` is below `/`), not
/// the component order a [`Path`] comparison would give.
///
/// # Errors
///
/// A directory that cannot be read, a directory entry that cannot be
/// inspected, or a path component that is not UTF-8 (a case name is text).
/// Every one is an error, never a silently skipped subtree.
pub fn suite_manifests(root: &Path) -> Result<Vec<SuiteManifest>, String> {
    let mut found = Vec::new();
    let mut pending = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = pending.pop() {
        let entries = std::fs::read_dir(&dir)
            .map_err(|error| format!("cannot read directory {}: {error}", dir.display()))?;
        for entry in entries {
            let entry = entry
                .map_err(|error| format!("cannot read directory {}: {error}", dir.display()))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
            let name = entry.file_name();
            let name = name.to_str().ok_or_else(|| {
                format!(
                    "{} is not a UTF-8 path; a conformance case is named by its path",
                    path.display()
                )
            })?;
            // A file directly in the root has an empty prefix and is not a case.
            let below_root = !prefix.is_empty();
            let relative = if below_root {
                format!("{prefix}/{name}")
            } else {
                name.to_owned()
            };
            if file_type.is_dir() {
                pending.push((path, relative));
            } else if below_root && name == SUITE_MANIFEST_NAME && file_type.is_file() {
                found.push(SuiteManifest { relative, path });
            }
        }
    }
    found.sort_unstable_by(|a, b| a.relative.cmp(&b.relative));
    Ok(found)
}
