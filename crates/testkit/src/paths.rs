// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Paths a test resolves against the workspace: the root above the calling
//! crate, and every file of one extension under a directory.
//!
//! A hygiene test reads the whole tree — every `.rs` under `crates/`, every
//! shipped example, every manifest — and each such test used to carry its own
//! root locator and its own directory walk. The locator here walks up from the
//! calling crate's manifest directory to the first ancestor whose `Cargo.toml`
//! declares a workspace, so it is right from any depth and encodes no `../..`;
//! the walk here is sorted, skips the directories no sweep wants, and treats
//! an unreadable directory as the error it is rather than as an empty result.
//! Neither exists on `wasm32-unknown-unknown`, which has no file system.

use std::path::{Path, PathBuf};

/// Directory names [`collect_files`] never enters, at any depth below the root
/// it is given: cargo's build output, git's object store, JavaScript
/// dependencies, and the checkouts other branches keep beneath the main one —
/// a sweep from a root holding those would read every branch's copy of every
/// file. The root itself is walked whatever its name.
pub const SKIPPED_DIRECTORIES: [&str; 4] = ["target", ".git", "node_modules", ".worktrees"];

/// The workspace root above `manifest_dir`: the first of its ancestors,
/// itself included, holding a `Cargo.toml` that declares a workspace — a
/// `[workspace]` table, or any `[workspace.<key>]` table, which cargo takes
/// as a root just the same. A manifest that declares only a package, as every
/// member's does, is walked past. Prefer [`crate::workspace_root!`], which
/// supplies the calling crate's manifest directory.
///
/// The result is the ancestor as it is spelled in `manifest_dir`, not
/// canonicalized: no symbolic link is resolved, so
/// `manifest_dir.strip_prefix(&root)` always succeeds and a path joined onto
/// the root is spelled the way `CARGO_MANIFEST_DIR` spells it. `manifest_dir`
/// need not exist; its ancestors are examined as written.
///
/// # Panics
///
/// When no ancestor declares a workspace.
#[must_use]
pub fn workspace_root_from(manifest_dir: &Path) -> PathBuf {
    manifest_dir
        .ancestors()
        .find(|dir| declares_workspace(dir))
        .map_or_else(
            || {
                panic!(
                    "no ancestor of {} holds a Cargo.toml with a [workspace] table",
                    manifest_dir.display()
                )
            },
            Path::to_path_buf,
        )
}

/// Whether `dir/Cargo.toml` exists and holds a `[workspace]` or
/// `[workspace.<key>]` table header.
fn declares_workspace(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("Cargo.toml")).is_ok_and(|text| {
        text.lines().any(|line| {
            line.trim()
                .strip_prefix('[')
                .and_then(|rest| rest.strip_suffix(']'))
                .is_some_and(|table| {
                    let table = table.trim();
                    table == "workspace" || table.starts_with("workspace.")
                })
        })
    })
}

/// Every file under `root`, recursively, whose extension is exactly
/// `extension` (`rs`; a leading `.` is accepted and ignored), sorted by path
/// so the order is the same on every run and every host. The directories
/// named in [`SKIPPED_DIRECTORIES`] are not entered, at any depth. A link to
/// a file is a file; a link to a directory is not entered, so a link cycle
/// cannot recurse forever; a link to nothing is neither. The extension is
/// compared exactly, case included.
///
/// # Panics
///
/// When `root` or any directory under it cannot be read — an absent root is
/// a test-authoring error, never an empty sweep — or when `extension` is
/// empty.
#[must_use]
pub fn collect_files(root: &Path, extension: &str) -> Vec<PathBuf> {
    let extension = extension.strip_prefix('.').unwrap_or(extension);
    assert!(
        !extension.is_empty(),
        "collect_files needs an extension, such as `rs`"
    );
    let mut out = Vec::new();
    walk(root, extension, &mut out);
    out.sort();
    out
}

/// [`collect_files`] for `rs`: every Rust source file under `root`.
#[must_use]
pub fn collect_rs(root: &Path) -> Vec<PathBuf> {
    collect_files(root, "rs")
}

/// Append every `extension` file under `dir` to `out`, unsorted.
fn walk(dir: &Path, extension: &str, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read directory {}: {error}", dir.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|error| panic!("read an entry of {}: {error}", dir.display()));
        let path = entry.path();
        let file_type = entry
            .file_type()
            .unwrap_or_else(|error| panic!("read the type of {}: {error}", path.display()));
        let (is_dir, is_file) = if file_type.is_symlink() {
            // Followed once, for the kind of thing it names: a linked file is
            // a file, a linked directory is not entered, a dangling link is
            // neither.
            std::fs::metadata(&path).map_or((false, false), |target| (false, target.is_file()))
        } else {
            (file_type.is_dir(), file_type.is_file())
        };
        if is_dir {
            let skipped = entry
                .file_name()
                .to_str()
                .is_some_and(|name| SKIPPED_DIRECTORIES.contains(&name));
            if !skipped {
                walk(&path, extension, out);
            }
        } else if is_file && path.extension().is_some_and(|found| found == extension) {
            out.push(path);
        }
    }
}
