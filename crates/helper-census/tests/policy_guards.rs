// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The production policy modes run over the same tree as their former guards.

use std::process::Command;

/// Run the workspace feature policy through its production CLI entry point.
#[test]
fn feature_policy_accepts_the_repository() {
    accepts("--no-features");
}

/// Run the Python Rust-test policy through its production CLI entry point.
#[test]
fn python_test_policy_accepts_the_repository() {
    accepts("--python-binding-tests");
}

/// Require the requested mode to succeed, preserving its diagnostics on failure.
fn accepts(mode: &str) {
    let root = purrdf_testkit::paths::workspace_root();
    let output = Command::new(env!("CARGO_BIN_EXE_helper-census"))
        .args(["--root", root.to_str().expect("root"), mode])
        .output()
        .expect("helper-census");
    assert!(
        output.status.success(),
        "{mode}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
