// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Checked-in golden files: the exact text a test produces, compared exactly.
//!
//! A golden holds the produced bytes verbatim — line endings and trailing
//! whitespace included — so a serializer's framing is pinned along with its
//! content. `PURRDF_REGENERATE_GOLDEN=1` rewrites each golden a test reaches
//! from what it produced instead of comparing; the diff is then the review.

use std::path::PathBuf;

/// Compare `actual` against `tests/golden/<name>`, or rewrite it when
/// `PURRDF_REGENERATE_GOLDEN=1`.
pub(crate) fn assert_golden(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    if std::env::var_os("PURRDF_REGENERATE_GOLDEN").is_some_and(|value| value == "1") {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|err| panic!("create {}: {err}", parent.display()));
        }
        std::fs::write(&path, actual)
            .unwrap_or_else(|err| panic!("write golden {}: {err}", path.display()));
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "read golden {} ({err}); PURRDF_REGENERATE_GOLDEN=1 writes it",
            path.display()
        )
    });
    assert!(
        actual == expected,
        "output differs from golden {}; PURRDF_REGENERATE_GOLDEN=1 rewrites it\n\
         --- expected\n{expected}\n--- actual\n{actual}",
        path.display()
    );
}
