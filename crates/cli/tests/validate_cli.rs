// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end `validate` coverage that drives the BUILT `purrdf` binary
//! (`env!("CARGO_BIN_EXE_purrdf")`) — never the library — so every assertion pins the shipped
//! executable's SHACL surface as an installed consumer meets it.
//!
//! ## What is pinned here
//!
//! * the canonical artifact is the W3C SHACL **validation report graph**, on stdout, with
//!   `--format sarif` as the named projection of the same run;
//! * a NON-CONFORMING graph still exits **0** — a decided verdict, exactly like `consistency
//!   false` and a `false` ASK — and the verdict reaches a shell as two `key value` lines on
//!   **stderr**, so stdout stays a well-formed document;
//! * every native source syntax and the pack container reach the identical verdict;
//! * the RDF 1.2 statement layer is validated, because `purrdf-shapes` projects reifier
//!   bindings and annotations into quads before it validates;
//! * `--shapes-graph` is load-bearing (a SHACL-SPARQL constraint that reads the shapes graph
//!   changes its answer with and without it);
//! * a tripped governor writes NO report and exits **3**;
//! * every inapplicable flag is refused BY NAME rather than accepted and ignored.

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// A `Command` for the built `purrdf` binary.
fn purrdf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_purrdf"))
}

/// Run `purrdf` with `args`, returning the captured [`Output`].
fn run(args: &[&str]) -> Output {
    purrdf()
        .args(args)
        .output()
        .expect("spawn the built purrdf binary")
}

/// Run `purrdf` with `args`, writing `stdin_bytes` to its standard input.
fn pipe(args: &[&str], stdin_bytes: &str) -> Output {
    let mut child = purrdf()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn purrdf");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(stdin_bytes.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("wait for purrdf")
}

/// stdout of an [`Output`] as a `String`.
fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// stderr of an [`Output`] as a `String`.
fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The exit code of an [`Output`].
fn code(out: &Output) -> i32 {
    out.status.code().expect("the process exited normally")
}

/// Write `contents` to `dir/name`, returning the path as a `String`.
fn write_file(dir: &Path, name: &str, contents: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, contents).expect("write fixture file");
    p.to_str().expect("temp path is valid UTF-8").to_owned()
}

/// A `sh:datatype` shape over `ex:Person`, the smallest shapes graph with one violation to
/// find and one node to leave alone.
const SHAPES: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <http://example.org/> .\n",
    "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
    "ex:PersonShape a sh:NodeShape ;\n",
    "  sh:targetClass ex:Person ;\n",
    "  sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .\n",
);

/// Two people: `alice`'s age is a string (one violation), `bob`'s is an integer.
const DATA: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:alice a ex:Person ; ex:age \"nope\" .\n",
    "ex:bob a ex:Person ; ex:age 42 .\n",
);

/// The same graph with the offending node removed: conforming.
const CONFORMING_DATA: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:bob a ex:Person ; ex:age 42 .\n",
);

/// An RDF 1.2 document whose annotation carries a decimal on a predicate the star shapes
/// graph below constrains to `xsd:integer`.
const STAR_DATA: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:alice ex:knows ex:bob {| ex:certainty 0.9 |} .\n",
);

/// A shapes graph whose focus nodes are the SUBJECTS of `ex:certainty` — which, in
/// [`STAR_DATA`], exist only in the RDF 1.2 statement layer.
const STAR_SHAPES: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <http://example.org/> .\n",
    "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
    "ex:AnnotationShape a sh:NodeShape ;\n",
    "  sh:targetSubjectsOf ex:certainty ;\n",
    "  sh:property [ sh:path ex:certainty ; sh:datatype xsd:integer ] .\n",
);

/// A SHACL-SPARQL constraint, which is what engages the execution governors (core constraint
/// evaluation reads the IR directly and charges nothing).
const SPARQL_SHAPES: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <http://example.org/> .\n",
    "ex:PersonShape a sh:NodeShape ;\n",
    "  sh:targetClass ex:Person ;\n",
    "  sh:sparql [\n",
    "    a sh:SPARQLConstraint ;\n",
    "    sh:message \"every person needs a name\" ;\n",
    "    sh:select \"\"\"SELECT $this WHERE { $this a <http://example.org/Person> . ",
    "FILTER NOT EXISTS { $this <http://example.org/name> ?n } }\"\"\"\n",
    "  ] .\n",
);

/// A SHACL-SPARQL constraint that is satisfied only when the SHAPES GRAPH is visible as a
/// named graph — the one thing `--shapes-graph` does.
const SHAPES_GRAPH_SHAPES: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <http://example.org/> .\n",
    "ex:PersonShape a sh:NodeShape ;\n",
    "  sh:targetClass ex:Person ;\n",
    "  sh:sparql [\n",
    "    a sh:SPARQLConstraint ;\n",
    "    sh:message \"the shapes graph must be visible\" ;\n",
    "    sh:select \"\"\"SELECT $this WHERE { $this a <http://example.org/Person> . ",
    "FILTER NOT EXISTS { GRAPH <http://example.org/shapes> { ",
    "<http://example.org/PersonShape> a <http://www.w3.org/ns/shacl#NodeShape> } } }\"\"\"\n",
    "  ] .\n",
);

/// The report graph's `sh:conforms` triple, as N-Triples, for either verdict.
fn conforms_triple(value: bool) -> String {
    format!(
        "<http://www.w3.org/ns/shacl#conforms> \"{value}\"^^\
         <http://www.w3.org/2001/XMLSchema#boolean> ."
    )
}

/// A non-conforming data graph produces the SHACL results graph on stdout, the verdict on
/// stderr, and exit **0**.
///
/// Exit 0 is the load-bearing assertion: the run did exactly what it was asked to do, and a
/// decided "your data violates your shapes" is no more a failure of this command than a
/// `false` ASK is a failure of `query`. Mapping it onto an error code would put it in the same
/// bucket as a corrupt pack, which is the flattening [`crate::error`] argues against.
#[test]
fn a_non_conforming_graph_reports_and_still_exits_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&["validate", "--shapes", &shapes, &data]);
    assert_eq!(code(&out), 0, "a decided verdict exits 0: {}", stderr(&out));

    let report = stdout(&out);
    assert!(
        report.contains(&conforms_triple(false)),
        "the report graph states the verdict:\n{report}"
    );
    assert!(
        report.contains("http://www.w3.org/ns/shacl#DatatypeConstraintComponent"),
        "the violated constraint component is named:\n{report}"
    );
    assert!(
        report.contains("<http://example.org/alice>"),
        "the focus node is named:\n{report}"
    );
    assert!(
        !report.contains("<http://example.org/bob>"),
        "the conforming node produces no result:\n{report}"
    );

    let verdict = stderr(&out);
    assert!(
        verdict.contains("shacl conforms false\n"),
        "the verdict reaches a shell without parsing stdout: {verdict}"
    );
    assert!(
        verdict.contains("shacl results 1\n"),
        "the result count is stated: {verdict}"
    );
}

/// A conforming data graph produces a `sh:conforms true` report with no results, and the same
/// exit 0.
#[test]
fn a_conforming_graph_reports_conforms_true() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data = write_file(dir.path(), "ok.ttl", CONFORMING_DATA);

    let out = run(&["validate", "--shapes", &shapes, &data]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains(&conforms_triple(true)),
        "{}",
        stdout(&out)
    );
    assert!(
        !stdout(&out).contains("ValidationResult"),
        "a conforming report carries no results:\n{}",
        stdout(&out)
    );
    assert!(stderr(&out).contains("shacl conforms true\n"));
    assert!(stderr(&out).contains("shacl results 0\n"));
}

/// EVERY native source syntax, plus the pack container, reaches the identical verdict.
///
/// Each variant is produced from the same Turtle by `purrdf convert`, so the only thing under
/// test is whether `validate` reads that syntax into the same graph. A syntax that silently
/// lost the type assertion or the offending literal would change the result count.
#[test]
fn every_native_source_format_reaches_the_same_verdict() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let seed = write_file(dir.path(), "seed.ttl", DATA);

    for (token, extension) in [
        ("turtle", "ttl"),
        ("trig", "trig"),
        ("ntriples", "nt"),
        ("nquads", "nq"),
        ("rdfxml", "rdf"),
        ("trix", "trix"),
        ("hextuples", "hext"),
        ("jsonld", "jsonld"),
        ("yamlld", "yamlld"),
        ("pack", "purrpck"),
    ] {
        let target = dir
            .path()
            .join(format!("data.{extension}"))
            .to_str()
            .expect("temp path is valid UTF-8")
            .to_owned();
        let converted = run(&["convert", "--from", "turtle", "--to", token, &seed, &target]);
        assert!(
            converted.status.success(),
            "seeding the `{token}` source failed: {}",
            stderr(&converted)
        );

        // The extension alone resolves the format — no `--from` needed, exactly as for
        // `convert`/`reason`.
        let out = run(&["validate", "--shapes", &shapes, &target]);
        assert_eq!(code(&out), 0, "`{token}`: {}", stderr(&out));
        assert!(
            stdout(&out).contains(&conforms_triple(false)),
            "`{token}` must reach the same verdict:\n{}",
            stdout(&out)
        );
        assert!(
            stderr(&out).contains("shacl results 1\n"),
            "`{token}` must find the same single violation: {}",
            stderr(&out)
        );
    }
}

/// A shapes graph in a non-Turtle syntax is read through the native codec into the same IR.
///
/// Turtle takes the privileged `parse_shapes` route (it is the boundary every other host uses,
/// and the one that recovers the document's prefix map for SHACL-AF); every other syntax is
/// parsed by the codec and handed to `shapes::from_dataset`. This pins that the second route
/// reaches the same shapes.
#[test]
fn a_non_turtle_shapes_graph_is_read_through_the_native_codec() {
    let dir = tempfile::tempdir().expect("tempdir");
    let turtle = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);
    let as_nt = dir
        .path()
        .join("shapes.nt")
        .to_str()
        .expect("temp path")
        .to_owned();
    let converted = run(&[
        "convert", "--from", "turtle", "--to", "ntriples", &turtle, &as_nt,
    ]);
    assert!(converted.status.success(), "{}", stderr(&converted));

    let out = run(&["validate", "--shapes", &as_nt, &data]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("shacl conforms false\n"),
        "{}",
        stderr(&out)
    );

    // And the explicit override reaches the same place as the inferred extension.
    let renamed = write_file(
        dir.path(),
        "shapes.bin",
        &std::fs::read_to_string(&as_nt).unwrap(),
    );
    let overridden = run(&[
        "validate",
        "--shapes",
        &renamed,
        "--shapes-from",
        "ntriples",
        &data,
    ]);
    assert_eq!(code(&overridden), 0, "{}", stderr(&overridden));
    assert!(stderr(&overridden).contains("shacl conforms false\n"));
}

/// The RDF 1.2 statement layer is validated: `purrdf-shapes` projects reifier bindings and
/// annotations into quads, so a shape targeting the SUBJECTS of an annotation predicate finds
/// the reifier as a focus node and checks the annotation value.
///
/// Without that projection the shapes graph would have no focus node at all and the report
/// would vacuously conform — which is exactly the silent pass this asserts against.
#[test]
fn rdf12_statement_metadata_is_validated() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "star-shapes.ttl", STAR_SHAPES);
    let data = write_file(dir.path(), "star.ttl", STAR_DATA);

    let out = run(&["validate", "--shapes", &shapes, &data]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let report = stdout(&out);
    assert!(
        report.contains(&conforms_triple(false)),
        "the annotation's decimal must violate the xsd:integer constraint:\n{report}"
    );
    assert!(
        report.contains("\"0.9\"^^<http://www.w3.org/2001/XMLSchema#decimal>"),
        "the offending ANNOTATION value is the reported value node:\n{report}"
    );
    assert!(
        report.contains("<http://example.org/certainty>"),
        "the annotation predicate is the result path:\n{report}"
    );
    assert!(
        stderr(&out).contains("shacl results 1\n"),
        "{}",
        stderr(&out)
    );
}

/// The same statement-layer graph reaches the same verdict through the pack container, which
/// is the one carrier that stores the star layer losslessly.
#[test]
fn rdf12_statement_metadata_survives_a_pack_source() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "star-shapes.ttl", STAR_SHAPES);
    let data = write_file(dir.path(), "star.ttl", STAR_DATA);
    let pack = dir
        .path()
        .join("star.purrpck")
        .to_str()
        .expect("temp path")
        .to_owned();
    let packed = run(&["convert", "--from", "turtle", "--to", "pack", &data, &pack]);
    assert!(packed.status.success(), "{}", stderr(&packed));

    let out = run(&["validate", "--shapes", &shapes, &pack]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("\"0.9\"^^<http://www.w3.org/2001/XMLSchema#decimal>"),
        "a pack carries the statement layer into the validator:\n{}",
        stdout(&out)
    );
}

/// `--format` reaches all nine RDF syntaxes AND SARIF, and every one of them describes the
/// same run.
#[test]
fn every_output_format_describes_the_same_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);

    for token in [
        "ntriples",
        "turtle",
        "trig",
        "nquads",
        "rdfxml",
        "trix",
        "hextuples",
        "jsonld",
        "yamlld",
    ] {
        let out = run(&["validate", "--shapes", &shapes, "--format", token, &data]);
        assert_eq!(code(&out), 0, "`{token}`: {}", stderr(&out));
        let body = stdout(&out);
        assert!(
            body.contains("DatatypeConstraintComponent"),
            "`{token}` must carry the violated component:\n{body}"
        );
        assert!(
            body.contains("conforms"),
            "`{token}` must carry the sh:conforms triple:\n{body}"
        );
        assert!(
            stderr(&out).contains("shacl conforms false\n"),
            "`{token}`: {}",
            stderr(&out)
        );
    }

    let sarif = run(&["validate", "--shapes", &shapes, "--format", "sarif", &data]);
    assert_eq!(code(&sarif), 0, "{}", stderr(&sarif));
    let log = stdout(&sarif);
    assert!(
        log.contains("\"version\": \"2.1.0\""),
        "the SARIF log declares its version:\n{log}"
    );
    assert!(
        log.contains("\"level\": \"error\""),
        "a sh:Violation maps to SARIF `error`:\n{log}"
    );
    assert!(
        log.contains("DatatypeConstraintComponent"),
        "the SARIF rule id names the violated component:\n{log}"
    );
    assert!(
        stderr(&sarif).contains("shacl conforms false\n"),
        "the verdict is on stderr whichever artifact stdout carries: {}",
        stderr(&sarif)
    );
}

/// `--shapes-graph` is load-bearing: a SHACL-SPARQL constraint that reads the shapes graph as
/// a named graph fails without the flag and passes with it.
///
/// The negative half is the point — without it the assertion could pass over a constraint that
/// was never evaluated at all.
#[test]
fn shapes_graph_exposes_the_shapes_to_shacl_sparql() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "sg.ttl", SHAPES_GRAPH_SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);

    let without = run(&["validate", "--shapes", &shapes, &data]);
    assert_eq!(code(&without), 0, "{}", stderr(&without));
    assert!(
        stderr(&without).contains("shacl conforms false\n"),
        "without the flag the shapes graph is invisible and the constraint fires: {}",
        stderr(&without)
    );

    let with = run(&[
        "validate",
        "--shapes",
        &shapes,
        "--shapes-graph",
        "http://example.org/shapes",
        &data,
    ]);
    assert_eq!(code(&with), 0, "{}", stderr(&with));
    assert!(
        stderr(&with).contains("shacl conforms true\n"),
        "with the flag the constraint finds the shapes graph: {}",
        stderr(&with)
    );
}

/// A tripped governor writes NO report and exits 3.
///
/// `purrdf_shapes::engine::GovernedValidation` deliberately has no partial-report variant —
/// every SHACL constraint is a negative claim, so a truncated solution bag cannot license a
/// `conforms`. This pins that the CLI carries that through unflattened: stdout is EMPTY, the
/// governor receipt is on stderr in the shared `key value` grammar, and the exit code is the
/// same 3 a governed `query` uses.
#[test]
fn a_tripped_governor_writes_no_report_and_exits_three() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "sparql-shapes.ttl", SPARQL_SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&["validate", "--shapes", &shapes, "--fuel", "0", &data]);
    assert_eq!(code(&out), 3, "a governor trip exits 3: {}", stderr(&out));
    assert!(
        stdout(&out).is_empty(),
        "a trip yields NO report, not a partial one:\n{}",
        stdout(&out)
    );

    let receipt = stderr(&out);
    assert!(
        receipt.starts_with("purrdf-governor-report 1\n"),
        "the shared governor-report banner:\n{receipt}"
    );
    assert!(
        receipt.contains("\noutcome budget-exhausted\n"),
        "{receipt}"
    );
    assert!(receipt.contains("\noperation validate\n"), "{receipt}");
    assert!(
        receipt.contains("\nreport none\n"),
        "the all-or-nothing receipt states that nothing was produced:\n{receipt}"
    );
    assert!(receipt.contains("\ntripped fuel-exhausted\n"), "{receipt}");
    assert!(receipt.contains("\nlimit fuel 0\n"), "{receipt}");
    assert!(
        !receipt.contains("shacl conforms"),
        "there is no verdict to state when there is no report:\n{receipt}"
    );
}

/// A shapes graph with NO SPARQL in it validates under any budget, including zero.
///
/// Core constraint evaluation reads the IR directly and spends no evaluator budget, which the
/// engine documents as the honest answer rather than an oversight. Without this control the
/// test above could be passing because `--fuel 0` trips everything.
#[test]
fn a_core_only_validation_is_unbothered_by_a_zero_budget() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&["validate", "--shapes", &shapes, "--fuel", "0", &data]);
    assert_eq!(
        code(&out),
        0,
        "a core-only validation charges no fuel: {}",
        stderr(&out)
    );
    assert!(stdout(&out).contains(&conforms_triple(false)));
}

/// The data graph may arrive on stdin, and `-` requires `--from` because it has no extension.
#[test]
fn stdin_data_requires_an_explicit_from_and_then_validates() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);

    let bare = pipe(&["validate", "--shapes", &shapes, "-"], DATA);
    assert_eq!(code(&bare), 2, "a usage error: stdin has no extension");
    assert!(
        stderr(&bare).contains("--from"),
        "the refusal names the missing flag: {}",
        stderr(&bare)
    );

    let explicit = pipe(
        &["validate", "--shapes", &shapes, "--from", "turtle", "-"],
        DATA,
    );
    assert_eq!(code(&explicit), 0, "{}", stderr(&explicit));
    assert!(stdout(&explicit).contains(&conforms_triple(false)));
    assert!(stderr(&explicit).contains("shacl conforms false\n"));
}

/// The SHAPES graph may arrive on stdin instead, under `--shapes-from`.
#[test]
fn stdin_shapes_are_read_under_shapes_from() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);

    let bare = pipe(&["validate", "--shapes", "-", &data], SHAPES);
    assert_eq!(code(&bare), 2, "a usage error: {}", stderr(&bare));

    let explicit = pipe(
        &[
            "validate",
            "--shapes",
            "-",
            "--shapes-from",
            "turtle",
            &data,
        ],
        SHAPES,
    );
    assert_eq!(code(&explicit), 0, "{}", stderr(&explicit));
    assert!(stderr(&explicit).contains("shacl conforms false\n"));
}

/// Both documents naming stdin is refused by name, never mis-read as two halves of one stream.
#[test]
fn two_stdin_documents_are_refused_by_name() {
    let out = pipe(
        &[
            "validate",
            "--shapes",
            "-",
            "--shapes-from",
            "turtle",
            "--from",
            "turtle",
            "-",
        ],
        DATA,
    );
    assert_eq!(code(&out), 2, "a usage error");
    let message = stderr(&out);
    assert!(
        message.contains("--shapes") && message.contains("standard input"),
        "the refusal names both readers: {message}"
    );
}

/// A malformed shapes graph and a malformed data graph are both runtime failures (exit 1),
/// distinct from the usage errors above and from a non-conforming verdict.
#[test]
fn malformed_documents_are_runtime_failures() {
    let dir = tempfile::tempdir().expect("tempdir");
    let good_shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let good_data = write_file(dir.path(), "data.ttl", DATA);
    let bad_shapes = write_file(dir.path(), "bad-shapes.ttl", "@@@ not turtle at all");
    let bad_data = write_file(dir.path(), "bad-data.ttl", "<a> <b> .");

    let shapes_failure = run(&["validate", "--shapes", &bad_shapes, &good_data]);
    assert_eq!(code(&shapes_failure), 1, "{}", stderr(&shapes_failure));
    assert!(
        stderr(&shapes_failure).contains("--shapes"),
        "the diagnostic names the document that failed: {}",
        stderr(&shapes_failure)
    );
    assert!(
        stdout(&shapes_failure).is_empty(),
        "a failed run writes no report"
    );

    let data_failure = run(&["validate", "--shapes", &good_shapes, &bad_data]);
    assert_eq!(code(&data_failure), 1, "{}", stderr(&data_failure));
    assert!(stdout(&data_failure).is_empty());
}

/// An unsupported SHACL construct hard-fails rather than being skipped.
///
/// A property shape with no `sh:path` is structurally incomplete; the engine refuses it
/// instead of validating the shapes it *could* understand and reporting a verdict that quietly
/// omits one constraint.
#[test]
fn a_structurally_incomplete_shape_hard_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(
        dir.path(),
        "pathless.ttl",
        concat!(
            "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
            "@prefix ex: <http://example.org/> .\n",
            "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
            "ex:PersonShape a sh:NodeShape ;\n",
            "  sh:targetClass ex:Person ;\n",
            "  sh:property [ sh:datatype xsd:integer ] .\n",
        ),
    );
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&["validate", "--shapes", &shapes, &data]);
    assert_eq!(
        code(&out),
        1,
        "an unusable shape is a runtime failure, not a silently skipped constraint: {}",
        stderr(&out)
    );
    assert!(stdout(&out).is_empty(), "no verdict is invented");
}

/// `--base` resolves relative IRIs in the data graph, and the resolution is what makes the
/// shapes graph apply at all.
///
/// The unbased run is the negative control, and it is an ERROR. It used to be a vacuous
/// PASS: the relative terms were interned verbatim, so they simply were not `ex:Person` and
/// `ex:alice`, no target selected a focus node, and the report conformed with zero results.
/// That is the worst possible outcome for a validator — a clean "conforms true" over a
/// document none of whose constraints were actually evaluated. A relative IRI with no base
/// in scope is now refused outright, so a conformance verdict is never reported over terms
/// that were never resolved.
#[test]
fn base_resolves_relative_iris_in_the_data_graph() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let relative = concat!(
        "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n",
        "<alice> rdf:type <Person> ; <age> \"nope\" .\n",
    );

    let unbased = pipe(
        &["validate", "--shapes", &shapes, "--from", "turtle", "-"],
        relative,
    );
    assert_eq!(
        code(&unbased),
        1,
        "a relative IRI with no base must be refused, never vacuously conform: {}",
        stderr(&unbased)
    );
    assert!(
        stderr(&unbased).contains("iri-relative-no-base"),
        "the refusal carries the code for the condition --base fixes: {}",
        stderr(&unbased)
    );
    assert!(
        !stderr(&unbased).contains("shacl conforms"),
        "no conformance verdict may be reported over unresolved terms: {}",
        stderr(&unbased)
    );

    let based = pipe(
        &[
            "validate",
            "--shapes",
            &shapes,
            "--from",
            "turtle",
            "--base",
            "http://example.org/",
            "-",
        ],
        relative,
    );
    assert_eq!(code(&based), 0, "{}", stderr(&based));
    assert!(
        stderr(&based).contains("shacl conforms false\n"),
        "the resolved IRIs must reach the same shape the absolute fixture does: {}",
        stderr(&based)
    );
    assert!(
        stdout(&based).contains("<http://example.org/alice>"),
        "the resolved focus node is the absolute IRI:\n{}",
        stdout(&based)
    );
}

/// `--base` with a pack data source is refused by name: a pack stores fully-resolved terms and
/// has no relative-IRI syntax, so the base would be accepted and silently unread.
#[test]
fn base_with_a_pack_data_source_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);
    let pack = dir
        .path()
        .join("data.purrpck")
        .to_str()
        .expect("temp path")
        .to_owned();
    let packed = run(&["convert", "--from", "turtle", "--to", "pack", &data, &pack]);
    assert!(packed.status.success(), "{}", stderr(&packed));

    let out = run(&[
        "validate",
        "--shapes",
        &shapes,
        "--base",
        "http://example.org/",
        &pack,
    ]);
    assert_eq!(code(&out), 2, "a usage error");
    assert!(
        stderr(&out).contains("--base"),
        "the refusal names the flag: {}",
        stderr(&out)
    );
}

/// `--loss-ledger` is LIVE for an RDF `--format` — the results graph really does cross a
/// serializer — and refused for `--format sarif`, which runs none.
#[test]
fn the_loss_ledger_is_live_for_rdf_and_refused_for_sarif() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);
    let ledger_path = dir
        .path()
        .join("validate.loss.json")
        .to_str()
        .expect("temp path")
        .to_owned();

    let rdf = run(&[
        &format!("--loss-ledger={ledger_path}"),
        "validate",
        "--shapes",
        &shapes,
        "--format",
        "rdfxml",
        &data,
    ]);
    assert_eq!(code(&rdf), 0, "{}", stderr(&rdf));
    let ledger: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&ledger_path).expect("ledger written"))
            .expect("the ledger is JSON");
    assert_eq!(ledger["schema_version"], 1, "the ledger's stable schema");
    assert!(
        ledger["losses"].is_array(),
        "the ledger records the ntriples -> rdfxml contract: {ledger}"
    );

    let sarif = run(&[
        "--loss-ledger",
        "validate",
        "--shapes",
        &shapes,
        "--format",
        "sarif",
        &data,
    ]);
    assert_eq!(code(&sarif), 2, "a usage error");
    assert!(
        stderr(&sarif).contains("--loss-ledger"),
        "the refusal names the flag: {}",
        stderr(&sarif)
    );
}

/// `--jsonld-options` requires a JSON-LD/YAML-LD `--format`, and is refused for SARIF and for
/// every other RDF syntax rather than accepted and ignored.
#[test]
fn jsonld_options_are_refused_unless_the_format_is_jsonld() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);
    let options = write_file(
        dir.path(),
        "jsonld-options.json",
        r#"{"version":1,"mode":"context","prefixes":{"sh":"http://www.w3.org/ns/shacl#"}}"#,
    );

    for format in ["ntriples", "turtle", "sarif"] {
        let out = run(&[
            "--jsonld-options",
            &options,
            "validate",
            "--shapes",
            &shapes,
            "--format",
            format,
            &data,
        ]);
        assert_eq!(code(&out), 2, "`{format}` must refuse it: {}", stderr(&out));
        assert!(
            stderr(&out).contains("--jsonld-options"),
            "`{format}`: the refusal names the flag: {}",
            stderr(&out)
        );
    }

    let accepted = run(&[
        "--jsonld-options",
        &options,
        "validate",
        "--shapes",
        &shapes,
        "--format",
        "jsonld",
        &data,
    ]);
    assert_eq!(code(&accepted), 0, "{}", stderr(&accepted));
    assert!(
        stdout(&accepted).contains("@context"),
        "the configured serializer really ran:\n{}",
        stdout(&accepted)
    );
}

/// The report can be written to a FILE, leaving stdout untouched and the verdict on stderr.
#[test]
fn the_report_can_be_written_to_a_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);
    let report = dir
        .path()
        .join("report.ttl")
        .to_str()
        .expect("temp path")
        .to_owned();

    let out = run(&[
        "validate", "--shapes", &shapes, "--format", "turtle", &data, &report,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).is_empty(), "the report went to the file");
    assert!(stderr(&out).contains("shacl conforms false\n"));
    let written = std::fs::read_to_string(&report).expect("report written");
    assert!(written.contains("DatatypeConstraintComponent"), "{written}");
}

/// The emitted report is ordinary RDF, so the rest of the binary can read it — which is the
/// concrete payoff of making the results graph the default artifact rather than SARIF.
#[test]
fn the_report_is_a_graph_the_binary_can_query() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);
    let report = dir
        .path()
        .join("report.nt")
        .to_str()
        .expect("temp path")
        .to_owned();

    let validated = run(&["validate", "--shapes", &shapes, &data, &report]);
    assert_eq!(code(&validated), 0, "{}", stderr(&validated));

    let queried = run(&[
        "query",
        "--data",
        &report,
        "--results-format",
        "csv",
        "SELECT ?focus WHERE { ?r <http://www.w3.org/ns/shacl#focusNode> ?focus }",
    ]);
    assert!(queried.status.success(), "{}", stderr(&queried));
    assert!(
        stdout(&queried).contains("http://example.org/alice"),
        "the report answers a SPARQL query about itself:\n{}",
        stdout(&queried)
    );
}

/// An unknown `--format` token is a clap usage error (exit 2), not a silently defaulted run.
#[test]
fn an_unknown_format_token_is_a_usage_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&["validate", "--shapes", &shapes, "--format", "yaml", &data]);
    assert_eq!(code(&out), 2, "clap rejects an unknown value with exit 2");
    assert!(!stderr(&out).is_empty());
}

/// `--shapes` is required: `validate` never invents an empty shapes graph (which would
/// vacuously conform).
#[test]
fn shapes_are_required() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&["validate", &data]);
    assert_eq!(code(&out), 2, "a missing required flag is a usage error");
    assert!(
        stderr(&out).contains("--shapes"),
        "the refusal names the missing flag: {}",
        stderr(&out)
    );
}
