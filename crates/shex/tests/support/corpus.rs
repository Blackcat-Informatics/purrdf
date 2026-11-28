// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Where the vendored shexTest suite lives, and its ShExC schemas in a directory.

use std::fs;
use std::path::{Path, PathBuf};

/// The vendored shexTest v2.1.0 suite, `vectors/shexTest`.
pub(crate) fn shex_test() -> PathBuf {
    purrdf_testkit::paths::workspace_root!().join("vectors/shexTest")
}

/// The suite's schema directory, `vectors/shexTest/schemas`.
pub(crate) fn schemas() -> PathBuf {
    shex_test().join("schemas")
}

/// Every `.shex` file directly under `dir`, sorted.
pub(crate) fn shex_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension().is_some_and(|x| x == "shex")).then_some(path)
        })
        .collect();
    files.sort();
    files
}
