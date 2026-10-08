// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exercise production glossary inputs and local/hosted gate parity.

use purrdf_testkit::TempDir;
use std::path::Path;
use std::process::{Command, Output};

const GATE: &str = "cargo run -q --locked -p helper-census -- --glossary-gate";
const PO: &str = "docs/book/po/zh-Hans.po";
const GLOSSARY: &str = "docs/book/po/glossary-zh-Hans.md";

fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let mut command = Command::new("git");
    for name in [
        "GIT_DIR",
        "GIT_INDEX_FILE",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
    ] {
        command.env_remove(name);
    }
    let output = command.arg("-C").arg(root).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn scan(root: &Path, po: &Path, glossary: &Path) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_helper-census"));
    for name in [
        "GIT_DIR",
        "GIT_INDEX_FILE",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
    ] {
        command.env_remove(name);
    }
    command
        .arg("--glossary-gate")
        .arg("--root")
        .arg(root)
        .arg("--po")
        .arg(po)
        .arg("--glossary")
        .arg(glossary)
        .output()
        .unwrap()
}

fn assert_verdict(output: &Output, success: bool, diagnostic: &str) {
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.success(), success, "{text}");
    assert!(text.contains(diagnostic), "missing {diagnostic:?}: {text}");
}

#[test]
fn external_inputs_and_content_selected_renamed_markdown() {
    let source = purrdf_testkit::paths::workspace_root();
    let fixture = TempDir::for_unit_test().unwrap();
    let root = fixture.path().join("repository");
    std::fs::create_dir(&root).unwrap();
    git(&root, &["init", "-q"]);
    write(
        &root,
        PO,
        &std::fs::read_to_string(source.join(PO)).unwrap(),
    );
    let external_po = fixture.path().join("external.po");
    let external_glossary = fixture.path().join("external-glossary.md");
    std::fs::copy(source.join(GLOSSARY), &external_glossary).unwrap();
    std::fs::write(
        &external_po,
        "msgid \"RDF toolkit\"\nmsgstr \"RDF 工具包。\"\n",
    )
    .unwrap();
    write(
        &root,
        "not-language-named.md",
        "普通中文。\n```text\n资料类型\n具名图\n```\n普通中文。\n",
    );
    git(&root, &["add", "."]);
    assert_verdict(
        &scan(&root, &external_po, &external_glossary),
        true,
        "1 tracked translated Markdown",
    );
    std::fs::rename(root.join("not-language-named.md"), root.join("renamed.md")).unwrap();
    git(&root, &["add", "-A"]);
    assert_verdict(
        &scan(&root, &external_po, &external_glossary),
        true,
        "1 tracked translated Markdown",
    );
    write(&root, "renamed.md", "普通中文。资料类型。\n");
    assert_verdict(
        &scan(&root, &external_po, &external_glossary),
        false,
        "renamed.md:1:",
    );
    write(&root, "renamed.md", "普通中文。\n");
    std::fs::write(&external_po, "msgid \"RDF toolkit\"\nmsgstr \"工具包。\"\n").unwrap();
    assert_verdict(
        &scan(&root, &external_po, &external_glossary),
        false,
        "external.po:",
    );
    std::fs::remove_file(&external_glossary).unwrap();
    assert_verdict(
        &scan(&root, &external_po, &external_glossary),
        false,
        "external-glossary.md",
    );
}

#[test]
fn production_parity_refuses_missing_native_glossary_callers() {
    let source = purrdf_testkit::paths::workspace_root();
    let fixture = TempDir::for_unit_test().unwrap();
    let root = fixture.path();
    let makefile = std::fs::read_to_string(source.join("Makefile")).unwrap();
    std::fs::create_dir(root.join("scripts")).unwrap();
    std::fs::copy(
        source.join("scripts/check-gate-parity.py"),
        root.join("scripts/check-gate-parity.py"),
    )
    .unwrap();
    for entry in std::fs::read_dir(source.join(".github/workflows")).unwrap() {
        let entry = entry.unwrap();
        let relative = format!(".github/workflows/{}", entry.file_name().to_str().unwrap());
        write(
            root,
            &relative,
            &std::fs::read_to_string(entry.path()).unwrap(),
        );
    }
    let parity = || {
        Command::new("python3")
            .arg(root.join("scripts/check-gate-parity.py"))
            .output()
            .unwrap()
    };
    write(root, "Makefile", &makefile);
    assert_verdict(&parity(), true, "hygiene gates");
    write(root, "Makefile", &makefile.replace(GATE, "true"));
    assert_verdict(&parity(), false, "helper-census --glossary-gate");
    // The docs job reaches check-i18n transitively; omit only that occurrence,
    // preserving make check's native gate to exercise the reverse direction.
    let split = makefile.find("check-i18n:").unwrap();
    let (before, after) = makefile.split_at(split);
    write(
        root,
        "Makefile",
        &format!("{before}{}", after.replacen(GATE, "true", 1)),
    );
    let ci = std::fs::read_to_string(root.join(".github/workflows/ci.yaml")).unwrap();
    write(root, ".github/workflows/ci.yaml", &ci.replace(GATE, "true"));
    assert_verdict(&parity(), false, "helper-census --glossary-gate");
}
