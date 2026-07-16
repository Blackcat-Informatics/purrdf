// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end smoke of the three core RDF subcommands, driving the built `purrdf` binary
//! over `example.org` fixtures written to a temp dir:
//!
//! The two graph/tabular carrier commands have their own production smoke in
//! `projection_cli.rs`.
//!
//! * `convert --from ttl --to nt` produces the expected N-Triples;
//! * `query --results-format json` returns a row, and the SAME query over a pack
//!   built via `convert --to pack` is byte-identical;
//! * `reason --regime rdfs` materializes the inferred `rdf:type` triple;
//! * `reason --regime owl-direct` exits with the unsupported-regime code 3.

use std::path::Path;
use std::process::Command;

/// The path to the built `purrdf` binary.
const PURRDF: &str = env!("CARGO_BIN_EXE_purrdf");

/// Run `purrdf` with `args`, returning (exit-code, stdout-bytes, stderr-string).
fn run(args: &[&str]) -> (i32, Vec<u8>, String) {
    let output = Command::new(PURRDF)
        .args(args)
        .output()
        .expect("spawn purrdf");
    (
        output.status.code().expect("exit code"),
        output.stdout,
        String::from_utf8(output.stderr).expect("utf-8 stderr"),
    )
}

/// Write `contents` to `dir/name` and return the path as an owned string.
fn write_fixture(dir: &Path, name: &str, contents: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, contents).expect("write fixture");
    path.to_str().expect("utf-8 path").to_owned()
}

#[test]
fn convert_turtle_to_ntriples_produces_expected_triples() {
    let dir = tempfile::tempdir().expect("temp dir");
    let input = write_fixture(
        dir.path(),
        "in.ttl",
        "@prefix ex: <http://example.org/> .\nex:alice ex:knows ex:bob .\n",
    );

    let (code, stdout, stderr) = run(&["convert", "--from", "ttl", "--to", "nt", &input, "-"]);
    assert_eq!(code, 0, "convert exits 0; stderr:\n{stderr}");
    let stdout = String::from_utf8(stdout).expect("utf-8 stdout");
    assert_eq!(
        stdout,
        "<http://example.org/alice> <http://example.org/knows> <http://example.org/bob> .\n",
        "convert must emit the canonical single N-Triples line"
    );
}

#[test]
fn query_json_row_matches_between_turtle_and_pack() {
    let dir = tempfile::tempdir().expect("temp dir");
    let input = write_fixture(
        dir.path(),
        "in.ttl",
        "@prefix ex: <http://example.org/> .\nex:alice ex:knows ex:bob .\n",
    );
    let query = "SELECT ?o WHERE { ?s <http://example.org/knows> ?o }";

    // Query over the Turtle source.
    let (code, ttl_stdout, stderr) =
        run(&["query", "--data", &input, "--results-format", "json", query]);
    assert_eq!(code, 0, "query over turtle exits 0; stderr:\n{stderr}");
    let ttl_stdout = String::from_utf8(ttl_stdout).expect("utf-8 stdout");
    assert!(
        ttl_stdout.contains("http://example.org/bob"),
        "the SELECT must return the bound object; got:\n{ttl_stdout}"
    );

    // Build a pack from the same source, then query it.
    let pack = dir.path().join("data.purrpck");
    let pack = pack.to_str().expect("utf-8 path");
    let (code, _, stderr) = run(&["convert", &input, pack]);
    assert_eq!(
        code, 0,
        "convert to pack exits 0 (formats inferred from extensions); stderr:\n{stderr}"
    );

    let (code, pack_stdout, stderr) =
        run(&["query", "--data", pack, "--results-format", "json", query]);
    assert_eq!(code, 0, "query over pack exits 0; stderr:\n{stderr}");
    let pack_stdout = String::from_utf8(pack_stdout).expect("utf-8 stdout");

    assert_eq!(
        ttl_stdout, pack_stdout,
        "the JSON result must be byte-identical whether queried over turtle or its pack"
    );
}

#[test]
fn reason_rdfs_infers_rdf_type() {
    let dir = tempfile::tempdir().expect("temp dir");
    let input = write_fixture(
        dir.path(),
        "sub.ttl",
        "@prefix ex: <http://example.org/> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
         ex:Dog rdfs:subClassOf ex:Animal .\n\
         ex:rex a ex:Dog .\n",
    );
    let output = dir.path().join("closure.nt");
    let output = output.to_str().expect("utf-8 path");

    let (code, _, stderr) = run(&["reason", "--regime", "rdfs", &input, output]);
    assert_eq!(code, 0, "reason exits 0; stderr:\n{stderr}");

    let closure = std::fs::read_to_string(output).expect("read closure");
    // RDFS subClassOf entailment: ex:rex is inferred to be an ex:Animal.
    assert!(
        closure.contains(
            "<http://example.org/rex> \
             <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
             <http://example.org/Animal> ."
        ),
        "the inferred `ex:rex a ex:Animal` triple must be present; got:\n{closure}"
    );
}

#[test]
fn reason_owl_direct_exits_with_unsupported_regime_code_3() {
    let dir = tempfile::tempdir().expect("temp dir");
    let input = write_fixture(
        dir.path(),
        "sub.ttl",
        "@prefix ex: <http://example.org/> .\nex:rex a ex:Dog .\n",
    );
    let output = dir.path().join("out.nt");
    let output = output.to_str().expect("utf-8 path");

    let (code, _, stderr) = run(&["reason", "--regime", "owl-direct", &input, output]);
    assert_eq!(
        code, 3,
        "owl-direct is an unsupported-regime boundary (exit 3); stderr:\n{stderr}"
    );
    assert!(
        !stderr.is_empty(),
        "the unsupported-regime failure must print a diagnostic to stderr"
    );
}
