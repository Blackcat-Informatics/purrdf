// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit, serialized qualification of the real staged-snapshot hook.
//! Uses a private index and restores exact working-tree catalogue bytes on exit.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const PO: &str = "docs/book/po/zh-Hans.po";
const POISON: &str = "\nmsgid \"RDF qualification probe\"\nmsgstr \"资格检验。\"\n";

struct Restore {
    path: PathBuf,
    bytes: Vec<u8>,
}

impl Drop for Restore {
    fn drop(&mut self) {
        std::fs::write(&self.path, &self.bytes).expect("restore exact catalogue bytes");
    }
}

fn git(root: &Path, index: &Path, args: &[&str], input: Option<&[u8]>) -> Output {
    let mut child = Command::new("git")
        .current_dir(root)
        .env("GIT_INDEX_FILE", index)
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("git");
    if let Some(bytes) = input {
        child.stdin.take().unwrap().write_all(bytes).unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn hook(root: &Path, index: &Path, logs: &Path, name: &str, success: bool) {
    let output = Command::new("bash")
        .arg(root.join(".githooks/pre-commit"))
        .current_dir(root)
        .env("GIT_INDEX_FILE", index)
        .env("CARGO_BUILD_JOBS", "8")
        .output()
        .expect("real pre-commit hook");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::write(logs.join(format!("{name}.log")), &text).unwrap();
    assert_eq!(output.status.success(), success, "{name}: {text}");
    if !success {
        assert!(
            text.contains("glossary-gate failed") && text.contains("RDF"),
            "{name}: {text}"
        );
    }
    println!("PASS: {name}; real hook status {}", output.status);
}

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    assert_eq!(
        args.len(),
        2,
        "usage: glossary_hook_probe ROOT LOG_DIRECTORY"
    );
    let root = PathBuf::from(&args[0]).canonicalize().unwrap();
    let real_index_output = Command::new("git")
        .current_dir(&root)
        .env_remove("GIT_INDEX_FILE")
        .args(["rev-parse", "--path-format=absolute", "--git-path", "index"])
        .output()
        .unwrap();
    assert!(real_index_output.status.success());
    let real_index = PathBuf::from(
        std::str::from_utf8(&real_index_output.stdout)
            .unwrap()
            .trim(),
    );
    let real_index_before = std::fs::read(&real_index).unwrap();
    let logs = PathBuf::from(&args[1]);
    std::fs::create_dir_all(&logs).unwrap();
    let scratch = purrdf_testkit::TempDir::for_unit_test().unwrap();
    let index = scratch.path().join("index");
    git(&root, &index, &["read-tree", "HEAD"], None);
    git(&root, &index, &["add", "-u"], None);
    git(
        &root,
        &index,
        &[
            "add",
            "crates/helper-census/tests/glossary_callers.rs",
            "crates/helper-census/examples/glossary_hook_probe.rs",
        ],
        None,
    );
    let clean_index = std::fs::read(&index).unwrap();
    let restore = Restore {
        path: root.join(PO),
        bytes: std::fs::read(root.join(PO)).unwrap(),
    };
    let catalogue_before = restore.bytes.clone();
    let mut poisoned = restore.bytes.clone();
    poisoned.extend_from_slice(POISON.as_bytes());
    let hash = git(
        &root,
        &index,
        &["hash-object", "-w", "--stdin"],
        Some(&poisoned),
    );
    let oid = std::str::from_utf8(&hash.stdout).unwrap().trim();
    git(
        &root,
        &index,
        &["update-index", "--cacheinfo", "100644", oid, PO],
        None,
    );
    hook(
        &root,
        &index,
        &logs,
        "poisoned-index-clean-working-tree",
        false,
    );
    std::fs::write(&index, clean_index).unwrap();
    std::fs::write(&restore.path, poisoned).unwrap();
    hook(
        &root,
        &index,
        &logs,
        "clean-index-poisoned-working-tree",
        true,
    );
    drop(restore);
    assert_eq!(
        std::fs::read(&real_index).unwrap(),
        real_index_before,
        "real index changed"
    );
    assert_eq!(
        std::fs::read(root.join(PO)).unwrap(),
        catalogue_before,
        "catalogue not restored"
    );
    println!("PASS: exact real-index and catalogue bytes unchanged");
}
