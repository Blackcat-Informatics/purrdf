// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Where the workspace is, and the Rust sources under a directory.
//!
//! A test that reads the repository — a vendored corpus, the committed
//! vocabulary, every crate's sources for a hygiene scan — needs the workspace
//! root. It is the nearest ancestor whose `Cargo.toml` declares `[workspace]`,
//! which is what Cargo itself resolves, and it is found from a directory rather
//! than by counting `..` segments, so a crate's depth below the root is not
//! part of the answer. Every member resolves the same root, so this crate's own
//! manifest directory is as good a starting point as the caller's.
//!
//! The root is canonical where the file system can canonicalize it, so paths
//! joined onto it compare equal to canonicalized paths found by a walk.

use std::path::{Path, PathBuf};

/// The workspace root: the nearest ancestor of `start` (itself included) whose
/// `Cargo.toml` has a line that is exactly `[workspace]` after trimming, or
/// `None` when no ancestor has one.
///
/// For a caller holding a path of its own — a manifest it was handed, a
/// directory it was asked to scan — rather than one inside the workspace
/// already. [`workspace_root`] is this from the workspace's own members.
#[must_use]
pub fn workspace_root_from(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| {
            std::fs::read_to_string(dir.join("Cargo.toml"))
                .is_ok_and(|text| text.lines().any(|line| line.trim() == "[workspace]"))
        })
        .map(Path::to_path_buf)
}

/// The workspace root, canonicalized when the file system allows.
///
/// # Panics
///
/// When no ancestor of this crate's manifest directory declares `[workspace]`,
/// which only a copy of the crate outside its workspace can observe.
#[must_use]
pub fn workspace_root() -> PathBuf {
    let root = workspace_root_from(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap_or_else(|| {
        panic!(
            "no ancestor of {} has a Cargo.toml declaring [workspace]",
            env!("CARGO_MANIFEST_DIR")
        )
    });
    root.canonicalize().unwrap_or(root)
}

/// Append every `.rs` file under `dir`, at any depth, to `out`.
///
/// Entries are visited in file-name order within each directory, so the order
/// appended is the same on every file system. A directory reached through a
/// symbolic link is walked like any other.
///
/// # Panics
///
/// When `dir` or a directory below it cannot be read. A scan that silently
/// skipped an unreadable directory would pass over exactly the sources it was
/// written to check.
pub fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("read an entry of {}: {error}", dir.display()))
                .path()
        })
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{collect_rs, workspace_root, workspace_root_from};
    use crate::TempDir;

    #[test]
    fn the_workspace_root_holds_the_workspace_manifest_and_this_crate() {
        let root = workspace_root();
        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("root manifest");
        assert!(manifest.lines().any(|line| line.trim() == "[workspace]"));
        assert!(root.join("crates/testkit/Cargo.toml").is_file());
    }

    #[test]
    fn a_member_directory_resolves_to_the_same_root() {
        let root = workspace_root();
        let from_member = workspace_root_from(&root.join("crates/testkit/src")).expect("found");
        assert_eq!(from_member, root);
    }

    #[test]
    fn a_member_manifest_without_the_workspace_table_is_passed_over() {
        let dir = TempDir::for_unit_test().expect("temp dir");
        let member = dir.path().join("outer/member");
        std::fs::create_dir_all(&member).expect("member dir");
        std::fs::write(member.join("Cargo.toml"), "[package]\nname = \"m\"\n").expect("member");
        std::fs::write(
            dir.path().join("outer/Cargo.toml"),
            "[package]\nname = \"o\"\n\n  [workspace]  \n",
        )
        .expect("outer");
        assert_eq!(
            workspace_root_from(&member),
            Some(dir.path().join("outer")),
            "the nearest ancestor declaring [workspace], trimmed"
        );
    }

    #[test]
    fn a_workspace_key_inside_another_line_is_not_the_table() {
        let dir = TempDir::for_unit_test().expect("temp dir");
        let inner = dir.path().join("a/b");
        std::fs::create_dir_all(&inner).expect("dirs");
        std::fs::write(
            dir.path().join("a/Cargo.toml"),
            "[package]\nworkspace = \"..\"\n# [workspace]\n",
        )
        .expect("manifest");
        let found = workspace_root_from(&inner);
        assert!(
            found
                .as_deref()
                .is_none_or(|root| !root.starts_with(dir.path())),
            "{found:?} is not inside the fixture"
        );
    }

    #[test]
    fn collect_rs_finds_nested_rust_sources_in_name_order_and_nothing_else() {
        let dir = TempDir::for_unit_test().expect("temp dir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("b/deep")).expect("dirs");
        std::fs::create_dir_all(root.join("a")).expect("dirs");
        for file in [
            "z.rs",
            "a/y.rs",
            "b/deep/x.rs",
            "b/notes.md",
            "b/rs",
            "c.rsx",
        ] {
            std::fs::write(root.join(file), "").expect("file");
        }
        let mut out = Vec::new();
        collect_rs(root, &mut out);
        let relative: Vec<String> = out
            .iter()
            .map(|path| {
                path.strip_prefix(root)
                    .expect("under the root")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert_eq!(relative, ["a/y.rs", "b/deep/x.rs", "z.rs"]);
    }

    #[test]
    fn collect_rs_appends_to_what_the_caller_already_holds() {
        let dir = TempDir::for_unit_test().expect("temp dir");
        std::fs::write(dir.path().join("one.rs"), "").expect("file");
        let mut out = vec![dir.path().join("earlier.rs")];
        collect_rs(dir.path(), &mut out);
        assert_eq!(
            out,
            [dir.path().join("earlier.rs"), dir.path().join("one.rs")]
        );
    }

    #[test]
    #[should_panic(expected = "read ")]
    fn collect_rs_refuses_an_unreadable_directory() {
        let dir = TempDir::for_unit_test().expect("temp dir");
        collect_rs(&dir.path().join("absent"), &mut Vec::new());
    }
}
