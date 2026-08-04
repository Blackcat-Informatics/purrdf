// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Process-boundary tests for the SPARQL UPDATE pipeline and its atomic governor trip.

use std::process::{Command, Output};

const PURRDF: &str = env!("CARGO_BIN_EXE_purrdf");
const INSERT: &str =
    "INSERT DATA { <http://example.org/new> <http://example.org/p> <http://example.org/value> }";

fn run(args: &[&str]) -> Output {
    Command::new(PURRDF)
        .args(args)
        .output()
        .expect("spawn purrdf")
}

fn fixture(dir: &std::path::Path) -> String {
    let path = dir.join("input.ttl");
    std::fs::write(
        &path,
        "<http://example.org/old> <http://example.org/p> <http://example.org/value> .\n",
    )
    .expect("write fixture");
    path.to_str().expect("UTF-8 path").to_owned()
}

#[test]
fn update_applies_then_serializes_the_committed_dataset() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = fixture(dir.path());
    let output = run(&["update", "--data", &input, "--to", "ntriples", INSERT]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let body = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(body.contains("<http://example.org/old>"), "{body}");
    assert!(body.contains("<http://example.org/new>"), "{body}");
}

#[test]
fn governed_update_trip_writes_no_dataset_and_exits_three() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = fixture(dir.path());
    let target = dir.path().join("must-not-exist.nt");
    let target = target.to_str().expect("UTF-8 path");
    let output = run(&[
        "update", "--data", &input, "--output", target, "--to", "ntriples", "--fuel", "0", INSERT,
    ]);

    assert_eq!(output.status.code(), Some(3));
    assert!(
        output.stdout.is_empty(),
        "a tripped mutation emits no dataset"
    );
    assert!(!std::path::Path::new(target).exists());
    let report = String::from_utf8(output.stderr).expect("UTF-8 report");
    assert!(report.starts_with("purrdf-governor-report 1\n"), "{report}");
    assert!(report.contains("\noperation update\n"), "{report}");
    assert!(report.contains("\ntripped fuel-exhausted\n"), "{report}");
    assert!(report.contains("\nmutation none\n"), "{report}");
}
