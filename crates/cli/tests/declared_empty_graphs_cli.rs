// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Declared empty named graphs through the shipped `purrdf` binary.
//!
//! A named graph a dataset declares without any row (`<g> { }`, `_:bg { }` in TriG)
//! either reaches the output or is reported: a pack carries it, so a pack round trip
//! returns it; a target with no spelling for an empty graph (N-Quads, HexTuples, every
//! single-graph syntax) drops it and records one `empty-named-graph-dropped` ledger
//! entry per graph. A dataset without such graphs records no such entry. All fixtures
//! use `example.org`.

mod support;
use support::{path, run, stderr, write_file};

const LOSS_CODE: &str = "\"code\": \"empty-named-graph-dropped\"";

/// A default-graph row, a named graph with a row, and two declared empty graphs: one
/// IRI-named, one blank-named.
const DECLARED_TRIG: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:s ex:p ex:o .\n",
    "ex:g1 { ex:s ex:p ex:o2 . }\n",
    "ex:empty { }\n",
    "_:bg { }\n",
);

/// [`DECLARED_TRIG`] without the two empty graphs.
const UNDECLARED_TRIG: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:s ex:p ex:o .\n",
    "ex:g1 { ex:s ex:p ex:o2 . }\n",
);

fn convert(args: &[&str]) -> (String, String) {
    let mut full = vec!["--loss-ledger", "convert"];
    full.extend_from_slice(args);
    let out = run(&full);
    assert!(
        out.status.success(),
        "convert {args:?} must succeed; stderr:\n{}",
        stderr(&out)
    );
    let output_path = args[1];
    (
        std::fs::read_to_string(output_path).unwrap_or_default(),
        stderr(&out),
    )
}

#[test]
fn a_pack_round_trip_keeps_iri_and_blank_named_empty_graphs() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let dir = dir.path();
    let source = write_file(dir, "in.trig", DECLARED_TRIG);
    let pack = path(dir, "x.pack");
    let back = path(dir, "back.trig");

    let (_, ledger) = convert(&[&source, &pack, "--to", "pack"]);
    assert!(
        !ledger.contains("\"code\""),
        "a pack carries declared graphs, so nothing is lost; got:\n{ledger}"
    );
    let (trig, ledger) = convert(&[&pack, &back, "--from", "pack", "--to", "trig"]);
    assert!(!ledger.contains("\"code\""), "got:\n{ledger}");
    assert!(
        trig.contains("<http://example.org/empty> {"),
        "the IRI-named empty graph must survive the pack; got:\n{trig}"
    );
    assert!(
        trig.lines()
            .any(|line| line.starts_with("_:") && line.contains('{')),
        "the blank-named empty graph must survive the pack; got:\n{trig}"
    );
    assert!(trig.contains("<http://example.org/g1> {"), "got:\n{trig}");
}

#[test]
fn a_query_over_a_pack_enumerates_its_declared_empty_graphs() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let dir = dir.path();
    let source = write_file(dir, "in.trig", DECLARED_TRIG);
    let pack = path(dir, "x.pack");
    convert(&[&source, &pack, "--to", "pack"]);

    let out = run(&[
        "query",
        "--data",
        &pack,
        "SELECT (COUNT(?g) AS ?n) WHERE { GRAPH ?g { } }",
    ]);
    assert!(out.status.success(), "stderr:\n{}", stderr(&out));
    let body = support::stdout(&out);
    assert!(
        body.contains("\"value\":\"3\""),
        "GRAPH ?g over the pack must see g1 and both declared empty graphs; got:\n{body}"
    );
}

#[test]
fn targets_without_an_empty_graph_spelling_report_each_dropped_graph() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let dir = dir.path();
    let source = write_file(dir, "in.trig", DECLARED_TRIG);
    for (target, extension) in [
        ("nquads", "nq"),
        ("hextuples", "hext"),
        ("turtle", "ttl"),
        ("ntriples", "nt"),
    ] {
        let output = path(dir, &format!("out.{extension}"));
        let (_, ledger) = convert(&[&source, &output, "--to", target]);
        assert_eq!(
            ledger.matches(LOSS_CODE).count(),
            2,
            "{target}: one entry per dropped empty graph; got:\n{ledger}"
        );
        assert!(
            ledger.contains("<http://example.org/empty>") && ledger.contains("_:"),
            "{target}: each entry names its graph; got:\n{ledger}"
        );
        // Every target names itself and the registered pair renders the loss as
        // intentional: the drop is what the target's grammar requires, not a defect.
        let registered = format!(
            "{LOSS_CODE},\n      \"from\": \"trig\",\n      \"to\": \"{target}\",\n      \
             \"intentional\": true"
        );
        assert_eq!(
            ledger.matches(&registered).count(),
            2,
            "{target}: got:\n{ledger}"
        );
    }
}

#[test]
fn targets_that_spell_empty_graphs_and_datasets_without_them_report_nothing_new() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let dir = dir.path();
    let declared = write_file(dir, "declared.trig", DECLARED_TRIG);
    for (target, extension) in [
        ("trig", "trig"),
        ("trix", "trix"),
        ("jsonld", "jsonld"),
        ("yamlld", "yamlld"),
        ("pack", "pack"),
    ] {
        let output = path(dir, &format!("declared-out.{extension}"));
        let (_, ledger) = convert(&[&declared, &output, "--to", target]);
        assert!(
            !ledger.contains(LOSS_CODE),
            "{target} writes empty graphs; got:\n{ledger}"
        );
    }

    let undeclared = write_file(dir, "undeclared.trig", UNDECLARED_TRIG);
    for (target, extension) in [("nquads", "nq"), ("hextuples", "hext"), ("turtle", "ttl")] {
        let output = path(dir, &format!("undeclared-out.{extension}"));
        let (_, ledger) = convert(&[&undeclared, &output, "--to", target]);
        assert!(
            !ledger.contains(LOSS_CODE),
            "{target}: no declared empty graph, no entry; got:\n{ledger}"
        );
    }
}
