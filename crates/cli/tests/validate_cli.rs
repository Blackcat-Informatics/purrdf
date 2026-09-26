// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

use std::path::Path;
use std::process::{Command, Output};

mod support;

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
///
/// A `BrokenPipe` from that write is EXPECTED, not a failure. Several cases here
/// pipe data to an invocation that is refused at the command line — a `-` stdin
/// input with no `--from`, say — and those refusals are decided BEFORE stdin is
/// read, which is the whole point: a malformed request should not require reading
/// the document first. So the child can exit and close the pipe while the parent
/// is still writing, and whether it does is a race between two processes.
///
/// Panicking on that turned a correct refusal into an intermittently red gate.
/// Every other write error still panics, and the assertions on exit code, stdout
/// and stderr are untouched — a child that exited early is judged by what it
/// returned, exactly as before.
fn pipe(args: &[&str], stdin_bytes: &str) -> Output {
    support::run_with_stdin(purrdf().args(args), stdin_bytes.as_bytes())
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
    // The banner is a LINE of the receipt rather than its first byte. `validate` writes its
    // own `shacl ` receipt lines to the same stream before the engine is reached — the
    // `shapes-provenance` line naming where these shapes came from — and a trip must not
    // suppress the answer to "which shapes were we validating against when the budget ran
    // out?", which is the first thing an operator needs in order to re-run with a bigger
    // budget.
    assert!(
        receipt
            .lines()
            .any(|line| line == "purrdf-governor-report 1"),
        "the shared governor-report banner:\n{receipt}"
    );
    assert!(
        receipt.starts_with("shacl shapes-provenance parsed\npurrdf-governor-report 1\n"),
        "the only thing ahead of the banner here is the provenance receipt:\n{receipt}"
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
    assert_eq!(stdout(&data_failure), "");
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

/// `--base` with an N-TRIPLES data graph is refused for the reason a pack one is: the data
/// PARSE is the only leg it has here (the shapes graph resolves against its own retrieval
/// IRI, and the report is serialized with no base), and N-Triples' grammar admits no
/// relative IRI reference. Naming `--format turtle`, which CAN write a base, changes
/// nothing: `validate` never hands the report writer this base.
#[test]
fn base_with_a_relative_incapable_data_graph_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data = write_file(
        dir.path(),
        "data.nt",
        "<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
         <http://example.org/Person> .\n",
    );

    for format in ["ntriples", "turtle"] {
        let out = run(&[
            "validate",
            "--shapes",
            &shapes,
            "--format",
            format,
            "--base",
            "http://example.org/",
            &data,
        ]);
        assert_eq!(code(&out), 2, "--format {format}: {}", stderr(&out));
        assert!(
            stderr(&out).contains("--base has no effect"),
            "--format {format}: the refusal names the flag: {}",
            stderr(&out)
        );
        assert!(
            stderr(&out).contains("the --from data graph"),
            "--format {format}: the refusal names the leg: {}",
            stderr(&out)
        );
    }
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
    assert_ne!(stderr(&out), "");
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

// ── The shapes graph's own base ────────────────────────────────────────────────
//
// A shapes graph is RDF, and its author may write `<PersonShape>` exactly as they would
// in any other Turtle document. The Turtle route used to reach a `parse_shapes` that took
// no base at all, so relative IRIs in a shapes graph could not resolve while the data
// graph in the same invocation resolved fine. These pin that the two routes agree.

/// A shapes graph whose node and property shapes are RELATIVE IRI references.
///
/// `<PersonShape>` and `<NamePropertyShape>` resolve against the shapes graph's base.
const RELATIVE_SHAPES: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "<PersonShape> a sh:NodeShape ;\n",
    "  sh:targetClass <http://example.org/Person> ;\n",
    "  sh:property <NamePropertyShape> .\n",
    "<NamePropertyShape> sh:path <http://example.org/name> ; sh:minCount 1 .\n",
);

/// One `ex:Person` with no `ex:name`: exactly one violation, but ONLY if the shape's
/// relative IRIs resolved and the target therefore selected a focus node.
const RELATIVE_SHAPES_DATA: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:alice a ex:Person .\n",
);

/// A Turtle shapes graph resolves its relative IRIs against its own `file://` retrieval IRI.
///
/// This is the reproduction: before the base reached `parse_shapes`, this exact command
/// failed outright with `iri-relative-no-base` because the Turtle fast path had no base
/// parameter to receive the derived retrieval IRI, while the identical bytes read as any
/// other syntax resolved through the shared seam.
#[test]
fn a_turtle_shapes_graph_resolves_relative_iris_against_its_retrieval_iri() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", RELATIVE_SHAPES);
    let data = write_file(dir.path(), "data.ttl", RELATIVE_SHAPES_DATA);

    let out = run(&["validate", "--shapes", &shapes, &data]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("shacl results 1\n"),
        "the resolved shape must select a focus node and find the violation: {}",
        stderr(&out)
    );

    // The source shape is named by its RESOLVED IRI — the shapes file's own `file://`
    // IRI with the last segment replaced — not by the bare `NamePropertyShape` token. The IRI
    // comes from the binary's OWN derivation (`purrdf_cli::file_retrieval_iri`), never a
    // second transcription of it in this harness: a local `format!("file://{path}")`
    // percent-encodes nothing and has no Windows answer at all, so it would agree with
    // itself while the binary emitted something else.
    let resolved =
        purrdf_cli::file_retrieval_iri(&shapes).expect("fixture has a file:// retrieval IRI");
    let resolved = resolved.replace("/shapes.ttl", "/NamePropertyShape");
    assert!(
        stdout(&out).contains(&format!(
            "<http://www.w3.org/ns/shacl#sourceShape> <{resolved}> ."
        )),
        "the source shape must be the resolved property shape IRI:\n{}",
        stdout(&out)
    );
    assert!(
        !stdout(&out).contains("<NamePropertyShape>"),
        "a bare relative reference must never reach the report:\n{}",
        stdout(&out)
    );
}

/// The Turtle route and the native-codec route resolve the SAME bytes identically.
///
/// The two arms differ only in the SHACL-AF prefix fallback, which this fixture does not
/// use, so their reports must be byte-for-byte equal. They were not: Turtle hard-failed
/// while TriG resolved, over one file copied under two names.
#[test]
fn the_turtle_and_non_turtle_shapes_routes_resolve_a_relative_iri_identically() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", RELATIVE_SHAPES_DATA);
    let as_turtle = write_file(dir.path(), "shapes.ttl", RELATIVE_SHAPES);
    let as_trig = write_file(dir.path(), "shapes.trig", RELATIVE_SHAPES);

    let turtle = run(&["validate", "--shapes", &as_turtle, &data]);
    let trig = run(&["validate", "--shapes", &as_trig, &data]);
    assert_eq!(code(&turtle), 0, "{}", stderr(&turtle));
    assert_eq!(code(&trig), 0, "{}", stderr(&trig));

    // The relative property shape resolves to the same sibling IRI for both files.
    let resolved =
        purrdf_cli::file_retrieval_iri(&as_turtle).expect("fixture has a file:// retrieval IRI");
    let resolved = resolved.replace("/shapes.ttl", "/NamePropertyShape");
    assert!(
        stdout(&turtle).contains(&format!(
            "<http://www.w3.org/ns/shacl#sourceShape> <{resolved}> ."
        )),
        "the report must identify the resolved property shape:\n{}",
        stdout(&turtle)
    );
    assert_eq!(
        stdout(&turtle),
        stdout(&trig),
        "the two shapes routes must produce the identical report"
    );
    assert_eq!(stderr(&turtle), stderr(&trig), "and the identical verdict");
}

/// A shapes graph on stdin has no retrieval IRI, so a relative IRI there is REFUSED.
///
/// The negative control, and the one that matters most: with no base derivable, the shape
/// cannot be resolved, no target selects a focus node, and a validator that carried on
/// would emit `conforms true` over a constraint it never evaluated. It must refuse
/// instead, and the refusal must name the condition.
#[test]
fn a_shapes_graph_on_stdin_with_a_relative_iri_is_refused_not_vacuously_passed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", RELATIVE_SHAPES_DATA);

    let out = pipe(
        &[
            "validate",
            "--shapes",
            "-",
            "--shapes-from",
            "turtle",
            &data,
        ],
        RELATIVE_SHAPES,
    );
    assert_eq!(
        code(&out),
        1,
        "a shapes graph that cannot resolve must fail: {}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains("iri-relative-no-base"),
        "the refusal carries the actionable code: {}",
        stderr(&out)
    );
    assert!(
        !stderr(&out).contains("shacl conforms"),
        "no conformance verdict may be reported over an unresolved shape: {}",
        stderr(&out)
    );
    assert!(
        stdout(&out).trim().is_empty(),
        "nothing partial is emitted:\n{}",
        stdout(&out)
    );
}

/// An in-document `@base` wins over the derived retrieval IRI, and works on stdin.
///
/// RFC-3986 5.1.1 puts the document's own base above the retrieval URI (5.1.3), so a
/// shapes graph that declares one is self-contained — which is what makes the stdin case
/// usable at all.
#[test]
fn an_at_base_in_the_shapes_graph_wins_over_the_retrieval_iri() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", RELATIVE_SHAPES_DATA);
    let based = format!("@base <http://example.org/shapes/> .\n{RELATIVE_SHAPES}");

    // From a FILE: the `@base` overrides the file's own retrieval IRI.
    let shapes = write_file(dir.path(), "shapes.ttl", &based);
    let from_file = run(&["validate", "--shapes", &shapes, &data]);
    assert_eq!(code(&from_file), 0, "{}", stderr(&from_file));
    assert!(
        stdout(&from_file).contains(concat!(
            "<http://www.w3.org/ns/shacl#sourceShape> ",
            "<http://example.org/shapes/NamePropertyShape> ."
        )),
        "the in-document base must win over the retrieval IRI:\n{}",
        stdout(&from_file)
    );
    assert!(
        !stdout(&from_file).contains("file://"),
        "the retrieval IRI must not appear once `@base` is declared:\n{}",
        stdout(&from_file)
    );

    // From STDIN, where there is no retrieval IRI at all, the same document resolves.
    let from_stdin = pipe(
        &[
            "validate",
            "--shapes",
            "-",
            "--shapes-from",
            "turtle",
            &data,
        ],
        &based,
    );
    assert_eq!(code(&from_stdin), 0, "{}", stderr(&from_stdin));
    assert!(
        stdout(&from_stdin).contains(concat!(
            "<http://www.w3.org/ns/shacl#sourceShape> ",
            "<http://example.org/shapes/NamePropertyShape> ."
        )),
        "a self-contained shapes graph needs no retrieval IRI:\n{}",
        stdout(&from_stdin)
    );
    assert_eq!(
        stdout(&from_file),
        stdout(&from_stdin),
        "file and stdin must produce the identical report under the document base"
    );
    assert_eq!(
        stderr(&from_file),
        stderr(&from_stdin),
        "and the identical verdict"
    );
}

/// A RELATIVE `--shapes-graph` resolves against the shapes document's own base — the base
/// the `sh:shapesGraph` it overrides would resolve against — rather than being interned
/// verbatim and refused deep inside the IR.
///
/// The load-bearing half is that the resolved graph is the one the SHACL-SPARQL constraint
/// reads: the shape below is satisfied only when the shapes graph is visible under exactly
/// the IRI the binary derived, so a resolution that landed anywhere else would fail here.
#[test]
fn a_relative_shapes_graph_resolves_against_the_shapes_document_base() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    // The retrieval IRI comes from the binary's OWN derivation, never a second
    // transcription of it in this harness — and it is derived from a path that already
    // exists, because the derivation canonicalizes.
    let shapes = write_file(dir.path(), "sg.ttl", SHAPES_GRAPH_SHAPES);
    let shapes_iri =
        purrdf_cli::file_retrieval_iri(&shapes).expect("fixture has a file:// retrieval IRI");
    let shapes_text = SHAPES_GRAPH_SHAPES.replace("http://example.org/shapes", &shapes_iri);
    let shapes = write_file(dir.path(), "sg.ttl", &shapes_text);

    // `sg.ttl` relative to the shapes document's own retrieval IRI IS that document.
    let out = run(&[
        "validate",
        "--shapes",
        &shapes,
        "--shapes-graph",
        "sg.ttl",
        &data,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("shacl conforms true\n"),
        "the relative flag must name the same graph `sh:shapesGraph <sg.ttl>` would: {}",
        stderr(&out)
    );
}

/// An ABSOLUTE `--shapes-graph` is carried lexical-verbatim, so the resolution above changes
/// nothing for an already-absolute invocation.
#[test]
fn an_absolute_shapes_graph_is_carried_verbatim() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "sg.ttl", SHAPES_GRAPH_SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&[
        "validate",
        "--shapes",
        &shapes,
        "--shapes-graph",
        "http://example.org/shapes",
        &data,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("shacl conforms true\n"),
        "an absolute graph name is unchanged by resolution: {}",
        stderr(&out)
    );
}

/// A relative `--shapes-graph` with NO base in scope is refused against the COMMAND LINE,
/// once, with a remedy the operator can actually apply.
///
/// Two defects are pinned here at the same time. The value used to travel into
/// `RdfDatasetBuilder::freeze`, which refused it as an un-internable IRI TERM and advised
/// adding an `@base`/`xml:base` DOCUMENT directive — a fix no document can apply to an argv
/// string — and which attached that same remedy BOTH inside the message and again as the
/// diagnostic's `detail`, so the sentence was printed twice in one line.
#[test]
fn a_relative_shapes_graph_with_no_base_names_the_command_line_once() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = pipe(
        &[
            "validate",
            "--shapes",
            "-",
            "--shapes-from",
            "turtle",
            "--shapes-graph",
            "sg",
            &data,
        ],
        SHAPES,
    );
    let err = stderr(&out);
    assert_eq!(code(&out), 2, "a malformed command line exits 2: {err}");
    assert!(
        err.contains("--shapes-graph `sg`") && err.contains("iri-relative-no-base"),
        "the refusal names the flag, its value and the shared code: {err}"
    );
    assert!(
        err.contains("command-line value"),
        "the remedy names the surface the value came from: {err}"
    );
    // The library's DOCUMENT remedy names `xml:base`; a `--shapes-graph` refusal must not,
    // because no RDF/XML document is involved and none could fix an argument.
    assert!(
        !err.contains("xml:base") && !err.contains("add a base to the document"),
        "a document directive is not the remedy for an argv value: {err}"
    );
    // …and whatever is said, it is said ONCE.
    assert_eq!(
        err.matches("iri-relative-no-base").count(),
        1,
        "the diagnostic is rendered once: {err}"
    );
    assert_eq!(
        err.matches("write the graph name in absolute form").count(),
        1,
        "the remedy is rendered once: {err}"
    );
    assert!(
        !err.contains("shacl conforms"),
        "no verdict may be reported for a request that named no graph: {err}"
    );
    assert!(
        stdout(&out).trim().is_empty(),
        "nothing partial is emitted:\n{}",
        stdout(&out)
    );
}

/// A MALFORMED `--shapes-graph` is a usage error naming the flag and the shared code, rather
/// than an IR-internment failure attributed to a term the operator never wrote.
#[test]
fn a_malformed_shapes_graph_is_a_named_usage_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "sg.ttl", SHAPES_GRAPH_SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&[
        "validate",
        "--shapes",
        &shapes,
        "--shapes-graph",
        "ht tp://example.org/shapes",
        &data,
    ]);
    let err = stderr(&out);
    assert_eq!(code(&out), 2, "a malformed command line exits 2: {err}");
    assert!(
        err.contains("--shapes-graph") && err.contains("iri-bad-scheme"),
        "the refusal names the flag and the shared code: {err}"
    );
    assert!(
        !err.contains("cannot be interned into the RDF IR"),
        "the value is refused at the command line, not at the IR boundary: {err}"
    );
}

// ── owl:imports in the shapes graph ──────────────────────────────────────────
//
// Jena's SHACL validator dereferences `owl:imports` over HTTP. PurRDF ships no HTTP client
// and must stay wasm32-clean, so the closure is caller-supplied — the same answer `entails
// --import` and `shex --import` give. An import whose ontology is already IN the shapes graph,
// or whose target the shapes graph describes with `sh:declare`, needs no pair; any other unresolved import is REFUSED, because a shapes graph whose shapes
// all live in an imported document would otherwise report `conforms true / results 0`
// against no shapes at all.

/// The W3C SHACL 1.2 core vocabulary, vendored beside the shapes engine.
const SHACL_TTL: &str = include_str!("../../shapes/spec/shacl.ttl");
/// The W3C SHACL 1.2 node-expression vocabulary; it `owl:imports <sh:>`.
const SHNEX_TTL: &str = include_str!("../../shapes/spec/shnex.ttl");
/// The W3C SHACL 1.2 SPARQL node-expression vocabulary; it `owl:imports <shnex:>`.
const SHNEX_SPARQL_TTL: &str = include_str!("../../shapes/spec/shnex-sparql.ttl");

/// A root shapes document that is nothing but an ontology header importing `shapes-a`.
const IMPORT_ROOT: &str = concat!(
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
    "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n",
    "<http://example.org/root> rdf:type owl:Ontology ;\n",
    "    owl:imports <http://example.org/shapes-a> .\n",
);

/// `shapes-a` carries a shape AND imports `shapes-b`, so the closure must be transitive.
const IMPORT_A: &str = concat!(
    "@prefix sh:   <http://www.w3.org/ns/shacl#> .\n",
    "@prefix owl:  <http://www.w3.org/2002/07/owl#> .\n",
    "@prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .\n",
    "@prefix ex:   <http://example.org/> .\n",
    "<http://example.org/shapes-a> owl:imports <http://example.org/shapes-b> .\n",
    "ex:AgeShape a sh:NodeShape ; sh:targetClass ex:Person ;\n",
    "    sh:property [ sh:path ex:age ; sh:datatype xsd:integer ;\n",
    "                  sh:message \"age must be an integer\" ] .\n",
);

/// `shapes-b` is reached ONLY through `shapes-a`; its shape firing proves transitivity.
const IMPORT_B: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <http://example.org/> .\n",
    "ex:NameShape a sh:NodeShape ; sh:targetClass ex:Person ;\n",
    "    sh:property [ sh:path ex:name ; sh:minCount 1 ;\n",
    "                  sh:message \"name is required\" ] .\n",
);

/// One `ex:Person` that violates BOTH imported shapes.
const IMPORT_DATA: &str = concat!(
    "@prefix ex:  <http://example.org/> .\n",
    "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n",
    "ex:alice rdf:type ex:Person ; ex:age \"not-a-number\" .\n",
);

/// `--import IRI=FILE` folds the shapes graph's TRANSITIVE `owl:imports` closure in, and the
/// shapes that only exist in imported documents decide the verdict.
#[test]
fn shapes_graph_imports_are_folded_transitively_from_the_import_table() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = write_file(dir.path(), "root.ttl", IMPORT_ROOT);
    let a = write_file(dir.path(), "a.ttl", IMPORT_A);
    let b = write_file(dir.path(), "b.ttl", IMPORT_B);
    let data = write_file(dir.path(), "data.ttl", IMPORT_DATA);

    let out = run(&[
        "validate",
        "--shapes",
        &root,
        "--import",
        &format!("http://example.org/shapes-a={a}"),
        "--import",
        &format!("http://example.org/shapes-b={b}"),
        &data,
    ]);
    let err = stderr(&out);
    assert_eq!(code(&out), 0, "a decided verdict exits 0: {err}");
    assert!(
        err.contains("shacl conforms false\n") && err.contains("shacl results 2\n"),
        "both imported shapes fire — the depth-2 one proves the closure is transitive, not \
         one level deep: {err}"
    );
    let report = stdout(&out);
    assert!(
        report.contains("age must be an integer") && report.contains("name is required"),
        "each imported shape's own message reaches the report: {report}"
    );
}

/// WITHOUT `--import`, an `owl:imports` whose ontology is not in the shapes graph is REFUSED
/// by name, with the pair that resolves it, and no verdict is written — validating the root
/// alone would decide `conforms true` against a shapes graph with no shapes in it.
#[test]
fn an_unresolved_shapes_import_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = write_file(dir.path(), "root.ttl", IMPORT_ROOT);
    let data = write_file(dir.path(), "data.ttl", IMPORT_DATA);

    let out = run(&["validate", "--shapes", &root, &data]);
    let err = stderr(&out);
    assert_eq!(
        code(&out),
        1,
        "an unresolved import is a runtime refusal: {err}"
    );
    assert!(
        err.contains("<http://example.org/shapes-a>"),
        "the refusal names the import it could not resolve: {err}"
    );
    assert!(
        err.contains("--import http://example.org/shapes-a=FILE"),
        "and the pair that resolves it: {err}"
    );
    assert!(
        !err.contains("shacl conforms"),
        "no verdict is decided over a shapes graph smaller than the one named: {err}"
    );
    assert!(stdout(&out).is_empty(), "and no report is written");
}

/// The W3C SHACL 1.2 vocabularies merged into ONE shapes document, beside a user shape:
/// `shnex.ttl` imports `sh:` and `shnex-sparql.ttl` imports `shnex:`, and both ontologies are
/// declared in the same document — so the closure is complete as written. It validates with
/// no `--import` and nothing on stderr but the receipt, and the user shape decides.
#[test]
fn merged_vocabulary_needs_no_import_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(
        dir.path(),
        "merged.ttl",
        &format!("{SHACL_TTL}\n{SHNEX_TTL}\n{SHNEX_SPARQL_TTL}\n{IMPORT_B}"),
    );
    let data = write_file(dir.path(), "data.ttl", IMPORT_DATA);

    let out = run(&["validate", "--shapes", &shapes, &data]);
    let err = stderr(&out);
    assert_eq!(code(&out), 0, "a complete closure is a decided run: {err}");
    assert!(
        err.contains("shacl conforms false\n") && err.contains("shacl results 1\n"),
        "the user shape beside the vocabulary fires: {err}"
    );
    assert!(
        !err.contains("warning") && !err.contains("owl:imports"),
        "an import the graph already holds is neither warned about nor required: {err}"
    );
}

/// The neighbour of [`merged_vocabulary_needs_no_import_flag`]: the same document WITHOUT
/// `shacl.ttl`, so `shnex.ttl`'s import of `sh:` names an ontology nothing declares. Exactly
/// that import is refused — `shnex:`, which `shnex-sparql.ttl` imports and `shnex.ttl`
/// declares, is still resolved in place and is not named.
#[test]
fn unresolved_import_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(
        dir.path(),
        "partial.ttl",
        &format!("{SHNEX_TTL}\n{SHNEX_SPARQL_TTL}\n{IMPORT_B}"),
    );
    let data = write_file(dir.path(), "data.ttl", IMPORT_DATA);

    let out = run(&["validate", "--shapes", &shapes, &data]);
    let err = stderr(&out);
    assert_eq!(
        code(&out),
        1,
        "an unresolved import is a runtime refusal: {err}"
    );
    assert!(
        err.contains("<http://www.w3.org/ns/shacl#>")
            && err.contains("--import http://www.w3.org/ns/shacl#=FILE"),
        "the refusal names the missing ontology and the pair that resolves it: {err}"
    );
    assert!(
        !err.contains("<http://www.w3.org/ns/shacl-node-expr#>"),
        "the import the graph already holds is not named: {err}"
    );
    assert!(stdout(&out).is_empty(), "no report is written");
}

/// The vendored W3C `sparql/node/prefixes-001` test: `ex:TestPrefixes owl:imports` the IRI
/// upstream publishes the test under, a node the document describes with `sh:declare` — SHACL's
/// `sh:prefixes/owl:imports*/sh:declare` path, reaching the `ex:` prefix declared there.
/// Nothing needs fetching: the import names a node the shapes graph holds.
const PREFIXES_001: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../vectors/shacl/sparql/node/prefixes-001.ttl"
);
/// The IRI the W3C suite publishes `prefixes-001` under — the document's own IRI.
const PREFIXES_001_IRI: &str = "http://datashapes.org/sh/tests/sparql/node/prefixes-001.test";

/// The W3C expected report for `prefixes-001`: one violation, on `ex:InvalidResource1`, whose
/// value is `test:Value` — reachable only if the `test:` and `ex:` prefixes the
/// `sh:prefixes/owl:imports*/sh:declare` path leads to were honoured by the `sh:select`.
fn assert_prefixes_001_verdict(out: &Output) {
    let err = stderr(out);
    assert_eq!(code(out), 0, "a decided verdict: {err}");
    assert!(
        err.contains("shacl conforms false\n") && err.contains("shacl results 1\n"),
        "the W3C verdict is non-conforming with exactly one result: {err}"
    );
    let report = stdout(out);
    assert!(
        report.contains(&format!(
            "<http://www.w3.org/ns/shacl#focusNode> <{PREFIXES_001_IRI}#InvalidResource1>"
        )),
        "the result is on ex:InvalidResource1: {report}"
    );
    assert!(
        report.contains("<http://www.w3.org/ns/shacl#value> <http://test.com/ns#Value>"),
        "with test:Value as its value, so the declared prefix resolved: {report}"
    );
    assert!(
        !report.contains(&format!(
            "<http://www.w3.org/ns/shacl#focusNode> <{PREFIXES_001_IRI}#ValidResource1>"
        )),
        "ex:ValidResource1 is not the focus node of any result: {report}"
    );
}

/// The W3C `prefixes-001` vector validates as written, with no `--import` and no
/// `--shapes-base`: its `owl:imports` target is the node the document itself describes with
/// `sh:declare` — SHACL's `sh:prefixes/owl:imports*/sh:declare` idiom — so the import is in
/// hand, and the run reaches the W3C verdict with the same file as data. `--shapes-base`
/// changes nothing about that, and `shacl pack` packs it as written too.
///
/// The neighbour: the same document with the target's `sh:declare` replaced by an
/// `rdfs:label` describes the target, but not as SHACL reads an import target, so the import
/// is refused by name — and the refusal names both remedies, `--shapes-base` and `--import`.
#[test]
fn the_w3c_prefix_idiom_needs_no_import_flag_and_a_labelled_target_is_refused() {
    assert_prefixes_001_verdict(&run(&["validate", "--shapes", PREFIXES_001, PREFIXES_001]));
    assert_prefixes_001_verdict(&run(&[
        "validate",
        "--shapes",
        PREFIXES_001,
        "--shapes-base",
        PREFIXES_001_IRI,
        PREFIXES_001,
    ]));

    let dir = tempfile::tempdir().expect("tempdir");
    let vector = std::fs::read_to_string(PREFIXES_001).expect("the vendored vector");
    let target = format!("<{PREFIXES_001_IRI}>\n  sh:declare [");
    assert_eq!(
        vector.matches(&target).count(),
        1,
        "the idiom's target, once"
    );
    let labelled = write_file(
        dir.path(),
        "prefixes-001-labelled.ttl",
        &vector.replace(
            &target,
            &format!("<{PREFIXES_001_IRI}>\n  rdfs:label \"prefixes-001\" .\n[]\n  sh:declare ["),
        ),
    );
    let refused = run(&["validate", "--shapes", &labelled, PREFIXES_001]);
    let err = stderr(&refused);
    assert_eq!(code(&refused), 1, "{err}");
    assert!(
        err.contains(&format!("<{PREFIXES_001_IRI}>")),
        "the refusal names the import: {err}"
    );
    assert!(
        err.contains(&format!("--shapes-base {PREFIXES_001_IRI}")),
        "and suggests reading the document under that IRI: {err}"
    );
    assert!(
        err.contains(&format!("--import {PREFIXES_001_IRI}=FILE")),
        "and the pair that resolves it: {err}"
    );
    assert!(stdout(&refused).is_empty(), "no report is written");

    // `shacl pack` packs the vector as written, and the product it writes reaches the same
    // verdict.
    let product = dir.path().join("prefixes-001.purrshp");
    let product_path = product.to_str().expect("utf8 path");
    let packed = run(&[
        "shacl",
        "pack",
        "--shapes",
        PREFIXES_001,
        "--out",
        product_path,
    ]);
    assert_eq!(code(&packed), 0, "{}", stderr(&packed));
    assert_prefixes_001_verdict(&run(&[
        "validate",
        "--shapes-product",
        product_path,
        PREFIXES_001,
    ]));
}

/// `--shapes-base` is the shapes document's PARSE base: a relative shape IRI resolves against
/// it, observed in the report's `sh:sourceShape`, while without it the same reference
/// resolves against the file's `file://` retrieval IRI. `--base` sets only the DATA graph's
/// base and leaves the shapes document alone. The flag is refused against `--shapes-product`,
/// which recorded its base when it was packed.
#[test]
fn shapes_base_is_the_shapes_documents_parse_base() {
    const RELATIVE_SHAPES: &str = concat!(
        "@prefix sh:  <http://www.w3.org/ns/shacl#> .\n",
        "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
        "@prefix ex:  <http://example.org/> .\n",
        "<#AgeShape> a sh:PropertyShape ; sh:targetClass ex:Person ;\n",
        "    sh:path ex:age ; sh:datatype xsd:integer .\n",
    );
    const SOURCE_SHAPE: &str = "<http://www.w3.org/ns/shacl#sourceShape>";
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "relative.ttl", RELATIVE_SHAPES);
    let data = write_file(dir.path(), "data.ttl", IMPORT_DATA);

    let based = run(&[
        "validate",
        "--shapes",
        &shapes,
        "--shapes-base",
        "http://example.org/shapes/doc",
        &data,
    ]);
    assert_eq!(code(&based), 0, "{}", stderr(&based));
    assert!(
        stdout(&based).contains(&format!(
            "{SOURCE_SHAPE} <http://example.org/shapes/doc#AgeShape>"
        )),
        "the relative shape IRI resolves against --shapes-base: {}",
        stdout(&based)
    );

    // An `@base` inside the document still wins inside it, as Turtle specifies.
    let declared = write_file(
        dir.path(),
        "declared.ttl",
        &format!("@base <http://example.org/declared/doc> .\n{RELATIVE_SHAPES}"),
    );
    let out = run(&[
        "validate",
        "--shapes",
        &declared,
        "--shapes-base",
        "http://example.org/shapes/doc",
        &data,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains(&format!(
            "{SOURCE_SHAPE} <http://example.org/declared/doc#AgeShape>"
        )),
        "the document's own @base wins: {}",
        stdout(&out)
    );

    for args in [
        vec!["validate", "--shapes", &shapes, &data],
        vec![
            "validate",
            "--shapes",
            &shapes,
            "--base",
            "http://example.org/shapes/doc",
            &data,
        ],
    ] {
        let out = run(&args);
        assert_eq!(code(&out), 0, "{args:?}: {}", stderr(&out));
        let report = stdout(&out);
        assert!(
            report.contains(&format!("{SOURCE_SHAPE} <file://"))
                && report.contains("relative.ttl#AgeShape>"),
            "{args:?}: without --shapes-base the shape resolves against the file: {report}"
        );
    }

    let product = dir.path().join("relative.purrshp");
    let product_path = product.to_str().expect("utf8 path");
    let packed = run(&["shacl", "pack", "--shapes", &shapes, "--out", product_path]);
    assert_eq!(code(&packed), 0, "{}", stderr(&packed));
    let refused = run(&[
        "validate",
        "--shapes-product",
        product_path,
        "--shapes-base",
        "http://example.org/shapes/doc",
        &data,
    ]);
    let err = stderr(&refused);
    assert_eq!(code(&refused), 2, "a usage error: {err}");
    assert!(
        err.contains("--shapes-base"),
        "the refusal names the flag: {err}"
    );
}

/// The imported vocabulary of [`EXTERNAL_IMPORT_SHAPES`]: the subclass axioms that make
/// `ex:ConstraintComponent` a constraint-component class and `ex:SPARQLAskValidator` an
/// ASK-validator class.
const EXTERNAL_IMPORT_VOCABULARY: &str = concat!(
    "@prefix ex: <http://example.org/ns#> .\n",
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
    "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "<http://example.org/validator-vocabulary> a owl:Ontology .\n",
    "ex:ConstraintComponent rdfs:subClassOf sh:ConstraintComponent .\n",
    "ex:SPARQLAskValidator rdfs:subClassOf sh:SPARQLAskValidator .\n",
);

/// A custom component with two parameters whose ASK validator flags every value that is
/// not their concatenation — the W3C `validator-001` mechanism, on `example.org` terms —
/// in a shapes graph that imports [`EXTERNAL_IMPORT_VOCABULARY`] by IRI. Its targets are
/// literals, so the file is its own data graph.
const EXTERNAL_IMPORT_SHAPES: &str = concat!(
    "@prefix ex: <http://example.org/ns#> .\n",
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
    "<http://example.org/validator-shapes> owl:imports <http://example.org/validator-vocabulary> .\n",
    "ex:TestConstraintComponent a ex:ConstraintComponent ;\n",
    "    sh:parameter ex:TestParameter1, ex:TestParameter2 ;\n",
    "    sh:validator [ a ex:SPARQLAskValidator ;\n",
    "        sh:ask \"ASK { FILTER (?value = CONCAT($test1, $test2)) }\" ] .\n",
    "ex:TestParameter1 a sh:Parameter ; sh:path ex:test1 ; sh:datatype xsd:string .\n",
    "ex:TestParameter2 a sh:Parameter ; sh:path ex:test2 ; sh:datatype xsd:string .\n",
    "ex:TestShape a sh:NodeShape ; ex:test1 \"Hello \" ; ex:test2 \"World\" ;\n",
    "    sh:targetNode \"Hallo Welt\", \"Hello World\" .\n",
);

/// A shapes graph that imports an ontology no document here holds is refused without
/// `--import`, naming the import and the flag that supplies it — and validates once a
/// document is named for it: one violation, on the focus node `"Hallo Welt"`.
#[test]
fn an_external_import_is_refused_until_a_document_is_named_for_it() {
    const VOCABULARY: &str = "http://example.org/validator-vocabulary";
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", EXTERNAL_IMPORT_SHAPES);

    let refused = run(&["validate", "--shapes", &shapes, &shapes]);
    let err = stderr(&refused);
    assert_eq!(code(&refused), 1, "{err}");
    assert!(
        err.contains(&format!("<{VOCABULARY}>"))
            && err.contains(&format!("--import {VOCABULARY}=FILE")),
        "the refusal names the external import and its pair: {err}"
    );
    assert!(stdout(&refused).is_empty(), "no report is written");

    let vocabulary = write_file(dir.path(), "vocabulary.ttl", EXTERNAL_IMPORT_VOCABULARY);
    let out = run(&[
        "validate",
        "--shapes",
        &shapes,
        "--import",
        &format!("{VOCABULARY}={vocabulary}"),
        &shapes,
    ]);
    let err = stderr(&out);
    assert_eq!(code(&out), 0, "a named document resolves the import: {err}");
    assert!(
        err.contains("shacl conforms false\n") && err.contains("shacl results 1\n"),
        "one violation: {err}"
    );
    assert!(
        stdout(&out).contains("<http://www.w3.org/ns/shacl#focusNode> \"Hallo Welt\""),
        "on the focus node \"Hallo Welt\": {}",
        stdout(&out)
    );
}

/// Naming ANY pair makes the closure mandatory: an import no pair resolves is refused by
/// name, and a pair the closure never reaches is refused as unused.
#[test]
fn a_named_import_table_must_resolve_the_whole_closure_and_be_fully_used() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = write_file(dir.path(), "root.ttl", IMPORT_ROOT);
    let a = write_file(dir.path(), "a.ttl", IMPORT_A);
    let b = write_file(dir.path(), "b.ttl", IMPORT_B);
    let data = write_file(dir.path(), "data.ttl", IMPORT_DATA);

    // Half a closure: `shapes-a` is supplied, the `shapes-b` it imports is not. Folding this
    // in would validate against a DIFFERENT shapes graph than the operator described.
    let partial = run(&[
        "validate",
        "--shapes",
        &root,
        "--import",
        &format!("http://example.org/shapes-a={a}"),
        &data,
    ]);
    let err = stderr(&partial);
    assert_eq!(code(&partial), 1, "a runtime refusal: {err}");
    assert!(
        err.contains("http://example.org/shapes-b") && err.contains("--import"),
        "the refusal names the unresolved IRI and the flag: {err}"
    );

    // A pair nothing imports would be read and never used.
    let unused = run(&[
        "validate",
        "--shapes",
        &root,
        "--import",
        &format!("http://example.org/shapes-a={a}"),
        "--import",
        &format!("http://example.org/shapes-b={b}"),
        "--import",
        &format!("http://example.org/nobody-imports-this={b}"),
        &data,
    ]);
    let err = stderr(&unused);
    assert_eq!(code(&unused), 2, "a usage error: {err}");
    assert!(
        err.contains("never reaches") && err.contains("nobody-imports-this"),
        "the refusal quotes the unused pair back: {err}"
    );

    // The degenerate form of the same fault: a pair against a shapes graph with no imports.
    let no_imports = run(&[
        "validate",
        "--shapes",
        &b,
        "--import",
        &format!("http://example.org/shapes-a={a}"),
        &data,
    ]);
    let err = stderr(&no_imports);
    assert_eq!(code(&no_imports), 2, "a usage error: {err}");
    assert!(
        err.contains("no owl:imports at all"),
        "the refusal says why the pair cannot be used: {err}"
    );
}

/// THE NEIGHBOURING VALID CASES. Every one of these must still SUCCEED.
///
/// Over-refusal is the mirror image of the silent drop the import refusal closes: an
/// `owl:imports` is only missing when the closure does not already hold the ontology it
/// names, and a graph that does hold it — by its `owl:Ontology` header or by an
/// `owl:versionIRI` — must validate with no `--import` at all.
#[test]
fn valid_shapes_graphs_are_not_refused_by_the_import_machinery() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", IMPORT_DATA);

    // 1. A shapes graph that imports an ontology it ALSO declares — by header, and by
    //    version IRI — and carries its own shape: the imports are resolved in place, the
    //    shape decides, and nothing is asked of the operator. The same document importing an
    //    ontology it does NOT declare is refused (`an_unresolved_shapes_import_is_refused_by_name`).
    let in_place = write_file(
        dir.path(),
        "in-place.ttl",
        &format!(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
             <http://example.org/in-place> owl:imports <http://example.org/present> ,\n\
                 <http://example.org/present/1.0> .\n\
             <http://example.org/present> a owl:Ontology ;\n\
                 owl:versionIRI <http://example.org/present/1.0> .\n{IMPORT_B}"
        ),
    );
    let out = run(&["validate", "--shapes", &in_place, &data]);
    let err = stderr(&out);
    assert_eq!(
        code(&out),
        0,
        "an import resolved in place is not a refusal: {err}"
    );
    assert!(
        err.contains("shacl results 1\n"),
        "the shapes graph's OWN shape still fires: {err}"
    );

    // 1b. A document importing its OWN IRI — by its in-document `@base`, and by its
    //     `file://` retrieval IRI through `<>` with no `@base` — names a document already
    //     loaded, from a node that is no `owl:Ontology`.
    for (name, header) in [
        (
            "self-base.ttl",
            "@base <http://example.org/shapes/self> .\n\
             <#Prefixes> <http://www.w3.org/2002/07/owl#imports> <> .\n",
        ),
        (
            "self-retrieval.ttl",
            "<#Prefixes> <http://www.w3.org/2002/07/owl#imports> <> .\n",
        ),
    ] {
        let own = write_file(dir.path(), name, &format!("{header}{IMPORT_B}"));
        let out = run(&["validate", "--shapes", &own, &data]);
        let err = stderr(&out);
        assert_eq!(
            code(&out),
            0,
            "{name}: a self-import is not a refusal: {err}"
        );
        assert!(err.contains("shacl results 1\n"), "{name}: {err}");
    }

    // 2. A shapes graph with no imports and no --import: byte-for-byte the pre-flag path.
    let plain = write_file(dir.path(), "plain.ttl", IMPORT_B);
    let out = run(&["validate", "--shapes", &plain, &data]);
    let err = stderr(&out);
    assert_eq!(code(&out), 0, "{err}");
    assert!(
        err.contains("shacl conforms false\n") && err.contains("shacl results 1\n"),
        "the overwhelmingly common invocation is unchanged: {err}"
    );

    // 3. An import CYCLE terminates and still folds. OWL 2 §3.4 defines the imports closure
    //    as the transitive one and explicitly permits `A` to import `B` to import `A`, so
    //    refusing a cycle would refuse an ontology the specification allows.
    let cyc_root = write_file(
        dir.path(),
        "cyc-root.ttl",
        concat!(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
            "<http://example.org/cyc-root> owl:imports <http://example.org/cyc-a> .\n",
        ),
    );
    let cyc_a = write_file(
        dir.path(),
        "cyc-a.ttl",
        &format!(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
             <http://example.org/cyc-a> owl:imports <http://example.org/cyc-b> .\n{IMPORT_B}"
        ),
    );
    let cyc_b = write_file(
        dir.path(),
        "cyc-b.ttl",
        concat!(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
            "<http://example.org/cyc-b> owl:imports <http://example.org/cyc-a> .\n",
        ),
    );
    let out = run(&[
        "validate",
        "--shapes",
        &cyc_root,
        "--import",
        &format!("http://example.org/cyc-a={cyc_a}"),
        "--import",
        &format!("http://example.org/cyc-b={cyc_b}"),
        &data,
    ]);
    let err = stderr(&out);
    assert_eq!(
        code(&out),
        0,
        "a cycle terminates rather than refusing: {err}"
    );
    assert!(
        err.contains("shacl results 1\n"),
        "and the shape reached through the cycle still fires: {err}"
    );
}

/// An IMPORTED document's own `@prefix` declarations reach its SHACL-SPARQL queries, and its
/// `sh:message` templates are substituted per solution, end to end.
///
/// The frozen IR does not retain a document's prefix map, so the fold has to carry every
/// document's prefixes forward — not just the root's. `xsd:` here is declared ONLY in the
/// imported document, and the `sh:select` that uses it is in that same document.
#[test]
fn an_imported_documents_prefixes_and_message_templates_survive_the_fold() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = write_file(
        dir.path(),
        "sp-root.ttl",
        concat!(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
            "<http://example.org/sp-root> owl:imports <http://example.org/sparql-a> .\n",
        ),
    );
    let imported = write_file(
        dir.path(),
        "sparql-a.ttl",
        concat!(
            "@prefix sh:  <http://www.w3.org/ns/shacl#> .\n",
            "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
            "@prefix ex:  <http://example.org/> .\n",
            "ex:LiteralTypeShape a sh:NodeShape ; sh:targetClass ex:Person ;\n",
            "    sh:sparql [ a sh:SPARQLConstraint ;\n",
            "        sh:message \"Property {$path} has an untyped literal: '{$value}' on {$this}.\" ;\n",
            "        sh:select \"\"\"SELECT $this ?path ?value WHERE {\n",
            "            $this ?path ?value . FILTER(isLiteral(?value))\n",
            "            FILTER(DATATYPE(?value) = xsd:string) }\"\"\" ] .\n",
        ),
    );
    let data = write_file(
        dir.path(),
        "sp-data.ttl",
        concat!(
            "@prefix ex:  <http://example.org/> .\n",
            "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n",
            "ex:alice rdf:type ex:Person ; ex:nickname \"Ali\" .\n",
        ),
    );

    let out = run(&[
        "validate",
        "--shapes",
        &root,
        "--import",
        &format!("http://example.org/sparql-a={imported}"),
        &data,
    ]);
    let err = stderr(&out);
    assert_eq!(code(&out), 0, "{err}");
    assert!(
        err.contains("shacl results 1\n"),
        "the imported SHACL-SPARQL constraint ran, so its `xsd:` prefix resolved: {err}"
    );
    assert!(
        stdout(&out).contains(
            "\"Property http://example.org/nickname has an untyped literal: 'Ali' on \
             http://example.org/alice.\""
        ),
        "every template is substituted from the solution's own bindings: {}",
        stdout(&out)
    );
}

/// A malformed `--import` pair is a usage error naming the argument, decided before any file
/// is opened — never a silently skipped import.
#[test]
fn malformed_shapes_import_pairs_are_usage_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = write_file(dir.path(), "root.ttl", IMPORT_ROOT);
    let a = write_file(dir.path(), "a.ttl", IMPORT_A);
    let data = write_file(dir.path(), "data.ttl", IMPORT_DATA);

    for (pair, needle) in [
        ("notapair", "has no `=`"),
        ("http://example.org/x=", "both halves"),
        ("=file.ttl", "both halves"),
        ("rel/path=a.ttl", "iri-non-absolute-base"),
    ] {
        let spec = pair.replace("a.ttl", &a);
        let out = run(&["validate", "--shapes", &root, "--import", &spec, &data]);
        let err = stderr(&out);
        assert_eq!(code(&out), 2, "`{pair}` is a usage error: {err}");
        assert!(
            err.contains("--import") && err.contains(needle),
            "`{pair}` names the flag and says what is wrong: {err}"
        );
    }

    // One IRI resolves to one document; the second pair would be read and never used.
    let duplicate = run(&[
        "validate",
        "--shapes",
        &root,
        "--import",
        &format!("http://example.org/shapes-a={a}"),
        "--import",
        &format!("http://example.org/shapes-a={a}"),
        &data,
    ]);
    let err = stderr(&duplicate);
    assert_eq!(code(&duplicate), 2, "{err}");
    assert!(
        err.contains("named twice"),
        "the duplicate is refused by name: {err}"
    );
}

// ── The incremental change lane ─────────────────────────────────────────────────────

/// A conforming base: nothing here violates [`SHAPES`], so anything the incremental lane
/// reports came out of the CHANGE rather than out of the graph it started from.
const CHANGE_BASE: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:bob a ex:Person ; ex:age 42 .\n",
    "ex:carol a ex:Person ; ex:age 31 .\n",
);

/// One row added on top of [`CHANGE_BASE`], introducing one violation.
const CHANGE_ADDED: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:alice a ex:Person ; ex:age \"nope\" .\n",
);

/// [`CHANGE_BASE`] merged with [`CHANGE_ADDED`] — the graph a FULL validation of the
/// change's result reads, and the oracle the incremental report is compared against.
const CHANGE_MERGED: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:bob a ex:Person ; ex:age 42 .\n",
    "ex:carol a ex:Person ; ex:age 31 .\n",
    "ex:alice a ex:Person ; ex:age \"nope\" .\n",
);

/// **The claim the whole lane rests on.** `--changes` re-validates only the focus nodes the
/// change can move, and the report it writes is byte-identical — content AND ordering — to a
/// full validation of the merged graph.
///
/// Byte-identical rather than "equivalent": a SHACL report is a deterministic artifact, blank
/// labels and all, and a lane that reached the same verdict through a differently-ordered
/// report would still have changed what this command emits.
#[test]
fn the_change_lane_reports_exactly_what_a_full_validation_of_the_merged_graph_does() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let base = write_file(dir.path(), "base.ttl", CHANGE_BASE);
    let added = write_file(dir.path(), "added.ttl", CHANGE_ADDED);
    let merged = write_file(dir.path(), "merged.ttl", CHANGE_MERGED);

    let incremental = run(&["validate", "--shapes", &shapes, "--changes", &added, &base]);
    let full = run(&["validate", "--shapes", &shapes, &merged]);

    let err = stderr(&incremental);
    assert_eq!(code(&incremental), 0, "{err}");
    assert_eq!(code(&full), 0, "{}", stderr(&full));
    assert!(
        err.contains("shacl change-expansion bounded 1\n"),
        "the expansion is BOUNDED and names its size, so the scope of the verdict below it is \
         never guessed at: {err}"
    );
    assert_eq!(
        stdout(&incremental),
        stdout(&full),
        "the incremental report must be the full report, byte for byte"
    );
    assert!(
        err.contains("shacl conforms false\n") && err.contains("shacl results 1\n"),
        "the added row's violation is found: {err}"
    );
}

/// The retract half is a real half: a row LEAVING the graph moves a verdict exactly as a row
/// joining it does, and the same identity against a full validation holds.
#[test]
fn removing_a_row_moves_a_verdict_and_reports_what_a_full_validation_would() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let base = write_file(dir.path(), "base.ttl", CHANGE_BASE);
    let retracted = write_file(
        dir.path(),
        "retracted.ttl",
        "@prefix ex: <http://example.org/> .\nex:bob ex:age 42 .\n",
    );
    // `ex:bob` keeps its type and loses its age, which is what `sh:datatype` no longer has
    // anything to say about — so this pair proves the lane tracks a removal, not that a
    // removal happens to violate something.
    let remaining = write_file(
        dir.path(),
        "remaining.ttl",
        "@prefix ex: <http://example.org/> .\nex:bob a ex:Person .\nex:carol a ex:Person ; \
         ex:age 31 .\n",
    );

    let incremental = run(&[
        "validate",
        "--shapes",
        &shapes,
        "--changes-removed",
        &retracted,
        &base,
    ]);
    let full = run(&["validate", "--shapes", &shapes, &remaining]);
    let err = stderr(&incremental);
    assert_eq!(code(&incremental), 0, "{err}");
    assert!(
        err.contains("shacl change-expansion bounded "),
        "a removal expands too: {err}"
    );
    assert_eq!(
        stdout(&incremental),
        stdout(&full),
        "removing a row reports what validating the graph without it reports"
    );
}

/// A shapes graph whose reads hide inside SPARQL query text has NO bounded change footprint,
/// so the lane validates the merged graph in full rather than under-reporting — and says on
/// stderr that it did.
///
/// The fallback is the half of this lane that cannot be skipped: a short expansion and a
/// clean bill of health are the same report.
#[test]
fn an_unbounded_change_footprint_falls_back_to_a_full_validation_and_says_so() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SPARQL_SHAPES);
    let base = write_file(
        dir.path(),
        "base.ttl",
        "@prefix ex: <http://example.org/> .\nex:bob a ex:Person ; ex:name \"Bob\" .\n",
    );
    let added = write_file(
        dir.path(),
        "added.ttl",
        "@prefix ex: <http://example.org/> .\nex:alice a ex:Person .\n",
    );
    let merged = write_file(
        dir.path(),
        "merged.ttl",
        "@prefix ex: <http://example.org/> .\nex:bob a ex:Person ; ex:name \"Bob\" .\n\
         ex:alice a ex:Person .\n",
    );

    let incremental = run(&["validate", "--shapes", &shapes, "--changes", &added, &base]);
    let full = run(&["validate", "--shapes", &shapes, &merged]);
    let err = stderr(&incremental);
    assert_eq!(code(&incremental), 0, "{err}");
    assert!(
        err.contains("shacl change-expansion everything "),
        "a SHACL-SPARQL shapes graph has no bounded footprint, and the receipt names the \
         construct responsible: {err}"
    );
    assert_eq!(
        stdout(&incremental),
        stdout(&full),
        "the fallback is a FULL validation of the merged graph"
    );
    assert!(
        err.contains("shacl results 1\n"),
        "the fallback still finds the violation the change introduced: {err}"
    );
}

/// Naming neither half leaves this command exactly as it was: no expansion line, and the
/// report of a whole-graph validation.
#[test]
fn a_run_with_no_change_documents_is_unchanged() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&["validate", "--shapes", &shapes, &data]);
    let err = stderr(&out);
    assert_eq!(code(&out), 0, "{err}");
    assert!(
        !err.contains("change-expansion"),
        "a run that named no change set must not report an expansion it never made: {err}"
    );
    assert!(err.contains("shacl results 1\n"), "{err}");
}

/// The governors bound this lane too, and they bound it where there is something to bound:
/// the `Everything` fallback runs the shapes graph's SPARQL, so `--fuel 0` trips it, writes
/// no report and exits 3 — exactly as it does on the whole-graph route.
///
/// The neighbouring valid case runs beside it: the SAME command under an ample ceiling
/// produces the ungoverned report unchanged, so this pins a governor rather than a lane that
/// refuses everything.
#[test]
fn a_governor_trips_the_change_lane_and_an_ample_ceiling_leaves_it_alone() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SPARQL_SHAPES);
    let base = write_file(
        dir.path(),
        "base.ttl",
        "@prefix ex: <http://example.org/> .\nex:bob a ex:Person ; ex:name \"Bob\" .\n",
    );
    let added = write_file(
        dir.path(),
        "added.ttl",
        "@prefix ex: <http://example.org/> .\nex:alice a ex:Person .\n",
    );
    let args = ["validate", "--shapes", &shapes, "--changes", &added, &base];

    let tripped = run(&[args.as_slice(), &["--fuel", "0"]].concat());
    let err = stderr(&tripped);
    assert_eq!(code(&tripped), 3, "a tripped governor exits 3: {err}");
    assert!(
        stdout(&tripped).is_empty(),
        "a trip writes NO report: {}",
        stdout(&tripped)
    );
    assert!(
        err.contains("outcome budget-exhausted"),
        "the governor receipt reaches stderr: {err}"
    );

    let ample = run(&[args.as_slice(), &["--fuel", "100000000"]].concat());
    let ungoverned = run(args.as_slice());
    assert_eq!(code(&ample), 0, "{}", stderr(&ample));
    assert_eq!(
        stdout(&ample),
        stdout(&ungoverned),
        "an ample ceiling changes nothing about the answer"
    );
}

/// `--changes-from` labels a change document, and a run that named none is refused rather
/// than validated with a flag that silently did nothing. The neighbouring valid case — the
/// same flag over a change document whose extension carries no syntax — runs beside it.
#[test]
fn a_changes_format_with_no_change_document_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let base = write_file(dir.path(), "base.ttl", CHANGE_BASE);
    let added = write_file(dir.path(), "added.unknown", CHANGE_ADDED);

    let refused = run(&[
        "validate",
        "--shapes",
        &shapes,
        "--changes-from",
        "turtle",
        &base,
    ]);
    let err = stderr(&refused);
    assert_eq!(code(&refused), 2, "{err}");
    assert!(
        err.contains("--changes-from") && err.contains("neither half"),
        "the refusal names the flag and what is missing: {err}"
    );

    let accepted = run(&[
        "validate",
        "--shapes",
        &shapes,
        "--changes-from",
        "turtle",
        "--changes",
        &added,
        &base,
    ]);
    assert_eq!(code(&accepted), 0, "{}", stderr(&accepted));
    assert!(
        stderr(&accepted).contains("shacl change-expansion bounded 1\n"),
        "the override is what let an extension-less change document be read: {}",
        stderr(&accepted)
    );
}

/// A change document may read standard input — but only if nothing else does. Refused naming
/// every document that asked for it, with the neighbouring valid case (exactly one of them)
/// beside it.
#[test]
fn a_change_document_may_have_stdin_but_only_if_nothing_else_does() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let base = write_file(dir.path(), "base.ttl", CHANGE_BASE);

    let refused = pipe(
        &[
            "validate",
            "--shapes",
            &shapes,
            "--from",
            "turtle",
            "--changes",
            "-",
            "--changes-from",
            "turtle",
            "-",
        ],
        CHANGE_ADDED,
    );
    let err = stderr(&refused);
    assert_eq!(code(&refused), 2, "{err}");
    assert!(
        err.contains("IN and --changes"),
        "the refusal names both documents that asked for the one stdin: {err}"
    );

    let accepted = pipe(
        &[
            "validate",
            "--shapes",
            &shapes,
            "--changes",
            "-",
            "--changes-from",
            "turtle",
            &base,
        ],
        CHANGE_ADDED,
    );
    assert_eq!(code(&accepted), 0, "{}", stderr(&accepted));
    assert!(
        stderr(&accepted).contains("shacl change-expansion bounded 1\n"),
        "one stdin reader is fine: {}",
        stderr(&accepted)
    );
}

/// The W3C SHACL 1.2 vocabulary's declaration of the built-in `sh:SPARQLExprExpression`,
/// verbatim: a `sh:NamedParameterExpressionFunction` with the two `sh:Parameter`s
/// `-prefixes` and `-sparqlExpr` and no `sh:bodyExpression`.
const SPARQL_EXPR_DECLARATION: &str = r#"
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

sh:SPARQLExprExpression a sh:NamedParameterExpressionFunction ;
  rdfs:label "SPARQL expr expression"@en ;
  rdfs:comment "The class of node expressions based on SPARQL expressions (sh:sparqlExpr)."@en ;
  rdfs:isDefinedBy sh: ;
  rdfs:subClassOf sh:NamedParameterExpression,
  sh:SPARQLExecutable ;
  sh:parameter sh:SPARQLExprExpression-prefixes,
  sh:SPARQLExprExpression-sparqlExpr .

sh:SPARQLExprExpression-prefixes a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:description "The prefixes that shall be applied before parsing the SPARQL query that gets derived from the sh:sparqlExpr expression. The object should define those prefixes using sh:declare."@en ;
  sh:name "prefixes"@en ;
  sh:nodeKind sh:BlankNodeOrIRI ;
  sh:path sh:prefixes .

sh:SPARQLExprExpression-sparqlExpr a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:datatype xsd:string ;
  sh:description "The SPARQL expression that is executed during evaluation of this node expression."@en ;
  sh:keyParameter true ;
  sh:name "SPARQL expr"@en ;
  sh:path sh:sparqlExpr .
"#;

/// A Warning-graded property shape whose value nodes are computed by `sh:values [
/// sh:sparqlExpr "ex:active" ; sh:prefixes ex:Prefixes ]`: the one value node is
/// `ex:active`, which `sh:in ( ex:retired )` refuses at every `ex:Person`.
const SPARQL_EXPR_SHAPES: &str = r#"
@prefix ex: <http://example.org/> .
ex:Prefixes sh:declare [ sh:prefix "ex" ; sh:namespace "http://example.org/"^^xsd:anyURI ] .
ex:PersonShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [
    sh:path ex:status ;
    sh:values [ sh:sparqlExpr "ex:active" ; sh:prefixes ex:Prefixes ] ;
    sh:in ( ex:retired ) ;
    sh:severity sh:Warning
  ] .
"#;

/// The issue's reproducer on the command line: a shapes graph carrying the vocabulary's
/// own `sh:SPARQLExprExpression` declaration loads, and its `sh:sparqlExpr` +
/// `sh:prefixes` expression is evaluated natively — each result's `sh:value` is the
/// computed `<http://example.org/active>`, which only the prefix-expanded expression
/// yields. The Warning results do not conform under the default conformance-disallow set
/// and conform under `sh:Violation` alone.
#[test]
fn cli_validate_evaluates_sparql_expr_beside_its_vocabulary_declaration() {
    const VALUE: &str = "<http://www.w3.org/ns/shacl#value> <http://example.org/active>";
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(
        dir.path(),
        "sparql-expr.ttl",
        &format!("{SPARQL_EXPR_DECLARATION}{SPARQL_EXPR_SHAPES}"),
    );
    let data = write_file(dir.path(), "data.ttl", DATA);

    let default = run(&["validate", "--shapes", &shapes, &data]);
    assert_eq!(code(&default), 0, "{}", stderr(&default));
    assert!(
        stderr(&default).contains("shacl conforms false\n"),
        "{}",
        stderr(&default)
    );
    assert!(
        stderr(&default).contains("shacl results 2\n"),
        "{}",
        stderr(&default)
    );
    let report = stdout(&default);
    assert_eq!(report.matches(VALUE).count(), 2, "{report}");

    let relaxed = run(&[
        "validate",
        "--shapes",
        &shapes,
        "--conformance-disallows",
        "http://www.w3.org/ns/shacl#Violation",
        &data,
    ]);
    assert_eq!(code(&relaxed), 0, "{}", stderr(&relaxed));
    assert!(
        stderr(&relaxed).contains("shacl conforms true\n"),
        "{}",
        stderr(&relaxed)
    );
    assert_eq!(stdout(&relaxed).matches(VALUE).count(), 2);
}

/// `--conformance-disallows` is the conformance-disallow set the run is judged against, on
/// the parse route and the product route alike: a Warning-only graph does not conform under
/// SHACL's default set and conforms under `sh:Violation` alone, the report echoes the named
/// set, and a value that is not an IRI is a usage error rather than a silently default run.
#[test]
fn cli_validate_conformance_disallows() {
    const WARNING_SHAPES: &str = concat!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
        "@prefix ex: <http://example.org/> .\n",
        "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
        "ex:PersonShape a sh:NodeShape ;\n",
        "  sh:targetClass ex:Person ;\n",
        "  sh:property [ sh:path ex:age ; sh:datatype xsd:integer ; sh:severity sh:Warning ] .\n",
    );
    const ECHO: &str =
        "<http://www.w3.org/ns/shacl#conformanceDisallows> <http://www.w3.org/ns/shacl#Violation>";
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "warning.ttl", WARNING_SHAPES);
    let data = write_file(dir.path(), "data.ttl", DATA);

    let default = run(&["validate", "--shapes", &shapes, &data]);
    assert_eq!(code(&default), 0, "{}", stderr(&default));
    assert!(stderr(&default).contains("shacl conforms false\n"));
    assert!(stdout(&default).contains(&conforms_triple(false)));
    assert!(
        !stdout(&default).contains("conformanceDisallows"),
        "the default set is echoed by stating none:\n{}",
        stdout(&default)
    );

    let violation = "http://www.w3.org/ns/shacl#Violation";
    let relaxed = run(&[
        "validate",
        "--shapes",
        &shapes,
        "--conformance-disallows",
        violation,
        &data,
    ]);
    assert_eq!(code(&relaxed), 0, "{}", stderr(&relaxed));
    assert!(stderr(&relaxed).contains("shacl conforms true\n"));
    assert!(stderr(&relaxed).contains("shacl results 1\n"));
    let report = stdout(&relaxed);
    assert!(report.contains(&conforms_triple(true)), "{report}");
    assert!(report.contains(ECHO), "the named set is echoed:\n{report}");

    let product = dir.path().join("warning.purrshp");
    let product_path = product.to_str().expect("utf8 path");
    let packed = run(&["shacl", "pack", "--shapes", &shapes, "--out", product_path]);
    assert_eq!(code(&packed), 0, "{}", stderr(&packed));
    let restored = run(&[
        "validate",
        "--shapes-product",
        product_path,
        "--conformance-disallows",
        violation,
        &data,
    ]);
    assert_eq!(code(&restored), 0, "{}", stderr(&restored));
    assert!(stderr(&restored).contains("shacl conforms true\n"));
    assert!(stdout(&restored).contains(ECHO));

    let refused = run(&[
        "validate",
        "--shapes",
        &shapes,
        "--conformance-disallows",
        "Violation",
        &data,
    ]);
    assert_eq!(code(&refused), 2, "a usage error: {}", stderr(&refused));
    assert!(stderr(&refused).contains("--conformance-disallows"));
}
