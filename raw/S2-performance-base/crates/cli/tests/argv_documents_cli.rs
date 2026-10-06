// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The command line's shared document-argument rules, driven through the built binary: one
//! standard input per process, and the one `--import IRI=FILE` parser.
//!
//! A refusal is a claim too, so every refusal here is paired with the neighbouring command
//! line that is valid and must still succeed (or reach a different, later refusal).

use std::process::Output;

mod support;
use support::{purrdf, stderr, write_file};

const PREMISE: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
    "ex:A rdfs:subClassOf ex:B .\n",
    "ex:x a ex:A .\n",
);
const CONCLUSION: &str = "@prefix ex: <http://example.org/> .\nex:x a ex:B .\n";

fn entails_with(args: &[&str], stdin: &str) -> Output {
    support::run_with_stdin(
        purrdf()
            .args(["entails", "--regime", "owl-rl", "--from", "turtle"])
            .args(args),
        stdin.as_bytes(),
    )
}

/// Two `-` documents are refused (exit 2, naming both); one `-` beside a file is answered.
#[test]
fn entails_refuses_two_stdins_and_answers_one() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let conclusion = write_file(dir.path(), "conclusion.ttl", CONCLUSION);

    let two = entails_with(&["--premise", "-", "--conclusion", "-"], PREMISE);
    assert_eq!(two.status.code(), Some(2), "{}", stderr(&two));
    let message = stderr(&two);
    assert!(
        message.contains("--premise and --conclusion each read standard input"),
        "{message}"
    );

    let one = entails_with(&["--premise", "-", "--conclusion", &conclusion], PREMISE);
    assert!(one.status.success(), "{}", stderr(&one));
    assert_eq!(
        String::from_utf8_lossy(&one.stdout),
        "mechanism strict-table\nentailment entailed\n"
    );
}

/// `validate` names every reader when three documents share stdin, and a lone stdin runs.
#[test]
fn validate_refuses_a_shared_stdin_across_data_shapes_and_changes() {
    let three = support::run_with_stdin(
        purrdf().args([
            "validate",
            "--shapes",
            "-",
            "--shapes-from",
            "turtle",
            "--changes",
            "-",
            "--from",
            "turtle",
            "-",
        ]),
        b"",
    );
    assert_eq!(three.status.code(), Some(2), "{}", stderr(&three));
    assert!(
        stderr(&three).contains("IN, --shapes and --changes each read standard input"),
        "{}",
        stderr(&three)
    );

    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let shapes = write_file(
        dir.path(),
        "shapes.ttl",
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    );
    let lone = support::run_with_stdin(
        purrdf().args(["validate", "--shapes", &shapes, "--from", "turtle", "-"]),
        b"<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
    );
    assert_eq!(lone.status.code(), Some(0), "{}", stderr(&lone));
}

/// `rules` counts `--srl` and its `--import` documents as readers, beside `IN`.
#[test]
fn rules_refuses_a_shared_stdin_across_input_srl_and_imports() {
    let out = support::run_with_stdin(
        purrdf().args([
            "rules",
            "--srl",
            "-",
            "--import",
            "http://example.org/rules=-",
            "--from",
            "turtle",
            "-",
        ]),
        b"",
    );
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("IN, --srl and --import http://example.org/rules=- each read"),
        "{}",
        stderr(&out)
    );
}

/// The one `--import` parser: a relative or repeated IRI half is refused before any file is
/// read; a neighbouring absolute, unrepeated pair passes the parser and reaches the later,
/// different refusal that the closure never reaches it.
#[test]
fn import_pairs_refuse_relative_and_repeated_iris_but_accept_an_absolute_one() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let conclusion = write_file(dir.path(), "conclusion.ttl", CONCLUSION);
    let document = write_file(dir.path(), "imported.ttl", CONCLUSION);
    let run = |pairs: &[&str]| {
        let mut args = vec!["--premise", "-", "--conclusion", conclusion.as_str()];
        for pair in pairs {
            args.extend(["--import", pair]);
        }
        entails_with(&args, PREMISE)
    };
    let relative = format!("foo={document}");
    let absolute = format!("http://example.org/a={document}");
    let other = format!("http://example.org/b={document}");
    let repeated = format!("http://example.org/a={conclusion}");

    let out = run(&[&relative]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("relative IRI reference"),
        "{}",
        stderr(&out)
    );

    let out = run(&[&absolute, &repeated]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(stderr(&out).contains("named twice"), "{}", stderr(&out));

    let out = run(&[&absolute, &other]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(stderr(&out).contains("never reaches"), "{}", stderr(&out));
}

const TWO_IMPORTS_PREMISE: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
    "ex:o a owl:Ontology ; owl:imports ex:one, ex:two .\n",
    "ex:tom a ex:Cat .\n",
);
const SCHEMA_ONE: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
    "ex:Cat rdfs:subClassOf ex:Animal .\n",
);
const SCHEMA_TWO: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
    "ex:Animal rdfs:subClassOf ex:Thing .\n",
);
const TOM_IS_A_THING: &str = "@prefix ex: <http://example.org/> .\nex:tom a ex:Thing .\n";

/// `rules --srl` counts an `--import IRI=-` document as a stdin reader: it is refused beside
/// `--srl -`, and it is the one reader that runs when everything else is a file.
#[test]
fn rules_runs_with_one_stdin_import_and_refuses_it_beside_another_reader() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(
        dir.path(),
        "data.ttl",
        "@prefix ex: <http://example.org/ns#> .\nex:a ex:n 1 .\n",
    );
    let srl = write_file(
        dir.path(),
        "rules.srl",
        "PREFIX ex: <http://example.org/ns#>\nIMPORTS <http://example.org/more>\n\
         RULE { ?x ex:q ?y } WHERE { ?x ex:n ?y }\n",
    );
    let imported = "PREFIX ex: <http://example.org/ns#>\n\
                    RULE { ?x ex:counted true } WHERE { ?x ex:q ?y }\n";

    let one = support::run_with_stdin(
        purrdf().args([
            "rules",
            "--srl",
            &srl,
            "--import",
            "http://example.org/more=-",
            "--to",
            "ntriples",
            &data,
        ]),
        imported.as_bytes(),
    );
    assert_eq!(one.status.code(), Some(0), "{}", stderr(&one));
    let graph = String::from_utf8_lossy(&one.stdout);
    assert!(
        graph.contains("<http://example.org/ns#a> <http://example.org/ns#counted>"),
        "the stdin import's rule ran: {graph}"
    );

    let two = support::run_with_stdin(
        purrdf().args([
            "rules",
            "--srl",
            "-",
            "--import",
            "http://example.org/more=-",
            "--to",
            "ntriples",
            &data,
        ]),
        imported.as_bytes(),
    );
    assert_eq!(two.status.code(), Some(2), "{}", stderr(&two));
    assert!(
        stderr(&two).contains("--srl and --import http://example.org/more=- each read"),
        "{}",
        stderr(&two)
    );
}

/// `validate --shapes-product` counts as a stdin reader beside `IN`: both `-` is refused, a
/// product file with a stdin data graph runs. (`--shapes` and `--shapes-product` are
/// mutually exclusive at the clap layer, so that pair never reaches the shared check.)
#[test]
fn validate_runs_with_a_product_file_and_refuses_a_product_on_stdin_beside_input() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let shapes = write_file(
        dir.path(),
        "shapes.ttl",
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    );
    let product = dir.path().join("shapes.purrshp");
    let product = product.to_str().expect("utf-8 path");
    let packed = purrdf()
        .args(["shacl", "pack", "--shapes", &shapes, "--out", product])
        .output()
        .expect("spawn the built CLI");
    assert_eq!(packed.status.code(), Some(0), "{}", stderr(&packed));
    let data = b"<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";

    let one = support::run_with_stdin(
        purrdf().args([
            "validate",
            "--shapes-product",
            product,
            "--from",
            "ntriples",
            "-",
        ]),
        data,
    );
    assert_eq!(one.status.code(), Some(0), "{}", stderr(&one));
    assert!(
        stderr(&one).contains("shacl conforms true"),
        "{}",
        stderr(&one)
    );

    let both = support::run_with_stdin(
        purrdf().args([
            "validate",
            "--shapes-product",
            "-",
            "--from",
            "ntriples",
            "-",
        ]),
        data,
    );
    assert_eq!(both.status.code(), Some(2), "{}", stderr(&both));
    assert!(
        stderr(&both).contains("IN and --shapes-product each read standard input"),
        "{}",
        stderr(&both)
    );
}

/// A repeated ontology IRI is refused by `entails` and `consistency`; two DISTINCT IRIs, each
/// imported by the premise, are accepted and both documents are used.
#[test]
fn a_repeated_import_iri_is_refused_and_two_distinct_iris_are_accepted() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let premise = write_file(dir.path(), "premise.ttl", TWO_IMPORTS_PREMISE);
    let one = write_file(dir.path(), "one.ttl", SCHEMA_ONE);
    let two = write_file(dir.path(), "two.ttl", SCHEMA_TWO);
    let conclusion = write_file(dir.path(), "conclusion.ttl", TOM_IS_A_THING);
    let pair_one = format!("http://example.org/one={one}");
    let pair_two = format!("http://example.org/two={two}");
    let repeated = format!("http://example.org/one={two}");

    let run = |args: &[&str]| purrdf().args(args).output().expect("spawn the built CLI");

    let ok = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &conclusion,
        "--import",
        &pair_one,
        "--import",
        &pair_two,
    ]);
    assert_eq!(ok.status.code(), Some(0), "{}", stderr(&ok));
    assert_eq!(
        String::from_utf8_lossy(&ok.stdout),
        "mechanism strict-table\nentailment entailed\n"
    );
    let refused = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &conclusion,
        "--import",
        &pair_one,
        "--import",
        &repeated,
    ]);
    assert_eq!(refused.status.code(), Some(2), "{}", stderr(&refused));
    assert!(
        stderr(&refused).contains("named twice"),
        "{}",
        stderr(&refused)
    );

    let ok = run(&[
        "consistency",
        &premise,
        "--import",
        &pair_one,
        "--import",
        &pair_two,
    ]);
    assert_eq!(ok.status.code(), Some(0), "{}", stderr(&ok));
    let refused = run(&[
        "consistency",
        &premise,
        "--import",
        &pair_one,
        "--import",
        &repeated,
    ]);
    assert_eq!(refused.status.code(), Some(2), "{}", stderr(&refused));
    assert!(
        stderr(&refused).contains("named twice"),
        "{}",
        stderr(&refused)
    );
}
