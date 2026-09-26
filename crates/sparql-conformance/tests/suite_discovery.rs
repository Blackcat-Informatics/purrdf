// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The discovery rule `sparql_conformance.rs` runs its cases from
//! (`paths::suite_manifests`), exercised over scratch trees: which files count,
//! the order they come back in, and which trees are refused.

use std::fs;
use std::path::Path;

use purrdf_sparql_conformance::paths::{SuiteManifest, suite_manifests};
use purrdf_testkit::{TempDir, temp_dir};

fn scratch() -> TempDir {
    temp_dir!("suite-discovery-").expect("create a scratch suite root")
}

fn touch(root: &Path, relative: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("a relative file path has a parent"))
        .expect("create the file's directory");
    fs::write(&path, "# example.org fixture\n").expect("write the file");
}

fn relatives(found: &[SuiteManifest]) -> Vec<&str> {
    found
        .iter()
        .map(|manifest| manifest.relative.as_str())
        .collect()
}

/// The order is the byte order of the `/`-joined relative path: `-` (0x2D) is
/// below `/` (0x2F), so `a-b/…` precedes `a/b/…`, which a component-wise
/// `Path` comparison would put the other way round.
#[test]
fn manifests_come_back_in_byte_order_of_the_relative_path() {
    let root = scratch();
    for relative in [
        "a/manifest.ttl",
        "a/b/manifest.ttl",
        "a-b/manifest.ttl",
        "z/manifest.ttl",
        "B/manifest.ttl",
    ] {
        touch(root.path(), relative);
    }
    let found = suite_manifests(root.path()).expect("a readable tree is discovered");
    assert_eq!(
        relatives(&found),
        [
            "B/manifest.ttl",
            "a-b/manifest.ttl",
            "a/b/manifest.ttl",
            "a/manifest.ttl",
            "z/manifest.ttl",
        ]
    );
    for manifest in &found {
        assert_eq!(manifest.path, root.path().join(&manifest.relative));
        assert!(
            manifest.path.is_file(),
            "{} must exist",
            manifest.path.display()
        );
    }
}

/// Only a regular file named exactly `manifest.ttl`, in a subdirectory of the
/// root, is a case. The control row (`g/manifest.ttl`) is found, so an empty
/// answer cannot pass this test by discovering nothing at all.
#[test]
fn only_manifest_ttl_below_the_root_is_a_case() {
    let root = scratch();
    for relative in [
        "manifest.ttl",
        "g/manifest.ttl",
        "g/manifest-all.ttl",
        "g/Manifest.ttl",
        "g/manifest.ttl.bak",
        "g/data.ttl",
    ] {
        touch(root.path(), relative);
    }
    // A directory with the manifest's name is descended into, never counted.
    touch(root.path(), "h/manifest.ttl/inner/manifest.ttl");
    let found = suite_manifests(root.path()).expect("a readable tree is discovered");
    assert_eq!(
        relatives(&found),
        ["g/manifest.ttl", "h/manifest.ttl/inner/manifest.ttl"]
    );
}

/// A symbolic link is neither counted nor followed, so a link cannot run one
/// manifest twice. The linked-to manifest itself is still found once.
#[cfg(unix)]
#[test]
fn symbolic_links_are_neither_counted_nor_followed() {
    use std::os::unix::fs::symlink;

    let root = scratch();
    touch(root.path(), "real/manifest.ttl");
    fs::create_dir_all(root.path().join("linked")).expect("create a directory for a link");
    symlink(
        root.path().join("real/manifest.ttl"),
        root.path().join("linked/manifest.ttl"),
    )
    .expect("link a file");
    symlink(root.path().join("real"), root.path().join("alias")).expect("link a directory");
    let found = suite_manifests(root.path()).expect("a readable tree is discovered");
    assert_eq!(relatives(&found), ["real/manifest.ttl"]);
}

/// A root that cannot be read is refused rather than read as an empty suite;
/// an existing empty root is its valid neighbour and discovers nothing.
#[test]
fn an_unreadable_root_is_refused_and_an_empty_one_is_not() {
    let root = scratch();
    let missing = root.path().join("absent");
    let error = suite_manifests(&missing).expect_err("a missing root must be refused");
    assert!(
        error.contains("cannot read directory") && error.contains("absent"),
        "the refusal names the directory: {error}"
    );
    assert_eq!(
        suite_manifests(root.path()).expect("an empty root is readable"),
        []
    );
}

/// A case is named by its path, so a path component that is not UTF-8 is
/// refused; a non-ASCII UTF-8 component beside it is the valid neighbour.
#[cfg(target_os = "linux")]
#[test]
fn a_non_utf8_path_component_is_refused_and_non_ascii_utf8_is_not() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt as _;

    let root = scratch();
    touch(root.path(), "caf\u{e9}/manifest.ttl");
    assert_eq!(
        relatives(&suite_manifests(root.path()).expect("UTF-8 names are discovered")),
        ["caf\u{e9}/manifest.ttl"]
    );

    let invalid = root.path().join(OsStr::from_bytes(b"caf\xe9"));
    fs::create_dir_all(&invalid).expect("create a non-UTF-8 directory");
    fs::write(invalid.join("manifest.ttl"), "# example.org fixture\n").expect("write the file");
    let error = suite_manifests(root.path()).expect_err("a non-UTF-8 name must be refused");
    assert!(error.contains("is not a UTF-8 path"), "{error}");
}
