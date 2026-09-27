// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Suite discovery: `paths::suite_manifests` finds exactly the files named
//! `manifest.ttl`, in a documented order, and reports what it cannot read.
//!
//! Every fixture tree is built under this target's `CARGO_TARGET_TMPDIR`.

use std::fs;
use std::path::Path;

use purrdf_sparql_conformance::paths::{
    DiscoveryError, DiscoveryOperation, SUITE_MANIFEST_NAME, suite_manifests,
};
use purrdf_testkit::TempDir;

/// A fresh fixture root.
fn fixture_root() -> TempDir {
    purrdf_testkit::temp_dir!("suite-discovery-").expect("create fixture root")
}

/// Create `relative` (and its parent directories) under `root` as a small file.
fn touch(root: &Path, relative: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("fixture path has a parent"))
        .expect("create fixture directories");
    fs::write(&path, "# fixture\n").expect("write fixture file");
}

/// The relative names discovery returns, checking each path against the root.
fn relatives(root: &Path) -> Vec<String> {
    let found = suite_manifests(root).expect("discovery succeeds");
    for manifest in &found {
        assert_eq!(
            manifest.path,
            root.join(&manifest.relative),
            "a discovered path is the root joined with its relative name"
        );
    }
    found.into_iter().map(|m| m.relative).collect()
}

#[test]
fn the_manifest_name_is_manifest_ttl() {
    assert_eq!(SUITE_MANIFEST_NAME, "manifest.ttl");
}

#[test]
fn every_file_named_manifest_ttl_is_found_and_nothing_else() {
    let root = fixture_root();
    let root = root.path();
    for manifest in [
        "manifest.ttl",
        "group-a/manifest.ttl",
        "group-b/nested/deeper/manifest.ttl",
        // A directory named like a manifest is walked, not matched.
        "manifest.ttl.d/manifest.ttl",
    ] {
        touch(root, manifest);
    }
    for decoy in [
        "group-a/query.rq",
        "group-a/manifest-all.ttl",
        "group-b/Manifest.ttl",
        "group-b/manifest.ttl.orig",
        "group-b/nested/xmanifest.ttl",
    ] {
        touch(root, decoy);
    }
    fs::create_dir_all(root.join("empty/also-empty")).expect("create empty directories");
    fs::create_dir_all(root.join("dir-named/manifest.ttl")).expect("create directory");

    assert_eq!(
        relatives(root),
        [
            "group-a/manifest.ttl",
            "group-b/nested/deeper/manifest.ttl",
            "manifest.ttl",
            "manifest.ttl.d/manifest.ttl",
        ]
    );
}

#[test]
fn the_order_is_by_components_compared_as_bytes() {
    let root = fixture_root();
    let root = root.path();
    // Created out of order, so a listing-order result would not pass by luck.
    for manifest in [
        "zeta/manifest.ttl",
        "a-b/manifest.ttl",
        "a/manifest.ttl",
        "a/b/manifest.ttl",
        "B/manifest.ttl",
        "a.b/manifest.ttl",
        "caf\u{e9}/manifest.ttl",
    ] {
        touch(root, manifest);
    }
    let expected = [
        // Upper-case letters are below lower-case ones as bytes.
        "B/manifest.ttl",
        // `a` is a prefix of `a-b` and `a.b`, so everything under `a/` comes
        // first, although `-` and `.` are below `/` as bytes.
        "a/b/manifest.ttl",
        "a/manifest.ttl",
        "a-b/manifest.ttl",
        "a.b/manifest.ttl",
        // The UTF-8 lead byte of `é` (0xC3) is above every ASCII byte.
        "caf\u{e9}/manifest.ttl",
        "zeta/manifest.ttl",
    ];
    assert_eq!(relatives(root), expected);
    assert_eq!(
        relatives(root),
        expected,
        "a second walk gives the same order"
    );
}

#[test]
fn an_empty_tree_is_an_empty_list_not_an_error() {
    let root = fixture_root();
    fs::create_dir_all(root.path().join("only/directories")).expect("create directories");
    assert_eq!(relatives(root.path()), Vec::<String>::new());
}

#[test]
fn a_missing_root_is_a_typed_error_naming_it() {
    let parent = fixture_root();
    let missing = parent.path().join("absent");
    let error = suite_manifests(&missing).expect_err("a missing root is refused");
    match &error {
        DiscoveryError::Io {
            operation, source, ..
        } => {
            assert_eq!(*operation, DiscoveryOperation::ReadDir);
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        DiscoveryError::NonUtf8Component { .. } => panic!("unexpected error: {error}"),
    }
    assert_eq!(error.path(), missing);
    assert!(
        error.to_string().contains(&missing.display().to_string()),
        "the message names the path: {error}"
    );

    // The neighbouring valid root, once it exists, is walked.
    touch(&missing, "group/manifest.ttl");
    assert_eq!(relatives(&missing), ["group/manifest.ttl"]);
}

#[test]
fn a_root_that_is_a_file_is_a_typed_error() {
    let parent = fixture_root();
    touch(parent.path(), "not-a-dir/manifest.ttl");
    let file = parent.path().join("not-a-dir/manifest.ttl");
    let error = suite_manifests(&file).expect_err("a file root is refused");
    assert!(
        matches!(
            error,
            DiscoveryError::Io {
                operation: DiscoveryOperation::ReadDir,
                ..
            }
        ),
        "{error}"
    );
    assert_eq!(error.path(), file);

    // Its directory, the valid neighbour, is walked and finds it.
    assert_eq!(
        relatives(&parent.path().join("not-a-dir")),
        ["manifest.ttl"]
    );
}

#[cfg(unix)]
#[test]
fn symbolic_links_are_followed_and_a_dangling_one_is_reported() {
    use std::os::unix::fs::symlink;

    let root = fixture_root();
    let root = root.path();
    touch(root, "real/manifest.ttl");
    touch(root, "elsewhere/target.ttl");
    symlink(root.join("real"), root.join("linked-dir")).expect("link a directory");
    fs::create_dir(root.join("real-file-link")).expect("create directory");
    symlink(
        root.join("elsewhere/target.ttl"),
        root.join("real-file-link").join(SUITE_MANIFEST_NAME),
    )
    .expect("link a file");
    assert_eq!(
        relatives(root),
        [
            "linked-dir/manifest.ttl",
            "real/manifest.ttl",
            "real-file-link/manifest.ttl",
        ]
    );

    let dangling = root.join("real/dangling");
    symlink(root.join("nowhere"), &dangling).expect("link to nothing");
    let error = suite_manifests(root).expect_err("a dangling link is refused");
    assert!(
        matches!(
            error,
            DiscoveryError::Io {
                operation: DiscoveryOperation::Metadata,
                ..
            }
        ),
        "{error}"
    );
    // Both routes into `real/` reach the link; either names a `dangling` path.
    assert_eq!(
        error.path().file_name(),
        Some(std::ffi::OsStr::new("dangling"))
    );
}

#[cfg(unix)]
#[test]
fn an_unreadable_directory_is_reported_not_skipped() {
    use std::os::unix::fs::PermissionsExt as _;

    let root = fixture_root();
    let root_path = root.path();
    touch(root_path, "open/manifest.ttl");
    touch(root_path, "locked/manifest.ttl");
    let locked = root_path.join("locked");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("lock directory");
    let listable = fs::read_dir(&locked).is_ok();
    let result = suite_manifests(root_path);
    // Restore access before any assertion, so the fixture can be removed.
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("unlock directory");

    if listable {
        // A privileged user can list a mode-000 directory; then nothing is
        // unreadable, and the walk must find both manifests.
        assert_eq!(
            result.expect("walk succeeds").len(),
            2,
            "a listable tree is walked completely"
        );
    } else {
        let error = result.expect_err("an unreadable directory is refused");
        assert!(
            matches!(
                error,
                DiscoveryError::Io {
                    operation: DiscoveryOperation::ReadDir,
                    ..
                }
            ),
            "{error}"
        );
        assert_eq!(error.path(), locked);
    }
    // Unlocked, the same tree is walked completely.
    assert_eq!(
        relatives(root_path),
        ["locked/manifest.ttl", "open/manifest.ttl"]
    );
}

#[cfg(target_os = "linux")]
#[test]
fn a_manifest_below_a_non_utf8_component_is_a_typed_error() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt as _;

    let root = fixture_root();
    let root = root.path();
    let bad = root.join(OsStr::from_bytes(b"group-\xff"));
    fs::create_dir(&bad).expect("create a non-UTF-8 directory");
    touch(root, "caf\u{e9}/manifest.ttl");

    // Neighbour: the non-UTF-8 directory holds no manifest, so nothing needs a
    // name from it and the walk succeeds.
    assert_eq!(relatives(root), ["caf\u{e9}/manifest.ttl"]);

    fs::write(bad.join(SUITE_MANIFEST_NAME), "# fixture\n").expect("write manifest");
    let error = suite_manifests(root).expect_err("an unnameable manifest is refused");
    assert!(
        matches!(error, DiscoveryError::NonUtf8Component { .. }),
        "{error}"
    );
    assert_eq!(error.path(), bad.join(SUITE_MANIFEST_NAME));
}

#[test]
fn the_live_suite_is_discovered_under_its_root() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("suite");
    let found = relatives(&root);
    assert!(
        found
            .iter()
            .any(|r| r == "w3c-sparql11/aggregates/manifest.ttl"),
        "the W3C SPARQL 1.1 aggregates group is discovered"
    );
    assert!(found.iter().any(|r| r == "purrdf-smoke/manifest.ttl"));
    for relative in &found {
        assert!(
            relative.ends_with("/manifest.ttl"),
            "every live manifest sits in a group directory: {relative}"
        );
    }
}
