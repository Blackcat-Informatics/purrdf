// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Process-boundary tests for the SPARQL UPDATE pipeline and its atomic governor trip.

mod support;
use support::run;

const INSERT: &str =
    "INSERT DATA { <http://example.org/new> <http://example.org/p> <http://example.org/value> }";

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
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let input = fixture(dir.path());
    let output = run(&["update", "--data", &input, "--to", "ntriples", INSERT]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stderr, [] as [_; 0]);
    let body = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(body.contains("<http://example.org/old>"), "{body}");
    assert!(body.contains("<http://example.org/new>"), "{body}");
}

#[test]
fn governed_update_trip_writes_no_dataset_and_exits_three() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
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

/// Run `update` over a TriG document and return the TriG it writes.
fn update_trig(document: &str, update: &str) -> String {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let path = dir.path().join("input.trig");
    std::fs::write(&path, document).expect("write fixture");
    let input = path.to_str().expect("UTF-8 path");
    let output = run(&["update", "--data", input, "--to", "trig", update]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    String::from_utf8(output.stdout).expect("UTF-8 output")
}

const DECLARED: &str = "@prefix ex: <http://example.org/> .\n\
    ex:a ex:p ex:c .\n\
    GRAPH ex:g {}\n\
    GRAPH ex:h { ex:a ex:b ex:c }\n\
    GRAPH _:bg {}\n";

#[test]
fn drop_all_writes_no_graph_the_input_declared() {
    assert_eq!(update_trig(DECLARED, "DROP ALL").trim(), "");
}

#[test]
fn drop_graph_writes_no_declaration_of_the_dropped_graph() {
    let body = update_trig(DECLARED, "DROP GRAPH <http://example.org/g>");
    assert!(!body.contains("<http://example.org/g>"), "{body}");
    assert!(body.contains("<http://example.org/h> {"), "{body}");
    assert!(body.contains("_:bg {}"), "{body}");
}

#[test]
fn a_no_op_update_keeps_the_input_declarations() {
    let body = update_trig(
        DECLARED,
        "DELETE DATA { <http://example.org/none> <http://example.org/p> <http://example.org/none> }",
    );
    assert!(body.contains("<http://example.org/g> {}"), "{body}");
    assert!(body.contains("_:bg {}"), "{body}");
    assert!(body.contains("<http://example.org/h> {"), "{body}");
}

#[test]
fn a_graph_emptied_by_delete_is_neither_enumerated_nor_written() {
    // The graph `g1` loses its only quad; `GRAPH ?g` later in the same request must
    // not bind it, and the result must not declare it. The declared empty `e` that no
    // operation touched still enumerates and is still written.
    let update = "DELETE DATA { GRAPH <http://example.org/g1> { \
        <http://example.org/a> <http://example.org/b> <http://example.org/c> } } ; \
        INSERT { ?g <http://example.org/p> \"var-enumerated\" } \
        WHERE { GRAPH ?g { BIND(1 AS ?x) } }";
    let body = update_trig(
        "<http://example.org/g1> { <http://example.org/a> <http://example.org/b> <http://example.org/c> }\n",
        update,
    );
    assert_eq!(body.trim(), "", "{body}");
    let body = update_trig(
        "<http://example.org/g1> { <http://example.org/a> <http://example.org/b> <http://example.org/c> }\n\
         GRAPH <http://example.org/e> {}\n",
        update,
    );
    assert!(!body.contains("g1"), "{body}");
    assert!(
        body.contains("<http://example.org/e> <http://example.org/p> \"var-enumerated\" ."),
        "{body}"
    );
    assert!(body.contains("<http://example.org/e> {}"), "{body}");
}
