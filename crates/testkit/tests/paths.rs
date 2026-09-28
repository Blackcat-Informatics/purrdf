// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace root is the nearest ancestor declaring a workspace, found
//! from the calling crate's manifest directory and never canonicalized; a
//! file sweep is sorted, filters by exact extension, skips the named
//! directories at every depth, and refuses an unreadable root.
//!
//! These run where a test can read a path; wasm32-unknown-unknown has no file
//! system, and the module does not exist there.

#![cfg(not(target_arch = "wasm32"))]

use std::path::{Path, PathBuf};

use purrdf_testkit::paths;
use purrdf_testkit::{temp_dir, workspace_root};

/// Write `contents` at `path`, creating its directories.
fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create the fixture directories");
    }
    std::fs::write(path, contents).expect("write the fixture file");
}

#[test]
fn the_macro_resolves_the_root_above_the_calling_crate() {
    let root = workspace_root!();
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(root.join("crates").join("testkit"), manifest_dir);
    assert_eq!(
        manifest_dir.strip_prefix(&root).map(Path::to_path_buf),
        Ok(PathBuf::from("crates").join("testkit")),
        "the root is an ancestor of the manifest directory as written, not canonicalized"
    );
    let text = std::fs::read_to_string(root.join("Cargo.toml")).expect("the root manifest");
    assert!(
        text.lines().any(|line| line.trim() == "[workspace]"),
        "{text}"
    );
    assert_eq!(paths::workspace_root_from(manifest_dir), root);
    assert_eq!(
        paths::workspace_root_from(&manifest_dir.join("src").join("prop")),
        root,
        "from any depth"
    );
    assert_eq!(
        paths::workspace_root_from(&root),
        root,
        "from the root itself"
    );
    let sources = paths::collect_rs(&manifest_dir.join("src"));
    for name in ["lib.rs", "paths.rs", "rng.rs", "prop/runner.rs"] {
        assert!(
            sources.contains(&manifest_dir.join("src").join(name)),
            "{name} in {sources:?}"
        );
    }
    assert!(sources.windows(2).all(|pair| pair[0] < pair[1]), "sorted");
}

#[test]
fn the_walk_passes_member_manifests_and_stops_at_the_nearest_workspace() {
    let dir = temp_dir!().expect("temp dir");
    // A workspace with a member: the member's manifest declares a package
    // only, and is walked past.
    let outer = dir.path().join("outer");
    write(
        &outer.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/member\"]\n",
    );
    let member = outer.join("crates").join("member");
    write(
        &member.join("Cargo.toml"),
        "[package]\nname = \"example-member\"\n",
    );
    write(&member.join("src").join("lib.rs"), "");
    assert_eq!(paths::workspace_root_from(&member), outer);
    assert_eq!(paths::workspace_root_from(&member.join("src")), outer);
    assert_eq!(
        paths::workspace_root_from(&member.join("src").join("absent")),
        outer,
        "the start need not exist"
    );
    // A nested workspace is the nearest one; a package whose manifest merely
    // mentions the word is not one.
    let inner = outer.join("inner");
    write(
        &inner.join("Cargo.toml"),
        "[package]\nname = \"inner\"\n\n[workspace]\n",
    );
    assert_eq!(paths::workspace_root_from(&inner.join("src")), inner);
    let mention = outer.join("mention");
    write(
        &mention.join("Cargo.toml"),
        "[package]\nname = \"workspace\"\n\n[package.metadata.workspace]\nnote = \"[workspace]\"\n",
    );
    assert_eq!(paths::workspace_root_from(&mention.join("src")), outer);
    // Any `[workspace.<key>]` table declares a workspace, and a header may
    // carry spaces inside its brackets.
    let by_package_table = dir.path().join("by-package-table");
    write(
        &by_package_table.join("Cargo.toml"),
        "[workspace.package]\nversion = \"0.0.0\"\n",
    );
    assert_eq!(
        paths::workspace_root_from(&by_package_table.join("crates").join("x")),
        by_package_table
    );
    let spaced = dir.path().join("spaced");
    write(&spaced.join("Cargo.toml"), "[ workspace ]\n");
    assert_eq!(
        paths::workspace_root_from(&spaced.join("crates").join("x")),
        spaced
    );
}

#[test]
#[should_panic(expected = "holds a Cargo.toml with a [workspace] table")]
fn a_path_under_no_workspace_is_refused() {
    let _ = paths::workspace_root_from(Path::new("/"));
}

#[test]
fn a_sweep_is_sorted_filtered_by_exact_extension_and_skips_the_named_directories() {
    let dir = temp_dir!().expect("temp dir");
    let root = dir.path();
    for relative in [
        "top.rs",
        "a/b.rs",
        "a/.hidden.rs",
        "a/c.txt",
        "a/target/hidden.rs",
        "z/y/x.rs",
        "z/y/x.RS",
        "z/y/rs",
        "noext",
        "target/d.rs",
        ".git/e.rs",
        "node_modules/f.rs",
        ".worktrees/other/g.rs",
    ] {
        write(&root.join(relative), "");
    }
    let expected: Vec<PathBuf> = ["a/.hidden.rs", "a/b.rs", "top.rs", "z/y/x.rs"]
        .iter()
        .map(|relative| root.join(relative))
        .collect();
    assert_eq!(paths::collect_rs(root), expected);
    assert_eq!(paths::collect_files(root, "rs"), expected);
    assert_eq!(
        paths::collect_files(root, ".rs"),
        expected,
        "a leading dot is accepted"
    );
    assert_eq!(
        paths::collect_files(root, "txt"),
        vec![root.join("a/c.txt")]
    );
    assert_eq!(
        paths::collect_files(root, "RS"),
        vec![root.join("z/y/x.RS")],
        "the extension is matched exactly"
    );
    assert_eq!(paths::collect_files(root, "md"), Vec::<PathBuf>::new());
    // The skip is by name below the root; the root itself is walked whatever
    // its name.
    assert_eq!(
        paths::collect_rs(&root.join("target")),
        vec![root.join("target/d.rs")]
    );
    assert_eq!(
        paths::collect_rs(&root.join(".worktrees")),
        vec![root.join(".worktrees/other/g.rs")]
    );
}

#[cfg(unix)]
#[test]
fn a_link_to_a_file_is_collected_and_a_link_to_a_directory_is_not_entered() {
    use std::os::unix::fs::symlink;
    let dir = temp_dir!().expect("temp dir");
    let root = dir.path();
    write(&root.join("src/real.rs"), "");
    symlink(root.join("src/real.rs"), root.join("src/alias.rs")).expect("link a file");
    symlink(root, root.join("src/cycle")).expect("link the root into itself");
    symlink(root.join("src/absent.rs"), root.join("src/dangling.rs")).expect("link nothing");
    assert_eq!(
        paths::collect_rs(root),
        vec![root.join("src/alias.rs"), root.join("src/real.rs")]
    );
}

#[test]
#[should_panic(expected = "read directory")]
fn an_unreadable_root_is_an_error_rather_than_an_empty_sweep() {
    let dir = temp_dir!().expect("temp dir");
    let _ = paths::collect_rs(&dir.path().join("absent"));
}

#[test]
#[should_panic(expected = "needs an extension")]
fn an_empty_extension_is_refused() {
    let dir = temp_dir!().expect("temp dir");
    let _ = paths::collect_files(dir.path(), ".");
}
