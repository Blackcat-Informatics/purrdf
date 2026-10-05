// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Path helpers for the conformance harness, and discovery of the manifests a
//! suite directory holds.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;
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

/// The ordinary leaf file name [`suite_manifests`] discovers, at any depth.
/// It also discovers the W3C data-r2 leaf name `extended-manifest.ttl`.
///
/// A manifest that aggregates others with `mf:include` must be named something
/// else (the vendored SEP-0009 corpus uses `manifest-all.ttl`), or discovery
/// would find it beside the manifests it includes and run their cases twice.
pub const SUITE_MANIFEST_NAME: &str = "manifest.ttl";

/// Every supported leaf spelling; the loader uses this same rule to refuse
/// auto-discovered aggregators that would execute their children twice.
pub(crate) fn is_suite_manifest_name(name: &OsStr) -> bool {
    name == SUITE_MANIFEST_NAME || name == "extended-manifest.ttl"
}

/// One manifest found by [`suite_manifests`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiteManifest {
    /// The manifest's path: the discovery root joined with [`Self::relative`].
    pub path: PathBuf,
    /// The manifest's path below the discovery root, its components joined with
    /// `/` on every platform (for example `w3c-sparql11/bind/manifest.ttl`).
    pub relative: String,
}

/// The file-system operation a [`DiscoveryError`] failed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryOperation {
    /// Opening a directory for listing.
    ReadDir,
    /// Reading the next entry of a directory listing.
    ReadEntry,
    /// Reading an entry's file type, or the metadata a symbolic link points at.
    Metadata,
}

impl fmt::Display for DiscoveryOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ReadDir => "reading directory",
            Self::ReadEntry => "reading an entry of directory",
            Self::Metadata => "reading metadata of",
        })
    }
}

/// Why [`suite_manifests`] could not produce a complete list.
///
/// Discovery never skips what it cannot read: a directory left out of the walk
/// would silently drop its manifests from the run.
#[derive(Debug)]
pub enum DiscoveryError {
    /// A file-system operation failed on `path`.
    Io {
        /// What was being done.
        operation: DiscoveryOperation,
        /// The path the operation failed on.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// A component of a discovered manifest's path below the root is not
    /// valid UTF-8, so the manifest cannot be given a case name without a lossy
    /// (and possibly colliding) rewrite.
    NonUtf8Component {
        /// The manifest's path.
        path: PathBuf,
    },
}

impl DiscoveryError {
    /// The path the error concerns.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Io { path, .. } | Self::NonUtf8Component { path } => path,
        }
    }
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                operation,
                path,
                source,
            } => write!(f, "{operation} {}: {source}", path.display()),
            Self::NonUtf8Component { path } => write!(
                f,
                "{}: a path component is not valid UTF-8, so the manifest cannot be named",
                path.display()
            ),
        }
    }
}

impl std::error::Error for DiscoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::NonUtf8Component { .. } => None,
        }
    }
}

/// Every file named [`SUITE_MANIFEST_NAME`] or `extended-manifest.ttl` at any
/// depth below `root`. Only exact spellings match.
///
/// Directories are descended into whatever their name; a symbolic link is
/// followed, so a linked directory is walked and a linked file is matched.
///
/// # Order
///
/// The result is sorted by the path below `root`, compared component by
/// component, each component as bytes. The order therefore does not depend on
/// the order the platform lists a directory in, and it equals the order of
/// sorted `PathBuf`s on Unix: `a/b/manifest.ttl` precedes `a-b/manifest.ttl`,
/// because the component `a` is a prefix of `a-b`.
///
/// # Errors
///
/// [`DiscoveryError::Io`] when `root` or any directory below it cannot be
/// listed, or an entry's type cannot be read (a missing root, a root that is not
/// a directory, a dangling symbolic link); [`DiscoveryError::NonUtf8Component`]
/// when a path component below `root` leading to a manifest is not valid UTF-8.
/// An empty tree is not an error: it yields an empty list.
pub fn suite_manifests(root: &Path) -> Result<Vec<SuiteManifest>, DiscoveryError> {
    let mut found: Vec<(Vec<String>, PathBuf)> = Vec::new();
    walk(root, &mut Vec::new(), &mut found)?;
    found.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(found
        .into_iter()
        .map(|(components, path)| SuiteManifest {
            path,
            relative: components.join("/"),
        })
        .collect())
}

/// Collect the manifests below `dir`, whose components below the root are
/// `prefix`.
fn walk(
    dir: &Path,
    prefix: &mut Vec<OsString>,
    found: &mut Vec<(Vec<String>, PathBuf)>,
) -> Result<(), DiscoveryError> {
    let io_error = |operation, path: &Path, source| DiscoveryError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    };
    let entries =
        std::fs::read_dir(dir).map_err(|e| io_error(DiscoveryOperation::ReadDir, dir, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| io_error(DiscoveryOperation::ReadEntry, dir, e))?;
        let path = entry.path();
        let mut file_type = entry
            .file_type()
            .map_err(|e| io_error(DiscoveryOperation::Metadata, &path, e))?;
        if file_type.is_symlink() {
            file_type = std::fs::metadata(&path)
                .map_err(|e| io_error(DiscoveryOperation::Metadata, &path, e))?
                .file_type();
        }
        let name = entry.file_name();
        if file_type.is_dir() {
            prefix.push(name);
            walk(&path, prefix, found)?;
            prefix.pop();
        } else if file_type.is_file() && is_suite_manifest_name(&name) {
            let components = prefix
                .iter()
                .chain(std::iter::once(&name))
                .map(|component| component.to_str().map(str::to_owned))
                .collect::<Option<Vec<String>>>()
                .ok_or_else(|| DiscoveryError::NonUtf8Component { path: path.clone() })?;
            found.push((components, path));
        }
    }
    Ok(())
}
