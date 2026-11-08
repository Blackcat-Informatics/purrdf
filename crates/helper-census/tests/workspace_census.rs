// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Repository-only checks over the caller's workspace and shared regex fixtures.

use std::collections::BTreeSet;

use purrdf_testkit::paths::{collect_rs_filtered, workspace_root_from};
use purrdf_testkit::workspace_root;

#[test]
fn the_calling_workspace_root_holds_the_shared_test_support() {
    let root = workspace_root!();
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("root manifest");
    assert!(manifest.lines().any(|line| line.trim() == "[workspace]"));
    let member = root.join("crates/testkit");
    assert!(member.join("Cargo.toml").is_file());
    assert_eq!(workspace_root_from(&member.join("src")), Some(root));
}

#[test]
fn the_pattern_list_is_exactly_the_workspace_patterns() {
    let root = workspace_root!();
    let crates = root.join("crates");
    let testkit = crates.join("testkit");
    let mut sources = Vec::new();
    collect_rs_filtered(&crates, &mut sources, &|path| {
        path != testkit && path.file_name().is_none_or(|name| name != "target")
    });
    let call = regex::Regex::new(r#"prop::string::regex\("((?:\\.|[^"\\])*)"\)"#)
        .expect("the source pattern compiles");
    let mut used = BTreeSet::new();
    for path in sources {
        let source = std::fs::read_to_string(path).expect("UTF-8 Rust source");
        for capture in call.captures_iter(&source) {
            let literal = format!("\"{}\"", &capture[1]);
            used.insert(
                syn::parse_str::<syn::LitStr>(&literal)
                    .expect("Rust string literal")
                    .value(),
            );
        }
    }
    assert!(
        used.len() > 40,
        "the scan found the workspace's patterns: {used:?}"
    );
    let patterns = std::fs::read_to_string(testkit.join("tests/regex-patterns.txt"))
        .expect("shared property generator fixtures");
    let listed: BTreeSet<String> = patterns
        .lines()
        .filter(|pattern| *pattern != "\\PC*")
        .map(str::to_owned)
        .collect();
    let unlisted: Vec<&String> = used.difference(&listed).collect();
    let stale: Vec<&String> = listed.difference(&used).collect();
    assert!(
        unlisted.is_empty(),
        "patterns used but not oracle-checked: {unlisted:?}"
    );
    assert!(
        stale.is_empty(),
        "patterns listed but no longer used: {stale:?}"
    );
}
