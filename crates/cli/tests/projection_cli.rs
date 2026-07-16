// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end `project`/`lift` carrier coverage over the built CLI.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const PURRDF: &str = env!("CARGO_BIN_EXE_purrdf");

fn run(args: &[&str]) -> Output {
    Command::new(PURRDF)
        .args(args)
        .output()
        .expect("spawn purrdf")
}

fn run_with_stdin(args: &[&str], stdin: &[u8]) -> Output {
    let mut child = Command::new(PURRDF)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn purrdf");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin)
        .expect("write stdin");
    child.wait_with_output().expect("wait")
}

fn write(path: &Path, bytes: &[u8]) -> String {
    std::fs::write(path, bytes).expect("write fixture");
    path.to_str().expect("UTF-8 path").to_owned()
}

fn lpg_config() -> &'static [u8] {
    br#"{
  "profile": "lpg-csv",
  "config": {
    "rdf_type": "https://example.org/type",
    "limits": {
      "max_artifacts": 16,
      "max_artifact_bytes": 1000000,
      "max_total_bytes": 4000000,
      "max_archive_bytes": 5000000,
      "max_term_depth": 16
    },
    "max_records": 1000
  }
}"#
}

const TURTLE: &[u8] = b"@prefix ex: <https://example.org/> .\nex:s ex:p ex:o .\n";

#[test]
fn project_is_byte_deterministic_and_lift_round_trips_with_ledgers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write(&dir.path().join("input.ttl"), TURTLE);
    let config = write(&dir.path().join("config.json"), lpg_config());
    let first = dir.path().join("first.tar");
    let second = dir.path().join("second.tar");
    let first_path = first.to_str().expect("first path");
    let second_path = second.to_str().expect("second path");

    let projected = run(&[
        "project",
        "--profile",
        "lpg-csv",
        "--config",
        &config,
        "--loss-ledger",
        &input,
        first_path,
    ]);
    assert!(
        projected.status.success(),
        "project failed: {}",
        String::from_utf8_lossy(&projected.stderr)
    );
    assert!(projected.stdout.is_empty());
    let ledger: serde_json::Value =
        serde_json::from_slice(&projected.stderr).expect("project ledger JSON");
    assert_eq!(ledger["schema_version"], 1);
    assert!(
        ledger["losses"]
            .as_array()
            .expect("loss array")
            .iter()
            .any(|entry| entry["code"] == "lpg-edge-semantics-lowered")
    );

    let repeated = run(&[
        "project",
        "--profile",
        "lpg-csv",
        "--config",
        &config,
        &input,
        second_path,
    ]);
    assert!(repeated.status.success());
    assert_eq!(
        std::fs::read(&first).expect("first archive"),
        std::fs::read(&second).expect("second archive")
    );

    let lifted = run(&[
        "lift",
        "--profile",
        "lpg-csv",
        "--config",
        &config,
        "--to",
        "nquads",
        "--loss-ledger",
        first_path,
        "-",
    ]);
    assert!(
        lifted.status.success(),
        "lift failed: {}",
        String::from_utf8_lossy(&lifted.stderr)
    );
    assert_eq!(
        String::from_utf8(lifted.stdout).expect("N-Quads"),
        "<https://example.org/s> <https://example.org/p> <https://example.org/o> .\n"
    );
    let ledger: serde_json::Value =
        serde_json::from_slice(&lifted.stderr).expect("lift ledger JSON");
    assert_eq!(ledger["schema_version"], 1);
    assert!(!ledger["losses"].as_array().expect("loss array").is_empty());
}

#[test]
fn stdin_stdout_paths_keep_binary_and_rdf_streams_clean() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = write(&dir.path().join("config.json"), lpg_config());
    let projected = run_with_stdin(
        &[
            "project",
            "--profile",
            "lpg-csv",
            "--config",
            &config,
            "--from",
            "turtle",
            "-",
            "-",
        ],
        TURTLE,
    );
    assert!(
        projected.status.success(),
        "project stdin failed: {}",
        String::from_utf8_lossy(&projected.stderr)
    );
    assert!(projected.stderr.is_empty());
    assert!(projected.stdout.len() >= 1_536);

    let lifted = run_with_stdin(
        &[
            "lift",
            "--profile",
            "lpg-csv",
            "--config",
            &config,
            "--to",
            "turtle",
            "-",
            "-",
        ],
        &projected.stdout,
    );
    assert!(
        lifted.status.success(),
        "lift stdin failed: {}",
        String::from_utf8_lossy(&lifted.stderr)
    );
    assert!(lifted.stderr.is_empty());
    let turtle = String::from_utf8(lifted.stdout).expect("Turtle");
    assert!(turtle.contains("https://example.org/s"));
}

#[test]
fn malformed_config_archive_and_double_stdin_fail_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write(&dir.path().join("input.ttl"), TURTLE);
    let bad_config = write(
        &dir.path().join("bad.json"),
        br#"{"profile":"lpg-csv","config":{"rdf_type":"relative"}}"#,
    );
    let output = run(&[
        "project",
        "--profile",
        "lpg-csv",
        "--config",
        &bad_config,
        &input,
        "-",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("configuration JSON"));

    let config = write(&dir.path().join("config.json"), lpg_config());
    let corrupt = write(&dir.path().join("corrupt.tar"), b"not an archive");
    let output = run(&[
        "lift",
        "--profile",
        "lpg-csv",
        "--config",
        &config,
        "--to",
        "turtle",
        &corrupt,
        "-",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("projection package error"));

    let output = run_with_stdin(
        &[
            "project",
            "--profile",
            "lpg-csv",
            "--config",
            "-",
            "--from",
            "turtle",
            "-",
            "-",
        ],
        lpg_config(),
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot both read from stdin"));
}
