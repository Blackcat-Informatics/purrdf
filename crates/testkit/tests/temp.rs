// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `TempDir` / `NamedTempFile`: distinct under concurrency, removed on drop,
//! under the target directory and never the system temporary directory, and
//! exclusive on creation.
//!
//! These run where a test can create a path; wasm32-unknown-unknown has no file
//! system, and the scratch-space types do not exist there.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Barrier;

use purrdf_testkit::{NamedTempFile, TempDir, temp_dir, temp_file};

/// Asserts `path` is under this target's `CARGO_TARGET_TMPDIR` and not under
/// the system temporary directory or `/tmp`.
fn assert_under_target(path: &Path) {
    assert!(
        path.starts_with(env!("CARGO_TARGET_TMPDIR")),
        "{} is not under CARGO_TARGET_TMPDIR",
        path.display()
    );
    assert!(
        !path.starts_with(std::env::temp_dir()),
        "{} is under the system temporary directory",
        path.display()
    );
    assert!(
        !path.starts_with("/tmp"),
        "{} is under /tmp",
        path.display()
    );
}

#[test]
fn sixteen_threads_get_sixteen_distinct_directories_all_removed_on_drop() {
    const THREADS: usize = 16;
    let barrier = Barrier::new(THREADS);
    let dirs: Vec<TempDir> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    temp_dir!().expect("create temp dir")
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("creator thread"))
            .collect()
    });

    let paths: BTreeSet<PathBuf> = dirs.iter().map(|dir| dir.path().to_path_buf()).collect();
    assert_eq!(paths.len(), THREADS, "every directory path is distinct");
    for dir in &dirs {
        assert!(
            dir.path().is_dir(),
            "{} exists while held",
            dir.path().display()
        );
        assert_under_target(dir.path());
        std::fs::write(dir.path().join("nested.txt"), b"contents").expect("write inside");
    }

    drop(dirs);
    for path in &paths {
        assert!(!path.exists(), "{} survived its drop", path.display());
    }
}

#[test]
fn a_prefix_names_the_directory() {
    let dir = temp_dir!("purrdf-testkit-prefix-").expect("prefixed temp dir");
    let name = dir
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .expect("utf-8 name");
    assert!(name.starts_with("purrdf-testkit-prefix-"), "{name}");
    assert_under_target(dir.path());
}

#[test]
fn a_prefix_that_is_not_a_plain_name_is_refused_and_a_plain_one_is_accepted() {
    for bad in ["../escape-", "nested/prefix-", "back\\slash-", ".", ".."] {
        let error = TempDir::with_prefix_in(bad, env!("CARGO_TARGET_TMPDIR"))
            .expect_err("a prefix with a separator or a dot name is refused");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput, "{bad:?}");
    }
    let plain = TempDir::with_prefix_in("plain.name-", env!("CARGO_TARGET_TMPDIR"))
        .expect("a plain prefix is accepted");
    assert!(plain.path().is_dir());
}

#[test]
fn a_file_is_writable_and_removed_on_drop() {
    let mut file = temp_file!().expect("create temp file");
    assert_under_target(file.path());
    file.write_all(b"payload").expect("write");
    file.flush().expect("flush");
    let path = file.path().to_path_buf();
    assert_eq!(std::fs::read(&path).expect("read back"), b"payload");
    drop(file);
    assert!(!path.exists(), "{} survived its drop", path.display());
}

#[test]
fn a_root_that_is_a_file_is_an_error_and_a_directory_root_is_accepted() {
    let holder = temp_dir!().expect("holder");
    let not_a_dir = holder.path().join("occupied");
    std::fs::write(&not_a_dir, b"").expect("occupy");
    assert!(TempDir::new_in(&not_a_dir).is_err(), "a file is not a root");
    assert!(
        NamedTempFile::new_in(&not_a_dir).is_err(),
        "a file is not a root"
    );
    let inner = TempDir::new_in(holder.path()).expect("a directory root is accepted");
    assert!(inner.path().starts_with(holder.path()));
}

#[test]
fn a_directory_the_test_removed_itself_is_not_a_drop_failure() {
    let dir = temp_dir!().expect("create temp dir");
    std::fs::remove_dir_all(dir.path()).expect("remove early");
    drop(dir);
}

#[test]
fn the_unit_test_root_is_the_directory_cargo_names_for_integration_tests() {
    // `for_unit_test` derives from the executable what cargo hands integration
    // tests as `CARGO_TARGET_TMPDIR`; an integration test binary lives in the
    // same build tree, so the two must agree for a host build.
    let dir = TempDir::for_unit_test().expect("unit-test temp dir");
    assert_eq!(
        dir.path().parent(),
        Some(Path::new(env!("CARGO_TARGET_TMPDIR"))),
        "the derived root differs from cargo's"
    );
    let file = NamedTempFile::for_unit_test().expect("unit-test temp file");
    assert_eq!(
        file.path().parent(),
        Some(Path::new(env!("CARGO_TARGET_TMPDIR")))
    );
}
